//! Antigravity CLI 二进制探查器

use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct TtagyDetector;

impl TtagyDetector {
    /// 自动发现本地可用 agy 二进制路径
    pub fn find_binary() -> Option<PathBuf> {
        if let Ok(env_path) = std::env::var("AGY_PATH") {
            let p = PathBuf::from(env_path);
            if p.is_file() {
                return Some(p);
            }
        }
        if let Ok(output) = std::process::Command::new("which").arg("agy").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    return Some(PathBuf::from(path_str));
                }
            }
        }
        let mut candidates = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            candidates.push(PathBuf::from(&home).join(".local/bin/agy"));
            candidates.push(PathBuf::from(&home).join("bin/agy"));
        }
        candidates.push(PathBuf::from("/usr/local/bin/agy"));
        candidates.push(PathBuf::from("/opt/homebrew/bin/agy"));
        candidates.into_iter().find(|p| p.is_file())
    }

    /// 验证二进制是否具备可用性
    pub async fn is_available() -> bool {
        let binary = match Self::find_binary() {
            Some(b) => b,
            None => return false,
        };
        match timeout(Duration::from_secs(3), Command::new(&binary).arg("--help").output()).await {
            Ok(Ok(out)) => out.status.success(),
            _ => false,
        }
    }
}
