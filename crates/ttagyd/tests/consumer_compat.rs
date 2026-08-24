use ttagy_core::{TtagyRequest, TtagyStreamEvent};

#[test]
fn test_ttsubs_legacy_payload_compatibility() {
    let json_payload = r#"{
        "session_id": "ttsubs_1740000000000",
        "prompt": "请校对以下字幕：你好世界",
        "model": "gemini-3.7-flash",
        "effort": "high"
    }"#;

    let req: Result<TtagyRequest, _> = serde_json::from_str(json_payload);
    assert!(req.is_ok(), "TtagyRequest must parse TTSubs payload seamlessly");

    let req = req.unwrap();
    assert_eq!(req.session_id, "ttsubs_1740000000000");
    assert_eq!(req.prompt, "请校对以下字幕：你好世界");
    assert_eq!(req.model, Some("gemini-3.7-flash".to_string()));
    assert_eq!(req.effort, Some("high".to_string()));
    assert_eq!(req.timeout_secs, 60, "Default timeout should be 60s");
}

#[test]
fn test_stream_event_backward_compatibility() {
    let init_event = TtagyStreamEvent::Init {
        session_id: "s0".into(),
        model: "gemini-3.7-flash".into(),
        effort: "low".into(),
        backend_mode: "daemon_uds".into(),
    };
    let json = serde_json::to_string(&init_event).unwrap();
    assert!(json.contains(r#""backend_mode":"daemon_uds""#));

    let content_delta = TtagyStreamEvent::ContentDelta {
        session_id: "s1".into(),
        text_delta: "Hello".into(),
        accumulated_chars: 5,
        elapsed_ms: 20.0,
    };
    let json = serde_json::to_string(&content_delta).unwrap();
    assert!(json.contains(r#""type":"agy:content_delta""#));

    let thinking_delta = TtagyStreamEvent::ThinkingDelta {
        session_id: "s1".into(),
        text_delta: "Let me think...".into(),
        elapsed_ms: 10.0,
    };
    let json = serde_json::to_string(&thinking_delta).unwrap();
    assert!(json.contains(r#""type":"agy:thinking_delta""#));

    let done = TtagyStreamEvent::Done {
        session_id: "s1".into(),
        full_content: "Hello World".into(),
        thinking_content: None,
        elapsed_ms: 125.0,
        prompt_tokens: Some(10),
        output_tokens: Some(2),
    };
    let json = serde_json::to_string(&done).unwrap();
    assert!(json.contains(r#""type":"agy:done""#));

    let err = TtagyStreamEvent::Error {
        session_id: "s1".into(),
        error_code: "TIMEOUT".into(),
        error_message: "timeout".into(),
        is_retryable: true,
    };
    let json = serde_json::to_string(&err).unwrap();
    assert!(json.contains(r#""type":"agy:error""#));
}
