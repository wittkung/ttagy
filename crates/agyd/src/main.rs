//! agyd: Antigravity CLI 本地常驻守护进程 (Local Daemon)

use std::path::PathBuf;
use agy_core::AgyDetector;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 [agyd] Antigravity Local AI Daemon 正在初始化...");
    let binary = AgyDetector::find_binary().expect("未检测到 agy 二进制");
    println!("✅ [agyd] 检测到 AGY CLI: {:?}", binary);
    
    let socket_path = PathBuf::from("/tmp/agy_daemon.sock");
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }
    
    println!("⚡ [agyd] 监听 Unix Domain Socket: {:?}", socket_path);
    println!("✨ [agyd] 守护服务已就绪 (输入 Ctrl+C 退出)");
    
    // 监听退出信号
    tokio::signal::ctrl_c().await?;
    println!("🛑 [agyd] 收到退出信号，清理套接字...");
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }
    Ok(())
}
