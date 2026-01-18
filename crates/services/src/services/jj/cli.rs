//! Jujutsu CLI wrapper for VCS operations.
//!
//! This module provides a wrapper around the `jj` command-line tool, handling:
//! - Repository initialization and cloning
//! - Change creation and management (jj's core abstraction)
//! - Diff and log operations
//! - Conflict resolution
//! - Git interop (push/fetch)
//!
//! Unlike Git, Jujutsu uses "changes" as its primary abstraction:
//! - Every working copy state is automatically tracked as a change
//! - Changes have stable "change IDs" that persist across rebases
//! - Commit IDs are implementation details
//! - No explicit staging area needed

use std::{
    ffi::{OsStr, OsString},
    path::Path,
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use utils::shell::resolve_executable_path_blocking;

#[derive(Debug, Error)]
pub enum JujutsuCliError {
    #[error("jj executable not found or not runnable")]
    NotAvailable,
    #[error("jj command failed: {0}")]
    CommandFailed(String),
    #[error("authentication failed: {0}")]
    AuthFailed(String),
    #[error("push rejected: {0}")]
    PushRejected(String),
    #[error("conflict resolution required")]
    ConflictResolutionRequired,
    #[error("parse error: {0}")]
    ParseError(String),
}

#[derive(Clone, Default)]
pub struct JujutsuCli;

/// Represents a Jujutsu change (the core abstraction in jj)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JjChange {
    /// Stable change ID (persists across rebases)
    pub change_id: String,
    /// Current commit ID (changes on rewrite)
    pub commit_id: String,
    /// Author information
    pub author: String,
    /// Committer information
    pub committer: String,
    /// Change description (commit message)
    pub description: String,
    /// Whether this change is empty (no file changes)
    pub is_empty: bool,
    /// Whether this change has conflicts
    pub has_conflicts: bool,
    /// Parent change IDs
    pub parents: Vec<String>,
    /// Branches pointing to this change
    pub branches: Vec<String>,
    /// Tags pointing to this change
    pub tags: Vec<String>,
}

/// Status information for a Jujutsu working copy
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JjStatus {
    /// Current working copy change ID
    pub working_copy_change_id: String,
    /// Whether there are uncommitted changes
    pub has_changes: bool,
    /// Whether there are conflicts to resolve
    pub has_conflicts: bool,
    /// List of conflicted files
    pub conflicted_files: Vec<String>,
    /// List of modified files (M)
    pub modified_files: Vec<String>,
    /// List of added files (A)
    pub added_files: Vec<String>,
    /// List of deleted files (D)
    pub deleted_files: Vec<String>,
}

/// Diff summary entry (similar to git's --name-status)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JjDiffSummary {
    /// Type of change (M, A, D, R)
    pub change_type: String,
    /// Path of the file
    pub path: String,
    /// Original path (for renames)
    pub old_path: Option<String>,
}

/// Options for diff operations
#[derive(Debug, Clone, Default)]
pub struct JjDiffOptions {
    /// Show changes relative to this revision
    pub from: Option<String>,
    /// Show changes up to this revision
    pub to: Option<String>,
    /// Filter to specific paths
    pub paths: Option<Vec<String>>,
    /// Show summary only (similar to git --name-status)
    pub summary: bool,
    /// Show stat (histogram of changes)
    pub stat: bool,
}

/// Options for log operations
#[derive(Debug, Clone, Default)]
pub struct JjLogOptions {
    /// Revision expression (e.g., "@", "main", "@-")
    pub revset: Option<String>,
    /// Maximum number of commits to show
    pub limit: Option<usize>,
    /// Show full commit IDs
    pub no_graph: bool,
}

impl JujutsuCli {
    pub fn new() -> Self {
        Self {}
    }

    /// Initialize a new Jujutsu repository
    pub fn init(&self, repo_path: &Path) -> Result<(), JujutsuCliError> {
        self.jj(repo_path, ["init", "--git"])?;
        Ok(())
    }

    /// Clone a Git repository using Jujutsu
    pub fn git_clone(&self, url: &str, dest_path: &Path) -> Result<(), JujutsuCliError> {
        // jj git clone doesn't take a repo_path context, run from parent
        let parent = dest_path
            .parent()
            .ok_or_else(|| JujutsuCliError::CommandFailed("Invalid destination path".into()))?;
        
        let dest_str = dest_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| JujutsuCliError::CommandFailed("Invalid destination name".into()))?;

