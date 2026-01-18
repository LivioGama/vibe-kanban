//! Utilities for checking and setting up Jujutsu (jj) version control

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::shell::resolve_executable_path;

#[derive(Debug, Error)]
pub enum JjError {
    #[error("jj executable not found. Please install Jujutsu:\n\n  • Homebrew (macOS/Linux): brew install jj\n  • Cargo: cargo install --locked jj-cli\n  • Binary downloads: https://github.com/martinvonz/jj/releases\n\nFor more information, visit: https://martinvonz.github.io/jj/")]
    NotInstalled,
    #[error("Failed to execute jj command: {0}")]
    CommandFailed(String),
    #[error("Repository not initialized as jj repo: {0}")]
    NotJjRepo(String),
}

/// Check if jj is installed and available in PATH
pub async fn check_jj_installed() -> Result<PathBuf, JjError> {
    resolve_executable_path("jj")
        .await
        .ok_or(JjError::NotInstalled)
}

/// Check if a directory is a jj repository
pub async fn is_jj_repo(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }

    // Check for .jj directory
    let jj_dir = path.join(".jj");
    if jj_dir.exists() && jj_dir.is_dir() {
        return true;
    }

    // Could also check parent directories recursively
    if let Some(parent) = path.parent() {
        let parent_jj = parent.join(".jj");
        if parent_jj.exists() && parent_jj.is_dir() {
            return true;
        }
    }

    false
}

/// Initialize a jj repository
pub async fn init_jj_repo(path: &Path) -> Result<(), JjError> {
    use tokio::process::Command;

    let jj_path = check_jj_installed().await?;

    let output = Command::new(&jj_path)
        .arg("init")
        .arg("--git")
        .current_dir(path)
        .output()
        .await
        .map_err(|e| JjError::CommandFailed(format!("Failed to run jj init: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(JjError::CommandFailed(format!(
            "jj init failed: {}",
            stderr
        )));
    }

    Ok(())
}

/// Get the default jj config content
pub fn get_default_jj_config() -> &'static str {
    r#"# Jujutsu configuration for Vibe Kanban
# For more information: https://martinvonz.github.io/jj/latest/config/

[user]
# name = "Your Name"
# email = "your.email@example.com"

[ui]
# Use the default diff tool
diff-editor = "diff"
# Show relative timestamps in logs
relative-timestamps = true
# Enable colored output
color = "auto"

[git]
# Automatically push branches to git remote
push-branch-prefix = ""
# Fetch from all remotes by default
auto-local-branch = true

[revsets]
# Log shows local branches plus main and master
log = "@ | branches() | (main | master)"

[aliases]
# Common shortcuts
st = ["status"]
l = ["log"]
show = ["show"]
"#
}

/// Write default jj config to user's config directory
pub async fn setup_jj_config() -> Result<PathBuf, JjError> {
    use std::io::Write;

    let config_dir = dirs::config_dir()
        .ok_or_else(|| JjError::CommandFailed("Could not determine config directory".into()))?
        .join("jj");

    tokio::fs::create_dir_all(&config_dir)
        .await
        .map_err(|e| JjError::CommandFailed(format!("Failed to create config dir: {}", e)))?;

    let config_path = config_dir.join("config.toml");

    // Only create config if it doesn't exist
    if !config_path.exists() {
        let mut file = std::fs::File::create(&config_path)
            .map_err(|e| JjError::CommandFailed(format!("Failed to create config file: {}", e)))?;

        file.write_all(get_default_jj_config().as_bytes())
            .map_err(|e| JjError::CommandFailed(format!("Failed to write config: {}", e)))?;
    }

    Ok(config_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid_toml() {
        let config = get_default_jj_config();
        // Just verify it parses as TOML
        assert!(toml::from_str::<toml::Value>(config).is_ok());
    }
}
