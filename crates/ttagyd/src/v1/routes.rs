//! V1 专有 RESTful 与 SSE 接口实现 (Superset API, Mesh & Telemetry Control Plane)

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use futures_util::Stream;
use serde::Deserialize;
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
use crate::mcp_manager::{McpManager, McpServerConfig};
use crate::message_bus::{ActorMessage, MessageBus};
use crate::session_store::{SessionMessage, SessionStore};
use crate::subagent_mesh::{SubagentMesh, SubagentSpec};
use crate::telemetry::{MetricsCollector, TraceManager};
use crate::workspace_manager::WorkspaceManager;

pub struct AppState {
    pub auth_token: Option<String>,
    pub max_concurrency: usize,
    pub semaphore: Arc<tokio::sync::Semaphore>,
    pub session_store: Arc<SessionStore>,
    pub mcp_manager: Arc<McpManager>,
    #[allow(dead_code)]
    pub workspace_manager: Arc<WorkspaceManager>,
    pub message_bus: Arc<MessageBus>,
    pub subagent_mesh: Arc<SubagentMesh>,
    pub metrics_collector: Arc<MetricsCollector>,
    pub trace_manager: Arc<TraceManager>,
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
        // 1. 模型与 Agent 目录
        .route("/models", get(list_models_handler))
        .route("/agents", get(list_agents_handler))
        // 2. 虚拟化 MCP 管控
        .route("/mcp/servers", get(list_mcp_servers_handler).post(mount_mcp_server_handler))
        .route("/mcp/servers/:name", delete(unmount_mcp_server_handler))
        .route("/mcp/tools", get(list_mcp_tools_handler))
        // 3. Stateful Session 状态机
        .route("/sessions", get(list_sessions_handler).post(create_session_handler))
        .route("/sessions/:id", get(get_session_handler).delete(delete_session_handler))
        .route("/sessions/:id/messages", post(append_message_handler))
        .route("/sessions/:id/compact", post(compact_session_handler))
        // 4. Subagent Mesh 协作网格
        .route("/subagents", get(list_subagents_handler))
        .route("/subagents/invoke", post(invoke_subagents_handler))
        .route("/subagents/message", post(send_subagent_message_handler))
        .route("/subagents/wait", post(wait_subagent_handler))
        .route("/subagents/kill_all", post(kill_all_subagents_handler))
        .route("/subagents/:id", get(get_subagent_handler))
        .route("/subagents/:id/kill", post(kill_cascade_subagent_handler))
        // 5. Telemetry & Observability
        .route("/telemetry/stats", get(get_telemetry_stats_handler))
        .route("/telemetry/traces/:id", get(get_trace_spans_handler))
        .route("/telemetry/spans", get(get_recent_spans_handler))
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

async fn list_models_handler() -> impl IntoResponse {
    let models = vec![
        serde_json::json!({
            "id": "gemini-3.7-flash",
            "name": "Gemini 3.7 Flash",
            "provider": "google",
            "context_window": 1048576,
            "max_output_tokens": 65536,
            "supported_efforts": ["low", "medium", "high", "none"],
            "supports_streaming": true,
            "supports_tool_calling": true,
            "supports_json_schema": true
        }),
        serde_json::json!({
            "id": "claude-sonnet-4-6",
            "name": "Claude 3.7 Sonnet",
            "provider": "anthropic",
            "context_window": 200000,
            "max_output_tokens": 8192,
            "supported_efforts": ["none"],
            "supports_streaming": true,
            "supports_tool_calling": true,
            "supports_json_schema": true
        })
    ];
    Json(serde_json::json!({ "models": models }))
}

async fn list_agents_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "agents": [
            {
                "id": "default",
                "name": "Default AI Assistant",
                "default_model": "gemini-3.7-flash",
                "default_effort": "low",
                "enabled_tools": ["chrome-devtools", "memory-debugger"]
            }
        ]
    }))
}

async fn list_mcp_servers_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let servers = state.mcp_manager.list_servers().await;
    Json(serde_json::json!({ "servers": servers }))
}

async fn mount_mcp_server_handler(
    State(state): State<Arc<AppState>>,
    Json(config): Json<McpServerConfig>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    state.mcp_manager.register_server(config).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "status": "mounted" }))))
}

async fn unmount_mcp_server_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    state.mcp_manager.unregister_server(&name).await
        .map_err(|e| (StatusCode::NOT_FOUND, e))?;
    Ok(Json(serde_json::json!({ "status": "unmounted", "name": name })))
}

