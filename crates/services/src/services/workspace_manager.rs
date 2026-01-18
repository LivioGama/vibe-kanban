use std::path::{Path, PathBuf};

use db::models::{repo::Repo, workspace::Workspace as DbWorkspace};
use sqlx::{Pool, Sqlite};
use thiserror::Error;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::{
    git::GitService,
    jj_workspace_manager::{JjWorkspaceManager, JjWorkspaceError, RepoJjSession},
    worktree_manager::{WorktreeCleanup, WorktreeError, WorktreeManager},
};

#[derive(Debug, Clone)]
pub struct RepoWorkspaceInput {
    pub repo: Repo,
    pub target_branch: String,
}

impl RepoWorkspaceInput {
    pub fn new(repo: Repo, target_branch: String) -> Self {
        Self {
            repo,
            target_branch,
        }
    }
}

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error(transparent)]
    Worktree(#[from] WorktreeError),
    #[error(transparent)]
    JjWorkspace(#[from] JjWorkspaceError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("No repositories provided")]
    NoRepositories,
    #[error("Partial workspace creation failed: {0}")]
    PartialCreation(String),
}

/// VCS type for a repository
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsType {
    Git,
    Jj,
}

/// Info about a single repo's worktree within a workspace
#[derive(Debug, Clone)]
pub struct RepoWorktree {
    pub repo_id: Uuid,
    pub repo_name: String,
    pub source_repo_path: PathBuf,
    pub worktree_path: PathBuf,
    pub vcs_type: VcsType,
    pub jj_change_id: Option<String>,
}

/// A container directory holding worktrees for all project repos
#[derive(Debug, Clone)]
pub struct WorktreeContainer {
    pub workspace_dir: PathBuf,
    pub worktrees: Vec<RepoWorktree>,
}

pub struct WorkspaceManager;

impl WorkspaceManager {
    /// Check if a repository is a jj repository
    pub fn is_jj_repo(repo_path: &Path) -> bool {
        let jj_manager = JjWorkspaceManager::new();
        jj_manager.is_jj_available() && jj_manager.is_jj_repo(repo_path).unwrap_or(false)
    }

    /// Detect if all repositories in the project are jj repositories
    pub fn are_all_jj_repos(repos: &[RepoWorkspaceInput]) -> bool {
        repos.iter().all(|repo| Self::is_jj_repo(&repo.repo.path))
    }

    /// Create jj sessions for all repositories
    /// This is the killer feature: all agents work in same directory with separate changes!
    pub async fn create_jj_sessions(
        repos: &[RepoWorkspaceInput],
        session_id: Uuid,
    ) -> Result<Vec<RepoJjSession>, WorkspaceError> {
        if repos.is_empty() {
            return Err(WorkspaceError::NoRepositories);
        }

        info!(
            "Creating jj sessions for {} repositories (session {})",
            repos.len(),
            session_id
        );

        let jj_manager = JjWorkspaceManager::new();
        let mut sessions = Vec::new();

        for input in repos {
            let change_id = jj_manager
                .create_session(&input.repo.path, session_id, None)
                .map_err(WorkspaceError::JjWorkspace)?;

            sessions.push(RepoJjSession {
                repo_id: input.repo.id,
                repo_name: input.repo.name.clone(),
                repo_path: input.repo.path.clone(),
                change_id,
                session_id,
            });

            info!(
                "Created jj session for repo '{}' with change ID: {}",
                input.repo.name, sessions.last().unwrap().change_id
            );
        }

        info!(
            "Successfully created {} jj sessions (all in same directories!)",
            sessions.len()
        );

        Ok(sessions)
    }

    /// Cleanup jj sessions by abandoning changes
    pub async fn cleanup_jj_sessions(sessions: &[RepoJjSession]) -> Result<(), WorkspaceError> {
        info!("Cleaning up {} jj sessions", sessions.len());

        let jj_manager = JjWorkspaceManager::new();
        jj_manager
            .batch_cleanup_sessions(sessions)
            .map_err(WorkspaceError::JjWorkspace)?;

        Ok(())
    }

    /// Detect the VCS type for a repository
    fn detect_vcs_type(repo_path: &Path) -> VcsType {
        let git_service = GitService::new();
        if git_service.is_jj_repo(repo_path).unwrap_or(false) {
            VcsType::Jj
        } else {
            VcsType::Git
        }
    }

    /// Create a workspace with worktrees for all repositories.
    /// On failure, rolls back any already-created worktrees.
    pub async fn create_workspace(
        workspace_dir: &Path,
        repos: &[RepoWorkspaceInput],
        branch_name: &str,
    ) -> Result<WorktreeContainer, WorkspaceError> {
        if repos.is_empty() {
            return Err(WorkspaceError::NoRepositories);
        }

        info!(
            "Creating workspace at {} with {} repositories",
            workspace_dir.display(),
            repos.len()
        );

        tokio::fs::create_dir_all(workspace_dir).await?;

        let mut created_worktrees: Vec<RepoWorktree> = Vec::new();

        for input in repos {
            let vcs_type = Self::detect_vcs_type(&input.repo.path);
            let worktree_path = workspace_dir.join(&input.repo.name);

            debug!(
                "Creating workspace for repo '{}' at {} (VCS: {:?})",
                input.repo.name,
                worktree_path.display(),
                vcs_type
            );

            let result = match vcs_type {
                VcsType::Git => {
                    // Use existing worktree logic
                    WorktreeManager::create_worktree(
                        &input.repo.path,
                        branch_name,
                        &worktree_path,
                        &input.target_branch,
                        true,
                    )
                    .await
                    .map(|_| None)
                    .map_err(WorkspaceError::Worktree)
                }
                VcsType::Jj => {
                    // Create a new jj change instead of a worktree
                    Self::create_jj_workspace(&input.repo.path, branch_name).await
                }
            };

            match result {
                Ok(jj_change_id) => {
                    created_worktrees.push(RepoWorktree {
                        repo_id: input.repo.id,
                        repo_name: input.repo.name.clone(),
                        source_repo_path: input.repo.path.clone(),
                        worktree_path: if vcs_type == VcsType::Jj {
                            // For jj, worktree_path is the repo itself
                            input.repo.path.clone()
                        } else {
                            worktree_path
                        },
                        vcs_type,
                        jj_change_id,
                    });
                }
                Err(e) => {
                    error!(
                        "Failed to create workspace for repo '{}': {}. Rolling back...",
                        input.repo.name, e
                    );

                    // Rollback: cleanup all worktrees we've created so far
                    Self::cleanup_created_worktrees(&created_worktrees).await;

                    // Also remove the workspace directory if it's empty
                    if let Err(cleanup_err) = tokio::fs::remove_dir(workspace_dir).await {
                        debug!(
                            "Could not remove workspace dir during rollback: {}",
                            cleanup_err
                        );
                    }

                    return Err(WorkspaceError::PartialCreation(format!(
                        "Failed to create workspace for repo '{}': {}",
                        input.repo.name, e
                    )));
                }
            }
        }

        info!(
            "Successfully created workspace with {} worktrees",
            created_worktrees.len()
        );

        Ok(WorktreeContainer {
            workspace_dir: workspace_dir.to_path_buf(),
            worktrees: created_worktrees,
        })
    }

    /// Ensure all worktrees in a workspace exist (for cold restart scenarios)
    pub async fn ensure_workspace_exists(
        workspace_dir: &Path,
        repos: &[Repo],
        branch_name: &str,
    ) -> Result<(), WorkspaceError> {
        if repos.is_empty() {
            return Err(WorkspaceError::NoRepositories);
        }

        // Try legacy migration first (single repo projects only)
        // Old layout had worktree directly at workspace_dir; new layout has it at workspace_dir/{repo_name}
        if repos.len() == 1 && Self::migrate_legacy_worktree(workspace_dir, &repos[0]).await? {
            return Ok(());
        }

        if !workspace_dir.exists() {
            tokio::fs::create_dir_all(workspace_dir).await?;
        }

        for repo in repos {
            let vcs_type = Self::detect_vcs_type(&repo.path);
            
            match vcs_type {
                VcsType::Git => {
                    let worktree_path = workspace_dir.join(&repo.name);

                    debug!(
                        "Ensuring worktree exists for repo '{}' at {}",
                        repo.name,
                        worktree_path.display()
                    );

                    WorktreeManager::ensure_worktree_exists(&repo.path, branch_name, &worktree_path)
                        .await?;
                }
                VcsType::Jj => {
                    // For jj repos, we don't need to ensure anything exists
                    // The workspace is the repo itself
                    debug!(
                        "Jj repo '{}' workspace is the repo itself at {}",
                        repo.name,
                        repo.path.display()
                    );
                }
            }
        }

        Ok(())
    }

    /// Create a jj workspace by creating a new change
    async fn create_jj_workspace(
        repo_path: &Path,
        branch_name: &str,
    ) -> Result<Option<String>, WorkspaceError> {
        let jj_manager = JjWorkspaceManager::new();
        let session_id = Uuid::new_v4(); // Generate a session ID for tracking
        let description = Some(format!("workspace: {}", branch_name));

        let change_id = jj_manager
            .create_session(repo_path, session_id, description.as_deref())
            .map_err(WorkspaceError::JjWorkspace)?;

        info!(
            "Created jj change {} for workspace in repo {}",
            change_id,
            repo_path.display()
        );

        Ok(Some(change_id))
    }

    /// Clean up all worktrees in a workspace
    pub async fn cleanup_workspace(
        workspace_dir: &Path,
        repos: &[Repo],
    ) -> Result<(), WorkspaceError> {
        info!("Cleaning up workspace at {}", workspace_dir.display());

        for repo in repos {
            let vcs_type = Self::detect_vcs_type(&repo.path);
            
            match vcs_type {
                VcsType::Git => {
                    let worktree_path = workspace_dir.join(&repo.name);
                    let cleanup = WorktreeCleanup::new(worktree_path, Some(repo.path.clone()));
                    WorktreeManager::cleanup_worktree(&cleanup).await?;
                }
                VcsType::Jj => {
                    // For jj, we don't need to clean up worktrees
                    // The change will remain in the repo history
                    debug!(
                        "Skipping worktree cleanup for jj repo '{}' (changes remain in history)",
                        repo.name
                    );
                }
            }
        }

        // Remove the workspace directory itself
        if workspace_dir.exists()
            && let Err(e) = tokio::fs::remove_dir_all(workspace_dir).await
        {
            debug!(
                "Could not remove workspace directory {}: {}",
                workspace_dir.display(),
                e
            );
        }

        Ok(())
    }

    /// Get the base directory for workspaces (same as worktree base dir)
    pub fn get_workspace_base_dir() -> PathBuf {
        WorktreeManager::get_worktree_base_dir()
    }

    /// Migrate a legacy single-worktree layout to the new workspace layout.
    /// Old layout: workspace_dir IS the worktree
    /// New layout: workspace_dir contains worktrees at workspace_dir/{repo_name}
    ///
    /// Returns Ok(true) if migration was performed, Ok(false) if no migration needed.
    pub async fn migrate_legacy_worktree(
        workspace_dir: &Path,
        repo: &Repo,
    ) -> Result<bool, WorkspaceError> {
        let expected_worktree_path = workspace_dir.join(&repo.name);

        // Detect old-style: workspace_dir exists AND has .git file (worktree marker)
        // AND expected new location doesn't exist
        let git_file = workspace_dir.join(".git");
        let is_old_style = workspace_dir.exists()
            && git_file.exists()
            && git_file.is_file() // .git file = worktree, .git dir = main repo
            && !expected_worktree_path.exists();

        if !is_old_style {
            return Ok(false);
        }

        info!(
            "Detected legacy worktree at {}, migrating to new layout",
            workspace_dir.display()
        );

        // Move old worktree to temp location (can't move into subdirectory of itself)
        let temp_name = format!(
            "{}-migrating",
            workspace_dir
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default()
        );
        let temp_path = workspace_dir.with_file_name(temp_name);

        WorktreeManager::move_worktree(&repo.path, workspace_dir, &temp_path).await?;

        // Create new workspace directory
        tokio::fs::create_dir_all(workspace_dir).await?;

        // Move worktree to final location using git worktree move
        WorktreeManager::move_worktree(&repo.path, &temp_path, &expected_worktree_path).await?;

        if temp_path.exists() {
            let _ = tokio::fs::remove_dir_all(&temp_path).await;
        }

        info!(
            "Successfully migrated legacy worktree to {}",
            expected_worktree_path.display()
        );

        Ok(true)
    }

    /// Helper to cleanup worktrees during rollback
    async fn cleanup_created_worktrees(worktrees: &[RepoWorktree]) {
        for worktree in worktrees {
            match worktree.vcs_type {
                VcsType::Git => {
                    let cleanup = WorktreeCleanup::new(
                        worktree.worktree_path.clone(),
                        Some(worktree.source_repo_path.clone()),
                    );

                    if let Err(e) = WorktreeManager::cleanup_worktree(&cleanup).await {
                        error!(
                            "Failed to cleanup worktree '{}' during rollback: {}",
                            worktree.repo_name, e
                        );
                    }
                }
                VcsType::Jj => {
                    // For jj, abandon the change if we have a change ID
                    if let Some(change_id) = &worktree.jj_change_id {
                        let jj = JujutsuCli::new();
                        if let Err(e) = jj.abandon(&worktree.source_repo_path, change_id) {
                            error!(
                                "Failed to abandon jj change '{}' for '{}' during rollback: {}",
                                change_id, worktree.repo_name, e
                            );
                        }
                    }
                }
            }
        }
    }

    pub async fn cleanup_orphan_workspaces(db: &Pool<Sqlite>) {
        if std::env::var("DISABLE_WORKTREE_ORPHAN_CLEANUP").is_ok() {
            debug!(
                "Orphan workspace cleanup is disabled via DISABLE_WORKTREE_ORPHAN_CLEANUP environment variable"
            );
            return;
        }

        let workspace_base_dir = Self::get_workspace_base_dir();
        if !workspace_base_dir.exists() {
            debug!(
                "Workspace base directory {} does not exist, skipping orphan cleanup",
                workspace_base_dir.display()
            );
            return;
        }

        let entries = match std::fs::read_dir(&workspace_base_dir) {
            Ok(entries) => entries,
            Err(e) => {
                error!(
                    "Failed to read workspace base directory {}: {}",
                    workspace_base_dir.display(),
                    e
                );
                return;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    warn!("Failed to read directory entry: {}", e);
                    continue;
                }
            };

            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let workspace_path_str = path.to_string_lossy().to_string();
            if let Ok(false) = DbWorkspace::container_ref_exists(db, &workspace_path_str).await {
                info!("Found orphaned workspace: {}", workspace_path_str);
                if let Err(e) = Self::cleanup_workspace_without_repos(&path).await {
                    error!(
                        "Failed to remove orphaned workspace {}: {}",
                        workspace_path_str, e
                    );
                } else {
                    info!(
                        "Successfully removed orphaned workspace: {}",
                        workspace_path_str
                    );
                }
            }
        }
    }

    async fn cleanup_workspace_without_repos(workspace_dir: &Path) -> Result<(), WorkspaceError> {
        info!(
            "Cleaning up orphaned workspace at {}",
            workspace_dir.display()
        );

        let entries = match std::fs::read_dir(workspace_dir) {
            Ok(entries) => entries,
            Err(e) => {
                debug!(
                    "Cannot read workspace directory {}, attempting direct removal: {}",
                    workspace_dir.display(),
                    e
                );
                return tokio::fs::remove_dir_all(workspace_dir)
                    .await
                    .map_err(WorkspaceError::Io);
            }
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir()
                && let Err(e) = WorktreeManager::cleanup_suspected_worktree(&path).await
            {
                warn!("Failed to cleanup suspected worktree: {}", e);
            }
        }

        if workspace_dir.exists()
            && let Err(e) = tokio::fs::remove_dir_all(workspace_dir).await
        {
            debug!(
                "Could not remove workspace directory {}: {}",
                workspace_dir.display(),
                e
            );
        }

        Ok(())
    }
}