        self.jj(parent, ["git", "clone", url, dest_str])?;
        Ok(())
    }

    /// Create a new change (equivalent to starting work on something new)
    pub fn new_change(&self, repo_path: &Path, description: Option<&str>) -> Result<String, JujutsuCliError> {
        let mut args = vec!["new"];
        if let Some(desc) = description {
            args.push("-m");
            args.push(desc);
        }
        let output = self.jj(repo_path, args)?;
        
        // Parse the change ID from output
        self.parse_change_id_from_new(&output)
    }

    /// Describe (set message for) the current change
    pub fn describe(&self, repo_path: &Path, message: &str) -> Result<(), JujutsuCliError> {
        self.jj(repo_path, ["describe", "-m", message])?;
        Ok(())
    }

    /// Describe a specific change by revision
    pub fn describe_revision(
        &self,
        repo_path: &Path,
        revision: &str,
        message: &str,
    ) -> Result<(), JujutsuCliError> {
        self.jj(repo_path, ["describe", "-r", revision, "-m", message])?;
        Ok(())
    }

    /// Push changes to the Git remote
    pub fn git_push(
        &self,
        repo_path: &Path,
        branch: Option<&str>,
    ) -> Result<(), JujutsuCliError> {
        let mut args = vec!["git", "push"];
        if let Some(b) = branch {
            args.push("--branch");
            args.push(b);
        }
        self.jj(repo_path, args)?;
        Ok(())
    }

    /// Fetch changes from the Git remote
    pub fn git_fetch(&self, repo_path: &Path, remote: Option<&str>) -> Result<(), JujutsuCliError> {
        let mut args = vec!["git", "fetch"];
        if let Some(r) = remote {
            args.push("--remote");
            args.push(r);
        }
        self.jj(repo_path, args)?;
        Ok(())
    }

    /// Get the diff for the current or specified change
    pub fn diff(
        &self,
        repo_path: &Path,
        opts: JjDiffOptions,
    ) -> Result<String, JujutsuCliError> {
        let mut args: Vec<OsString> = vec!["diff".into()];
        
        if let Some(from) = opts.from {
            args.push("--from".into());
            args.push(from.into());
        }
        
        if let Some(to) = opts.to {
            args.push("--to".into());
            args.push(to.into());
        }
        
        if opts.summary {
            args.push("--summary".into());
        }
        
        if opts.stat {
            args.push("--stat".into());
        }
        
        if let Some(paths) = opts.paths {
            for path in paths {
                args.push(path.into());
            }
        }
        
        self.jj(repo_path, args)
    }

    /// Get diff summary (similar to git diff --name-status)
    pub fn diff_summary(
        &self,
        repo_path: &Path,
        from: Option<&str>,
        to: Option<&str>,
        paths: Option<Vec<String>>,
    ) -> Result<Vec<JjDiffSummary>, JujutsuCliError> {
        let opts = JjDiffOptions {
            from: from.map(|s| s.to_string()),
            to: to.map(|s| s.to_string()),
            paths,
            summary: true,
            stat: false,
        };
        
        let output = self.diff(repo_path, opts)?;
        self.parse_diff_summary(&output)
    }

    /// Query the change history
    pub fn log(
        &self,
        repo_path: &Path,
        opts: JjLogOptions,
    ) -> Result<Vec<JjChange>, JujutsuCliError> {
        let mut args = vec!["log"];
        
        let revset;
        if let Some(ref r) = opts.revset {
            args.push("-r");
            revset = r.clone();
            args.push(&revset);
        }
        
        let limit_str;
        if let Some(lim) = opts.limit {
            args.push("-n");
            limit_str = lim.to_string();
            args.push(&limit_str);
        }
        
        if opts.no_graph {
            args.push("--no-graph");
        }
        
        // Request JSON output for easier parsing
        args.push("--template");
        args.push(r#"
{
  "change_id": change_id,
  "commit_id": commit_id,
  "author": author,
  "committer": committer,
  "description": description,
  "is_empty": empty,
  "has_conflicts": conflict,
  "parents": parents,
  "branches": branches,
  "tags": tags
}
"#);
        
        let output = self.jj(repo_path, args)?;
        self.parse_log_json(&output)
    }

    /// Get the status of the working copy
    pub fn status(&self, repo_path: &Path) -> Result<JjStatus, JujutsuCliError> {
        let output = self.jj(repo_path, ["status"])?;
        self.parse_status(&output)
    }

    /// Rebase changes (typically not needed due to jj's automatic rebase)
    pub fn rebase(
        &self,
        repo_path: &Path,
        source: &str,
        destination: &str,
    ) -> Result<(), JujutsuCliError> {
        self.jj(repo_path, ["rebase", "-s", source, "-d", destination])?;
        Ok(())
    }

    /// Resolve conflicts in the working copy
    pub fn resolve(&self, repo_path: &Path, paths: &[String]) -> Result<(), JujutsuCliError> {
        if paths.is_empty() {
            self.jj(repo_path, ["resolve"])?;
        } else {
            let mut args: Vec<OsString> = vec!["resolve".into()];
            for path in paths {
                args.push(path.into());
            }
            self.jj(repo_path, args)?;
        }
        Ok(())
    }

    /// Get the list of conflicted files
    pub fn get_conflicted_files(&self, repo_path: &Path) -> Result<Vec<String>, JujutsuCliError> {
        // jj status shows conflicts
        let output = self.jj(repo_path, ["status"])?;
        self.parse_conflicted_files(&output)
    }

    /// Mark conflicts as resolved for specific files
    pub fn mark_resolved(&self, repo_path: &Path, paths: &[String]) -> Result<(), JujutsuCliError> {
        if paths.is_empty() {
            return Ok(());
        }
        
        let mut args: Vec<OsString> = vec!["resolve".into(), "--mark-resolved".into()];
        for path in paths {
            args.push(path.into());
        }
        self.jj(repo_path, args)?;
        Ok(())
    }

    /// Get current change ID for working copy
    pub fn current_change_id(&self, repo_path: &Path) -> Result<String, JujutsuCliError> {
        let output = self.jj(repo_path, ["log", "-r", "@", "-T", "change_id"])?;
        Ok(output.trim().to_string())
    }

    /// Abandon a change (remove it from history)
    pub fn abandon(&self, repo_path: &Path, revision: &str) -> Result<(), JujutsuCliError> {
        self.jj(repo_path, ["abandon", revision])?;
        Ok(())
    }

    /// Squash changes into their parent
    pub fn squash(
        &self,
        repo_path: &Path,
        revision: Option<&str>,
        message: Option<&str>,
    ) -> Result<(), JujutsuCliError> {
        let mut args = vec!["squash"];
        
        if let Some(rev) = revision {
            args.push("-r");
            args.push(rev);
        }
        
        if let Some(msg) = message {
            args.push("-m");
            args.push(msg);
        }
        
        self.jj(repo_path, args)?;
        Ok(())
    }

    /// Edit a change (move working copy to a specific change)
    pub fn edit(&self, repo_path: &Path, revision: &str) -> Result<(), JujutsuCliError> {
        self.jj(repo_path, ["edit", revision])?;
        Ok(())
    }

    /// Create or update a branch
    pub fn branch_create(
        &self,
        repo_path: &Path,
        branch_name: &str,
        revision: Option<&str>,
    ) -> Result<(), JujutsuCliError> {
        let mut args = vec!["branch", "create", branch_name];
        
        if let Some(rev) = revision {
            args.push("-r");
            args.push(rev);
        }
        
        self.jj(repo_path, args)?;
        Ok(())
    }

    /// Set an existing branch to a new revision
    pub fn branch_set(
        &self,
        repo_path: &Path,
        branch_name: &str,
        revision: &str,
    ) -> Result<(), JujutsuCliError> {
        self.jj(repo_path, ["branch", "set", branch_name, "-r", revision])?;
        Ok(())
    }

    /// List all branches
    pub fn branch_list(&self, repo_path: &Path) -> Result<Vec<String>, JujutsuCliError> {
        let output = self.jj(repo_path, ["branch", "list"])?;
        Ok(output.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
    }
}

// Private implementation methods
impl JujutsuCli {
    /// Ensure `jj` is available on PATH
    fn ensure_available(&self) -> Result<(), JujutsuCliError> {
        let jj = resolve_executable_path_blocking("jj").ok_or(JujutsuCliError::NotAvailable)?;
        let out = Command::new(&jj)
            .arg("--version")
            .output()
            .map_err(|_| JujutsuCliError::NotAvailable)?;
        if out.status.success() {
            Ok(())
        } else {
            Err(JujutsuCliError::NotAvailable)
        }
    }

    /// Run jj command and return stdout on success
    fn jj_impl<I, S>(
        &self,
        repo_path: &Path,
        args: I,
    ) -> Result<Vec<u8>, JujutsuCliError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.ensure_available()?;
        let jj = resolve_executable_path_blocking("jj").ok_or(JujutsuCliError::NotAvailable)?;
        
        let mut cmd = Command::new(&jj);
        cmd.current_dir(repo_path);
        
        for arg in args {
            cmd.arg(arg);
        }
        
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        tracing::trace!(
            repo = ?repo_path,
            "Running jj command: {:?}",
            cmd
        );
        
        let output = cmd
            .output()
            .map_err(|e| JujutsuCliError::CommandFailed(e.to_string()))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(self.classify_error(stderr));
        }
        
        Ok(output.stdout)
    }

    fn jj<I, S>(&self, repo_path: &Path, args: I) -> Result<String, JujutsuCliError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let out = self.jj_impl(repo_path, args)?;
        Ok(String::from_utf8_lossy(&out).to_string())
    }

    fn classify_error(&self, msg: String) -> JujutsuCliError {
        let lower = msg.to_ascii_lowercase();
        
        if lower.contains("authentication failed")
            || lower.contains("could not read username")
            || lower.contains("invalid username or password")
        {
            JujutsuCliError::AuthFailed(msg)
        } else if lower.contains("rejected")
            || lower.contains("non-fast-forward")
            || lower.contains("failed to push")
        {
            JujutsuCliError::PushRejected(msg)
        } else if lower.contains("conflict")
            || lower.contains("needs to be resolved")
        {
            JujutsuCliError::ConflictResolutionRequired
        } else {
            JujutsuCliError::CommandFailed(msg)
        }
    }

    /// Parse change ID from `jj new` output
    fn parse_change_id_from_new(&self, output: &str) -> Result<String, JujutsuCliError> {
        // jj new typically outputs something like:
        // "Working copy now at: kmkuslsw 3d0c8c7e (empty) (no description set)"
        // We want to extract the change ID (first hash-like string after "at:")
        
        for line in output.lines() {
            if line.contains("Working copy now at:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(idx) = parts.iter().position(|&p| p == "at:") {
                    if let Some(change_id) = parts.get(idx + 1) {
                        return Ok(change_id.to_string());
                    }
                }
            }
        }
        
        Err(JujutsuCliError::ParseError(
            "Could not parse change ID from jj new output".into()
        ))
    }

    /// Parse status output
    fn parse_status(&self, output: &str) -> Result<JjStatus, JujutsuCliError> {
        let mut working_copy_change_id = String::new();
        let mut has_changes = false;
        let mut has_conflicts = false;
        let mut conflicted_files = Vec::new();
        let mut modified_files = Vec::new();
        let mut added_files = Vec::new();
        let mut deleted_files = Vec::new();
        
        for line in output.lines() {
            let line = line.trim();
            
            if line.starts_with("Working copy changes:") {
                has_changes = true;
            } else if line.contains("conflict") {
                has_conflicts = true;
            } else if line.starts_with("Working copy :") {
                // Extract change ID
                if let Some(id_part) = line.split_whitespace().nth(3) {
                    working_copy_change_id = id_part.to_string();
                }
            }
            
            // Parse file changes from status output
            // Format is typically: "M file.txt" or "A file.txt" or "D file.txt"
            if has_changes && !line.is_empty() && line.len() > 2 {
                let chars: Vec<char> = line.chars().collect();
                if chars.len() >= 2 && chars[1] == ' ' {
                    let status_char = chars[0];
                    let path = &line[2..].trim();
                    
                    match status_char {
                        'M' => modified_files.push(path.to_string()),
                        'A' => added_files.push(path.to_string()),
                        'D' => deleted_files.push(path.to_string()),
                        _ => {}
                    }
                }
            }
            
            // Look for file paths that have conflicts (typically listed in status)
            if has_conflicts && !line.is_empty() && !line.starts_with("Working") {
                if let Some(file) = line.split_whitespace().last() {
                    if !file.is_empty() {
                        conflicted_files.push(file.to_string());
                    }
                }
            }
        }
        
        // If we couldn't find change ID in status, fetch it separately
        if working_copy_change_id.is_empty() {
            working_copy_change_id = self.current_change_id(
                Path::new(".") // This is a fallback, ideally should pass repo_path
            )?;
        }
        
        Ok(JjStatus {
            working_copy_change_id,
            has_changes,
            has_conflicts,
            conflicted_files,
            modified_files,
            added_files,
            deleted_files,
        })
    }

    /// Parse conflicted files from status output
    fn parse_conflicted_files(&self, output: &str) -> Result<Vec<String>, JujutsuCliError> {
        let mut files = Vec::new();
        let mut in_conflict_section = false;
        
        for line in output.lines() {
            let line = line.trim();
            
            if line.contains("conflicts:") {
                in_conflict_section = true;
                continue;
            }
            
            if in_conflict_section {
                if line.is_empty() {
                    break;
                }
                
                // Conflict files are typically listed with markers
                if let Some(file) = line.split_whitespace().last() {
                    if !file.is_empty() {
                        files.push(file.to_string());
                    }
                }
            }
        }
        
        Ok(files)
    }

    /// Parse diff summary output (from --summary flag)
    fn parse_diff_summary(&self, output: &str) -> Result<Vec<JjDiffSummary>, JujutsuCliError> {
        let mut entries = Vec::new();
        
        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            // Format is typically: "M file.txt" or "A file.txt" or "D file.txt" or "R old.txt => new.txt"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            
            let change_type = parts[0].to_string();
            
            // Handle rename: "R old.txt => new.txt"
            if change_type == "R" && parts.len() >= 4 && parts[2] == "=>" {
                entries.push(JjDiffSummary {
                    change_type,
                    path: parts[3].to_string(),
                    old_path: Some(parts[1].to_string()),
                });
            } else if parts.len() >= 2 {
                // Normal case: "M file.txt", "A file.txt", "D file.txt"
                entries.push(JjDiffSummary {
                    change_type,
                    path: parts[1].to_string(),
                    old_path: None,
                });
            }
        }
        
        Ok(entries)
    }

    /// Parse log output in JSON format
    fn parse_log_json(&self, output: &str) -> Result<Vec<JjChange>, JujutsuCliError> {
        let mut changes = Vec::new();
        
        // The output may contain multiple JSON objects, one per line or separated
        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() || !line.starts_with('{') {
                continue;
            }
            
            match serde_json::from_str::<JjChange>(line) {
                Ok(change) => changes.push(change),
                Err(e) => {
                    tracing::warn!("Failed to parse jj log JSON line: {}", e);
                    // Continue parsing other lines rather than failing completely
                }
            }
        }
        
        Ok(changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_change_id_from_new() {
        let cli = JujutsuCli::new();
        let output = "Working copy now at: kmkuslsw 3d0c8c7e (empty) (no description set)\nParent commit      : rlvkpnrz 2f4a3311 main | Initial commit";
        
        let change_id = cli.parse_change_id_from_new(output).unwrap();
        assert_eq!(change_id, "kmkuslsw");
    }

    #[test]
    fn test_parse_status() {
        let cli = JujutsuCli::new();
        let output = r#"Working copy : pzsxstzt 3d0c8c7e (no description set)
Working copy changes:
M file.txt
A new_file.txt"#;
        
        let status = cli.parse_status(output);
        assert!(status.is_ok());
        let status = status.unwrap();
        assert!(status.has_changes);
        assert!(!status.has_conflicts);
        assert_eq!(status.modified_files.len(), 1);
        assert_eq!(status.modified_files[0], "file.txt");
        assert_eq!(status.added_files.len(), 1);
        assert_eq!(status.added_files[0], "new_file.txt");
    }

    #[test]
    fn test_parse_diff_summary() {
        let cli = JujutsuCli::new();
        let output = r#"M file.txt
A new_file.txt
D old_file.txt
R old_name.txt => new_name.txt"#;
        
        let summary = cli.parse_diff_summary(output).unwrap();
        assert_eq!(summary.len(), 4);
        
        assert_eq!(summary[0].change_type, "M");
        assert_eq!(summary[0].path, "file.txt");
        assert_eq!(summary[0].old_path, None);
        
        assert_eq!(summary[1].change_type, "A");
        assert_eq!(summary[1].path, "new_file.txt");
        
        assert_eq!(summary[2].change_type, "D");
        assert_eq!(summary[2].path, "old_file.txt");
        
        assert_eq!(summary[3].change_type, "R");
        assert_eq!(summary[3].path, "new_name.txt");
        assert_eq!(summary[3].old_path, Some("old_name.txt".to_string()));
    }

    #[test]
    fn test_classify_error_auth() {
        let cli = JujutsuCli::new();
        let err = cli.classify_error("Authentication failed".to_string());
        assert!(matches!(err, JujutsuCliError::AuthFailed(_)));
    }

    #[test]
    fn test_classify_error_push_rejected() {
        let cli = JujutsuCli::new();
        let err = cli.classify_error("Push rejected: non-fast-forward".to_string());
        assert!(matches!(err, JujutsuCliError::PushRejected(_)));
    }

    #[test]
    fn test_classify_error_conflict() {
        let cli = JujutsuCli::new();
        let err = cli.classify_error("Conflict needs to be resolved".to_string());
        assert!(matches!(err, JujutsuCliError::ConflictResolutionRequired));
    }
}
