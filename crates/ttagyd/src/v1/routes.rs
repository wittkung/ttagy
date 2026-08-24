//! V1 专有 RESTful 与 SSE 接口实现 (Frozen API Pipeline)

use axum::{
    extract::State,
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures_util::Stream;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use ttagy_core::{
    resolve_model_name,
    v1::{NdjsonParser, ParsedStreamItem, TtagyRequest, TtagyStreamEvent},
    SandboxGuard, StderrDrainer, TtagyDetector,
};

pub struct AppState {
    pub auth_token: Option<String>,
    pub max_concurrency: usize,
    pub semaphore: Arc<tokio::sync::Semaphore>,
}

/// 带有 Stream Drop 感知的流包装器 (客户端连接断开时立即触发 Cancel)
pub struct GuardedStream<S> {
    inner: S,
    cancel_token: CancellationToken,
}

impl<S> GuardedStream<S> {
    pub fn new(inner: S, cancel_token: CancellationToken) -> Self {
        Self { inner, cancel_token }
    }
}

impl<S: Stream + Unpin> Stream for GuardedStream<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

impl<S> Drop for GuardedStream<S> {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/stream", post(stream_handler))
        .with_state(state)
}

async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let available = TtagyDetector::is_available().await;
    let binary = TtagyDetector::find_binary().map(|p| p.to_string_lossy().to_string());
    let available_permits = state.semaphore.available_permits();
    Json(serde_json::json!({
        "status": if available { "ok" } else { "unavailable" },
        "version": "v1",
        "service": "ttagyd",
        "agy_binary": binary,
        "available": available,
        "max_concurrency": state.max_concurrency,
        "available_permits": available_permits
    }))
}

