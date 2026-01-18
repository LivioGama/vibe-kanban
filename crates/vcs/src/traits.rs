use crate::error::VcsError;
use crate::types::*;
use std::path::Path;

/// Core repository operations
///
/// This trait represents the basic operations for a VCS repository,
/// including initialization, opening, and querying repository state.
/// 
/// Note: We don't require Sync because git2::Repository is not Sync.
/// Users should wrap in Arc<Mutex<_>> if they need to share across threads.
pub trait VcsRepository: Send {
    /// Initialize a new repository
    fn init(path: &Path) -> Result<Self, VcsError>
    where
        Self: Sized;

    /// Open an existing repository
    fn open(path: &Path) -> Result<Self, VcsError>
    where
        Self: Sized;

    /// Clone from a remote URL
    fn clone(url: &str, path: &Path) -> Result<Self, VcsError>
    where
        Self: Sized;

    /// Get the working directory path
    fn work_dir(&self) -> &Path;

    /// Check if repository is in a clean state (no conflicts, no ongoing operations)
    fn is_clean(&self) -> Result<bool, VcsError>;

    /// Get the current head information
    fn head(&self) -> Result<HeadInfo, VcsError>;

    /// Check if the repository exists and is valid
    fn is_valid(&self) -> bool;
}

/// Change/commit management operations
///
/// This trait handles creation, modification, and querying of changes/commits.
/// Note: "change" is the general term used here, which maps to:
/// - Git: commit
/// - Jujutsu: change (jj's term)
pub trait VcsChanges: VcsRepository {
    /// Create a new change/commit with given message
    ///
    /// For Git: This commits the current working directory changes
    /// For Jujutsu: This creates a new change without worktree checkout
    fn create_change(&self, message: &str) -> Result<ChangeId, VcsError>;

    /// Create a change with options
    fn create_change_with_options(
        &self,
        message: &str,
        options: CreateChangeOptions,
    ) -> Result<ChangeId, VcsError>;

    /// Amend the current change/commit
    fn amend_change(&self, message: Option<&str>) -> Result<(), VcsError>;

    /// Get information about a specific change
    fn get_change(&self, id: &ChangeId) -> Result<ChangeInfo, VcsError>;

    /// List changes/commits in the repository
    fn list_changes(&self, filter: ChangeFilter) -> Result<Vec<ChangeInfo>, VcsError>;

    /// Abandon/delete a change (jj) or reset/revert (git)
    fn abandon_change(&self, id: &ChangeId) -> Result<(), VcsError>;

    /// Check if a change exists
    fn change_exists(&self, id: &ChangeId) -> Result<bool, VcsError>;
}

/// Branch management operations
pub trait VcsBranches: VcsRepository {
    /// Create a new branch
    fn create_branch(&self, name: &str, base: Option<&ChangeId>) -> Result<(), VcsError>;

    /// Delete a branch
    fn delete_branch(&self, name: &str) -> Result<(), VcsError>;

    /// Rename a branch
    fn rename_branch(&self, old_name: &str, new_name: &str) -> Result<(), VcsError>;

    /// List all branches
    fn list_branches(&self) -> Result<Vec<BranchInfo>, VcsError>;

    /// Get current branch name (if on a branch)
    fn current_branch(&self) -> Result<Option<String>, VcsError>;

    /// Switch to a different branch/change
    ///
    /// For Git: This does a checkout
    /// For Jujutsu: This updates the working copy (but jj can work without checkouts)
    fn switch_to(&self, target: &BranchOrChange) -> Result<(), VcsError>;

    /// Check if a branch exists
    fn branch_exists(&self, name: &str) -> Result<bool, VcsError>;

    /// Check if branch name is valid
    fn is_branch_name_valid(&self, name: &str) -> bool;
}

/// Remote repository operations
pub trait VcsRemotes: VcsRepository {
    /// Fetch changes from remote
    fn fetch(&self, options: FetchOptions) -> Result<(), VcsError>;

    /// Push changes to remote
    fn push(&self, options: PushOptions) -> Result<(), VcsError>;

    /// Check if remote branch exists
    fn remote_branch_exists(&self, remote: &str, branch: &str) -> Result<bool, VcsError>;

    /// Get remote URL
    fn get_remote_url(&self, name: &str) -> Result<String, VcsError>;

    /// Set remote URL
    fn set_remote_url(&self, name: &str, url: &str) -> Result<(), VcsError>;

    /// List all remotes
    fn list_remotes(&self) -> Result<Vec<String>, VcsError>;
}

/// Diff and status operations
pub trait VcsDiff: VcsRepository {
    /// Get diff between two changes/commits
    fn diff_changes(&self, from: &ChangeId, to: &ChangeId) -> Result<Vec<FileDiff>, VcsError>;

    /// Get diff for uncommitted changes (working copy vs current change)
    fn diff_uncommitted(&self) -> Result<Vec<FileDiff>, VcsError>;

    /// Get status of files in working copy
    fn status(&self) -> Result<Vec<FileStatus>, VcsError>;

    /// Check if there are uncommitted changes
    fn has_uncommitted_changes(&self) -> Result<bool, VcsError>;
}

/// Conflict detection and resolution
pub trait VcsConflicts: VcsRepository {
    /// Check if there are any conflicts
    fn has_conflicts(&self) -> Result<bool, VcsError>;

    /// List all conflicted files
    fn list_conflicts(&self) -> Result<Vec<ConflictInfo>, VcsError>;

    /// Mark a conflict as resolved
    fn resolve_conflict(&self, path: &Path) -> Result<(), VcsError>;

    /// Abort ongoing operation (merge/rebase)
    fn abort_operation(&self) -> Result<(), VcsError>;

    /// Get the type of ongoing operation, if any
    fn ongoing_operation(&self) -> Result<Option<ConflictOperation>, VcsError>;
}

/// Combined trait representing a full VCS backend
///
/// This is the main trait that users will interact with, combining all
/// VCS capabilities into a single interface.
pub trait VcsBackend:
    VcsRepository + VcsChanges + VcsBranches + VcsRemotes + VcsDiff + VcsConflicts
{
    /// Get backend type
    fn backend_type(&self) -> crate::factory::VcsBackendType;

    /// Get a human-readable description of this backend
    fn description(&self) -> String {
        format!("{:?} backend at {}", self.backend_type(), self.work_dir().display())
    }
}
