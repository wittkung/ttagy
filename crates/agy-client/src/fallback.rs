//! In-Process Direct Spawn 兜底引擎

use std::process::Stdio;
use std::time::{Duration, Instant};
use futures_util::Stream;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_stream::wrappers::ReceiverStream;

use agy_core::{
    AgyDetector, AgyRequest, AgyStreamEvent, NdjsonParser, ParsedChunk, SandboxGuard,
};

pub struct FallbackDriver;

impl FallbackDriver {
    pub async fn stream_chat(
        request: AgyRequest,
    ) -> Result<impl Stream<Item = Result<AgyStreamEvent, String>>, String> {
        let binary = AgyDetector::find_binary().ok_or_else(|| {
            "未检测到 Antigravity CLI (agy) 二进制。请先在终端安装并认证。".to_string()
        })?;

        let sandbox = SandboxGuard::create("fallback_spawn", true)
            .map_err(|e| format!("创建隔离沙箱失败: {}", e))?;

        let model_name = request.model.clone().unwrap_or_else(|| "gemini-3.7-flash".to_string());
        let effort = request.effort.clone().unwrap_or_else(|| "high".to_string());
        let session_id = request.session_id.clone();
        let timeout_secs = request.timeout_secs;

        let mut cmd = Command::new(binary);
        cmd.current_dir(&sandbox.sandbox_path)
            .arg("-p")
            .arg(&request.prompt)
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

        let mut child = cmd.spawn().map_err(|e| format!("启动 agy 进程失败: {}", e))?;
        let stdout = child.stdout.take().ok_or_else(|| "无法获取 stdout".to_string())?;

        let (tx, rx) = mpsc::channel(64);
        let start_time = Instant::now();

        tokio::spawn(async move {
            let _keep_sandbox = sandbox; // 保持沙箱直到任务完成
            let _ = tx.send(Ok(AgyStreamEvent::Init {
                session_id: session_id.clone(),
                model: model_name,
                effort,
                backend_mode: "fallback_direct_spawn".to_string(),
            })).await;

            let mut reader = BufReader::new(stdout).lines();
            let mut full_content = String::new();
            let mut thinking_content = String::new();
            let inactivity_timeout = Duration::from_secs(timeout_secs);

            loop {
                let next_line_fut = reader.next_line();
                tokio::pin!(next_line_fut);

                match timeout(inactivity_timeout, &mut next_line_fut).await {
                    Ok(Ok(Some(line))) => {
                        match NdjsonParser::parse_line(&line) {
                            ParsedChunk::ThinkingDelta(delta) => {
                                thinking_content.push_str(&delta);
                                let elapsed = start_time.elapsed().as_secs_f64() * 1000.0;
                                let _ = tx.send(Ok(AgyStreamEvent::ThinkingDelta {
                                    session_id: session_id.clone(),
                                    text_delta: delta,
                                    elapsed_ms: elapsed,
                                })).await;
                            }
                            ParsedChunk::ContentDelta(delta) => {
                                full_content.push_str(&delta);
                                let elapsed = start_time.elapsed().as_secs_f64() * 1000.0;
                                let _ = tx.send(Ok(AgyStreamEvent::ContentDelta {
                                    session_id: session_id.clone(),
                                    text_delta: delta,
                                    accumulated_chars: full_content.chars().count(),
                                    elapsed_ms: elapsed,
                                })).await;
                            }
                            ParsedChunk::Result(res) => {
                                if full_content.is_empty() {
                                    full_content = res;
                                }
                            }
                            ParsedChunk::Ignored => {}
                        }
                    }
                    Ok(Ok(None)) => {
                        // EOF
                        let elapsed = start_time.elapsed().as_secs_f64() * 1000.0;
                        let _ = tx.send(Ok(AgyStreamEvent::Done {
                            session_id: session_id.clone(),
                            full_content,
                            thinking_content: if thinking_content.is_empty() { None } else { Some(thinking_content) },
                            elapsed_ms: elapsed,
                            prompt_tokens: None,
                            output_tokens: None,
                        })).await;
                        break;
                    }
                    Ok(Err(e)) => {
                        let _ = tx.send(Err(format!("读取 stdout 异常: {}", e))).await;
                        break;
                    }
                    Err(_) => {
                        let _ = tx.send(Err(format!("等待 Token 输出超时 ({}s)", timeout_secs))).await;
                        let _ = child.kill().await;
                        break;
                    }
                }
            }
        });

        Ok(ReceiverStream::new(rx))
    }
}