async fn list_mcp_tools_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tools = state.mcp_manager.list_tools().await;
    Json(serde_json::json!({ "tools": tools }))
}

async fn list_sessions_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let sessions = state.session_store.list_sessions().await;
    Json(serde_json::json!({ "sessions": sessions }))
}

async fn create_session_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agent_id = payload.get("agent_id").and_then(|v| v.as_str()).unwrap_or("default").to_string();
    let model = payload.get("model").and_then(|v| v.as_str()).unwrap_or("gemini-3.7-flash").to_string();

    let meta = state.session_store.create_session(agent_id, model).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok((StatusCode::CREATED, Json(meta)))
}

async fn get_session_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let handle = state.session_store.get_session_handle(&id).await
        .map_err(|_| (StatusCode::NOT_FOUND, format!("Session '{}' not found", id)))?;
    let session = handle.read().await;
    Ok(Json(session.clone()))
}

async fn delete_session_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let _ = state.session_store.delete_session(&id).await;
    Json(serde_json::json!({ "status": "deleted", "session_id": id }))
}

async fn append_message_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(msg): Json<SessionMessage>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    state.session_store.append_message(&id, msg).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::json!({ "status": "appended" })))
}

async fn compact_session_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (before, after) = state.session_store.compact_session(&id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(serde_json::json!({
        "status": "compacted",
        "session_id": id,
        "messages_before": before,
        "messages_after": after
    })))
}

// --------------------------- Subagent Mesh Handlers ---------------------------

#[derive(Debug, Deserialize)]
pub struct InvokeSubagentRequest {
    pub parent_id: Option<String>,
    pub subagents: Vec<SubagentSpec>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub sender_id: String,
    pub recipient_id: String,
    pub content: String,
    pub reply_to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WaitSubagentRequest {
    pub waiter_id: String,
    pub waitee_id: String,
    #[serde(default = "default_wait_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_wait_timeout_ms() -> u64 {
    5000
}

async fn list_subagents_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let nodes = state.subagent_mesh.list_subagents().await;
    Json(serde_json::json!({
        "mesh_version": "v1",
        "total": nodes.len(),
        "active_nodes": nodes
    }))
}

async fn invoke_subagents_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<InvokeSubagentRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let spawned = state.subagent_mesh.invoke_subagents(payload.parent_id, payload.subagents).await
        .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, e))?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "status": "spawned",
        "spawned_subagents": spawned
    }))))
}

async fn send_subagent_message_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let msg = ActorMessage {
        message_id: format!("msg_{}", uuid::Uuid::new_v4()),
        sender_id: payload.sender_id,
        recipient_id: payload.recipient_id,
        content: payload.content,
        reply_to: payload.reply_to,
        created_at: now,
    };

    state.message_bus.send_message(msg).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok(Json(serde_json::json!({ "status": "delivered" })))
}

async fn wait_subagent_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<WaitSubagentRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    state.subagent_mesh.wait_for_peer(&payload.waiter_id, &payload.waitee_id).await
        .map_err(|e| (StatusCode::CONFLICT, e))?;

    let timeout_duration = Duration::from_millis(payload.timeout_ms.min(5000));
    tokio::time::sleep(timeout_duration).await;

    state.subagent_mesh.release_wait_for_peer(&payload.waiter_id, &payload.waitee_id).await;

    Ok(Json(serde_json::json!({
        "status": "waited",
        "waiter": payload.waiter_id,
        "waitee": payload.waitee_id
    })))
}

async fn kill_all_subagents_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let killed = state.subagent_mesh.kill_all().await;
    Json(serde_json::json!({
        "status": "all_killed",
        "count": killed.len(),
        "terminated_subagents": killed
    }))
}

async fn get_subagent_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let node = state.subagent_mesh.get_subagent(&id).await
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Subagent '{}' not found", id)))?;
    Ok(Json(node))
}

async fn kill_cascade_subagent_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let killed = state.subagent_mesh.kill_cascade(&id).await;
    Json(serde_json::json!({
        "status": "killed_cascade",
        "root_target": id,
        "total_terminated": killed.len(),
        "terminated_subagents": killed
    }))
}

// --------------------------- Telemetry Handlers ---------------------------

