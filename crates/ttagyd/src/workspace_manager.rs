//! TTAgy Workspace Isolation Engine: 基于 Git Worktree 的动态分支工作区隔离与沙箱管理

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceMode {
    Inherit,
    Branch,
    Share,
}

#[derive(Debug)]
struct CleanupTask {
    repo_root: PathBuf,
    worktree_path: PathBuf,
    branch_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub id: String,
    pub mode: WorkspaceMode,
    pub path: PathBuf,
    pub branch_name: Option<String>,
}

pub struct WorkspaceGuard {
    pub info: WorkspaceInfo,
    repo_root: PathBuf,
    gc_tx: mpsc::UnboundedSender<CleanupTask>,
    is_cleaned: Arc<AtomicBool>,
}

impl WorkspaceGuard {
    pub fn environment_variables(&self) -> Vec<(String, String)> {
        let mut envs = Vec::new();
        match &self.info.mode {
            WorkspaceMode::Inherit => {
                envs.push(("AGY_WORKSPACE_MODE".into(), "inherit".into()));
                envs.push((
                    "AGY_WORKSPACE_ROOT".into(),
                    self.info.path.to_string_lossy().into(),
                ));
            }
            WorkspaceMode::Branch => {
                envs.push(("AGY_WORKSPACE_MODE".into(), "branch".into()));
                envs.push((
                    "AGY_WORKSPACE_ROOT".into(),
                    self.info.path.to_string_lossy().into(),
                ));
                if let Some(ref br) = self.info.branch_name {
                    envs.push(("AGY_BRANCH_NAME".into(), br.clone()));
                }
                envs.push((
                    "GIT_WORK_TREE".into(),
                    self.info.path.to_string_lossy().into(),
                ));
            }
            WorkspaceMode::Share => {
                envs.push(("AGY_WORKSPACE_MODE".into(), "share".into()));
                envs.push((
                    "AGY_WORKSPACE_ROOT".into(),
                    self.info.path.to_string_lossy().into(),
                ));
            }
        }
        envs
    }

    pub async fn cleanup_now(self) -> Result<(), String> {
        if self.is_cleaned.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        WorkspaceManager::execute_cleanup(
            &self.repo_root,
            &self.info.path,
            self.info.branch_name.as_deref(),
        )
        .await
    }
}

impl Drop for WorkspaceGuard {
    fn drop(&mut self) {
        if !self.is_cleaned.swap(true, Ordering::SeqCst) {
            if matches!(self.info.mode, WorkspaceMode::Branch | WorkspaceMode::Share) {
                let task = CleanupTask {
                    repo_root: self.repo_root.clone(),
                    worktree_path: self.info.path.clone(),
                    branch_name: self.info.branch_name.clone(),
                };
                let _ = self.gc_tx.send(task);
            }
        }
    }
}

pub struct WorkspaceManager {
    repo_root: PathBuf,
    base_worktree_dir: PathBuf,
    gc_tx: mpsc::UnboundedSender<CleanupTask>,
}

impl WorkspaceManager {
    pub fn new(repo_root: PathBuf, base_worktree_dir: PathBuf) -> Arc<Self> {
        let (gc_tx, mut gc_rx) = mpsc::unbounded_channel::<CleanupTask>();

        let manager = Arc::new(Self {
            repo_root: repo_root.clone(),
            base_worktree_dir,
            gc_tx,
        });

        tokio::spawn(async move {
            while let Some(task) = gc_rx.recv().await {
                let _ = Self::execute_cleanup(
                    &task.repo_root,
                    &task.worktree_path,
                    task.branch_name.as_deref(),
                )
                .await;
            }
        });

        manager
    }

    pub async fn provision(
        &self,
        mode: WorkspaceMode,
        parent_path: Option<&Path>,
    ) -> Result<WorkspaceGuard, String> {
        let id = Uuid::new_v4().to_string();

        match mode {
            WorkspaceMode::Inherit => {
                let target_path = parent_path
                    .map(PathBuf::from)
                    .unwrap_or_else(|| self.repo_root.clone());
                Ok(WorkspaceGuard {
                    info: WorkspaceInfo {
                        id,
                        mode: WorkspaceMode::Inherit,
                        path: target_path,
                        branch_name: None,
                    },
                    repo_root: self.repo_root.clone(),
                    gc_tx: self.gc_tx.clone(),
                    is_cleaned: Arc::new(AtomicBool::new(false)),
                })
            }

            WorkspaceMode::Branch => {
                let branch = format!("ttagy/sandbox/{}", id);
                let target_path = self.base_worktree_dir.join(&id);

                tokio::fs::create_dir_all(&self.base_worktree_dir)
                    .await
                    .map_err(|e| e.to_string())?;

                // 执行 git worktree add
                let add_res = Self::run_git_cmd(
                    &self.repo_root,
                    &[
                        "worktree",
                        "add",
                        target_path.to_str().unwrap_or_default(),
                        "-b",
                        &branch,
                        "HEAD",
                    ],
                )
                .await;

                if add_res.is_err() {
                    // Fallback to simple directory creation if not inside a git repo
                    let _ = tokio::fs::create_dir_all(&target_path).await;
                }

                Ok(WorkspaceGuard {
                    info: WorkspaceInfo {
                        id,
                        mode: WorkspaceMode::Branch,
                        path: target_path,
                        branch_name: Some(branch),
                    },
                    repo_root: self.repo_root.clone(),
                    gc_tx: self.gc_tx.clone(),
                    is_cleaned: Arc::new(AtomicBool::new(false)),
                })
            }

            WorkspaceMode::Share => {
                let target_path = self.base_worktree_dir.join("shared");
                let branch = "ttagy/shared/main".to_string();

                if !target_path.exists() {
                    let _ = tokio::fs::create_dir_all(&target_path).await;
                }

                Ok(WorkspaceGuard {
                    info: WorkspaceInfo {
                        id,
                        mode: WorkspaceMode::Share,
                        path: target_path,
                        branch_name: Some(branch),
                    },
                    repo_root: self.repo_root.clone(),
                    gc_tx: self.gc_tx.clone(),
                    is_cleaned: Arc::new(AtomicBool::new(false)),
                })
            }
        }
    }

    pub async fn execute_cleanup(
        repo_root: &Path,
        worktree_path: &Path,
        branch_name: Option<&str>,
    ) -> Result<(), String> {
        let path_str = worktree_path.to_str().unwrap_or_default();
        let _ = Self::run_git_cmd(repo_root, &["worktree", "unlock", path_str]).await;
        let _ = Self::run_git_cmd(repo_root, &["worktree", "remove", "--force", path_str]).await;
        let _ = Self::run_git_cmd(repo_root, &["worktree", "prune", "--expire=now"]).await;

        if let Some(branch) = branch_name {
            let _ = Self::run_git_cmd(repo_root, &["branch", "-D", branch]).await;
        }

        if worktree_path.exists() {
            let _ = tokio::fs::remove_dir_all(worktree_path).await;
        }
        Ok(())
    }

    pub async fn reconcile_orphans(&self) -> Result<(), String> {
        let _ = Self::run_git_cmd(&self.repo_root, &["worktree", "prune", "--expire=now"]).await;

        if self.base_worktree_dir.exists() {
            if let Ok(mut entries) = tokio::fs::read_dir(&self.base_worktree_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_dir() {
                        let _ = tokio::fs::remove_dir_all(&path).await;
                    }
                }
            }
        }
        Ok(())
    }

    async fn run_git_cmd(cwd: &Path, args: &[&str]) -> Result<String, String> {
        let output = Command::new("git")
            .current_dir(cwd)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
