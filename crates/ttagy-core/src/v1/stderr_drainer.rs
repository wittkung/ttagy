//! 异步双工非阻塞 Stderr 排空器与有界环形日志缓冲区 (Zero-Deadlock Async Stderr Drainer)

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::process::ChildStderr;
use tokio::sync::Mutex;

/// 固定容量的有界环形日志缓冲区（线程安全，防 OOM 且防管道阻塞）
#[derive(Debug, Clone)]
pub struct RollingBuffer {
    buffer: VecDeque<u8>,
    max_bytes: usize,
    total_bytes_dropped: usize,
}

impl RollingBuffer {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(max_bytes),
            max_bytes,
            total_bytes_dropped: 0,
        }
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if self.buffer.len() >= self.max_bytes {
                self.buffer.pop_front();
                self.total_bytes_dropped += 1;
            }
            self.buffer.push_back(b);
        }
    }

    pub fn to_string_lossy(&self) -> String {
        let (s1, s2) = self.buffer.as_slices();
        let mut text = String::from_utf8_lossy(s1).to_string();
        text.push_str(&String::from_utf8_lossy(s2));
        if self.total_bytes_dropped > 0 {
            format!("[... 截断前置 {} 字节 ...]\n{}", self.total_bytes_dropped, text)
        } else {
            text
        }
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn total_dropped(&self) -> usize {
        self.total_bytes_dropped
    }
}

/// 异步排空 stderr 并收集末尾日志的句柄
pub struct StderrDrainer {
    buffer: Arc<Mutex<RollingBuffer>>,
    join_handle: tokio::task::JoinHandle<()>,
}

impl StderrDrainer {
    pub fn spawn(mut stderr: ChildStderr, max_buffer_bytes: usize) -> Self {
        let buffer = Arc::new(Mutex::new(RollingBuffer::new(max_buffer_bytes)));
        let buffer_clone = buffer.clone();

        let join_handle = tokio::spawn(async move {
            let mut chunk = [0u8; 4096];
            while let Ok(n) = stderr.read(&mut chunk).await {
                if n == 0 {
                    break;
                }
                let mut buf = buffer_clone.lock().await;
                buf.push_bytes(&chunk[..n]);
            }
        });

        Self { buffer, join_handle }
    }

    /// 提取当前已收集的 stderr 文本
    pub async fn get_logs(&self) -> String {
        let buf = self.buffer.lock().await;
        buf.to_string_lossy()
    }

    /// 取消后台读取任务
    pub fn abort(&self) {
        self.join_handle.abort();
    }
}
