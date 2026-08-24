//! Mock Antigravity CLI (mock-agy)
//!
//! 零云端配额消耗、100% 确定性的离线 CLI 仿真双重桩，支持多种故障注入与混沌场景。

use std::io::{self, Write};
use std::thread;
use std::time::Duration;

struct MockConfig {
    scenario: String,
    delay_ms: u64,
}

impl MockConfig {
    fn parse_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut scenario = std::env::var("MOCK_AGY_SCENARIO").unwrap_or_else(|_| "stream_normal".to_string());
        let mut delay_ms = 0u64;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "-p" | "--prompt" => {
                    if i + 1 < args.len() {
                        let prompt = &args[i + 1];
                        if let Some(pos) = prompt.find("scenario:") {
                            let sub = &prompt[pos + 9..];
                            let name = sub.split_whitespace().next().unwrap_or(sub);
                            scenario = name.to_string();
                        }
                        i += 1;
                    }
                }
                "--scenario" => {
                    if i + 1 < args.len() {
                        scenario = args[i + 1].clone();
                        i += 1;
                    }
                }
                "--delay-ms" => {
                    if i + 1 < args.len() {
                        if let Ok(d) = args[i + 1].parse() {
                            delay_ms = d;
                        }
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }

        Self { scenario, delay_ms }
    }
}

fn emit_line(line: &str, delay_ms: u64) {
    if delay_ms > 0 {
        thread::sleep(Duration::from_millis(delay_ms));
    }
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = writeln!(handle, "{}", line);
    let _ = handle.flush();
}

fn main() {
    let config = MockConfig::parse_args();

    match config.scenario.as_str() {
        "stream_normal" => {
            emit_line(
                r#"{"type":"step_update","step_update":{"thought_delta":"正在深度推导中..."}}"#,
                config.delay_ms,
            );
            emit_line(
                r#"{"type":"step_update","step_update":{"text_delta":"你好，我是 Antigravity AI 助手。"}}"#,
                config.delay_ms,
            );
            emit_line(
                r#"{"type":"step_update","step_update":{"text_delta":"很高兴为您服务！"}}"#,
                config.delay_ms,
            );
            emit_line(
                r#"{"type":"result","result":{"content":"你好，我是 Antigravity AI 助手。很高兴为您服务！","thinking_content":"正在深度推导中...","usage":{"prompt_tokens":25,"completion_tokens":12}}}"#,
                config.delay_ms,
            );
        }

        "structured_json" => {
            emit_line(
                r#"{"type":"step_update","step_update":{"thought_delta":"正在生成结构化 JSON..."}}"#,
                config.delay_ms,
            );
            emit_line(
                r#"{"type":"result","result":{"content":"```json\n{\n  \"status\": \"success\",\n  \"task\": \"compression\",\n  \"files_count\": 3\n}\n```","usage":{"prompt_tokens":30,"completion_tokens":20}}}"#,
                config.delay_ms,
            );
        }

        "stderr_flood" => {
            // 启动并发线程向 stderr 写入 10MB 数据 (测试父进程是否发生 Pipe 缓冲区死锁)
            let stderr_handle = thread::spawn(|| {
                let stderr = io::stderr();
                let mut handle = stderr.lock();
                let chunk = [b'E'; 4096];
                let total_chunks = (10 * 1024 * 1024) / chunk.len(); // 10MB
                for _ in 0..total_chunks {
                    if handle.write_all(&chunk).is_err() {
                        break;
                    }
                }
                let _ = handle.flush();
            });

            emit_line(
                r#"{"type":"step_update","step_update":{"text_delta":"在大量 stderr 日志轰炸下仍然正常输出。"}}"#,
                config.delay_ms,
            );
            emit_line(
                r#"{"type":"result","result":{"content":"在大量 stderr 日志轰炸下仍然正常输出。","usage":{"prompt_tokens":10,"completion_tokens":5}}}"#,
                config.delay_ms,
            );

            let _ = stderr_handle.join();
        }

        "malformed_ndjson" => {
            emit_line(
                "[SYSTEM DIAGNOSTIC] Loading weights from local cache...",
                config.delay_ms,
            );
            emit_line(
                r#"{"type":"step_update","step_update":{"text_delta":"有效内容片段 1。"}}"#,
                config.delay_ms,
            );
            emit_line("```json\n{\"broken\": true", config.delay_ms);
            emit_line(
                r#"{"type":"result","result":{"content":"有效内容片段 1。"}}"#,
                config.delay_ms,
            );
        }

        "abort_hang" => {
            for _ in 0..300 {
                thread::sleep(Duration::from_millis(100));
            }
        }

        "quota_error" => {
            let _ = writeln!(io::stderr(), "Error: Resource quota exceeded for project.");
            emit_line(
                r#"{"type":"error","error":"Resource quota exceeded for project"}"#,
                config.delay_ms,
            );
            std::process::exit(1);
        }

        "empty_output" => {
            let _ = writeln!(io::stderr(), "Fatal execution panic in native runtime.");
            let _ = io::stderr().flush();
            std::process::exit(1);
        }

        _ => {
            emit_line(
                r#"{"type":"result","result":{"content":"Default mock response"}}"#,
                config.delay_ms,
            );
        }
    }
}
