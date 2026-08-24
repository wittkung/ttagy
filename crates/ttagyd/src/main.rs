//! TTAgy 宿主节点守护服务 (Private Agent Host Node Daemon)
//!
//! 具备物理版本隔离路由体系，挂载 /api/v1 (已冻结 API) 与未来版本。

mod v1;

use axum::{
    body::Body,
    extract::Request,
    http::{Method, StatusCode},
    middleware::{self, Next},
    response::Response,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tower_http::cors::{Any, CorsLayer};
use v1::AppState;

struct Config {
    host: String,
    port: u16,
    token: Option<String>,
    max_concurrency: usize,
}

impl Config {
    fn parse_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut host = "127.0.0.1".to_string();
        let mut port = 8970u16;
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

        Self { host, port, token, max_concurrency }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::parse_args();
    println!("⚡ Starting TTAgy Private Node on {}:{} (Concurrency: {})", config.host, config.port, config.max_concurrency);

    let state = Arc::new(AppState {
        auth_token: config.token.clone(),
        max_concurrency: config.max_concurrency,
        semaphore: Arc::new(Semaphore::new(config.max_concurrency)),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    let auth_state = state.clone();
    let app = Router::new()
        .nest("/api/v1", v1::router(state.clone()))
        .layer(middleware::from_fn(move |req, next| {
            let s = auth_state.clone();
            auth_middleware(s, req, next)
        }))
        .layer(cors);

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("🚀 TTAgy Private Node ready. Accepting connections at http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn auth_middleware(
    state: Arc<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
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
