//! NDJSON 流式行事件解析器

use serde_json::Value;

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
    pub fn parse_line(line: &str) -> ParsedChunk {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return ParsedChunk::Ignored;
        }

        let val: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => return ParsedChunk::Ignored,
        };

        let ev_name = val.get("event")
            .or_else(|| val.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if ev_name == "step_update" {
            if let Some(step) = val.get("step_update") {
                if let Some(thought) = step.get("thought_delta")
                    .or_else(|| step.get("reasoning_delta"))
                    .or_else(|| step.get("thinking_delta"))
                    .and_then(|v| v.as_str()) {
                    return ParsedChunk::ThinkingDelta(thought.to_string());
                }
                if let Some(delta) = step.get("text_delta")
                    .or_else(|| step.get("content_delta"))
                    .and_then(|v| v.as_str()) {
                    return ParsedChunk::ContentDelta(delta.to_string());
                }
            }
        } else if ev_name == "content" || ev_name == "message" {
            if let Some(chunk) = val.get("content").or_else(|| val.get("text")).and_then(|v| v.as_str()) {
                return ParsedChunk::ContentDelta(chunk.to_string());
            }
        } else if ev_name == "result" {
            if let Some(res_obj) = val.get("result") {
                if let Some(err) = res_obj.get("error").and_then(|v| v.as_str()) {
                    if !err.is_empty() {
                        return ParsedChunk::Error(err.to_string());
                    }
                }
                let status = res_obj.get("status").and_then(|v| v.as_str()).unwrap_or("");
                if status == "ERROR" {
                    let err = res_obj.get("error").and_then(|v| v.as_str()).unwrap_or("AGY worker execution error");
                    return ParsedChunk::Error(err.to_string());
                }
                let content = res_obj.get("response")
                    .or_else(|| res_obj.get("content"))
                    .or_else(|| res_obj.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                return ParsedChunk::Result(content.to_string());
            }
        }

        ParsedChunk::Ignored
    }
}
