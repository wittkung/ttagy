//! TTAgy 混沌与操作系统边界压测套件 (Chaos & Boundary Stress Suite)

use futures_util::StreamExt;
use std::path::PathBuf;
use ttagy_client::{FallbackDriver, TtagyRequest, TtagyStreamEvent};

fn get_mock_agy_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target");
    path.push("debug");
    path.push("mock-agy");
    if !path.exists() {
        panic!("mock-agy binary not found at {}", path.display());
    }
    path
}

#[tokio::test]
async fn test_chaos_10mb_stderr_flood_no_deadlock() {
    let mock_path = get_mock_agy_path();
    std::env::set_var("AGY_PATH", mock_path.to_str().unwrap());

    let req = TtagyRequest {
        prompt: "scenario:stderr_flood".to_string(),
        model: Some("gemini-3.7-flash".to_string()),
        effort: Some("none".to_string()),
        timeout_secs: 10,
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let mut stream = FallbackDriver::stream_chat(req)
        .await
        .expect("启动 Fallback 驱动成功");

    let mut full_text = String::new();
    let mut got_done = false;

    while let Some(res) = stream.next().await {
        let ev = res.expect("读取事件无异常");
        match ev {
            TtagyStreamEvent::ContentDelta { text_delta, .. } => {
                full_text.push_str(&text_delta);
            }
            TtagyStreamEvent::Done { .. } => {
                got_done = true;
            }
            _ => {}
        }
    }

    assert!(got_done, "必须正常完成 Done 事件");
    assert!(full_text.contains("正常输出"), "必须提取到正常内容");
    assert!(
        start.elapsed().as_secs() < 5,
        "10MB Stderr 洪泛下必须在 5 秒内完成，无任何内核管道死锁"
    );
}

#[tokio::test]
async fn test_chaos_malformed_ndjson_recovery() {
    let mock_path = get_mock_agy_path();
    std::env::set_var("AGY_PATH", mock_path.to_str().unwrap());

    let req = TtagyRequest {
        prompt: "scenario:malformed_ndjson".to_string(),
        model: Some("gemini-3.7-flash".to_string()),
        effort: Some("none".to_string()),
        timeout_secs: 5,
        ..Default::default()
    };

    let mut stream = FallbackDriver::stream_chat(req).await.unwrap();
    let mut content = String::new();

    while let Some(res) = stream.next().await {
        if let Ok(TtagyStreamEvent::ContentDelta { text_delta, .. }) = res {
            content.push_str(&text_delta);
        }
    }

    assert_eq!(content, "有效内容片段 1。");
}

#[tokio::test]
async fn test_chaos_empty_output_stderr_diagnostics() {
    let mock_path = get_mock_agy_path();
    std::env::set_var("AGY_PATH", mock_path.to_str().unwrap());

    let req = TtagyRequest {
        prompt: "scenario:empty_output".to_string(),
        model: Some("gemini-3.7-flash".to_string()),
        effort: Some("none".to_string()),
        timeout_secs: 5,
        ..Default::default()
    };

    let mut stream = FallbackDriver::stream_chat(req).await.unwrap();
    let mut error_msg = String::new();

    while let Some(res) = stream.next().await {
        if let Ok(TtagyStreamEvent::Error { error_message, .. }) = res {
            error_msg = error_message;
        }
    }

    assert!(
        error_msg.contains("Fatal execution panic"),
        "必须从 Stderr 提取关键故障诊断信息，实际收到: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_chaos_abort_stream_immediate_kill() {
    let mock_path = get_mock_agy_path();
    std::env::set_var("AGY_PATH", mock_path.to_str().unwrap());

    let req = TtagyRequest {
        prompt: "scenario:abort_hang".to_string(),
        model: Some("gemini-3.7-flash".to_string()),
        effort: Some("none".to_string()),
        timeout_secs: 10,
        ..Default::default()
    };

    let stream = FallbackDriver::stream_chat(req).await.unwrap();
    drop(stream);

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
}
