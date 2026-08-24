//! 模型安全校验、别名精准重定向与透明透传解析器 (Transparent Model Resolver)

/// 规范别名精准映射表
pub const CANONICAL_ALIASES: &[(&str, &str)] = &[
    ("default", "gemini-3.7-flash"),
    ("gemini", "gemini-3.7-flash"),
    ("gemini-flash", "gemini-3.7-flash"),
    ("gemini-pro", "gemini-3.1-pro"),
    ("claude", "claude-sonnet-4-6"),
    ("sonnet", "claude-sonnet-4-6"),
    ("opus", "claude-opus-4-6-thinking"),
    ("gpt", "gpt-oss-120b-medium"),
    ("oss", "gpt-oss-120b-medium"),
];

/// 解析并规范化模型名称：
/// 1. 验证安全字符集 `^[a-zA-Z0-9_.:-]+$` (防止命令注入)
/// 2. 精准别名重定向 (如 "default", "gemini" -> "gemini-3.7-flash")
/// 3. 自定义及新模型名称 100% 透明透传 (如 "claude-3.7-sonnet" 原样保留)
pub fn resolve_model_name(input: Option<&str>) -> Result<String, &'static str> {
    let raw = match input {
        Some(s) if !s.trim().is_empty() => s.trim(),
        _ => return Ok("gemini-3.7-flash".to_string()),
    };

    // 1. 安全字符检查
    if !raw.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':') {
        return Err("Invalid characters in model name");
    }

    let lower = raw.to_lowercase();

    // 2. 精准别名匹配
    for &(alias, canonical) in CANONICAL_ALIASES {
        if lower == alias {
            return Ok(canonical.to_string());
        }
    }

    // 3. 原样透传用户指定模型名称
    Ok(raw.to_string())
}
