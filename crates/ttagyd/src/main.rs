//! ttagyd: Antigravity CLI 私有节点守护服务 (Private Agent Host Node)

use std::net::SocketAddr;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Json,
    },
    routing::{get, post},
    Router,
};
use futures_util::Stream;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::timeout;
use tokio_stream::wrappers::ReceiverStream;
use tower_http::cors::CorsLayer;

use ttagy_core::{TtagyDetector, TtagyRequest, TtagyStreamEvent, NdjsonParser, ParsedChunk, SandboxGuard};

#[derive(Clone)]
struct AppState {
    auth_token: Option<String>,
    limiter: Arc<Semaphore>,
    binary_path: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    binary_path: String,
    available_slots: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut host = "127.0.0.1".to_string();
    let mut port: u16 = 8970;
    let mut auth_token: Option<String> = None;
    let mut max_concurrency: usize = 8;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--host" if i + 1 < args.len() => {
                host = args[i + 1].clone();
                i += 2;
            }
            "--port" if i + 1 < args.len() => {
                port = args[i + 1].parse().unwrap_or(8970);
                i += 2;
            }
            "--token" if i + 1 < args.len() => {
                auth_token = Some(args[i + 1].clone());
                i += 2;
            }
            "--concurrency" if i + 1 < args.len() => {
                max_concurrency = args[i + 1].parse().unwrap_or(8);
                i += 2;
            }
            _ => i += 1,
        }
    }

    println!("🚀 [ttagyd] 正在启动 Antigravity 私有 Agent 节点...");
    let binary = TtagyDetector::find_binary().expect("未检测到本地 agy 二进制");
    let binary_str = binary.to_string_lossy().to_string();
    println!("✅ [ttagyd] 绑定本地 AGY 二进制: {}", binary_str);

    let state = AppState {
        auth_token,
        limiter: Arc::new(Semaphore::new(max_concurrency)),
        binary_path: binary_str,
    };

    let app = Router::new()
        .route("/api/v1/health", get(health_handler))
        .route("/api/v1/stream", post(stream_handler))
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    println!("⚡ [ttagyd] 节点已就绪，正在监听: http://{}", addr);
    if state.auth_token.is_some() {
        println!("🔒 [ttagyd] 安全鉴权已开启 (Bearer Token Guard Active)");
    } else {
        println!("⚠️ [ttagyd] 警告: 未配置 --token，仅限受信任网络运行");
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    println!("🛑 [ttagyd] 服务已安全关闭。");
    Ok(())
}

async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        version: "0.1.0",
        binary_path: state.binary_path.clone(),
        available_slots: state.limiter.available_permits(),
    })
}

