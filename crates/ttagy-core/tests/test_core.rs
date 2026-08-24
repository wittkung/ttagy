use ttagy_core::{
    resolve_model_name, NdjsonParser, ParsedChunk, ParsedStreamItem, RollingBuffer, SandboxGuard,
    TtagyStreamEvent,
};

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
fn test_ndjson_composite_and_nested_items() {
    // 1. 同一行既有 thought 也有 text
    let composite_line = r#"{"type":"step_update","step_update":{"thought_delta":"思考中","text_delta":"输出中"}}"#;
    let items = NdjsonParser::parse_line_items(composite_line);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0], ParsedStreamItem::ThinkingDelta("思考中".to_string()));
    assert_eq!(items[1], ParsedStreamItem::ContentDelta("输出中".to_string()));

    // 2. 嵌套 message 结构与 Token Usage 提取
    let result_line = r#"{"type":"result","result":{"content":"完成","usage":{"prompt_tokens":120,"completion_tokens":45}}}"#;
    let result_items = NdjsonParser::parse_line_items(result_line);
    assert_eq!(result_items.len(), 1);
    if let ParsedStreamItem::Done { content, usage, .. } = &result_items[0] {
        assert_eq!(content, "完成");
        assert!(usage.is_some());
        let u = usage.as_ref().unwrap();
        assert_eq!(u.prompt_tokens, Some(120));
        assert_eq!(u.output_tokens, Some(45));
    } else {
        panic!("Expected ParsedStreamItem::Done");
    }
}

#[test]
fn test_rolling_buffer_flood_and_bounds() {
    let mut buf = RollingBuffer::new(64 * 1024); // 64KB
    // 注入 1MB 随机数据
    let large_data = vec![b'A'; 1024 * 1024];
    buf.push_bytes(&large_data);

    // 验证容量严格受限
    assert_eq!(buf.len(), 64 * 1024);
    assert_eq!(buf.total_dropped(), 1024 * 1024 - 64 * 1024);
    let str_repr = buf.to_string_lossy();
    assert!(str_repr.contains("截断前置"));
}

#[test]
fn test_model_resolution_and_passthrough() {
    // 别名解析
    assert_eq!(resolve_model_name(Some("default")).unwrap(), "gemini-3.7-flash");
    assert_eq!(resolve_model_name(Some("gemini")).unwrap(), "gemini-3.7-flash");
    assert_eq!(resolve_model_name(Some("sonnet")).unwrap(), "claude-sonnet-4-6");
    assert_eq!(resolve_model_name(Some("opus")).unwrap(), "claude-opus-4-6-thinking");

    // 自定义新模型透明透传（杜绝子串篡改）
    assert_eq!(resolve_model_name(Some("claude-3.7-sonnet")).unwrap(), "claude-3.7-sonnet");
    assert_eq!(resolve_model_name(Some("gemini-3.7-pro")).unwrap(), "gemini-3.7-pro");
    assert_eq!(resolve_model_name(Some("deepseek-r1:70b")).unwrap(), "deepseek-r1:70b");

    // 非法字符拦截
    assert!(resolve_model_name(Some("model; rm -rf /")).is_err());
}

#[test]
fn test_sandbox_lifecycle() {
    let guard = SandboxGuard::create("test_unit", true).expect("创建沙箱成功");
    assert!(guard.sandbox_path.exists());
    assert!(guard.log_path.to_string_lossy().contains("agy_execution.log"));
    let path = guard.sandbox_path.clone();
    drop(guard);
    assert!(!path.exists());
}

#[test]
fn test_stream_event_serialization() {
    let ev = TtagyStreamEvent::ContentDelta {
        session_id: "s1".to_string(),
        text_delta: "hello".to_string(),
        accumulated_chars: 5,
        elapsed_ms: 12.5,
    };
    let json = serde_json::to_string(&ev).unwrap();
    assert!(json.contains("agy:content_delta"));
}