async fn get_telemetry_stats_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let permits = state.semaphore.available_permits();
    let stats = state.metrics_collector.get_snapshot(permits);
    Json(serde_json::json!({ "stats": stats }))
}

async fn get_trace_spans_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let spans = state.trace_manager.get_trace_spans(&id).await;
    Json(serde_json::json!({ "trace_id": id, "spans": spans }))
}

async fn get_recent_spans_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let spans = state.trace_manager.get_recent_spans(50).await;
    Json(serde_json::json!({ "spans": spans }))
}

// --------------------------- Chat Stream Handler ---------------------------

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
    let metrics = state.metrics_collector.clone();

    tokio::spawn(async move {
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
                return;
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
                metrics.record_request(false, 0, 0, 0);
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

        if let Some(ref agent) = payload.agent {
            if !agent.is_empty() {
                cmd.arg("--agent").arg(agent);
            }
        }

        if let Some(ref mode) = payload.mode {
            if !mode.is_empty() {
                cmd.arg("--mode").arg(mode);
            }
        }

        if let Some(ref conv) = payload.conversation_id {
            if !conv.is_empty() {
                cmd.arg("--conversation").arg(conv);
            }
        }

        if payload.continue_last == Some(true) {
            cmd.arg("--continue");
        }

        if let Some(ref proj) = payload.project {
            if !proj.is_empty() {
                cmd.arg("--project").arg(proj);
            }
        }

        for dir in &payload.add_dirs {
            if !dir.is_empty() {
                cmd.arg("--add-dir").arg(dir);
            }
        }

        if payload.sandbox == Some(true) {
            cmd.arg("--sandbox");
        }

        if let Some(ref schema) = payload.json_schema {
            if !schema.is_empty() {
                cmd.arg("--json-schema").arg(schema);
            }
        }

        cmd.arg("--output-format")
            .arg("stream-json")
            .arg("--log-file")
            .arg(&sandbox.log_path);

        if payload.disable_slash_commands.unwrap_or(true) {
            cmd.arg("--disable-slash-commands");
        }

        if payload.dangerously_skip_permissions.unwrap_or(true) {
            cmd.arg("--dangerously-skip-permissions");
        }

        cmd.stdout(std::process::Stdio::piped())
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
                metrics.record_request(false, 0, 0, 0);
                return;
            }
        };

        let stdout = match child.stdout.take() {
            Some(o) => o,
            None => {
                let _ = child.kill().await;
                metrics.record_request(false, 0, 0, 0);
                return;
            }
        };

        let stderr_drainer = child.stderr.take().map(|err| StderrDrainer::spawn(err, 64 * 1024));
        let mut reader = BufReader::new(stdout).lines();
        let timeout_duration = Duration::from_secs(timeout_secs);
        let mut accumulated_text = String::new();
        let mut accumulated_thinking = String::new();

        loop {
            tokio::select! {
                biased;

                _ = worker_cancel_token.cancelled() => {
                    let _ = child.kill().await;
                    metrics.record_request(false, 0, 0, 0);
                    break;
                }

                _ = tx.closed() => {
                    let _ = child.kill().await;
                    metrics.record_request(false, 0, 0, 0);
                    break;
                }

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
                                        let prompt_tokens = usage.as_ref().and_then(|u| u.prompt_tokens);
                                        let output_tokens = usage.as_ref().and_then(|u| u.output_tokens);

                                        let ev = TtagyStreamEvent::Done {
                                            session_id: session_id.clone(),
                                            full_content: accumulated_text.clone(),
                                            thinking_content: final_thinking,
                                            elapsed_ms,
                                            prompt_tokens,
                                            output_tokens,
                                        };
                                        if let Ok(json_str) = serde_json::to_string(&ev) {
                                            let _ = tx.send(Ok(Event::default().data(json_str))).await;
                                        }
                                        metrics.record_request(
                                            true,
                                            prompt_tokens.unwrap_or(0),
                                            output_tokens.unwrap_or(0),
                                            0,
                                        );
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
                                        metrics.record_request(false, 0, 0, 0);
                                        let _ = child.kill().await;
                                        return;
                                    }
                                }
                            }
                        }
                        Ok(Ok(None)) => {
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
                                metrics.record_request(false, 0, 0, 0);
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
                                metrics.record_request(true, 0, 0, 0);
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
                            metrics.record_request(false, 0, 0, 0);
                            break;
                        }
                        Err(_) => {
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
                            metrics.record_request(false, 0, 0, 0);
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
