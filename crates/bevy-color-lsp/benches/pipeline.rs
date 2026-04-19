#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::cast_precision_loss,     // bench: usize → u64 for throughput; acceptable in bench context
    clippy::cast_possible_truncation, // bench: line count fits in u32 for any realistic file size
    clippy::uninlined_format_args,   // synth_source format is clear as-is
    clippy::format_push_string,      // push_str(&format!(...)) in bench synth is simpler than write! with trait import
    clippy::cargo_common_metadata,   // bench binary, not published
    clippy::multiple_crate_versions, // transitive dep conflict we don't control
)]
// NOTE: `tracing::instrument` overhead is not benched here because it requires a build-flag
// comparison (--features tracing/release_max_level_off) rather than a criterion measurement.

use std::sync::Arc;

use bevy_color_lsp::{
    color::parse_hex,
    detectors::detect_all,
    document::{byte_ranges_to_lsp, byte_to_position, position_to_byte, Document, DocumentStore},
    named_colors::lookup_named,
    parser::parse,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tower_lsp::lsp_types::{Position, Range, Url};

const COLORS_PER_FN: usize = 6;

fn synth_source(num_fns: usize) -> String {
    let mut s = String::with_capacity(num_fns * 220);
    s.push_str("use bevy::prelude::*;\nuse bevy_color::palettes;\n\n");
    for i in 0..num_fns {
        s.push_str(&format!(
            r#"fn f{i}() {{
    let a = Color::srgb(0.{}, 0.5, 0.25);
    let b = Color::hsl(180.0, 0.5, 0.5);
    let c = Color::srgb_u8(200, 100, 50);
    let d = Color::WHITE;
    let e = Srgba::hex("ff8800");
    let g = palettes::tailwind::BLUE_500;
    let _ = (a, b, c, d, e, g);
}}

"#,
            i % 10
        ));
    }
    s
}

// Exercises `strip_rust_suffix` on every numeric arg (vs `synth_source`
// which uses bare `0.5` literals). Lets us measure the suffix-strip path.
fn synth_source_suffixed(num_fns: usize) -> String {
    let mut s = String::with_capacity(num_fns * 240);
    s.push_str("use bevy::prelude::*;\n\n");
    for i in 0..num_fns {
        s.push_str(&format!(
            r"fn f{i}() {{
    let a = Color::srgb(0.{}f32, 0.5f32, 0.25f32);
    let b = Color::hsla(180.0f32, 0.5f32, 0.5f32, 1.0f32);
    let c = Color::srgb_u8(200u8, 100u8, 50u8);
    let _ = (a, b, c);
}}

",
            i % 10
        ));
    }
    s
}

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");
    for &n in &[1usize, 10, 50, 200] {
        let src = synth_source(n);
        group.throughput(Throughput::Bytes(src.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &src, |b, src| {
            b.iter(|| parse(src).unwrap());
        });
    }
    group.finish();
}

fn bench_detect(c: &mut Criterion) {
    let mut group = c.benchmark_group("detect");
    for &n in &[1usize, 10, 50, 200] {
        let src = synth_source(n);
        let tree = parse(&src).unwrap();
        let expected = COLORS_PER_FN * n;
        group.throughput(Throughput::Elements(expected as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &src, |b, src| {
            b.iter(|| {
                let m = detect_all(&tree, src);
                assert!(m.len() >= expected, "got {} expected {}", m.len(), expected);
                m
            });
        });
    }
    group.finish();
}

fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline_cold");
    for &n in &[1usize, 10, 50, 200] {
        let src = synth_source(n);
        group.throughput(Throughput::Bytes(src.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &src, |b, src| {
            b.iter(|| {
                let mut doc = Document::new(src.clone());
                doc.colors()
            });
        });
    }
    group.finish();
}

fn bench_cached_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline_cached");
    for &n in &[1usize, 10, 50, 200] {
        let src = synth_source(n);
        let mut doc = Document::new(src.clone());
        let _ = doc.colors();
        group.bench_with_input(BenchmarkId::from_parameter(n), &src, |b, _| {
            b.iter(|| doc.colors());
        });
    }
    group.finish();
}

