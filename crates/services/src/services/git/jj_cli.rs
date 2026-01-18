//! Jujutsu (jj) Git Interop for GitHub/GitLab Workflows
//!
//! This module provides integration with Jujutsu version control system (jj)
//! to enable smooth workflows with git-based forges like GitHub and GitLab.
//!
//! Key operations:
//! - `jj git fetch`: Sync changes from remote git repositories
//! - `jj git push`: Push jj changes to git branches for PR creation
//! - `jj git export`: Ensure git refs are up to date from jj state
//! - `jj git import`: Import git refs into jj state
//!
//! Reference: https://docs.jj-vcs.dev/latest/github/

use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::Command,
};

use thiserror::Error;
use utils::shell::resolve_executable_path_blocking;

#[derive(Debug, Error)]
pub enum JjCliError {
    #[error("jj executable not found or not runnable")]
    NotAvailable,
    #[error("jj command failed: {0}")]
    CommandFailed(String),
    #[error("not a jj repository: {0}")]
    NotJjRepo(String),
    #[error("authentication failed: {0}")]
    AuthFailed(String),
    #[error("push rejected: {0}")]
    PushRejected(String),
    #[error("git backend not initialized")]
    NoGitBackend,
}

#[derive(Clone, Default)]
pub struct JjCli;

impl JjCli {
    pub fn new() -> Self {
        Self {}
    }

    /// Check if jj is available on the system
    pub fn is_available(&self) -> bool {
        resolve_executable_path_blocking("jj")
            .and_then(|path| {
                Command::new(path)
                    .arg("--version")
                    .output()
                    .ok()
                    .map(|output| output.status.success())
            })
            .unwrap_or(false)
    }

    /// Ensure jj executable is available
    fn ensure_available(&self) -> Result<(), JjCliError> {
        if self.is_available() {
            Ok(())
        } else {
            Err(JjCliError::NotAvailable)
        }
    }

    /// Check if a directory is a jj repository
    pub fn is_jj_repo(&self, path: &Path) -> Result<bool, JjCliError> {
        self.ensure_available()?;
        
        let output = self.jj_raw(path, &["root"])?;
        Ok(!output.is_empty())
    }

    /// Check if the jj repo has a git backend
    pub fn has_git_backend(&self, repo_path: &Path) -> Result<bool, JjCliError> {
        self.ensure_available()?;
        
        // Check if .jj/repo/store/git exists
        let git_store = repo_path.join(".jj").join("repo").join("store").join("git");
        Ok(git_store.exists())
    }

    /// Sync changes from git remote repositories
    /// Equivalent to: jj git fetch [--remote <remote>] [--branch <branch>]
    pub fn git_fetch(
        &self,
        repo_path: &Path,
        remote: Option<&str>,
        branch: Option<&str>,
    ) -> Result<(), JjCliError> {
        self.ensure_available()?;
        
        if !self.has_git_backend(repo_path)? {
            return Err(JjCliError::NoGitBackend);
        }

        let mut args = vec![OsString::from("git"), OsString::from("fetch")];
        
        if let Some(remote_name) = remote {
            args.push(OsString::from("--remote"));
            args.push(OsString::from(remote_name));
        }
        
        if let Some(branch_name) = branch {
            args.push(OsString::from("--branch"));
            args.push(OsString::from(branch_name));
        }

        self.jj(repo_path, args)?;
        Ok(())
    }

    /// Export jj commits to git branches
    /// Ensures git refs are up to date from jj state
    /// Equivalent to: jj git export
    pub fn git_export(&self, repo_path: &Path) -> Result<(), JjCliError> {
        self.ensure_available()?;
        
        if !self.has_git_backend(repo_path)? {
            return Err(JjCliError::NoGitBackend);
        }

        let args = vec![OsString::from("git"), OsString::from("export")];
        self.jj(repo_path, args)?;
        Ok(())
    }

    /// Import git refs into jj state
    /// Updates jj state to match git refs
    /// Equivalent to: jj git import
    pub fn git_import(&self, repo_path: &Path) -> Result<(), JjCliError> {
        self.ensure_available()?;
        
        if !self.has_git_backend(repo_path)? {
            return Err(JjCliError::NoGitBackend);
        }

        let args = vec![OsString::from("git"), OsString::from("import")];
        self.jj(repo_path, args)?;
        Ok(())
    }