async fn stream_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TtagyRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, String)> {
    let permit = state
        .semaphore
        .clone()
        .try_acquire_owned()
        .map_err(|_| (StatusCode::TOO_MANY_REQUESTS, "Server concurrency limit reached".into()))?;

    let binary = TtagyDetector::find_binary()
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "Antigravity CLI (agy) not found on host".into()))?;

    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(128);
    let cancel_token = CancellationToken::new();
    let worker_cancel_token = cancel_token.clone();

    let session_id = payload.session_id.clone();
    let model = match resolve_model_name(payload.model.as_deref()) {
        Ok(m) => m,
        Err(err) => return Err((StatusCode::BAD_REQUEST, err.into())),
    };
    let effort = payload.effort.clone().unwrap_or_else(|| "low".into());
    let timeout_secs = payload.timeout_secs.max(15);

    tokio::spawn(async move {
        // _permit 与 sandbox 生命周期在当前任务闭包内
        let _permit = permit;
        let start_time = std::time::Instant::now();

        // 1. 发送初始化事件
        let init_event = TtagyStreamEvent::Init {
            session_id: session_id.clone(),
            model: model.clone(),
            effort: effort.clone(),
            backend_mode: "daemon_tcp".into(),
        };
        if let Ok(json_str) = serde_json::to_string(&init_event) {
            if tx.send(Ok(Event::default().data(json_str))).await.is_err() {
                return; // 客户端已立即断开
            }
        }

        // 2. 在独立隔离沙箱运行 agy
        let sandbox = match SandboxGuard::create("ttagyd_v1_worker", true) {
            Ok(s) => s,
            Err(e) => {
                let err_event = TtagyStreamEvent::Error {
                    session_id: session_id.clone(),
                    error_code: "SANDBOX_FAILED".into(),
                    error_message: format!("Failed to create isolated sandbox: {}", e),
                    is_retryable: true,
                };
                if let Ok(json_str) = serde_json::to_string(&err_event) {
                    let _ = tx.send(Ok(Event::default().data(json_str))).await;
                }
                return;
            }
        };

        let mut cmd = Command::new(binary);
        cmd.current_dir(&sandbox.sandbox_path)
            .arg("-p")
            .arg(&payload.prompt)
            .arg("--model")
            .arg(&model)
            .kill_on_drop(true);

        #[cfg(unix)]
        cmd.process_group(0);

        if !effort.is_empty() && effort != "none" {
            cmd.arg("--effort").arg(&effort);
        }

        if let Some(ref schema) = payload.json_schema {
            if !schema.is_empty() {
                cmd.arg("--json-schema").arg(schema);
            }
        }

        cmd.arg("--output-format")
            .arg("stream-json")
            .arg("--disable-slash-commands")
            .arg("--dangerously-skip-permissions")
            .arg("--log-file")
            .arg(&sandbox.log_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let err_event = TtagyStreamEvent::Error {
                    session_id: session_id.clone(),
                    error_code: "SPAWN_FAILED".into(),
                    error_message: format!("Failed to spawn agy worker: {}", e),
                    is_retryable: true,
                };
                if let Ok(json_str) = serde_json::to_string(&err_event) {
                    let _ = tx.send(Ok(Event::default().data(json_str))).await;
                }
                return;
            }
        };

        let stdout = match child.stdout.take() {
            Some(o) => o,
            None => {
                let _ = child.kill().await;
                return;
            }
        };

        // 启动 Stderr 异步非阻塞排空器 (64KB 环形缓冲，彻底杜绝 Pipe 死锁)
        let stderr_drainer = child.stderr.take().map(|err| StderrDrainer::spawn(err, 64 * 1024));

        let mut reader = BufReader::new(stdout).lines();
        let timeout_duration = Duration::from_secs(timeout_secs);
        let mut accumulated_text = String::new();
        let mut accumulated_thinking = String::new();

        loop {
            tokio::select! {
                biased;

                // 客户端主动断开 / Abort 取消触发
                _ = worker_cancel_token.cancelled() => {
                    let _ = child.kill().await;
                    break;
                }

                // 下游 Channel Receiver 被 Drop
                _ = tx.closed() => {
                    let _ = child.kill().await;
                    break;
                }

                // 行读取事件与超时控制
                line_res = tokio::time::timeout(timeout_duration, reader.next_line()) => {
                    match line_res {
                        Ok(Ok(Some(line))) => {
                            let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;
                            let items = NdjsonParser::parse_line_items(&line);

                            for item in items {
                                match item {
                                    ParsedStreamItem::ThinkingDelta(delta) => {
                                        accumulated_thinking.push_str(&delta);
                                        let ev = TtagyStreamEvent::ThinkingDelta {
                                            session_id: session_id.clone(),
                                            text_delta: delta,
                                            elapsed_ms,
                                        };
                                        if let Ok(json_str) = serde_json::to_string(&ev) {
                                            if tx.send(Ok(Event::default().data(json_str))).await.is_err() {
                                                let _ = child.kill().await;
                                                return;
                                            }
                                        }
                                    }
                                    ParsedStreamItem::ContentDelta(delta) => {
                                        accumulated_text.push_str(&delta);
                                        let ev = TtagyStreamEvent::ContentDelta {
                                            session_id: session_id.clone(),
                                            text_delta: delta,
                                            accumulated_chars: accumulated_text.chars().count(),
                                            elapsed_ms,
                                        };
                                        if let Ok(json_str) = serde_json::to_string(&ev) {
                                            if tx.send(Ok(Event::default().data(json_str))).await.is_err() {
                                                let _ = child.kill().await;
                                                return;
                                            }
                                        }
                                    }
                                    ParsedStreamItem::Done { content, thinking_content: tc, usage } => {
                                        if !content.is_empty() {
                                            accumulated_text = content;
                                        }
                                        let final_thinking = tc.or_else(|| {
                                            if accumulated_thinking.is_empty() { None } else { Some(accumulated_thinking.clone()) }
                                        });
                                        let ev = TtagyStreamEvent::Done {
                                            session_id: session_id.clone(),
                                            full_content: accumulated_text.clone(),
                                            thinking_content: final_thinking,
                                            elapsed_ms,
                                            prompt_tokens: usage.as_ref().and_then(|u| u.prompt_tokens),
                                            output_tokens: usage.as_ref().and_then(|u| u.output_tokens),
                                        };
                                        if let Ok(json_str) = serde_json::to_string(&ev) {
                                            let _ = tx.send(Ok(Event::default().data(json_str))).await;
                                        }
                                        let _ = child.kill().await;
                                        return;
                                    }
                                    ParsedStreamItem::Error { code, message } => {
                                        let ev = TtagyStreamEvent::Error {
                                            session_id: session_id.clone(),
                                            error_code: code,
                                            error_message: message,
                                            is_retryable: false,
                                        };
                                        if let Ok(json_str) = serde_json::to_string(&ev) {
                                            let _ = tx.send(Ok(Event::default().data(json_str))).await;
                                        }
                                        let _ = child.kill().await;
                                        return;
                                    }
                                }
                            }
                        }
                        Ok(Ok(None)) => {
                            // EOF
                            let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;
                            let stderr_logs = if let Some(ref d) = stderr_drainer {
                                d.get_logs().await
                            } else {
                                String::new()
                            };

                            if accumulated_text.is_empty() && !stderr_logs.trim().is_empty() {
                                let ev = TtagyStreamEvent::Error {
                                    session_id: session_id.clone(),
                                    error_code: "EXECUTION_EMPTY_OUTPUT".into(),
                                    error_message: format!("Worker produced empty stdout. Stderr: {}", stderr_logs),
                                    is_retryable: false,
                                };
                                if let Ok(json_str) = serde_json::to_string(&ev) {
                                    let _ = tx.send(Ok(Event::default().data(json_str))).await;
                                }
                            } else {
                                let ev = TtagyStreamEvent::Done {
                                    session_id: session_id.clone(),
                                    full_content: accumulated_text,
                                    thinking_content: if accumulated_thinking.is_empty() { None } else { Some(accumulated_thinking) },
                                    elapsed_ms,
                                    prompt_tokens: None,
                                    output_tokens: None,
                                };
                                if let Ok(json_str) = serde_json::to_string(&ev) {
                                    let _ = tx.send(Ok(Event::default().data(json_str))).await;
                                }
                            }
                            break;
                        }
                        Ok(Err(e)) => {
                            let _ = child.kill().await;
                            let err_event = TtagyStreamEvent::Error {
                                session_id: session_id.clone(),
                                error_code: "IO_ERROR".into(),
                                error_message: format!("Stream read IO error: {}", e),
                                is_retryable: false,
                            };
                            if let Ok(json_str) = serde_json::to_string(&err_event) {
                                let _ = tx.send(Ok(Event::default().data(json_str))).await;
                            }
                            break;
                        }
                        Err(_) => {
                            // Timeout
                            let stderr_logs = if let Some(ref d) = stderr_drainer {
                                d.get_logs().await
                            } else {
                                String::new()
                            };
                            let _ = child.kill().await;
                            let err_event = TtagyStreamEvent::Error {
                                session_id: session_id.clone(),
                                error_code: "TIMEOUT".into(),
                                error_message: format!(
                                    "Request timed out after {} seconds. Recent stderr: {}",
                                    timeout_secs,
                                    if stderr_logs.is_empty() { "None" } else { &stderr_logs }
                                ),
                                is_retryable: true,
                            };
                            if let Ok(json_str) = serde_json::to_string(&err_event) {
                                let _ = tx.send(Ok(Event::default().data(json_str))).await;
                            }
                            break;
                        }
                    }
                }
            }
        }
    });

    let receiver_stream = ReceiverStream::new(rx);
    let guarded_stream = GuardedStream::new(receiver_stream, cancel_token);
    Ok(Sse::new(guarded_stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}
