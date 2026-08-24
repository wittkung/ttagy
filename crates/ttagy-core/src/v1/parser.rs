//! NDJSON 流式行事件解析器 (Layer 2 协议解耦解析器)

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageMetadata {
    pub prompt_tokens: Option<usize>,
    pub output_tokens: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedStreamItem {
    ThinkingDelta(String),
    ContentDelta(String),
    Done {
        content: String,
        thinking_content: Option<String>,
        usage: Option<UsageMetadata>,
    },
    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedChunk {
    ThinkingDelta(String),
    ContentDelta(String),
    Result(String),
    Error(String),
    Ignored,
}

pub struct NdjsonParser;

impl NdjsonParser {
    /// 完整流式解析：单行可能产出多个事件 (如同一更新中同时包含思考增量和正文增量)
    pub fn parse_line_items(line: &str) -> Vec<ParsedStreamItem> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        let val: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        let mut items = Vec::new();
        let ev_name = val.get("event")
            .or_else(|| val.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match ev_name {
            "step_update" => {
                if let Some(step) = val.get("step_update") {
                    if let Some(thought) = step.get("thought_delta")
                        .or_else(|| step.get("reasoning_delta"))
                        .or_else(|| step.get("thinking_delta"))
                        .and_then(|v| v.as_str()) {
                        if !thought.is_empty() {
                            items.push(ParsedStreamItem::ThinkingDelta(thought.to_string()));
                        }
                    }
                    if let Some(delta) = step.get("text_delta")
                        .or_else(|| step.get("content_delta"))
                        .and_then(|v| v.as_str()) {
                        if !delta.is_empty() {
                            items.push(ParsedStreamItem::ContentDelta(delta.to_string()));
                        }
                    }
                }
            }
            "content" => {
                if let Some(chunk) = val.get("content").or_else(|| val.get("text")).and_then(|v| v.as_str()) {
                    if !chunk.is_empty() {
                        items.push(ParsedStreamItem::ContentDelta(chunk.to_string()));
                    }
                }
            }
            "message" => {
                let content = val.get("content")
                    .or_else(|| val.get("text"))
                    .or_else(|| val.get("message").and_then(|m| m.get("content").or_else(|| m.get("text"))))
                    .and_then(|v| v.as_str());
                if let Some(chunk) = content {
                    if !chunk.is_empty() {
                        items.push(ParsedStreamItem::ContentDelta(chunk.to_string()));
                    }
                }
            }
            "result" | "done" => {
                if let Some(res_obj) = val.get("result").or_else(|| Some(&val)) {
                    let status = res_obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    if status == "ERROR" || res_obj.get("error").is_some() {
                        let err_msg = res_obj.get("error")
                            .and_then(|v| if v.is_string() { v.as_str() } else { v.get("message").and_then(|m| m.as_str()) })
                            .unwrap_or("AGY worker execution error");
                        items.push(ParsedStreamItem::Error {
                            code: "AGY_ERROR".to_string(),
                            message: err_msg.to_string(),
                        });
                        return items;
                    }

                    let content = res_obj.get("response")
                        .or_else(|| res_obj.get("content"))
                        .or_else(|| res_obj.get("text"))
                        .or_else(|| res_obj.get("structured_output"))
                        .and_then(|v| if v.is_string() { v.as_str() } else { None })
                        .unwrap_or("");

                    let thinking = res_obj.get("thinking_content")
                        .or_else(|| res_obj.get("thought"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let usage = res_obj.get("usage").map(|u| UsageMetadata {
                        prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).map(|n| n as usize),
                        output_tokens: u.get("completion_tokens").or_else(|| u.get("output_tokens")).and_then(|v| v.as_u64()).map(|n| n as usize),
                    });

                    items.push(ParsedStreamItem::Done {
                        content: content.to_string(),
                        thinking_content: thinking,
                        usage,
                    });
                }
            }
            "error" => {
                let err_msg = val.get("error")
                    .or_else(|| val.get("message"))
                    .and_then(|v| if v.is_string() { v.as_str() } else { v.get("message").and_then(|m| m.as_str()) })
                    .unwrap_or("Unknown CLI error");
                items.push(ParsedStreamItem::Error {
                    code: "CLI_ERROR".to_string(),
                    message: err_msg.to_string(),
                });
            }
            _ => {}
        }

        items
    }

    /// 兼容旧版单个 chunk 返回接口
    pub fn parse_line(line: &str) -> ParsedChunk {
        let items = Self::parse_line_items(line);
        for item in items {
            match item {
                ParsedStreamItem::ThinkingDelta(d) => return ParsedChunk::ThinkingDelta(d),
                ParsedStreamItem::ContentDelta(d) => return ParsedChunk::ContentDelta(d),
                ParsedStreamItem::Done { content, .. } => return ParsedChunk::Result(content),
                ParsedStreamItem::Error { message, .. } => return ParsedChunk::Error(message),
            }
        }
        ParsedChunk::Ignored
    }
}