    /// Push jj changes to git remote branches
    /// Bridges jj changes to git branches for PR creation
    /// Equivalent to: jj git push [--remote <remote>] [--branch <branch>] [--change <change>]
    pub fn git_push(
        &self,
        repo_path: &Path,
        remote: Option<&str>,
        branch: Option<&str>,
        change: Option<&str>,
        force: bool,
    ) -> Result<(), JjCliError> {
        self.ensure_available()?;
        
        if !self.has_git_backend(repo_path)? {
            return Err(JjCliError::NoGitBackend);
        }

        let mut args = vec![OsString::from("git"), OsString::from("push")];
        
        if let Some(remote_name) = remote {
            args.push(OsString::from("--remote"));
            args.push(OsString::from(remote_name));
        }
        
        if let Some(branch_name) = branch {
            args.push(OsString::from("--branch"));
            args.push(OsString::from(branch_name));
        }
        
        if let Some(change_id) = change {
            args.push(OsString::from("--change"));
            args.push(OsString::from(change_id));
        }

        if force {
            args.push(OsString::from("--force"));
        }

        match self.jj(repo_path, args) {
            Ok(_) => Ok(()),
            Err(JjCliError::CommandFailed(msg)) => Err(self.classify_error(msg)),
            Err(err) => Err(err),
        }
    }

    /// Create a branch pointing to the current change
    /// Useful for preparing changes for git push
    pub fn branch_create(
        &self,
        repo_path: &Path,
        branch_name: &str,
        revision: Option<&str>,
    ) -> Result<(), JjCliError> {
        self.ensure_available()?;

        let mut args = vec![
            OsString::from("branch"),
            OsString::from("create"),
            OsString::from(branch_name),
        ];
        
        if let Some(rev) = revision {
            args.push(OsString::from("--revision"));
            args.push(OsString::from(rev));
        }

        self.jj(repo_path, args)?;
        Ok(())
    }

    /// Set a branch to point to a specific revision
    pub fn branch_set(
        &self,
        repo_path: &Path,
        branch_name: &str,
        revision: &str,
    ) -> Result<(), JjCliError> {
        self.ensure_available()?;

        let args = vec![
            OsString::from("branch"),
            OsString::from("set"),
            OsString::from(branch_name),
            OsString::from("--revision"),
            OsString::from(revision),
        ];

        self.jj(repo_path, args)?;
        Ok(())
    }

    /// Get the current change ID
    pub fn get_current_change_id(&self, repo_path: &Path) -> Result<String, JjCliError> {
        self.ensure_available()?;

        let args = vec![
            OsString::from("log"),
            OsString::from("--no-graph"),
            OsString::from("--limit"),
            OsString::from("1"),
            OsString::from("--template"),
            OsString::from("change_id"),
        ];

        let output = self.jj(repo_path, args)?;
        Ok(output.trim().to_string())
    }

    /// Create a new change (for parallel agent sessions)
    /// This creates a new change as a child of the current change
    /// Equivalent to: jj new [--message <message>]
    pub fn new_change(&self, repo_path: &Path, message: Option<&str>) -> Result<String, JjCliError> {
        self.ensure_available()?;

        let mut args = vec![OsString::from("new")];
        
        if let Some(msg) = message {
            args.push(OsString::from("--message"));
            args.push(OsString::from(msg));
        }

        self.jj(repo_path, args)?;
        
        // Get the change ID of the newly created change
        self.get_current_change_id(repo_path)
    }

    /// Edit (checkout) a specific change
    /// Equivalent to: jj edit <change_id>
    pub fn edit_change(&self, repo_path: &Path, change_id: &str) -> Result<(), JjCliError> {
        self.ensure_available()?;

        let args = vec![
            OsString::from("edit"),
            OsString::from(change_id),
        ];

        self.jj(repo_path, args)?;
        Ok(())
    }

    /// Abandon a change (cleanup for agent sessions)
    /// This removes the change without merging it
    /// Equivalent to: jj abandon <change_id>
    pub fn abandon_change(&self, repo_path: &Path, change_id: &str) -> Result<(), JjCliError> {
        self.ensure_available()?;

        let args = vec![
            OsString::from("abandon"),
            OsString::from(change_id),
        ];

        self.jj(repo_path, args)?;
        Ok(())
    }

