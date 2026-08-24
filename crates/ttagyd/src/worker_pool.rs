//! TTAgy Sub-2ms Warm Worker Pool & Process Duplexing Controller

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{mpsc, Mutex, OwnedSemaphorePermit, Semaphore};
use ttagy_core::v1::{NdjsonParser, ParsedStreamItem, TtagyRequest, TtagyStreamEvent};
use ttagy_core::StderrDrainer;

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub binary_path: PathBuf,
    pub min_idle: usize,
    pub max_capacity: usize,
    pub max_turns_per_worker: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::from("agy"),
            min_idle: 2,
            max_capacity: 8,
            max_turns_per_worker: 100,
        }
    }
}

pub struct WorkerInstance {
    pub id: usize,
    child: Child,
    stdin: ChildStdin,
    stdout_lines: Lines<BufReader<ChildStdout>>,
    _stderr_drainer: StderrDrainer,
    pub turn_count: usize,
    pub created_at: Instant,
    pub is_healthy: bool,
}

impl WorkerInstance {
    pub async fn spawn(id: usize, binary: &Path) -> Result<Self, String> {
        let mut cmd = Command::new(binary);
        cmd.arg("worker")
            .arg("--input-format")
            .arg("stream-json")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--disable-slash-commands")
            .arg("--dangerously-skip-permissions")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        #[cfg(unix)]
        cmd.process_group(0);

        let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn worker: {}", e))?;
        let stdin = child.stdin.take().ok_or("Failed to open worker stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open worker stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to open worker stderr")?;

        let stderr_drainer = StderrDrainer::spawn(stderr, 64 * 1024);
        let stdout_lines = BufReader::new(stdout).lines();

        Ok(Self {
            id,
            child,
            stdin,
            stdout_lines,
            _stderr_drainer: stderr_drainer,
            turn_count: 0,
            created_at: Instant::now(),
            is_healthy: true,
        })
    }

    pub async fn execute_request(
        &mut self,
        req: &TtagyRequest,
        tx: &mpsc::Sender<Result<axum::response::sse::Event, std::convert::Infallible>>,
        cancel_token: &tokio_util::sync::CancellationToken,
    ) -> Result<(), String> {
        let req_json = serde_json::to_string(req).map_err(|e| e.to_string())?;
        self.stdin
            .write_all(format!("{}\n", req_json).as_bytes())
            .await
            .map_err(|e| {
                self.is_healthy = false;
                format!("Failed to write to worker stdin: {}", e)
            })?;
        self.stdin.flush().await.map_err(|e| e.to_string())?;

        self.turn_count += 1;
        let start_time = Instant::now();
        let timeout_duration = Duration::from_secs(req.timeout_secs.max(10));

        loop {
            tokio::select! {
                biased;

                _ = cancel_token.cancelled() => {
                    let _ = self.stdin.write_all(b"{\"type\":\"cancel\"}\n").await;
                    let _ = self.stdin.flush().await;
                    return Ok(());
                }

                _ = tx.closed() => {
                    let _ = self.stdin.write_all(b"{\"type\":\"cancel\"}\n").await;
                    let _ = self.stdin.flush().await;
                    return Ok(());
                }

                line_res = tokio::time::timeout(timeout_duration, self.stdout_lines.next_line()) => {
                    match line_res {
                        Ok(Ok(Some(line))) => {
                            let items = NdjsonParser::parse_line_items(&line);
                            let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;

                            for item in items {
                                match item {
                                    ParsedStreamItem::ThinkingDelta(delta) => {
                                        let ev = TtagyStreamEvent::ThinkingDelta {
                                            session_id: req.session_id.clone(),
                                            text_delta: delta,
                                            elapsed_ms,
                                        };
                                        if let Ok(js) = serde_json::to_string(&ev) {
                                            let _ = tx.send(Ok(axum::response::sse::Event::default().data(js))).await;
                                        }
                                    }
                                    ParsedStreamItem::ContentDelta(delta) => {
                                        let ev = TtagyStreamEvent::ContentDelta {
                                            session_id: req.session_id.clone(),
                                            text_delta: delta,
                                            accumulated_chars: 0,
                                            elapsed_ms,
                                        };
                                        if let Ok(js) = serde_json::to_string(&ev) {
                                            let _ = tx.send(Ok(axum::response::sse::Event::default().data(js))).await;
                                        }
                                    }
                                    ParsedStreamItem::Done { content, thinking_content, usage } => {
                                        let ev = TtagyStreamEvent::Done {
                                            session_id: req.session_id.clone(),
                                            full_content: content,
                                            thinking_content,
                                            elapsed_ms,
                                            prompt_tokens: usage.as_ref().and_then(|u| u.prompt_tokens),
                                            output_tokens: usage.as_ref().and_then(|u| u.output_tokens),
                                        };
                                        if let Ok(js) = serde_json::to_string(&ev) {
                                            let _ = tx.send(Ok(axum::response::sse::Event::default().data(js))).await;
                                        }
                                        return Ok(());
                                    }
                                    ParsedStreamItem::Error { code, message } => {
                                        let ev = TtagyStreamEvent::Error {
                                            session_id: req.session_id.clone(),
                                            error_code: code,
                                            error_message: message,
                                            is_retryable: false,
                                        };
                                        if let Ok(js) = serde_json::to_string(&ev) {
                                            let _ = tx.send(Ok(axum::response::sse::Event::default().data(js))).await;
                                        }
                                        return Ok(());
                                    }
                                }
                            }
                        }
                        Ok(Ok(None)) => {
                            self.is_healthy = false;
                            return Err("Worker EOF received".into());
                        }
                        Ok(Err(e)) => {
                            self.is_healthy = false;
                            return Err(format!("Worker IO error: {}", e));
                        }
                        Err(_) => {
                            self.is_healthy = false;
                            return Err("Worker response timed out".into());
                        }
                    }
                }
            }
        }
    }
}

