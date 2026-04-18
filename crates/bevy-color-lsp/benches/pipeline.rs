use bevy_color_lsp::detectors::detect_all;
use bevy_color_lsp::document::{byte_ranges_to_lsp, Document, DocumentStore};
use bevy_color_lsp::parser::parse;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tower_lsp::lsp_types::Url;

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
                store.update(&uri, src.clone());
                store.colors_for(&uri)
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_parse,
    bench_detect,
    bench_full_pipeline,
    bench_cached_pipeline,
    bench_byte_ranges,
    bench_lsp_request_cycle,
);
criterion_main!(benches);