    /// List all changes with their IDs and descriptions
    /// Returns a list of (change_id, description) tuples
    pub fn list_changes(&self, repo_path: &Path, limit: Option<usize>) -> Result<Vec<(String, String)>, JjCliError> {
        self.ensure_available()?;

        let mut args = vec![
            OsString::from("log"),
            OsString::from("--no-graph"),
            OsString::from("--template"),
            OsString::from("change_id ++ \"|\" ++ description"),
        ];

        if let Some(n) = limit {
            args.push(OsString::from("--limit"));
            args.push(OsString::from(n.to_string()));
        }

        let output = self.jj(repo_path, args)?;
        
        let changes = output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, '|').collect();
                if parts.len() == 2 {
                    Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
                } else {
                    None
                }
            })
            .collect();

        Ok(changes)
    }

    /// Get detailed information about a change
    pub fn get_change_info(&self, repo_path: &Path, change_id: &str) -> Result<String, JjCliError> {
        self.ensure_available()?;

        let args = vec![
            OsString::from("show"),
            OsString::from(change_id),
        ];

        self.jj(repo_path, args)
    }

    /// Sync with git and ensure all refs are up to date
    /// This combines import, export, and fetch operations
    pub fn sync_with_git(
        &self,
        repo_path: &Path,
        remote: Option<&str>,
    ) -> Result<(), JjCliError> {
        // Import git refs first
        self.git_import(repo_path)?;
        
        // Fetch from remote
        self.git_fetch(repo_path, remote, None)?;
        
        // Import again to get the fetched changes
        self.git_import(repo_path)?;
        
        // Export jj state to git
        self.git_export(repo_path)?;
        
        Ok(())
    }

    /// Prepare a change for PR by creating a branch and pushing
    pub fn prepare_for_pr(
        &self,
        repo_path: &Path,
        branch_name: &str,
        remote: &str,
    ) -> Result<(), JjCliError> {
        // Get current change ID (kept for potential future use)
        let _change_id = self.get_current_change_id(repo_path)?;
        
        // Create branch pointing to current change
        self.branch_create(repo_path, branch_name, Some("@"))?;
        
        // Export to git
        self.git_export(repo_path)?;
        
        // Push to remote
        self.git_push(repo_path, Some(remote), Some(branch_name), None, false)?;
        
        Ok(())
    }

    /// Run jj command and return output
    fn jj<I>(&self, repo_path: &Path, args: I) -> Result<String, JjCliError>
    where
        I: IntoIterator<Item = OsString>,
    {
        self.jj_raw(repo_path, args)
    }

    /// Low-level jj execution
    fn jj_raw<I, S>(&self, repo_path: &Path, args: I) -> Result<String, JjCliError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let jj_path = resolve_executable_path_blocking("jj")
            .ok_or(JjCliError::NotAvailable)?;

        let output = Command::new(jj_path)
            .current_dir(repo_path)
            .args(args)
            .output()
            .map_err(|e| JjCliError::CommandFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(JjCliError::CommandFailed(stderr.to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Classify error messages into specific error types
    fn classify_error(&self, msg: String) -> JjCliError {
        let msg_lower = msg.to_lowercase();
        
        if msg_lower.contains("authentication") || msg_lower.contains("permission denied") {
            JjCliError::AuthFailed(msg)
        } else if msg_lower.contains("rejected") || msg_lower.contains("non-fast-forward") {
            JjCliError::PushRejected(msg)
        } else if msg_lower.contains("not a jj repo") {
            JjCliError::NotJjRepo(msg)
        } else {
            JjCliError::CommandFailed(msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jj_cli_available() {
        let jj = JjCli::new();
        // Test will pass whether jj is installed or not
        let _ = jj.is_available();
    }

    #[test]
    fn test_classify_error() {
        let jj = JjCli::new();
        
        let auth_err = jj.classify_error("Authentication failed".to_string());
        assert!(matches!(auth_err, JjCliError::AuthFailed(_)));
        
        let push_err = jj.classify_error("Push rejected: non-fast-forward".to_string());
        assert!(matches!(push_err, JjCliError::PushRejected(_)));
        
        let repo_err = jj.classify_error("Error: Not a jj repo".to_string());
        assert!(matches!(repo_err, JjCliError::NotJjRepo(_)));
    }
}
