//! Common test utilities for VCS backends (Git and Jujutsu)
//!
//! This module provides test infrastructure that works across both VCS backends,
//! allowing tests to be parameterized by backend type.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use git2::{Repository, build::CheckoutBuilder};
use services::services::git::{GitCli, GitService};
use services::services::jj::JujutsuCli;
use tempfile::TempDir;

/// VCS backend type for parameterized tests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsBackend {
    Git,
    Jujutsu,
}

impl VcsBackend {
    /// Returns all backends that should be tested
    pub fn all() -> Vec<VcsBackend> {
        vec![VcsBackend::Git, VcsBackend::Jujutsu]
    }

    /// Returns only backends that are available in the current environment
    pub fn available() -> Vec<VcsBackend> {
        let mut backends = vec![VcsBackend::Git]; // Git is always available via git2
        
        // Check if jj is available by trying to run it
        if is_jj_available() {
            backends.push(VcsBackend::Jujutsu);
        }
        
        backends
    }

    /// Returns the backend name as a string
    pub fn name(&self) -> &'static str {
        match self {
            VcsBackend::Git => "git",
            VcsBackend::Jujutsu => "jj",
        }
    }
}

/// Test repository context that works with both Git and Jujutsu
pub struct VcsTestRepo {
    pub backend: VcsBackend,
    pub root: TempDir,
    pub repo_path: PathBuf,
}

impl VcsTestRepo {
    /// Initialize a new test repository with the specified backend
    pub fn init(backend: VcsBackend) -> Self {
        let root = TempDir::new().expect("create temp dir");
        let repo_path = root.path().join("repo");
        
        match backend {
            VcsBackend::Git => {
                let service = GitService::new();
                service
                    .initialize_repo_with_main_branch(&repo_path)
                    .expect("init git repo");
                configure_git_user(&repo_path, "Test User", "test@example.com");
                checkout_git_branch(&repo_path, "main");
            }
            VcsBackend::Jujutsu => {
                let jj = JujutsuCli::new();
                jj.init(&repo_path).expect("init jj repo");
                // JJ automatically creates a working copy change
            }
        }
        
        Self {
            backend,
            root,
            repo_path,
        }
    }

    /// Write a file in the repository
    pub fn write_file(&self, rel_path: &str, content: &str) {
        write_file(&self.repo_path, rel_path, content);
    }

    /// Commit changes (Git) or describe current change (JJ)
    pub fn commit(&self, message: &str) -> Result<String, Box<dyn std::error::Error>> {
        match self.backend {
            VcsBackend::Git => {
                let service = GitService::new();
                service.commit(&self.repo_path, message)?;
                let head = service.get_head_info(&self.repo_path)?;
                Ok(head.oid)
            }
            VcsBackend::Jujutsu => {
                let jj = JujutsuCli::new();
                jj.describe(&self.repo_path, message)?;
                // Create a new change for the next commit
                jj.new_change(&self.repo_path, None)?;
                Ok("jj-change-id".to_string()) // JJ uses stable change IDs
            }
        }
    }

    /// Create a new branch
    pub fn create_branch(&self, name: &str) {
        match self.backend {
            VcsBackend::Git => {
                let repo = Repository::open(&self.repo_path).unwrap();
                let head = repo.head().unwrap().peel_to_commit().unwrap();
                let _ = repo.branch(name, &head, true).unwrap();
            }
            VcsBackend::Jujutsu => {
                let jj = JujutsuCli::new();
                jj.branch_create(&self.repo_path, name, None)
                    .expect("create jj branch");
            }
        }
    }

    /// Checkout a branch (or revision in JJ)
    pub fn checkout(&self, branch_name: &str) {
        match self.backend {
            VcsBackend::Git => {
                checkout_git_branch(&self.repo_path, branch_name);
            }
            VcsBackend::Jujutsu => {
                let jj = JujutsuCli::new();
                // In JJ, we edit the change associated with the branch
                jj.edit(&self.repo_path, branch_name)
                    .expect("checkout jj branch");
            }
        }
    }

    /// Check if working tree is clean
    pub fn is_clean(&self) -> bool {
        match self.backend {
            VcsBackend::Git => {
                let service = GitService::new();
                service
                    .is_worktree_clean(&self.repo_path)
                    .unwrap_or(false)
            }
            VcsBackend::Jujutsu => {
                let jj = JujutsuCli::new();
                jj.status(&self.repo_path)
                    .map(|status| !status.has_changes)
                    .unwrap_or(false)
            }
        }
    }
}

/// Write a file at the given path
pub fn write_file<P: AsRef<Path>>(base: P, rel: &str, content: &str) {
    let path = base.as_ref().join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
}

/// Configure git user for a repository
pub fn configure_git_user(repo_path: &Path, name: &str, email: &str) {
    let repo = Repository::open(repo_path).unwrap();
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", name).unwrap();
    cfg.set_str("user.email", email).unwrap();
}

/// Checkout a git branch
pub fn checkout_git_branch(repo_path: &Path, name: &str) {
    let repo = Repository::open(repo_path).unwrap();
    repo.set_head(&format!("refs/heads/{name}")).unwrap();
    let mut co = CheckoutBuilder::new();
    co.force();
    repo.checkout_head(Some(&mut co)).unwrap();
}

/// Helper to add a file to git index
pub fn git_add_path(repo_path: &Path, path: &str) {
    let git = GitCli::new();
    git.git(repo_path, ["add", path]).unwrap();
}

/// Macro to run a test with all available VCS backends
#[macro_export]
macro_rules! test_with_backends {
    ($test_name:ident, $test_fn:expr) => {
        #[test]
        fn $test_name() {
            for backend in $crate::vcs_test_utils::VcsBackend::available() {
                println!("Running {} with backend: {:?}", stringify!($test_name), backend);
                $test_fn(backend);
            }
        }
    };
}

/// Macro to run a test only with Git (for git-specific features)
#[macro_export]
macro_rules! test_git_only {
    ($test_name:ident, $test_fn:expr) => {
        #[test]
        fn $test_name() {
            println!("Running {} with Git backend", stringify!($test_name));
            $test_fn($crate::vcs_test_utils::VcsBackend::Git);
        }
    };
}

/// Macro to run a test only with Jujutsu (for jj-specific features)
#[macro_export]
macro_rules! test_jj_only {
    ($test_name:ident, $test_fn:expr) => {
        #[test]
        fn $test_name() {
            if !$crate::vcs_test_utils::is_jj_available() {
                eprintln!("Skipping {}: jj not available", stringify!($test_name));
                return;
            }
            println!("Running {} with Jujutsu backend", stringify!($test_name));
            $test_fn($crate::vcs_test_utils::VcsBackend::Jujutsu);
        }
    };
}

/// Check if jj is available in the system
pub fn is_jj_available() -> bool {
    use std::process::Command;
    Command::new("jj")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}
