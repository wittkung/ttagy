//! 沙箱管理器：强制在独立临时目录运行并注入独立日志，消除 35k Tokens 目录树泄漏与文件锁冲突

use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug)]
pub struct SandboxGuard {
    pub sandbox_path: PathBuf,
    pub log_path: PathBuf,
    pub auto_cleanup: bool,
}

impl SandboxGuard {
    pub fn create(prefix: &str, auto_cleanup: bool) -> std::io::Result<Self> {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir_name = format!("{}_{}", prefix, ts);
        let sandbox_path = std::env::temp_dir().join("local_ai_sandboxes").join(dir_name);
        std::fs::create_dir_all(&sandbox_path)?;
        let log_path = sandbox_path.join("agy_execution.log");
        Ok(Self {
            sandbox_path,
            log_path,
            auto_cleanup,
        })
    }
}

impl Drop for SandboxGuard {
    fn drop(&mut self) {
        if self.auto_cleanup && self.sandbox_path.exists() {
            let _ = std::fs::remove_dir_all(&self.sandbox_path);
        }
    }
}
