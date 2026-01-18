// Example: Using Jujutsu (jj) Git Interop for GitHub Workflows
//
// This example demonstrates how to use jj with Git-based forges
// like GitHub and GitLab for pull request workflows.

use std::path::Path;
use services::GitService;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let git_service = GitService::new();
    let repo_path = Path::new("/path/to/your/jj/repo");

    // Check if this is a jj repository
    if !git_service.is_jj_repo(repo_path)? {
        println!("This is not a jj repository!");
        return Ok(());
    }

    println!("✓ Detected jj repository");

    // Example 1: Sync with remote before starting work
    println!("\n1. Syncing with remote...");
    git_service.jj_sync_with_git(repo_path, Some("origin"))?;
    println!("✓ Synced with origin");

    // Example 2: After making changes, prepare for PR
    println!("\n2. Preparing changes for PR...");
    let branch_name = "feature/my-awesome-feature";
    
    // This will:
    // - Create a branch pointing to current change
    // - Export to git
    // - Push to remote
    git_service.jj_prepare_for_pr(
        repo_path,
        branch_name,
        "origin"
    )?;
    println!("✓ Branch '{}' pushed to origin", branch_name);
    println!("  You can now create a PR on GitHub/GitLab!");

    // Example 3: Manual workflow (more control)
    println!("\n3. Manual workflow example...");
    
    // Create a branch at current revision
    git_service.jj_branch_create(repo_path, "another-branch", Some("@"))?;
    println!("✓ Created branch 'another-branch'");
    
    // Export jj changes to git
    git_service.jj_git_export(repo_path)?;
    println!("✓ Exported to git");
    
    // Push to remote
    git_service.jj_git_push(
        repo_path,
        Some("origin"),
        Some("another-branch"),
        None,
        false // don't force
    )?;
    println!("✓ Pushed 'another-branch' to origin");

    // Example 4: Fetch updates from remote
    println!("\n4. Fetching updates...");
    git_service.jj_git_fetch(repo_path, Some("origin"), None)?;
    git_service.jj_git_import(repo_path)?;
    println!("✓ Fetched and imported updates from origin");

    // Example 5: Update existing branch
    println!("\n5. Updating existing branch...");
    git_service.jj_branch_set(repo_path, branch_name, "@")?;
    git_service.jj_git_export(repo_path)?;
    git_service.jj_git_push(
        repo_path,
        Some("origin"),
        Some(branch_name),
        None,
        true // force push since we're updating
    )?;
    println!("✓ Updated and force-pushed '{}'", branch_name);

    println!("\n✓ All operations completed successfully!");
    Ok(())
}

// Example with error handling
fn example_with_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    use services::git::JjCliError;
    
    let git_service = GitService::new();
    let repo_path = Path::new("/path/to/repo");
    
    // Handle specific errors
    match git_service.jj_git_push(
        repo_path,
        Some("origin"),
        Some("my-branch"),
        None,
        false
    ) {
        Ok(_) => println!("Push successful!"),
        Err(e) => {
            let error_msg = format!("{}", e);
            if error_msg.contains("Authentication failed") {
                println!("Authentication error: Please check your credentials");
            } else if error_msg.contains("Push rejected") {
                println!("Push rejected: Try force push or pull first");
            } else {
                println!("Push failed: {}", error_msg);
            }
        }
    }
    
    Ok(())
}

// Example: Complete workflow for contributing to a project
fn complete_contribution_workflow() -> Result<(), Box<dyn std::error::Error>> {
    let git_service = GitService::new();
    let repo_path = Path::new("/path/to/repo");
    
    println!("=== Complete Contribution Workflow ===\n");
    
    // Step 1: Sync with upstream
    println!("1. Syncing with upstream...");
    git_service.jj_sync_with_git(repo_path, Some("origin"))?;
    
    // Step 2: Make your changes (using jj commands externally)
    println!("2. Make your changes using jj commands");
    println!("   $ jj new main -m 'My feature'");
    println!("   $ # Make code changes");
    println!("   $ jj commit");
    
    // Step 3: Prepare for PR
    println!("\n3. Preparing for PR...");
    git_service.jj_prepare_for_pr(
        repo_path,
        "feature/my-contribution",
        "origin"
    )?;
    
    // Step 4: Create PR on GitHub/GitLab
    println!("\n4. Create PR:");
    println!("   - Go to GitHub/GitLab");
    println!("   - Create PR from 'feature/my-contribution' to 'main'");
    
    // Step 5: Make updates based on review
    println!("\n5. After review, make updates:");
    println!("   $ jj new @- -m 'Address review comments'");
    println!("   $ # Make changes");
    println!("   $ jj commit");
    
    // Step 6: Push updates
    println!("\n6. Pushing updates...");
    git_service.jj_git_export(repo_path)?;
    git_service.jj_git_push(
        repo_path,
        Some("origin"),
        Some("feature/my-contribution"),
        None,
        true // force push to update PR
    )?;
    
    println!("\n✓ Contribution workflow complete!");
    println!("  Your PR is updated and ready for merge!");
    
    Ok(())
}
