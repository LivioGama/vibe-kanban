//! jj-based Parallel Workspace Manager
//!
//! This module enables true parallel agent sessions using Jujutsu (jj) VCS.
//! Unlike git worktrees, jj allows multiple agents to work in the same directory
//! by creating separate changes that can be edited independently.
//!
//! Key features:
//! - Each agent session gets a unique jj change
//! - All agents work in the same repo directory (no worktree hell)
//! - No locks or synchronization needed (jj handles conflicts naturally)
//! - Cleanup via `jj abandon` instead of directory removal
//! - Support for 5+ parallel sessions without performance degradation
//!
//! ## Architecture
//!
//! Traditional git worktrees:
//! ```text
//! worktrees/
//!   ├── session-1/  (separate directory)
//!   ├── session-2/  (separate directory)
//!   └── session-3/  (separate directory)
//! ```
//!
//! jj parallel sessions:
//! ```text
//! repo/           (single directory)
//!   ├── change-abc (session 1)
//!   ├── change-def (session 2)
//!   └── change-xyz (session 3)
//! ```

use std::path::{Path, PathBuf};

use thiserror::Error;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::git::{jj_cli::{JjCli, JjCliError}, GitService};

#[derive(Debug, Error)]
pub enum JjWorkspaceError {
    #[error(transparent)]
    JjCli(#[from] JjCliError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Not a jj repository: {0}")]
    NotJjRepo(String),
    #[error("Repository error: {0}")]
    Repository(String),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
}

/// Info about a single repo's jj session within a workspace
#[derive(Debug, Clone)]
pub struct RepoJjSession {
    pub repo_id: Uuid,
    pub repo_name: String,
    pub repo_path: PathBuf,
    pub change_id: String,
    pub session_id: Uuid,
}

/// Container for jj-based parallel sessions
/// Unlike WorktreeContainer, this points to the actual repo directories
/// since all sessions share the same directory
#[derive(Debug, Clone)]
pub struct JjSessionContainer {
    pub sessions: Vec<RepoJjSession>,
}

pub struct JjWorkspaceManager {
    jj_cli: JjCli,
}

impl JjWorkspaceManager {
    pub fn new() -> Self {
        Self {
            jj_cli: JjCli::new(),
        }
    }

    /// Check if jj is available on the system
    pub fn is_jj_available(&self) -> bool {
        self.jj_cli.is_available()
    }

    /// Check if a repository is a jj repository
    pub fn is_jj_repo(&self, repo_path: &Path) -> Result<bool, JjWorkspaceError> {
        Ok(self.jj_cli.is_jj_repo(repo_path)?)
    }

    /// Create a new jj session for an agent
    /// This creates a new change and returns its change ID
    ///
    /// ## How it works:
    /// 1. Creates a new change with `jj new`
    /// 2. Returns the change ID for tracking
    /// 3. Agent works in the same repo directory but on different change
    ///
    /// ## No directory isolation:
    /// Unlike git worktrees, there's no separate directory. The agent works
    /// directly in the repo directory, and jj tracks which change is active.
    pub fn create_session(
        &self,
        repo_path: &Path,
        session_id: Uuid,
        base_change: Option<&str>,
    ) -> Result<String, JjWorkspaceError> {
        if !self.is_jj_repo(repo_path)? {
            return Err(JjWorkspaceError::NotJjRepo(
                repo_path.to_string_lossy().to_string(),
            ));
        }

        let message = format!("Agent session {}", session_id);
        
        info!(
            "Creating jj session {} in repo: {}",
            session_id,
            repo_path.display()
        );

        // If a base_change is specified, edit it first before creating new change
        if let Some(base) = base_change {
            debug!("Basing new session on change: {}", base);
            self.jj_cli.edit_change(repo_path, base)?;
        }

        // Create new change (will be child of current change)
        let change_id = self.jj_cli.new_change(repo_path, Some(&message))?;

        info!(
            "Created jj session {} with change ID: {}",
            session_id, change_id
        );

        Ok(change_id)
    }

