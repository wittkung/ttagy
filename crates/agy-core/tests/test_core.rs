use agy_core::{NdjsonParser, ParsedChunk, SandboxGuard, AgyStreamEvent};

#[test]
fn test_ndjson_parsing() {
    let line1 = r#"{"type":"step_update","step_update":{"text_delta":"你好，我是 AI"}}"#;
    assert_eq!(
        NdjsonParser::parse_line(line1),
        ParsedChunk::ContentDelta("你好，我是 AI".to_string())
    );

    let line2 = r#"{"type":"step_update","step_update":{"thought_delta":"正在深度思考..."}}"#;
    assert_eq!(
        NdjsonParser::parse_line(line2),
        ParsedChunk::ThinkingDelta("正在深度思考...".to_string())
    );

    let line3 = r#"{"type":"result","result":{"content":"最终输出"}}"#;
    assert_eq!(
        NdjsonParser::parse_line(line3),
        ParsedChunk::Result("最终输出".to_string())
    );
}

#[test]
fn test_sandbox_lifecycle() {
    let guard = SandboxGuard::create("test_unit", true).expect("创建沙箱成功");
    assert!(guard.sandbox_path.exists());
    assert!(guard.log_path.to_string_lossy().contains("agy_execution.log"));
    let path = guard.sandbox_path.clone();
    drop(guard);
    assert!(!path.exists()); // 验证 auto_cleanup 自动清除
}

#[test]
fn test_stream_event_serialization() {
    let ev = AgyStreamEvent::ContentDelta {
        session_id: "s1".to_string(),
        text_delta: "hello".to_string(),
        accumulated_chars: 5,
        elapsed_ms: 12.5,
    };
    let json = serde_json::to_string(&ev).unwrap();
    assert!(json.contains("agy:content_delta"));
}
