#[cfg(test)]
mod jj_integration_tests {
    use std::path::Path;
    use tempfile::TempDir;
    use crate::services::git::{GitService, JjCli};

    fn setup_test_jj_repo() -> Result<TempDir, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let jj = JjCli::new();
        
        // Skip tests if jj is not available
        if !jj.is_available() {
            return Err("jj not available".into());
        }
        
        // Initialize jj repo with git backend
        std::process::Command::new("jj")
            .current_dir(temp_dir.path())
            .args(["git", "init", "--git-repo", "."])
            .output()?;
        
        Ok(temp_dir)
    }

    #[test]
    fn test_jj_is_available() {
        let jj = JjCli::new();
        // Test passes whether jj is installed or not
        let _ = jj.is_available();
    }

    #[test]
    fn test_jj_repo_detection() {
        let jj = JjCli::new();
        if !jj.is_available() {
            eprintln!("Skipping test: jj not available");
            return;
        }

        if let Ok(temp_dir) = setup_test_jj_repo() {
            let result = jj.is_jj_repo(temp_dir.path());
            assert!(result.is_ok());
            if let Ok(is_jj) = result {
                assert!(is_jj);
            }
        }
    }

    #[test]
    fn test_jj_git_backend_detection() {
        let jj = JjCli::new();
        if !jj.is_available() {
            eprintln!("Skipping test: jj not available");
            return;
        }

        if let Ok(temp_dir) = setup_test_jj_repo() {
            let result = jj.has_git_backend(temp_dir.path());
            assert!(result.is_ok());
            if let Ok(has_backend) = result {
                assert!(has_backend);
            }
        }
    }

    #[test]
    fn test_jj_git_export() {
        let jj = JjCli::new();
        if !jj.is_available() {
            eprintln!("Skipping test: jj not available");
            return;
        }

        if let Ok(temp_dir) = setup_test_jj_repo() {
            let result = jj.git_export(temp_dir.path());
            assert!(result.is_ok(), "git export should succeed: {:?}", result);
        }
    }

    #[test]
    fn test_jj_git_import() {
        let jj = JjCli::new();
        if !jj.is_available() {
            eprintln!("Skipping test: jj not available");
            return;
        }

        if let Ok(temp_dir) = setup_test_jj_repo() {
            let result = jj.git_import(temp_dir.path());
            assert!(result.is_ok(), "git import should succeed: {:?}", result);
        }
    }

    #[test]
    fn test_jj_branch_create() {
        let jj = JjCli::new();
        if !jj.is_available() {
            eprintln!("Skipping test: jj not available");
            return;
        }

        if let Ok(temp_dir) = setup_test_jj_repo() {
            let result = jj.branch_create(temp_dir.path(), "test-branch", Some("@"));
            assert!(result.is_ok(), "branch create should succeed: {:?}", result);
        }
    }

    #[test]
    fn test_git_service_jj_integration() {
        let jj = JjCli::new();
        if !jj.is_available() {
            eprintln!("Skipping test: jj not available");
            return;
        }

        if let Ok(temp_dir) = setup_test_jj_repo() {
            let git_service = GitService::new();
            
            // Test is_jj_repo
            let result = git_service.is_jj_repo(temp_dir.path());
            assert!(result.is_ok());
            if let Ok(is_jj) = result {
                assert!(is_jj);
            }
            
            // Test git export
            let export_result = git_service.jj_git_export(temp_dir.path());
            assert!(export_result.is_ok(), "jj git export should succeed: {:?}", export_result);
            
            // Test git import
            let import_result = git_service.jj_git_import(temp_dir.path());
            assert!(import_result.is_ok(), "jj git import should succeed: {:?}", import_result);
        }
    }

    #[test]
    fn test_jj_error_classification() {
        let jj = JjCli::new();
        
        use crate::services::git::JjCliError;
        
        let auth_err = jj.classify_error("Authentication failed: permission denied".to_string());
        assert!(matches!(auth_err, JjCliError::AuthFailed(_)));
        
        let push_err = jj.classify_error("Push rejected: non-fast-forward update".to_string());
        assert!(matches!(push_err, JjCliError::PushRejected(_)));
        
        let repo_err = jj.classify_error("Error: Not a jj repo at this path".to_string());
        assert!(matches!(repo_err, JjCliError::NotJjRepo(_)));
        
        let generic_err = jj.classify_error("Some other error".to_string());
        assert!(matches!(generic_err, JjCliError::CommandFailed(_)));
    }

    #[test]
    fn test_jj_workflow_simulation() {
        let jj = JjCli::new();
        if !jj.is_available() {
            eprintln!("Skipping test: jj not available");
            return;
        }

        if let Ok(temp_dir) = setup_test_jj_repo() {
            // Simulate a workflow: export -> import cycle
            let export_result = jj.git_export(temp_dir.path());
            assert!(export_result.is_ok());
            
            let import_result = jj.git_import(temp_dir.path());
            assert!(import_result.is_ok());
            
            // Create a branch
            let branch_result = jj.branch_create(temp_dir.path(), "feature-branch", Some("@"));
            assert!(branch_result.is_ok());
            
            // Export again to update git refs
            let final_export = jj.git_export(temp_dir.path());
            assert!(final_export.is_ok());
        }
    }
}
