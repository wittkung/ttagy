//! TTAgy 跨语言一致性契约测试套件 (Rust CTS Runner)

use futures_util::StreamExt;
use std::path::PathBuf;
use ttagy_client::{ClientBuilder, TtagyRequest, TtagyStreamEvent};

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
async fn test_cts_stream_normal() {
    let mock_path = get_mock_agy_path();
    std::env::set_var("AGY_PATH", mock_path.to_str().unwrap());

    let client = ClientBuilder::default()
        .auto_fallback(true)
        .build()
        .unwrap();

    let req = TtagyRequest {
        prompt: "scenario:stream_normal".to_string(),
        model: Some("gemini-3.7-flash".to_string()),
        effort: Some("low".to_string()),
        ..Default::default()
    };

    let mut stream = client.stream_chat(req).await.unwrap();
    let mut event_types = Vec::new();
    let mut final_content = String::new();

    while let Some(res) = stream.next().await {
        let ev = res.unwrap();
        match ev {
            TtagyStreamEvent::Init { .. } => event_types.push("agy:init"),
            TtagyStreamEvent::ThinkingDelta { .. } => event_types.push("agy:thinking_delta"),
            TtagyStreamEvent::ContentDelta { text_delta, .. } => {
                event_types.push("agy:content_delta");
                final_content.push_str(&text_delta);
            }
            TtagyStreamEvent::Done { full_content, .. } => {
                event_types.push("agy:done");
                final_content = full_content;
            }
            TtagyStreamEvent::Error { .. } => event_types.push("agy:error"),
        }
    }

    assert_eq!(
        event_types,
        vec![
            "agy:init",
            "agy:thinking_delta",
            "agy:content_delta",
            "agy:content_delta",
            "agy:done"
        ]
    );
    assert_eq!(final_content, "你好，我是 Antigravity AI 助手。很高兴为您服务！");
}

#[tokio::test]
async fn test_cts_quota_error() {
    let mock_path = get_mock_agy_path();
    std::env::set_var("AGY_PATH", mock_path.to_str().unwrap());

    let client = ClientBuilder::default()
        .auto_fallback(true)
        .build()
        .unwrap();

    let req = TtagyRequest {
        prompt: "scenario:quota_error".to_string(),
        model: Some("gemini-3.7-flash".to_string()),
        effort: Some("low".to_string()),
        ..Default::default()
    };

    let mut stream = client.stream_chat(req).await.unwrap();
    let mut err_code = String::new();
    let mut err_msg = String::new();

    while let Some(res) = stream.next().await {
        let ev = res.unwrap();
        if let TtagyStreamEvent::Error { error_code, error_message, .. } = ev {
            err_code = error_code;
            err_msg = error_message;
        }
    }

    assert_eq!(err_code, "CLI_ERROR");
    assert!(err_msg.contains("Resource quota exceeded"));
}
