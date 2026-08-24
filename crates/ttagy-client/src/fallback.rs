//! In-Process Direct Spawn 兜底引擎

use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_stream::wrappers::ReceiverStream;

use ttagy_core::{
    resolve_model_name, NdjsonParser, ParsedStreamItem, SandboxGuard, StderrDrainer, TtagyDetector,
    TtagyRequest, TtagyStreamEvent,
};

pub struct FallbackDriver;

impl FallbackDriver {
    pub async fn stream_chat(
        request: TtagyRequest,
    ) -> Result<ReceiverStream<Result<TtagyStreamEvent, String>>, String> {
        let binary = TtagyDetector::find_binary().ok_or_else(|| {
            "未检测到 Antigravity CLI (agy) 二进制。请先在终端安装并认证。".to_string()
        })?;

        let sandbox = SandboxGuard::create("fallback_spawn", true)
            .map_err(|e| format!("创建隔离沙箱失败: {}", e))?;

        let model_name = resolve_model_name(request.model.as_deref())
            .unwrap_or_else(|_| "gemini-3.7-flash".to_string());
        let effort = request.effort.clone().unwrap_or_else(|| "high".to_string());
        let session_id = request.session_id.clone();
        let timeout_secs = request.timeout_secs;

        let mut cmd = Command::new(binary);
        cmd.current_dir(&sandbox.sandbox_path)
            .arg("-p")
            .arg(&request.prompt)
            .arg("--model")
            .arg(&model_name)
            .kill_on_drop(true);

        #[cfg(unix)]
        cmd.process_group(0);

        if !effort.is_empty() && effort != "none" {
            cmd.arg("--effort").arg(&effort);
        }

        if let Some(ref schema) = request.json_schema {
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
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| format!("启动 agy 进程失败: {}", e))?;
        let stdout = child.stdout.take().ok_or_else(|| "无法获取 stdout".to_string())?;
        let stderr_drainer = child.stderr.take().map(|err| StderrDrainer::spawn(err, 64 * 1024));

        let (tx, rx) = mpsc::channel(64);
        let start_time = Instant::now();

        tokio::spawn(async move {
            let _keep_sandbox = sandbox; // 保持沙箱直到任务完成
            if tx.send(Ok(TtagyStreamEvent::Init {
                session_id: session_id.clone(),
                model: model_name,
                effort,
                backend_mode: "fallback_direct_spawn".to_string(),
            })).await.is_err() {
                let _ = child.kill().await;
                return;
            }

            let mut reader = BufReader::new(stdout).lines();
            let mut full_content = String::new();
            let mut thinking_content = String::new();
            let inactivity_timeout = Duration::from_secs(timeout_secs);

            loop {
                let next_line_fut = reader.next_line();
                tokio::pin!(next_line_fut);

                match timeout(inactivity_timeout, &mut next_line_fut).await {
                    Ok(Ok(Some(line))) => {
                        let elapsed = start_time.elapsed().as_secs_f64() * 1000.0;
                        let items = NdjsonParser::parse_line_items(&line);

                        for item in items {
                            match item {
                                ParsedStreamItem::ThinkingDelta(delta) => {
                                    thinking_content.push_str(&delta);
                                    let ev = TtagyStreamEvent::ThinkingDelta {
                                        session_id: session_id.clone(),
                                        text_delta: delta,
                                        elapsed_ms: elapsed,
                                    };
                                    if tx.send(Ok(ev)).await.is_err() {
                                        let _ = child.kill().await;
                                        return;
                                    }
                                }
                                ParsedStreamItem::ContentDelta(delta) => {
                                    full_content.push_str(&delta);
                                    let ev = TtagyStreamEvent::ContentDelta {
                                        session_id: session_id.clone(),
                                        text_delta: delta,
                                        accumulated_chars: full_content.chars().count(),
                                        elapsed_ms: elapsed,
                                    };
                                    if tx.send(Ok(ev)).await.is_err() {
                                        let _ = child.kill().await;
                                        return;
                                    }
                                }
                                ParsedStreamItem::Done { content, thinking_content: tc, usage } => {
                                    if !content.is_empty() {
                                        full_content = content;
                                    }
                                    let final_thinking = tc.or_else(|| {
                                        if thinking_content.is_empty() { None } else { Some(thinking_content.clone()) }
                                    });
                                    let ev = TtagyStreamEvent::Done {
                                        session_id: session_id.clone(),
                                        full_content: full_content.clone(),
                                        thinking_content: final_thinking,
                                        elapsed_ms: elapsed,
                                        prompt_tokens: usage.as_ref().and_then(|u| u.prompt_tokens),
                                        output_tokens: usage.as_ref().and_then(|u| u.output_tokens),
                                    };
                                    let _ = tx.send(Ok(ev)).await;
                                    let _ = child.kill().await;
                                    return;
                                }
                                ParsedStreamItem::Error { code, message } => {
                                    let ev = TtagyStreamEvent::Error {
                                        session_id: session_id.clone(),
                                        error_code: code,
                                        error_message: message,
                                        is_retryable: true,
                                    };
                                    let _ = tx.send(Ok(ev)).await;
                                    let _ = child.kill().await;
                                    return;
                                }
                            }
                        }
                    }
                    Ok(Ok(None)) => {
                        // EOF
                        let elapsed = start_time.elapsed().as_secs_f64() * 1000.0;
                        let stderr_logs = if let Some(ref d) = stderr_drainer {
                            d.get_logs().await
                        } else {
                            String::new()
                        };

                        if full_content.is_empty() && !stderr_logs.trim().is_empty() {
                            let _ = tx.send(Ok(TtagyStreamEvent::Error {
                                session_id: session_id.clone(),
                                error_code: "EXECUTION_EMPTY_OUTPUT".to_string(),
                                error_message: format!("Empty output produced. Stderr: {}", stderr_logs),
                                is_retryable: false,
                            })).await;
                        } else {
                            let _ = tx.send(Ok(TtagyStreamEvent::Done {
                                session_id: session_id.clone(),
                                full_content,
                                thinking_content: if thinking_content.is_empty() { None } else { Some(thinking_content) },
                                elapsed_ms: elapsed,
                                prompt_tokens: None,
                                output_tokens: None,
                            })).await;
                        }
                        break;
                    }
                    Ok(Err(e)) => {
                        let _ = tx.send(Err(format!("读取 stdout 异常: {}", e))).await;
                        let _ = child.kill().await;
                        break;
                    }
                    Err(_) => {
                        let stderr_logs = if let Some(ref d) = stderr_drainer {
                            d.get_logs().await
                        } else {
                            String::new()
                        };
                        let _ = tx.send(Err(format!(
                            "等待 Token 输出超时 ({}s). Stderr: {}",
                            timeout_secs,
                            if stderr_logs.is_empty() { "None" } else { &stderr_logs }
                        ))).await;
                        let _ = child.kill().await;
                        break;
                    }
                }
            }
        });

        Ok(ReceiverStream::new(rx))
    }
}
