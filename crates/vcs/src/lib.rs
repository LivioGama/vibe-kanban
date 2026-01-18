//! VCS Abstraction Layer
//!
//! This crate provides a trait-based abstraction over version control systems,
//! supporting both Git and Jujutsu (jj).
//!
//! # Design Goals
//!
//! - **Clean trait interface**: Operations are grouped by concern
//! - **No worktree complexity**: Especially important for jj path
//! - **Parallel agent support**: Multiple agents can work on different changes
//! - **Minimal migration impact**: Existing Git code continues to work
//!
//! # Example
//!
//! ```no_run
//! use vcs::{VcsFactory, VcsConfig, VcsBackendType, VcsChanges};
//! use std::path::PathBuf;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = VcsConfig {
//!     backend_type: VcsBackendType::Git,
//!     path: PathBuf::from("/path/to/repo"),
//! };
//!
//! let vcs = VcsFactory::create(&config)?;
//! let change_id = vcs.create_change("My change")?;
//! println!("Created change: {}", change_id.as_str());
//! # Ok(())
//! # }
//! ```

mod error;
mod factory;
mod traits;
mod types;

#[cfg(feature = "git")]
mod backend;

pub use error::VcsError;
pub use factory::{VcsBackendType, VcsConfig, VcsFactory};
pub use traits::{
    VcsBackend, VcsBranches, VcsChanges, VcsConflicts, VcsDiff, VcsRemotes, VcsRepository,
};
pub use types::{
    BranchInfo, BranchOrChange, ChangeFilter, ChangeId, ChangeInfo, ConflictInfo,
    ConflictSides, CreateChangeOptions, DiffContent, FetchOptions,
    FileChangeType, FileDiff, FileStatus, FileStatusKind, HeadInfo, PushOptions,
};

#[cfg(feature = "git")]
pub use backend::git::GitRepository;