pub struct WorkerPool {
    config: PoolConfig,
    idle_workers: Arc<Mutex<Vec<WorkerInstance>>>,
    semaphore: Arc<Semaphore>,
    worker_counter: AtomicUsize,
}

impl WorkerPool {
    pub async fn new(config: PoolConfig) -> Arc<Self> {
        let pool = Arc::new(Self {
            semaphore: Arc::new(Semaphore::new(config.max_capacity)),
            idle_workers: Arc::new(Mutex::new(Vec::with_capacity(config.max_capacity))),
            worker_counter: AtomicUsize::new(1),
            config,
        });

        for _ in 0..pool.config.min_idle {
            let id = pool.worker_counter.fetch_add(1, Ordering::Relaxed);
            if let Ok(worker) = WorkerInstance::spawn(id, &pool.config.binary_path).await {
                pool.idle_workers.lock().await.push(worker);
            }
        }

        pool
    }

    pub async fn acquire(self: &Arc<Self>) -> Result<PooledWorkerGuard, String> {
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| "Pool semaphore closed".to_string())?;

        let mut idle = self.idle_workers.lock().await;
        if let Some(worker) = idle.pop() {
            return Ok(PooledWorkerGuard {
                worker: Some(worker),
                pool: self.clone(),
                _permit: permit,
            });
        }
        drop(idle);

        let id = self.worker_counter.fetch_add(1, Ordering::Relaxed);
        let worker = WorkerInstance::spawn(id, &self.config.binary_path).await?;
        Ok(PooledWorkerGuard {
            worker: Some(worker),
            pool: self.clone(),
            _permit: permit,
        })
    }

    pub async fn recycle(&self, mut worker: WorkerInstance) {
        if !worker.is_healthy || worker.turn_count >= self.config.max_turns_per_worker {
            let _ = worker.child.kill().await;
            self.replenish().await;
            return;
        }

        let mut idle = self.idle_workers.lock().await;
        if idle.len() < self.config.max_capacity {
            idle.push(worker);
        } else {
            let _ = worker.child.kill().await;
        }
    }

    async fn replenish(&self) {
        let pool_clone = self.config.clone();
        let idle_clone = self.idle_workers.clone();
        let counter = self.worker_counter.fetch_add(1, Ordering::Relaxed);

        tokio::spawn(async move {
            if let Ok(w) = WorkerInstance::spawn(counter, &pool_clone.binary_path).await {
                idle_clone.lock().await.push(w);
            }
        });
    }
}

pub struct PooledWorkerGuard {
    worker: Option<WorkerInstance>,
    pool: Arc<WorkerPool>,
    _permit: OwnedSemaphorePermit,
}

impl std::ops::Deref for PooledWorkerGuard {
    type Target = WorkerInstance;
    fn deref(&self) -> &Self::Target {
        self.worker.as_ref().unwrap()
    }
}

impl std::ops::DerefMut for PooledWorkerGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.worker.as_mut().unwrap()
    }
}

impl Drop for PooledWorkerGuard {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let pool = self.pool.clone();
            tokio::spawn(async move {
                pool.recycle(worker).await;
            });
        }
    }
}
