use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a single change/commit ID in the VCS
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChangeId(String);

impl ChangeId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ChangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ChangeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Information about the current HEAD/working position
#[derive(Debug, Clone)]
pub struct HeadInfo {
    /// Current branch name (if on a branch)
    pub branch: Option<String>,
    /// Current change/commit ID
    pub change_id: ChangeId,
    /// Human-readable description
    pub description: String,
}

/// Change/commit metadata
#[derive(Debug, Clone)]
pub struct ChangeInfo {
    pub id: ChangeId,
    pub parent_ids: Vec<ChangeId>,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub description: String,
    pub is_empty: bool,
}

/// Filter for listing changes
#[derive(Debug, Clone, Default)]
pub struct ChangeFilter {
    pub branch: Option<String>,
    pub author: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

/// Reference to either a branch or a specific change
#[derive(Debug, Clone)]
pub enum BranchOrChange {
    Branch(String),
    Change(ChangeId),
}

impl From<String> for BranchOrChange {
    fn from(s: String) -> Self {
        Self::Branch(s)
    }
}

impl From<ChangeId> for BranchOrChange {
    fn from(id: ChangeId) -> Self {
        Self::Change(id)
    }
}

/// Information about a branch
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub change_id: ChangeId,
    pub is_current: bool,
    pub is_remote: bool,
    pub last_updated: DateTime<Utc>,
}

/// File diff information
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub change_type: FileChangeType,
    pub additions: usize,
    pub deletions: usize,
    pub content: Option<DiffContent>,
}

/// Type of change to a file
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
}

/// Content of a diff
#[derive(Debug, Clone)]
pub struct DiffContent {
    pub old_content: Option<Vec<u8>>,
    pub new_content: Option<Vec<u8>>,
}

/// Status of a file in the working copy
#[derive(Debug, Clone)]
pub struct FileStatus {
    pub path: String,
    pub status: FileStatusKind,
}

/// Kind of file status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatusKind {
    Untracked,
    Modified,
    Added,
    Deleted,
    Conflicted,
}

/// Information about a conflict
///
/// In jj's model, conflicts are first-class and can be committed.
/// The operation type is less important than the conflict content itself.
#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub path: String,
    pub sides: ConflictSides,
}

/// The conflicting sides in a 3-way merge
///
/// This represents jj's conflict model where conflicts have:
/// - A base (common ancestor)
/// - Ours (left side)
/// - Theirs (right side)
#[derive(Debug, Clone)]
pub struct ConflictSides {
    pub base: Option<ChangeId>,
    pub ours: ChangeId,
    pub theirs: ChangeId,
}

/// Options for creating a change
#[derive(Debug, Clone, Default)]
pub struct CreateChangeOptions {
    /// Working directory where files are located
    pub working_dir: Option<PathBuf>,
    /// Whether to automatically stage all changes
    pub stage_all: bool,
    /// Parent change(s) to base this change on
    pub parents: Vec<ChangeId>,
}

/// Options for pushing changes
#[derive(Debug, Clone, Default)]
pub struct PushOptions {
    /// Remote name (defaults to "origin")
    pub remote: Option<String>,
    /// Branch to push (defaults to current branch)
    pub branch: Option<String>,
    /// Force push
    pub force: bool,
}

/// Options for fetching changes
#[derive(Debug, Clone, Default)]
pub struct FetchOptions {
    /// Remote name (defaults to "origin")
    pub remote: Option<String>,
    /// Prune deleted remote branches
    pub prune: bool,
}
