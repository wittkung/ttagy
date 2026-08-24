//! 统一 Local AI 客户端入口

use std::path::PathBuf;
use futures_util::Stream;

pub use agy_core::{AgyRequest, AgyResponse, AgyStreamEvent};
use crate::fallback::FallbackDriver;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub socket_path: PathBuf,
    pub http_url: Option<String>,
    pub auto_fallback: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from("/tmp/local_ai_daemon.sock"),
            http_url: Some("http://127.0.0.1:8970".to_string()),
            auto_fallback: true,
        }
    }
}

pub struct AgyClient {
    config: ClientConfig,
}

impl AgyClient {
    pub fn new(config: ClientConfig) -> Self {
        Self { config }
    }

    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// 执行流式推导，优先尝试 Daemon IPC 通信，未启动或断开时平滑降级
    pub async fn stream_chat(
        &self,
        request: AgyRequest,
    ) -> Result<impl Stream<Item = Result<AgyStreamEvent, String>>, String> {
        // 探测本地 UDS Socket 是否存在且可连接
        let is_daemon_live = self.config.socket_path.exists();

        if is_daemon_live {
            // TODO: 连接 Daemon UDS 流
        }

        if self.config.auto_fallback {
            return FallbackDriver::stream_chat(request).await;
        }

        Err("Local AI Daemon 未运行且未开启 auto_fallback".to_string())
    }

    /// 一次性聚合推导
    pub async fn chat(&self, request: AgyRequest) -> Result<AgyResponse, String> {
        use futures_util::StreamExt;
        let start_time = std::time::Instant::now();
        let session_id = request.session_id.clone();
        let model = request.model.clone();
        let mut stream = self.stream_chat(request).await?;

        let mut final_content = String::new();
        let mut thinking_content = None;

        while let Some(ev_res) = stream.next().await {
            let ev = ev_res?;
            match ev {
                AgyStreamEvent::ThinkingDelta { text_delta, .. } => {
                    thinking_content.get_or_insert_with(String::new).push_str(&text_delta);
                }
                AgyStreamEvent::ContentDelta { text_delta, .. } => {
                    final_content.push_str(&text_delta);
                }
                AgyStreamEvent::Done { full_content, thinking_content: tc, .. } => {
                    final_content = full_content;
                    if tc.is_some() {
                        thinking_content = tc;
                    }
                }
                AgyStreamEvent::Error { error_message, .. } => {
                    return Err(error_message);
                }
                _ => {}
            }
        }

        Ok(AgyResponse {
            session_id,
            status: "success".to_string(),
            content: final_content,
            thinking_content,
            model,
            elapsed_ms: start_time.elapsed().as_secs_f64() * 1000.0,
            prompt_tokens: None,
            output_tokens: None,
            error_message: None,
        })
    }
}

#[derive(Default)]
pub struct ClientBuilder {
    config: ClientConfig,
}

impl ClientBuilder {
    pub fn socket_path(mut self, path: PathBuf) -> Self {
        self.config.socket_path = path;
        self
    }

    pub fn auto_fallback(mut self, enable: bool) -> Self {
        self.config.auto_fallback = enable;
        self
    }

    pub fn build(self) -> Result<AgyClient, String> {
        Ok(AgyClient::new(self.config))
    }
}
