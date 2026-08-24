use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

pub use ttagy_core::{TtagyRequest, TtagyResponse, TtagyStreamEvent};
use crate::fallback::FallbackDriver;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// 远程私有 Agent 节点地址，如 "http://127.0.0.1:8970"
    pub base_url: Option<String>,
    /// Unix Domain Socket 路径，如 "/tmp/ttagy.sock"
    pub socket_path: Option<PathBuf>,
    /// 安全 Bearer Token
    pub auth_token: Option<String>,
    /// 当远程节点不可达时是否自动回退至本地沙箱直调
    pub auto_fallback: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: None,
            socket_path: Some(PathBuf::from("/tmp/ttagy.sock")),
            auth_token: None,
            auto_fallback: true,
        }
    }
}

pub struct TtagyClient {
    pub config: ClientConfig,
    http_client: reqwest::Client,
}

impl TtagyClient {
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// 执行流式推导：优先尝试 TCP 远程节点或本地 UDS，失败时降级至本地沙箱 Worker
    pub async fn stream_chat(
        &self,
        request: TtagyRequest,
    ) -> Result<ReceiverStream<Result<TtagyStreamEvent, String>>, String> {
        // 1. 若配置了远程 TCP 节点，发起 HTTP/SSE 请求
        if let Some(ref base_url) = self.config.base_url {
            let url = format!("{}/api/v1/stream", base_url.trim_end_matches('/'));
            let mut req_builder = self.http_client.post(&url).json(&request);
            if let Some(ref token) = self.config.auth_token {
                req_builder = req_builder.header("Authorization", format!("Bearer {}", token));
            }

            match req_builder.send().await {
                Ok(resp) if resp.status().is_success() => {
                    let (tx, rx) = mpsc::channel(64);
                    let mut stream = resp.bytes_stream();

                    tokio::spawn(async move {
                        let mut buffer = String::new();
                        while let Some(chunk_res) = stream.next().await {
                            match chunk_res {
                                Ok(bytes) => {
                                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                                    let mut lines: Vec<String> = buffer.split('\n').map(|s| s.to_string()).collect();
                                    buffer = lines.pop().unwrap_or_default();

                                    for line in lines {
                                        let trimmed = line.trim();
                                        if trimmed.starts_with("data:") {
                                            let json_str = trimmed.trim_start_matches("data:").trim();
                                            if !json_str.is_empty() {
                                                if let Ok(ev) = serde_json::from_str::<TtagyStreamEvent>(json_str) {
                                                    if tx.send(Ok(ev)).await.is_err() {
                                                        return;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(format!("读取远程 SSE 异常: {}", e))).await;
                                    break;
                                }
                            }
                        }
                    });

                    return Ok(ReceiverStream::new(rx));
                }
                Ok(resp) => {
                    if !self.config.auto_fallback {
                        return Err(format!("远程 Agent 节点返回错误状态: {}", resp.status()));
                    }
                }
                Err(e) => {
                    if !self.config.auto_fallback {
                        return Err(format!("连接远程 Agent 节点失败: {}", e));
                    }
                }
            }
        }

        // 2. 本地沙箱进程直调兜底
        if self.config.auto_fallback {
            return FallbackDriver::stream_chat(request).await;
        }

        Err("未配置可用远程节点且未启用 auto_fallback".to_string())
    }

    /// 一次性聚合推导
    pub async fn chat(&self, request: TtagyRequest) -> Result<TtagyResponse, String> {
        let start_time = std::time::Instant::now();
        let session_id = request.session_id.clone();
        let model = request.model.clone();
        let mut stream = self.stream_chat(request).await?;

        let mut final_content = String::new();
        let mut thinking_content = None;

        while let Some(ev_res) = stream.next().await {
            let ev = ev_res?;
            match ev {
                TtagyStreamEvent::ThinkingDelta { text_delta, .. } => {
                    thinking_content.get_or_insert_with(String::new).push_str(&text_delta);
                }
                TtagyStreamEvent::ContentDelta { text_delta, .. } => {
                    final_content.push_str(&text_delta);
                }
                TtagyStreamEvent::Done { full_content, thinking_content: tc, .. } => {
                    final_content = full_content;
                    if tc.is_some() {
                        thinking_content = tc;
                    }
                }
                TtagyStreamEvent::Error { error_message, .. } => {
                    return Err(error_message);
                }
                _ => {}
            }
        }

        Ok(TtagyResponse {
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
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.config.base_url = Some(url.into());
        self
    }

    pub fn socket_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.socket_path = Some(path.into());
        self
    }

    pub fn auth_token(mut self, token: impl Into<String>) -> Self {
        self.config.auth_token = Some(token.into());
        self
    }

    pub fn auto_fallback(mut self, enable: bool) -> Self {
        self.config.auto_fallback = enable;
        self
    }

    pub fn build(self) -> Result<TtagyClient, String> {
        Ok(TtagyClient::new(self.config))
    }
}
