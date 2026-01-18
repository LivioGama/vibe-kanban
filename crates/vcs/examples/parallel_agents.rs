//! Example: VCS-based Parallel Agent Coordination
//!
//! Run with: cargo run --example parallel_agents --features git

use std::path::PathBuf;
use vcs::{BranchOrChange, ChangeId, CreateChangeOptions, VcsBackend, VcsConfig, VcsFactory};

/// Represents a task assigned to an AI agent
pub struct AgentTask {
    pub id: String,
    pub change_id: ChangeId,
    pub base_change: ChangeId,
    pub description: String,
}

/// Coordinator for managing parallel agent tasks
pub struct ParallelAgentCoordinator {
    vcs: Box<dyn VcsBackend>,
}

impl ParallelAgentCoordinator {
    /// Create a new coordinator for the given repository
    pub fn new(config: VcsConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let vcs = VcsFactory::create(&config)?;
        Ok(Self { vcs })
    }

    /// Create a new task for an agent to work on
    pub fn create_task(
        &self,
        task_id: &str,
        description: &str,
    ) -> Result<AgentTask, Box<dyn std::error::Error>> {
        let base = self.vcs.head()?.change_id;
        let branch_name = format!("agent-task-{}", task_id);
        self.vcs.create_branch(&branch_name, Some(&base))?;

        Ok(AgentTask {
            id: task_id.to_string(),
            change_id: base.clone(),
            base_change: base,
            description: description.to_string(),
        })
    }

    /// Complete a task by creating a change
    pub fn complete_task(
        &self,
        task: &AgentTask,
        message: &str,
    ) -> Result<ChangeId, Box<dyn std::error::Error>> {
        let branch = BranchOrChange::Branch(format!("agent-task-{}", task.id));
        self.vcs.switch_to(&branch)?;

        let options = CreateChangeOptions {
            stage_all: true,
            ..Default::default()
        };
        let change_id = self.vcs.create_change_with_options(message, options)?;
        Ok(change_id)
    }

    /// List all active agent tasks
    pub fn list_tasks(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let branches = self.vcs.list_branches()?;
        let tasks: Vec<String> = branches
            .into_iter()
            .filter_map(|b| {
                if b.name.starts_with("agent-task-") {
                    Some(b.name.trim_start_matches("agent-task-").to_string())
                } else {
                    None
                }
            })
            .collect();
        Ok(tasks)
    }

    /// Clean up a completed task
    pub fn cleanup_task(&self, task_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let branch_name = format!("agent-task-{}", task_id);
        self.vcs.delete_branch(&branch_name)?;
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use vcs::VcsBackendType;

    println!("VCS Parallel Agent Coordination Example\n");

    let config = VcsConfig {
        backend_type: VcsBackendType::Git,
        path: PathBuf::from("./my-project"),
    };

    let coordinator = ParallelAgentCoordinator::new(config)?;

    println!("Creating tasks for parallel agents...");
    let task1 = coordinator.create_task("001", "Implement user authentication")?;
    println!("✓ Task 1: {}", task1.description);

    let task2 = coordinator.create_task("002", "Add database migrations")?;
    println!("✓ Task 2: {}", task2.description);

    let task3 = coordinator.create_task("003", "Update API documentation")?;
    println!("✓ Task 3: {}", task3.description);

    println!("\nActive tasks:");
    for task_id in coordinator.list_tasks()? {
        println!("  → Agent task: {}", task_id);
    }

    println!("\nCompleting tasks...");
    let _change1 = coordinator.complete_task(&task1, "feat: Add JWT authentication")?;
    println!("✓ Agent 001 completed");

    let _change2 = coordinator.complete_task(&task2, "feat: Add migration framework")?;
    println!("✓ Agent 002 completed");

    let _change3 = coordinator.complete_task(&task3, "docs: Update API reference")?;
    println!("✓ Agent 003 completed");

    println!("\nCleaning up...");
    coordinator.cleanup_task("001")?;
    coordinator.cleanup_task("002")?;
    coordinator.cleanup_task("003")?;

    println!("✓ All tasks completed!\n");

    Ok(())
}
