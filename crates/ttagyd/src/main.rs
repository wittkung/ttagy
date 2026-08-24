//! TTAgy 宿主节点守护服务 (Private Agent Host Node Daemon & Subagent Mesh Runtime)
//!
//! 支持 Unix Domain Socket (/tmp/ttagy.sock) 极速本地 IPC 与 TCP HTTP/SSE 远程双模并发监听。

pub mod mcp_manager;
pub mod message_bus;
pub mod session_store;
pub mod subagent_mesh;
pub mod telemetry;
pub mod worker_pool;
pub mod workspace_manager;
mod v1;

use axum::{
    body::Body,
    extract::Request,
    http::{Method, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};
use mcp_manager::McpManager;
use message_bus::MessageBus;
use session_store::SessionStore;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use subagent_mesh::SubagentMesh;
use telemetry::{MetricsCollector, TraceManager};
use tokio::sync::Semaphore;
use tower::Service;
use tower_http::cors::{Any, CorsLayer};
use v1::AppState;
use workspace_manager::WorkspaceManager;

struct Config {
    host: String,
    port: u16,
    socket_path: Option<PathBuf>,
    token: Option<String>,
    max_concurrency: usize,
}

impl Config {
    fn parse_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut host = "127.0.0.1".to_string();
        let mut port = 8970u16;
        let mut socket_path = std::env::var("TTAGY_SOCKET_PATH")
            .ok()
            .map(PathBuf::from)
            .or_else(|| Some(PathBuf::from("/tmp/ttagy.sock")));
        let mut token = std::env::var("TTAGY_AUTH_TOKEN").ok();
        let mut max_concurrency = 4usize;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--host" => {
                    if i + 1 < args.len() {
                        host = args[i + 1].clone();
                        i += 1;
                    }
                }
                "--port" | "-p" => {
                    if i + 1 < args.len() {
                        if let Ok(p) = args[i + 1].parse() {
                            port = p;
                        }
                        i += 1;
                    }
                }
                "--socket" | "-s" => {
                    if i + 1 < args.len() {
                        socket_path = Some(PathBuf::from(args[i + 1].clone()));
                        i += 1;
                    }
                }
                "--no-socket" => {
                    socket_path = None;
                }
                "--token" | "-t" => {
                    if i + 1 < args.len() {
                        token = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--concurrency" | "-c" => {
                    if i + 1 < args.len() {
                        if let Ok(c) = args[i + 1].parse() {
                            max_concurrency = c;
                        }
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }

        Self {
            host,
            port,
            socket_path,
            token,
            max_concurrency,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::parse_args();
    println!(
        "⚡ Starting TTAgy Daemon on {}:{} (Concurrency: {})",
        config.host, config.port, config.max_concurrency
    );

    let storage_dir = std::env::var("TTAGY_STORAGE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut p = std::env::temp_dir();
            p.push("ttagy_storage");
            p
        });

    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let worktree_base = storage_dir.join("workspaces");

    let session_store = Arc::new(SessionStore::new(storage_dir));
    let mcp_manager = Arc::new(McpManager::new());
    let workspace_manager = WorkspaceManager::new(current_dir, worktree_base);
    let message_bus = Arc::new(MessageBus::new());
    let subagent_mesh = Arc::new(SubagentMesh::new(config.max_concurrency * 4));
    let metrics_collector = MetricsCollector::new();
    let trace_manager = Arc::new(TraceManager::new(1000));

    // 启动前孤儿 Worktree 对账与自愈
    let _ = workspace_manager.reconcile_orphans().await;

    let state = Arc::new(AppState {
        auth_token: config.token.clone(),
        max_concurrency: config.max_concurrency,
        semaphore: Arc::new(Semaphore::new(config.max_concurrency)),
        session_store,
        mcp_manager,
        workspace_manager,
        message_bus,
        subagent_mesh,
        metrics_collector: metrics_collector.clone(),
        trace_manager,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any);

    let auth_state = state.clone();
    let metrics_state = state.clone();

    let app = Router::new()
        // Prometheus metrics 端点 (免鉴权)
        .route("/metrics", get(move || {
            let s = metrics_state.clone();
            async move {
                let permits = s.semaphore.available_permits();
                s.metrics_collector.render_prometheus(permits)
            }
        }))
        .nest("/api/v1", v1::router(state.clone()))
        .layer(middleware::from_fn(move |req, next| {
            let s = auth_state.clone();
            auth_middleware(s, req, next)
        }))
        .layer(cors);

    let mut tasks = Vec::new();

    // 1. TCP 监听服务
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let tcp_listener = tokio::net::TcpListener::bind(addr).await?;
    println!("🚀 TTAgy TCP Node ready at http://{}", addr);
    let tcp_app = app.clone();
    tasks.push(tokio::spawn(async move {
        let _ = axum::serve(tcp_listener, tcp_app).await;
    }));

    // 2. Unix Domain Socket (UDS) 极速本地 IPC 服务
    #[cfg(unix)]
    if let Some(sock_path) = config.socket_path {
        if sock_path.exists() {
            let _ = tokio::fs::remove_file(&sock_path).await;
        }
        if let Some(parent) = sock_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        let uds_listener = tokio::net::UnixListener::bind(&sock_path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&sock_path, std::fs::Permissions::from_mode(0o600));
        }
        println!("⚡ TTAgy UDS IPC ready at unix://{}", sock_path.display());

        let uds_app = app;
        tasks.push(tokio::spawn(async move {
            loop {
                match uds_listener.accept().await {
                    Ok((stream, _)) => {
                        let io = hyper_util::rt::TokioIo::new(stream);
                        let tower_service = uds_app.clone();
                        tokio::spawn(async move {
                            let hyper_service = hyper::service::service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
                                let mut s = tower_service.clone();
                                async move {
                                    let req = req.map(Body::new);
                                    let resp = s.call(req).await.unwrap();
                                    Ok::<_, std::convert::Infallible>(resp)
                                }
                            });
                            let _ = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                                .serve_connection(io, hyper_service)
                                .await;
                        });
                    }
                    Err(e) => {
                        eprintln!("[ttagyd] UDS accept error: {}", e);
                    }
                }
            }
        }));
    }

    futures_util::future::select_all(tasks).await.0?;
    Ok(())
}

async fn auth_middleware(
    state: Arc<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if req.uri().path() == "/metrics" {
        return Ok(next.run(req).await);
    }

    if let Some(ref required_token) = state.auth_token {
        if !required_token.is_empty() {
            if let Some(auth_header) = req.headers().get("Authorization") {
                if let Ok(auth_str) = auth_header.to_str() {
                    let token = auth_str.trim_start_matches("Bearer ").trim();
                    if token == required_token {
                        return Ok(next.run(req).await);
                    }
                }
            }
            return Err(StatusCode::UNAUTHORIZED);
        }
    }
    Ok(next.run(req).await)
}
