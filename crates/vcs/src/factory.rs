use crate::error::VcsError;
use crate::traits::VcsBackend;
use std::path::{Path, PathBuf};

/// Type of VCS backend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsBackendType {
    Git,
    Jujutsu,
}

/// Configuration for VCS backend
#[derive(Debug, Clone)]
pub struct VcsConfig {
    pub backend_type: VcsBackendType,
    pub path: PathBuf,
}

/// Factory for creating VCS backends
pub struct VcsFactory;

impl VcsFactory {
    /// Create a backend based on configuration
    #[cfg(feature = "git")]
    pub fn create(config: &VcsConfig) -> Result<Box<dyn VcsBackend>, VcsError> {
        use crate::traits::VcsRepository;
        
        match config.backend_type {
            VcsBackendType::Git => {
                let git_repo = crate::backend::git::GitRepository::open(&config.path)?;
                Ok(Box::new(git_repo))
            }
            VcsBackendType::Jujutsu => {
                Err(VcsError::InvalidOperation(
                    "Jujutsu backend not yet implemented".to_string(),
                ))
            }
        }
    }

    #[cfg(not(feature = "git"))]
    pub fn create(_config: &VcsConfig) -> Result<Box<dyn VcsBackend>, VcsError> {
        Err(VcsError::InvalidOperation(
            "No VCS backend features enabled".to_string(),
        ))
    }

    /// Auto-detect backend from existing repository
    pub fn detect(path: &Path) -> Result<VcsBackendType, VcsError> {
        if path.join(".jj").exists() {
            Ok(VcsBackendType::Jujutsu)
        } else if path.join(".git").exists() {
            Ok(VcsBackendType::Git)
        } else {
            Err(VcsError::repo_not_found(path))
        }
    }

    /// Create a backend by auto-detecting the type
    pub fn auto_detect(path: &Path) -> Result<Box<dyn VcsBackend>, VcsError> {
        let backend_type = Self::detect(path)?;
        Self::create(&VcsConfig {
            backend_type,
            path: path.to_path_buf(),
        })
    }
}
