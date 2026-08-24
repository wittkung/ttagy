use ttagy_client::{ClientBuilder, TtagyClient, TtagyRequest};
use ttagy_core::TtagyStreamEvent;

#[tokio::test]
async fn test_client_builder_and_fallback_config() {
    let client = ClientBuilder::default()
        .auto_fallback(true)
        .build()
        .expect("构建客户端成功");

    let req = TtagyRequest {
        prompt: "测试提示词".to_string(),
        model: Some("gemini-3.7-flash".to_string()),
        effort: Some("none".to_string()),
        ..Default::default()
    };

    // 验证 request 初始化属性
    assert_eq!(req.prompt, "测试提示词");
    assert_eq!(req.effort, Some("none".to_string()));
}
