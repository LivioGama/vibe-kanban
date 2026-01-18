use std::path::Path;
use thiserror::Error;

/// Errors that can occur during VCS operations
#[derive(Debug, Error)]
pub enum VcsError {
    #[error("Repository not found: {0}")]
    RepositoryNotFound(String),

    #[error("Invalid change ID: {0}")]
    InvalidChangeId(String),

    #[error("Branch not found: {0}")]
    BranchNotFound(String),

    #[error("Conflict in files: {0:?}")]
    Conflicts(Vec<String>),

    #[error("Uncommitted changes in working copy")]
    DirtyWorkingCopy,

    #[error("Operation in progress: {0}")]
    OperationInProgress(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Push rejected: {0}")]
    PushRejected(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Backend-specific error: {0}")]
    Backend(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

impl VcsError {
    /// Create a RepositoryNotFound error from a path
    pub fn repo_not_found(path: &Path) -> Self {
        Self::RepositoryNotFound(path.display().to_string())
    }

    /// Create a Backend error from any error type
    pub fn backend<E: std::error::Error>(error: E) -> Self {
        Self::Backend(error.to_string())
    }
}