fn bench_byte_ranges(c: &mut Criterion) {
    let mut group = c.benchmark_group("byte_ranges_to_lsp");
    for &n in &[10usize, 100, 1000] {
        let src = synth_source(n);
        let tree = parse(&src).unwrap();
        let mut matches = detect_all(&tree, &src);
        matches.sort_by_key(|m| (m.start_byte, m.end_byte));
        group.throughput(Throughput::Elements(matches.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &matches, |b, ms| {
            b.iter(|| byte_ranges_to_lsp(&src, ms));
        });
    }
    group.finish();
}

fn bench_lsp_request_cycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("store_update_then_colors");
    for &n in &[10usize, 100, 500] {
        let src = synth_source(n);
        let store = DocumentStore::default();
        let uri = Url::parse("file:///bench.rs").unwrap();
        store.open(uri.clone(), src.clone());
        group.throughput(Throughput::Bytes(src.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &src, |b, src| {
            b.iter(|| {
                store.replace(&uri, src.clone());
                store.colors_for(&uri)
            });
        });
    }
    group.finish();
}

fn bench_incremental_keystroke(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_keystroke");
    for &n in &[10usize, 50, 200, 500] {
        let src = synth_source(n);
        group.throughput(Throughput::Bytes(src.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &src, |b, src| {
            let store = DocumentStore::default();
            let uri = Url::parse("file:///bench.rs").unwrap();
            store.open(uri.clone(), src.clone());
            let _ = store.colors_for(&uri);
            let mid_line = (src.matches('\n').count() / 2) as u32;
            let edit_range = Range {
                start: Position { line: mid_line, character: 0 },
                end: Position { line: mid_line, character: 0 },
            };
            b.iter(|| {
                store.apply_change(&uri, Some(edit_range), " ");
                store.colors_for(&uri)
            });
        });
    }
    group.finish();
}

fn bench_detect_suffixed(c: &mut Criterion) {
    let mut group = c.benchmark_group("detect_suffixed");
    for &n in &[50usize, 200] {
        let src = synth_source_suffixed(n);
        let tree = parse(&src).unwrap();
        group.throughput(Throughput::Bytes(src.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &src, |b, src| {
            b.iter(|| detect_all(&tree, src));
        });
    }
    group.finish();
}

fn bench_parse_hex(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_hex");
    let corpus = ["FF8800", "#abc", "abcd", "FF000080", "#A1B2C3D4", "xyz"];
    group.bench_function("mixed_corpus", |b| {
        b.iter(|| {
            let mut acc = 0_u32;
            for s in corpus {
                if let Some(c) = parse_hex(s) {
                    acc = acc.wrapping_add(c.r.to_bits());
                }
            }
            acc
        });
    });
    group.finish();
}

fn bench_palette_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("palette_lookup_named");
    // Mix of hits across three tables + one miss.
    let names = ["TOMATO", "BLUE_500", "RED", "MIDNIGHT_BLUE", "ROSE_950", "NOT_A_COLOR"];
    group.bench_function("mixed_corpus", |b| {
        b.iter(|| {
            let mut hit = 0_u32;
            for n in names {
                if lookup_named(n).is_some() {
                    hit += 1;
                }
            }
            hit
        });
    });
    group.finish();
}

fn bench_full_resync(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_resync_keystroke");
    for &n in &[10usize, 50, 200, 500] {
        let src = synth_source(n);
        group.throughput(Throughput::Bytes(src.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &src, |b, src| {
            let store = DocumentStore::default();
            let uri = Url::parse("file:///bench.rs").unwrap();
            store.open(uri.clone(), src.clone());
            let _ = store.colors_for(&uri);
            b.iter(|| {
                store.replace(&uri, src.clone());
                store.colors_for(&uri)
            });
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Helper: compute line starts from raw text (mirrors the private impl).
// ---------------------------------------------------------------------------
fn compute_line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

// ---------------------------------------------------------------------------
// Palette-heavy synth source: every argument is a palette or bevy const.
// Exercises `bevy_const` + `palette` detectors under dense load.
// ---------------------------------------------------------------------------
fn synth_source_palette_heavy(num_fns: usize) -> String {
    let mut s = String::with_capacity(num_fns * 260);
    s.push_str("use bevy::prelude::*;\nuse bevy_color::palettes;\n\n");
    for i in 0..num_fns {
        s.push_str(&format!(
            r"fn f{i}() {{
    let a = Color::WHITE;
    let b = Color::BLACK;
    let c = palettes::css::TOMATO;
    let d = palettes::css::MIDNIGHT_BLUE;
    let e = palettes::tailwind::BLUE_500;
    let f = palettes::tailwind::ROSE_950;
    let _ = (a, b, c, d, e, f);
}}

"
        ));
    }
    s
}

// ---------------------------------------------------------------------------
// No-color source: only plain `let x = 42;` statements — no color literals.
// ---------------------------------------------------------------------------
fn synth_source_no_color(num_fns: usize) -> String {
    let mut s = String::with_capacity(num_fns * 60);
    s.push_str("use bevy::prelude::*;\n\n");
    for i in 0..num_fns {
        s.push_str(&format!(
            r"fn f{i}() {{
    let x = {i};
    let y = {i} * 2;
}}

"
        ));
    }
    s
}

// ---------------------------------------------------------------------------
// UTF-8 multibyte synth source: emoji + CJK in comments/string literals.
// ---------------------------------------------------------------------------
fn synth_source_multibyte(num_fns: usize) -> String {
    let mut s = String::with_capacity(num_fns * 300);
    s.push_str("use bevy::prelude::*;\nuse bevy_color::palettes;\n\n");
    for i in 0..num_fns {
        s.push_str(&format!(
            r"// 颜色 palette 🎨 function {i}
fn f{i}() {{
    // 🎨 set color 颜色
    let a = Color::srgb(0.{}, 0.5, 0.25);
    let b = palettes::css::TOMATO;
    let _ = (a, b);
}}

",
            i % 10
        ));
    }
    s
}

// ---------------------------------------------------------------------------
// bench_position_conversions — hot-path UTF-16 ↔ byte offset math
// ---------------------------------------------------------------------------
fn bench_position_conversions(c: &mut Criterion) {
    let mut group = c.benchmark_group("position_conversions");

    for &n in &[10usize, 100, 1000] {
        let src = synth_source(n);
        let line_starts = compute_line_starts(&src);
        let total_lines = line_starts.len() as u32;
        let total_bytes = src.len();

        // Three probe points: start, middle, end.
        let positions = [
            ("start", Position { line: 0, character: 0 }),
            ("middle", Position { line: total_lines / 2, character: 5 }),
            ("end", Position { line: total_lines.saturating_sub(2), character: 1 }),
        ];
        let byte_offsets = [0usize, total_bytes / 2, total_bytes.saturating_sub(1)];

        group.throughput(Throughput::Bytes(src.len() as u64));

        // byte_to_position probes
        for (label, byte) in
            [("start", byte_offsets[0]), ("middle", byte_offsets[1]), ("end", byte_offsets[2])]
        {
            group.bench_with_input(
                BenchmarkId::new(format!("byte_to_position/{}", label), n),
                &(src.clone(), byte),
                |b, (src, byte)| {
                    b.iter(|| byte_to_position(src, *byte));
                },
            );
        }

        // position_to_byte probes
        for (label, pos) in &positions {
            group.bench_with_input(
                BenchmarkId::new(format!("position_to_byte/{}", label), n),
                &(src.clone(), line_starts.clone(), *pos),
                |b, (src, ls, pos)| {
                    b.iter(|| position_to_byte(src, ls, *pos));
                },
            );
        }
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// bench_concurrent_store — parking_lot contention under N worker threads.
//
// NOTE: this bench is stochastic. Variance is higher than single-thread
// benches because thread scheduling affects measured wall time. Treat results
// as order-of-magnitude indicators, not precise latency numbers.
// ---------------------------------------------------------------------------
fn bench_concurrent_store(c: &mut Criterion) {
    const OPS_PER_THREAD: usize = 100;

    let mut group = c.benchmark_group("concurrent_store");
    let src = synth_source(50);
    let uri = Arc::new(Url::parse("file:///bench_concurrent.rs").unwrap());

    for &num_threads in &[4usize, 8] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_threads),
            &num_threads,
            |b, &num_threads| {
                b.iter(|| {
                    let store = Arc::new(DocumentStore::default());
                    store.open((*uri).clone(), src.clone());

                    std::thread::scope(|scope| {
                        for t in 0..num_threads {
                            let store = Arc::clone(&store);
                            let uri = Arc::clone(&uri);
                            let src = src.clone();
                            scope.spawn(move || {
                                for op in 0..OPS_PER_THREAD {
                                    if (t + op) % 3 == 0 {
                                        // write-like: replace full text
                                        store.apply_change(&uri, None, &src);
                                    } else {
                                        // read-like: query colors
                                        let _ = store.colors_for(&uri);
                                    }
                                }
                            });
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// bench_palette_heavy_source — palette + bevy_const detectors under dense load
// ---------------------------------------------------------------------------
fn bench_palette_heavy_source(c: &mut Criterion) {
    let mut group = c.benchmark_group("palette_heavy_source");
    for &n in &[10usize, 50, 200] {
        let src = synth_source_palette_heavy(n);
        let tree = parse(&src).unwrap();
        group.throughput(Throughput::Bytes(src.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &src, |b, src| {
            b.iter(|| detect_all(&tree, src));
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// bench_empty_and_no_color — verify detector early-exits are cheap.
//
// Two data points in one group: truly empty source and a 200-fn source that
// contains no color literals whatsoever.
// ---------------------------------------------------------------------------
fn bench_empty_and_no_color(c: &mut Criterion) {
    let mut group = c.benchmark_group("empty_and_no_color");

    // Variant A: empty string
    {
        let src = String::new();
        let tree = parse(&src).unwrap();
        group.throughput(Throughput::Bytes(1)); // avoid zero to keep criterion happy
        group.bench_with_input(BenchmarkId::from_parameter("empty"), &src, |b, src| {
            b.iter(|| detect_all(&tree, src));
        });
    }

    // Variant B: 200 fns, zero color literals
    {
        let src = synth_source_no_color(200);
        let tree = parse(&src).unwrap();
        group.throughput(Throughput::Bytes(src.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter("no_color_200"), &src, |b, src| {
            b.iter(|| detect_all(&tree, src));
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// bench_large_source — 2000-fn (~512 KB) stress test for detect_all + colors().
// Reduced sample count to keep total bench time sane.
// ---------------------------------------------------------------------------
fn bench_large_source(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_source");
    group.sample_size(10);

    let src = synth_source(2000);
    group.throughput(Throughput::Bytes(src.len() as u64));

    group.bench_with_input(BenchmarkId::from_parameter(2000), &src, |b, src| {
        b.iter(|| {
            let mut doc = Document::new(src.clone());
            doc.colors()
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// bench_utf8_multibyte — detect_all + byte_ranges_to_lsp on emoji/CJK source
// ---------------------------------------------------------------------------
fn bench_utf8_multibyte(c: &mut Criterion) {
    let mut group = c.benchmark_group("utf8_multibyte");
    for &n in &[50usize, 200] {
        let src = synth_source_multibyte(n);
        let tree = parse(&src).unwrap();
        let matches = {
            let mut m = detect_all(&tree, &src);
            m.sort_by_key(|m| (m.start_byte, m.end_byte));
            m
        };
        group.throughput(Throughput::Bytes(src.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &src, |b, src| {
            b.iter(|| {
                let m = detect_all(&tree, src);
                byte_ranges_to_lsp(src, &m)
            });
        });
        // prevent unused-variable warning for matches used only to warm the variable
        let _ = matches.len();
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// bench_lsp_cycle_loop — full keystroke-by-keystroke LSP session simulation.
//
// Per iteration: open(full src), then 50 single-char apply_change events each
// followed by colors_for. Simulates a realistic editing session.
// ---------------------------------------------------------------------------
fn bench_lsp_cycle_loop(c: &mut Criterion) {
    const KEYSTROKES: usize = 50;

    let mut group = c.benchmark_group("lsp_cycle_loop");
    let src = synth_source(200);
    let uri = Url::parse("file:///bench_cycle.rs").unwrap();
    let mid_line = (src.matches('\n').count() / 2) as u32;
    let edit_range = Range {
        start: Position { line: mid_line, character: 0 },
        end: Position { line: mid_line, character: 0 },
    };

    group.throughput(Throughput::Elements(KEYSTROKES as u64));

    group.bench_function("200_fns", |b| {
        b.iter(|| {
            let store = DocumentStore::default();
            store.open(uri.clone(), src.clone());
            for _ in 0..KEYSTROKES {
                store.apply_change(&uri, Some(edit_range), " ");
                let _ = store.colors_for(&uri);
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse,
    bench_detect,
    bench_detect_suffixed,
    bench_parse_hex,
    bench_palette_lookup,
    bench_full_pipeline,
    bench_cached_pipeline,
    bench_byte_ranges,
    bench_lsp_request_cycle,
    bench_incremental_keystroke,
    bench_full_resync,
    bench_position_conversions,
    bench_concurrent_store,
    bench_palette_heavy_source,
    bench_empty_and_no_color,
    bench_large_source,
    bench_utf8_multibyte,
    bench_lsp_cycle_loop,
);
criterion_main!(benches);
