use ttagy_client::{ClientBuilder, TtagyRequest};

#[tokio::test]
async fn test_client_builder_and_fallback_config() {
    let client = ClientBuilder::default()
        .base_url("http://127.0.0.1:8970")
        .socket_path("/tmp/ttagy.sock")
        .auth_token("secret")
        .auto_fallback(true)
        .build()
        .expect("构建客户端成功");

    assert_eq!(client.config.base_url, Some("http://127.0.0.1:8970".to_string()));
    assert_eq!(client.config.auth_token, Some("secret".to_string()));
    assert!(client.config.auto_fallback);

    let req = TtagyRequest {
        prompt: "测试提示词".to_string(),
        model: Some("gemini-3.7-flash".to_string()),
        effort: Some("none".to_string()),
        ..Default::default()
    };

    assert_eq!(req.prompt, "测试提示词");
    assert_eq!(req.effort, Some("none".to_string()));
}
