//! TTAgy Security & Secret Redaction Engine

pub struct RedactionEngine;

impl RedactionEngine {
    pub fn sanitize(input: &str) -> String {
        let mut result = input.to_string();

        // 1. Sanitize Bearer tokens
        if let Some(pos) = result.find("Bearer ") {
            let start = pos + 7;
            let end = result[start..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                .map(|p| start + p)
                .unwrap_or(result.len());
            if end > start {
                let token_slice = &result[start..end];
                if token_slice.len() > 4 {
                    result = format!("{}[REDACTED:BEARER_TOKEN]{}", &result[..start], &result[end..]);
                }
            }
        }

        // 2. Sanitize sk- OpenAI / Anthropic style API keys
        if let Some(pos) = result.find("sk-") {
            let start = pos;
            let end = result[start..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                .map(|p| start + p)
                .unwrap_or(result.len());
            if end > start + 3 {
                result = format!("{}[REDACTED:API_KEY]{}", &result[..start], &result[end..]);
            }
        }

        // 3. Sanitize ghp_ GitHub tokens
        if let Some(pos) = result.find("ghp_") {
            let start = pos;
            let end = result[start..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                .map(|p| start + p)
                .unwrap_or(result.len());
            if end > start + 4 {
                result = format!("{}[REDACTED:GITHUB_TOKEN]{}", &result[..start], &result[end..]);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_bearer() {
        let raw = "Authorization: Bearer secret_token_123456 in header";
        let clean = RedactionEngine::sanitize(raw);
        assert_eq!(clean, "Authorization: Bearer [REDACTED:BEARER_TOKEN] in header");
    }

    #[test]
    fn test_sanitize_sk_api_key() {
        let raw = "Connecting with API key sk-ant-api03-abcdef123456 for LLM";
        let clean = RedactionEngine::sanitize(raw);
        assert_eq!(clean, "Connecting with API key [REDACTED:API_KEY] for LLM");
    }
}