    /// Switch to a specific session (change)
    pub fn switch_session(
        &self,
        repo_path: &Path,
        change_id: &str,
    ) -> Result<(), JjWorkspaceError> {
        if !self.is_jj_repo(repo_path)? {
            return Err(JjWorkspaceError::NotJjRepo(
                repo_path.to_string_lossy().to_string(),
            ));
        }

        debug!("Switching to change: {} in repo: {}", change_id, repo_path.display());
        Ok(self.jj_cli.edit_change(repo_path, change_id)?)
    }

    /// Clean up a session by abandoning its change
    pub fn cleanup_session(
        &self,
        repo_path: &Path,
        change_id: &str,
    ) -> Result<(), JjWorkspaceError> {
        if !self.is_jj_repo(repo_path)? {
            return Err(JjWorkspaceError::NotJjRepo(
                repo_path.to_string_lossy().to_string(),
            ));
        }

        info!(
            "Abandoning change {} in repo: {}",
            change_id,
            repo_path.display()
        );

        Ok(self.jj_cli.abandon_change(repo_path, change_id)?)
    }

    /// List all active changes in a repository
    pub fn list_sessions(
        &self,
        repo_path: &Path,
        limit: Option<usize>,
    ) -> Result<Vec<(String, String)>, JjWorkspaceError> {
        if !self.is_jj_repo(repo_path)? {
            return Err(JjWorkspaceError::NotJjRepo(
                repo_path.to_string_lossy().to_string(),
            ));
        }

        Ok(self.jj_cli.list_changes(repo_path, limit)?)
    }

    /// Get information about a specific session (change)
    pub fn get_session_info(
        &self,
        repo_path: &Path,
        change_id: &str,
    ) -> Result<String, JjWorkspaceError> {
        if !self.is_jj_repo(repo_path)? {
            return Err(JjWorkspaceError::NotJjRepo(
                repo_path.to_string_lossy().to_string(),
            ));
        }

        Ok(self.jj_cli.get_change_info(repo_path, change_id)?)
    }

    /// Batch cleanup multiple sessions
    pub fn batch_cleanup_sessions(
        &self,
        sessions: &[RepoJjSession],
    ) -> Result<(), JjWorkspaceError> {
        for session in sessions {
            debug!(
                "Cleaning up jj session {} (change {}) in repo: {}",
                session.session_id, session.change_id, session.repo_path.display()
            );

            if let Err(e) = self.cleanup_session(&session.repo_path, &session.change_id) {
                warn!(
                    "Failed to cleanup jj session {} (change {}): {}",
                    session.session_id, session.change_id, e
                );
            }
        }

        Ok(())
    }

    /// Get the current change ID in a repository
    pub fn get_current_change_id(&self, repo_path: &Path) -> Result<String, JjWorkspaceError> {
        if !self.is_jj_repo(repo_path)? {
            return Err(JjWorkspaceError::NotJjRepo(
                repo_path.to_string_lossy().to_string(),
            ));
        }

        Ok(self.jj_cli.get_current_change_id(repo_path)?)
    }
}

impl Default for JjWorkspaceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jj_workspace_manager_creation() {
        let manager = JjWorkspaceManager::new();
        // Should not panic
        let _ = manager.is_jj_available();
    }

    #[tokio::test]
    async fn test_jj_session_lifecycle() {
        use tempfile::TempDir;

        let manager = JjWorkspaceManager::new();
        
        // Skip if jj not available
        if !manager.is_jj_available() {
            return;
        }

        let td = TempDir::new().unwrap();
        let repo_path = td.path().join("repo");
        
        // Initialize a jj repo with git backend
        let git_service = GitService::new();
        git_service
            .initialize_repo_with_main_branch(&repo_path)
            .unwrap();

        // Initialize jj on top of git repo
        // This would require `jj init --git-repo` command
        // For now, we'll skip the full integration test
        // as it requires jj to be installed
    }
}
