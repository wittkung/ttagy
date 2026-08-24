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
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use ttagy_core::{
    v1::{NdjsonParser, ParsedChunk, TtagyRequest, TtagyStreamEvent},
    SandboxGuard, TtagyDetector,
};

pub struct AppState {
    pub auth_token: Option<String>,
    pub max_concurrency: usize,
    pub semaphore: Arc<tokio::sync::Semaphore>,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/stream", post(stream_handler))
        .with_state(state)
}

async fn health_handler() -> impl IntoResponse {
    let available = TtagyDetector::is_available().await;
    let binary = TtagyDetector::find_binary().map(|p| p.to_string_lossy().to_string());
    Json(serde_json::json!({
        "status": if available { "ok" } else { "unavailable" },
        "version": "v1",
        "service": "ttagyd",
        "agy_binary": binary,
        "available": available
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
    let session_id = payload.session_id.clone();
    let model = payload.model.clone().unwrap_or_else(|| "gemini-3.7-flash".into());
    let effort = payload.effort.clone().unwrap_or_else(|| "high".into());
    let timeout_secs = payload.timeout_secs;

    tokio::spawn(async move {
        let _permit = permit;
        let start_time = std::time::Instant::now();

        // 1. 发送初始化事件
        let init_event = TtagyStreamEvent::Init {
            session_id: session_id.clone(),
            model: model.clone(),
            effort: effort.clone(),
            backend_mode: "remote_daemon_v1".into(),
        };
        if let Ok(json_str) = serde_json::to_string(&init_event) {
            let _ = tx.send(Ok(Event::default().data(json_str))).await;
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
            .arg(&model);

        if !effort.is_empty() && effort != "none" && (model.contains("3.7") || model.contains("pro")) {
            cmd.arg("--effort").arg(&effort);
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

        let stdout = child.stdout.take();
        if let Some(out) = stdout {
            let mut reader = BufReader::new(out).lines();
            let timeout_duration = Duration::from_secs(timeout_secs);
            let mut accumulated_text = String::new();
            let mut accumulated_thinking = String::new();

            loop {
                let next_line_future = reader.next_line();
                tokio::pin!(next_line_future);

                match tokio::time::timeout(timeout_duration, &mut next_line_future).await {
                    Ok(Ok(Some(line))) => {
                        let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;
                        match NdjsonParser::parse_line(&line) {
                            ParsedChunk::ThinkingDelta(delta) => {
                                accumulated_thinking.push_str(&delta);
                                let ev = TtagyStreamEvent::ThinkingDelta {
                                    session_id: session_id.clone(),
                                    text_delta: delta,
                                    elapsed_ms,
                                };
                                if let Ok(json_str) = serde_json::to_string(&ev) {
                                    let _ = tx.send(Ok(Event::default().data(json_str))).await;
                                }
                            }
                            ParsedChunk::ContentDelta(delta) => {
                                accumulated_text.push_str(&delta);
                                let ev = TtagyStreamEvent::ContentDelta {
                                    session_id: session_id.clone(),
                                    text_delta: delta,
                                    accumulated_chars: accumulated_text.chars().count(),
                                    elapsed_ms,
                                };
                                if let Ok(json_str) = serde_json::to_string(&ev) {
                                    let _ = tx.send(Ok(Event::default().data(json_str))).await;
                                }
                            }
                            ParsedChunk::Result(res) => {
                                if accumulated_text.is_empty() {
                                    accumulated_text = res;
                                }
                                let ev = TtagyStreamEvent::Done {
                                    session_id: session_id.clone(),
                                    full_content: accumulated_text.clone(),
                                    thinking_content: if accumulated_thinking.is_empty() { None } else { Some(accumulated_thinking.clone()) },
                                    elapsed_ms,
                                    prompt_tokens: None,
                                    output_tokens: None,
                                };
                                if let Ok(json_str) = serde_json::to_string(&ev) {
                                    let _ = tx.send(Ok(Event::default().data(json_str))).await;
                                }
                                let _ = child.kill().await;
                                break;
                            }
                            ParsedChunk::Ignored => {}
                        }
                    }
                    Ok(Ok(None)) => {
                        let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;
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
                        let _ = child.kill().await;
                        let err_event = TtagyStreamEvent::Error {
                            session_id: session_id.clone(),
                            error_code: "INACTIVITY_TIMEOUT".into(),
                            error_message: format!("Worker silent for more than {}s (Inactivity Timeout)", timeout_secs),
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
    });

    let stream = ReceiverStream::new(rx);
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}