async fn stream_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<TtagyRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, axum::Error>>>, StatusCode> {
    // 鉴权校验
    if let Some(ref required_token) = state.auth_token {
        let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
        let expected = format!("Bearer {}", required_token);
        if auth_header != Some(expected.as_str()) {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    let (tx, rx) = mpsc::channel::<Result<Event, axum::Error>>(64);
    let binary = state.binary_path.clone();
    let limiter = state.limiter.clone();

    tokio::spawn(async move {
        let permit = match limiter.acquire_owned().await {
            Ok(p) => p,
            Err(_) => {
                let _ = tx.send(Ok(Event::default().data(
                    serde_json::to_string(&TtagyStreamEvent::Error {
                        session_id: req.session_id.clone(),
                        error_code: "SEMAPHORE_CLOSED".to_string(),
                        error_message: "服务并发调度器已关闭".to_string(),
                        is_retryable: true,
                    }).unwrap_or_default(),
                ))).await;
                return;
            }
        };

        let sandbox = match SandboxGuard::create("ttagyd_session", true) {
            Ok(s) => s,
            Err(e) => {
                let _ = tx.send(Ok(Event::default().data(
                    serde_json::to_string(&TtagyStreamEvent::Error {
                        session_id: req.session_id.clone(),
                        error_code: "SANDBOX_FAILED".to_string(),
                        error_message: format!("创建隔离沙箱失败: {}", e),
                        is_retryable: false,
                    }).unwrap_or_default(),
                ))).await;
                return;
            }
        };

        let model_name = req.model.clone().unwrap_or_else(|| "gemini-3.7-flash".to_string());
        let effort = req.effort.clone().unwrap_or_else(|| "high".to_string());
        let session_id = req.session_id.clone();
        let timeout_secs = req.timeout_secs;

        let mut cmd = Command::new(binary);
        cmd.current_dir(&sandbox.sandbox_path)
            .arg("-p")
            .arg(&req.prompt)
            .arg("--model")
            .arg(&model_name);

        if !effort.is_empty() && effort != "none" {
            cmd.arg("--effort").arg(&effort);
        }

        cmd.arg("--output-format")
            .arg("stream-json")
            .arg("--disable-slash-commands")
            .arg("--dangerously-skip-permissions")
            .arg("--log-file")
            .arg(&sandbox.log_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(Ok(Event::default().data(
                    serde_json::to_string(&TtagyStreamEvent::Error {
                        session_id: session_id.clone(),
                        error_code: "SPAWN_ERROR".to_string(),
                        error_message: format!("启动进程失败: {}", e),
                        is_retryable: false,
                    }).unwrap_or_default(),
                ))).await;
                return;
            }
        };

        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => return,
        };

        let start_time = Instant::now();
        let _ = tx.send(Ok(Event::default().data(
            serde_json::to_string(&TtagyStreamEvent::Init {
                session_id: session_id.clone(),
                model: model_name,
                effort,
                backend_mode: "daemon_ipc".to_string(),
            }).unwrap_or_default(),
        ))).await;

        let mut reader = BufReader::new(stdout).lines();
        let mut full_content = String::new();
        let mut thinking_content = String::new();
        let inactivity_timeout = Duration::from_secs(timeout_secs);

        loop {
            let next_line_fut = reader.next_line();
            tokio::pin!(next_line_fut);

            match timeout(inactivity_timeout, &mut next_line_fut).await {
                Ok(Ok(Some(line))) => match NdjsonParser::parse_line(&line) {
                    ParsedChunk::ThinkingDelta(delta) => {
                        thinking_content.push_str(&delta);
                        let elapsed = start_time.elapsed().as_secs_f64() * 1000.0;
                        let _ = tx.send(Ok(Event::default().data(
                            serde_json::to_string(&TtagyStreamEvent::ThinkingDelta {
                                session_id: session_id.clone(),
                                text_delta: delta,
                                elapsed_ms: elapsed,
                            }).unwrap_or_default(),
                        ))).await;
                    }
                    ParsedChunk::ContentDelta(delta) => {
                        full_content.push_str(&delta);
                        let elapsed = start_time.elapsed().as_secs_f64() * 1000.0;
                        let _ = tx.send(Ok(Event::default().data(
                            serde_json::to_string(&TtagyStreamEvent::ContentDelta {
                                session_id: session_id.clone(),
                                text_delta: delta,
                                accumulated_chars: full_content.chars().count(),
                                elapsed_ms: elapsed,
                            }).unwrap_or_default(),
                        ))).await;
                    }
                    ParsedChunk::Result(res) => {
                        if full_content.is_empty() {
                            full_content = res;
                        }
                    }
                    ParsedChunk::Ignored => {}
                },
                Ok(Ok(None)) => {
                    let elapsed = start_time.elapsed().as_secs_f64() * 1000.0;
                    let _ = tx.send(Ok(Event::default().data(
                        serde_json::to_string(&TtagyStreamEvent::Done {
                            session_id: session_id.clone(),
                            full_content,
                            thinking_content: if thinking_content.is_empty() {
                                None
                            } else {
                                Some(thinking_content)
                            },
                            elapsed_ms: elapsed,
                            prompt_tokens: None,
                            output_tokens: None,
                        }).unwrap_or_default(),
                    ))).await;
                    break;
                }
                _ => {
                    let _ = child.kill().await;
                    break;
                }
            }
        }

        drop(permit);
        drop(sandbox);
    });

    Ok(Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default()))
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("无法监听 Ctrl+C 关闭信号");
}
