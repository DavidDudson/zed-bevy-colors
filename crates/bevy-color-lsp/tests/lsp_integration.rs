#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::redundant_closure_for_method_calls, // integration test closures are explicit for clarity
    clippy::uninlined_format_args, // test assertions are clear as-is
    clippy::cast_possible_truncation, // test-only: content_length parse; runtime correctness validated by test
    clippy::manual_assert,          // if n == 0 { panic!(...) } is explicit and clear in test context
    clippy::cargo_common_metadata,  // test binary, not published
    clippy::multiple_crate_versions, // transitive dep conflict we don't control
)]

use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout, Command};
use tokio::time::timeout;

const BIN: &str = env!("CARGO_BIN_EXE_bevy-color-lsp");

async fn write_msg(stdin: &mut ChildStdin, msg: &Value) {
    let body = serde_json::to_string(msg).unwrap();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    stdin.write_all(header.as_bytes()).await.unwrap();
    stdin.write_all(body.as_bytes()).await.unwrap();
    stdin.flush().await.unwrap();
}

async fn read_msg(reader: &mut BufReader<ChildStdout>) -> Value {
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await.unwrap();
        if n == 0 {
            panic!("eof while reading headers");
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = rest.trim().parse().unwrap();
        }
    }
    let mut buf = vec![0u8; content_length];
    reader.read_exact(&mut buf).await.unwrap();
    serde_json::from_slice(&buf).unwrap()
}

async fn read_until_response_id(reader: &mut BufReader<ChildStdout>, id: i64) -> Value {
    loop {
        let msg = timeout(Duration::from_secs(5), read_msg(reader))
            .await
            .expect("timeout waiting for message");
        if msg.get("id").and_then(|v| v.as_i64()) == Some(id) {
            return msg;
        }
    }
}

#[tokio::test]
async fn handshake_and_document_color() {
    let mut child = Command::new(BIN)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn lsp binary");

    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());

    write_msg(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": null,
                "capabilities": {}
            }
        }),
    )
    .await;
    let init_resp = read_until_response_id(&mut reader, 1).await;
    assert_eq!(init_resp["result"]["capabilities"]["colorProvider"], json!(true));

    write_msg(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }),
    )
    .await;

    let uri = "file:///fake.rs";
    let text = "fn x() { let c = Color::srgb(1.0, 0.5, 0.0); let w = Color::WHITE; }";
    write_msg(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "rust",
                    "version": 1,
                    "text": text
                }
            }
        }),
    )
    .await;

    write_msg(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/documentColor",
            "params": {
                "textDocument": { "uri": uri }
            }
        }),
    )
    .await;
    let resp = read_until_response_id(&mut reader, 2).await;
    let colors = resp["result"].as_array().expect("colors array");
    assert_eq!(colors.len(), 2, "expected srgb + WHITE colors, got {:?}", colors);

    let first = &colors[0];
    let red = first["color"]["red"].as_f64().unwrap();
    let green = first["color"]["green"].as_f64().unwrap();
    let blue = first["color"]["blue"].as_f64().unwrap();
    assert!((red - 1.0).abs() < 0.01);
    assert!((green - 0.5).abs() < 0.01);
    assert!(blue.abs() < 0.01);

    write_msg(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "shutdown",
            "params": null
        }),
    )
    .await;
    let _ = read_until_response_id(&mut reader, 3).await;
    write_msg(&mut stdin, &json!({"jsonrpc": "2.0", "method": "exit"})).await;

    let _ = timeout(Duration::from_secs(3), child.wait()).await;
}
