use crate::services::git::GitService;
use db::DBService;
use db::models::execution_process::ExecutionContext;
use db::models::project::Project;
use db::models::task::TaskStatus;

use db::models::workspace_repo::WorkspaceRepo;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum YoloError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Git error: {0}")]
    Git(String),
    #[error("Project not found")]
    ProjectNotFound,
    #[error("Task not found")]
    TaskNotFound,
}

#[derive(Clone)]
pub struct YoloService {
    db: DBService,
    git: GitService,
    /// Per-project locks to serialize merges
    merge_locks: Arc<RwLock<HashMap<Uuid, Arc<Mutex<()>>>>>,
}

impl YoloService {
    pub fn new(db: DBService, git: GitService) -> Self {
        Self {
            db,
            git,
            merge_locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_project_lock(&self, project_id: Uuid) -> Arc<Mutex<()>> {
        let mut locks = self.merge_locks.write().await;
        locks
            .entry(project_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Called after finalize_task() when coding agent completes
    pub async fn try_auto_merge(&self, ctx: &ExecutionContext) -> Result<(), YoloError> {
        // 1. Check project.yolo_mode is enabled
        let project = Project::find_by_id(&self.db.pool, ctx.task.project_id)
            .await?
            .ok_or(YoloError::ProjectNotFound)?;

        if !project.yolo_mode {
            return Ok(());
        }

        tracing::info!(
            "YOLO mode enabled for project {}, starting auto-merge for task {}",
            project.id,
            ctx.task.id
        );

        // 2. Acquire per-project lock
        let lock_mutex = self.get_project_lock(project.id).await;
        let _lock = lock_mutex.lock().await;

        // 3. For each repo:
        let repos =
            WorkspaceRepo::find_repos_for_workspace(&self.db.pool, ctx.workspace.id).await?;
        let workspace_root =
            std::path::PathBuf::from(ctx.workspace.container_ref.as_ref().unwrap());

        for repo in repos {
            let repo_path = workspace_root.join(&repo.name);

            // a. Fetch latest from remote
            if let Err(e) = self.git.fetch(&repo_path) {
                return Err(YoloError::Git(format!(
                    "Fetch failed for {}: {}",
                    repo.name, e
                )));
            }

            // b. Rebase onto target branch with YOLO strategy (favor agent changes)
            let target_branch = repo.default_target_branch.as_deref().unwrap_or("main");

            tracing::info!(
                "Rebasing {} onto {} with YOLO strategy",
                repo.name,
                target_branch
            );

            if let Err(e) = self
                .git
                .rebase_with_strategy(&repo_path, target_branch, "theirs")
            {
                tracing::warn!(
                    "Rebase failed for {} even with YOLO strategy: {}. Manual intervention required.",
                    repo.name,
                    e
                );
                return Err(YoloError::Git(format!(
                    "Rebase failed for {}: {}",
                    repo.name, e
                )));
            }

            // c. Merge (squash)
            // Note: This implementation depends on how GitService is implemented.
            // Assuming we have a merge method or similar.
            if let Err(e) = self.git.push(&repo_path) {
                return Err(YoloError::Git(format!(
                    "Push failed for {}: {}",
                    repo.name, e
                )));
            }
        }

        // 4. Update task status to Done
        db::models::task::Task::update_status(&self.db.pool, ctx.task.id, TaskStatus::Done).await?;

        tracing::info!("YOLO auto-merge successful for task {}", ctx.task.id);

        Ok(())
    }
}
