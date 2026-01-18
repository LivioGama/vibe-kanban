# VCS Abstraction Layer Design

## Executive Summary

This document outlines a trait-based abstraction layer to support both **Git** and **Jujutsu (jj)** version control systems in the Vibe Kanban codebase. The design prioritizes clean interfaces, minimal migration impact, and leverages jj's no-worktree paradigm for simpler parallel agent workflows.

---

## Background

### Current Git Usage Patterns

The codebase currently uses a **hybrid approach**:
- **libgit2** (via `git2` crate): Read-only graph operations, repository queries
- **Git CLI**: Destructive/working-tree mutations (checkout, merge, rebase, commit)

**Key locations:**
- `crates/services/src/services/git.rs` - Core git service with graph queries, branch management
- `crates/services/src/services/git/cli.rs` - CLI-based operations for safety
- `crates/services/src/services/worktree_manager.rs` - Worktree lifecycle management

### Why Jujutsu?

**Jujutsu (jj)** is a next-generation VCS that offers:

1. **No worktree complexity**: Every operation works directly with commits, no checkout needed
2. **True parallel workflows**: Multiple agents can work on different changes simultaneously without worktree conflicts
3. **Automatic conflict tracking**: Conflicts are first-class objects that can be resolved later
4. **Git interoperability**: jj can work with Git backends while providing better UX
5. **Simpler mental model**: No staging area, no detached HEAD states

**Perfect fit for Vibe Kanban's multi-agent architecture** where multiple AI agents work on different tasks in parallel.

---

## Design Principles

1. **Trait-based abstraction**: Define clear traits that both implementations satisfy
2. **Backend-agnostic operations**: Application code should not know which VCS is in use
3. **Minimal migration impact**: Existing Git code continues to work; new code can use abstraction
4. **Progressive adoption**: Start with core operations, expand as needed
5. **Type safety**: Use Rust's type system to prevent misuse
6. **No leaky abstractions**: Hide VCS-specific details from callers

---

## Core Abstractions

### 1. Repository Operations

```rust
/// Represents a VCS repository (Git or Jujutsu)
pub trait VcsRepository: Send + Sync {
    /// Initialize a new repository
    fn init(path: &Path) -> Result<Self, VcsError> where Self: Sized;
    
    /// Open an existing repository
    fn open(path: &Path) -> Result<Self, VcsError> where Self: Sized;
    
    /// Clone from a remote URL
    fn clone(url: &str, path: &Path) -> Result<Self, VcsError> where Self: Sized;
    
    /// Get the working directory path
    fn work_dir(&self) -> &Path;
    
    /// Check if repository is in a clean state (no conflicts, no ongoing operations)
    fn is_clean(&self) -> Result<bool, VcsError>;
    
    /// Get the current head information
    fn head(&self) -> Result<HeadInfo, VcsError>;
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
```

### 2. Change Management

```rust
/// Represents a single change/commit in the VCS
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChangeId(String);

impl ChangeId {
    pub fn new(id: String) -> Self {
        Self(id)
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
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

/// Trait for change/commit operations
pub trait VcsChanges {
    /// Create a new change/commit with given message
    fn create_change(&self, message: &str) -> Result<ChangeId, VcsError>;
    
    /// Amend the current change/commit
    fn amend_change(&self, message: Option<&str>) -> Result<(), VcsError>;
    
    /// Get information about a specific change
    fn get_change(&self, id: &ChangeId) -> Result<ChangeInfo, VcsError>;
    
    /// List changes/commits in the repository
    fn list_changes(&self, filter: ChangeFilter) -> Result<Vec<ChangeInfo>, VcsError>;
    
    /// Abandon/delete a change (jj) or reset/revert (git)
    fn abandon_change(&self, id: &ChangeId) -> Result<(), VcsError>;
}

#[derive(Debug, Clone, Default)]
pub struct ChangeFilter {
    pub branch: Option<String>,
    pub author: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}
```

### 3. Branch Operations

```rust
/// Branch management operations
pub trait VcsBranches {
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
    fn switch_to(&self, target: &BranchOrChange) -> Result<(), VcsError>;
}

#[derive(Debug, Clone)]
pub enum BranchOrChange {
    Branch(String),
    Change(ChangeId),
}

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub change_id: ChangeId,
    pub is_current: bool,
    pub is_remote: bool,
    pub last_updated: DateTime<Utc>,
}
```

### 4. Remote Operations

```rust
/// Remote repository operations
pub trait VcsRemotes {
    /// Fetch changes from remote
    fn fetch(&self, remote: Option<&str>) -> Result<(), VcsError>;
    
    /// Push changes to remote
    fn push(&self, remote: Option<&str>, branch: Option<&str>) -> Result<(), VcsError>;
    
    /// Check if remote branch exists
    fn remote_branch_exists(&self, remote: &str, branch: &str) -> Result<bool, VcsError>;
    
    /// Get remote URL
    fn get_remote_url(&self, name: &str) -> Result<String, VcsError>;
    
    /// Set remote URL
    fn set_remote_url(&self, name: &str, url: &str) -> Result<(), VcsError>;
}
```

### 5. Diff Operations

```rust
/// Diff and status operations
pub trait VcsDiff {
    /// Get diff between two changes/commits
    fn diff_changes(&self, from: &ChangeId, to: &ChangeId) -> Result<Vec<FileDiff>, VcsError>;
    
    /// Get diff for uncommitted changes (working copy)
    fn diff_uncommitted(&self) -> Result<Vec<FileDiff>, VcsError>;
    
    /// Get status of files in working copy
    fn status(&self) -> Result<Vec<FileStatus>, VcsError>;
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub change_type: FileChangeType,
    pub additions: usize,
    pub deletions: usize,
    pub content: Option<DiffContent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
}

#[derive(Debug, Clone)]
pub struct DiffContent {
    pub old_content: Option<Vec<u8>>,
    pub new_content: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct FileStatus {
    pub path: String,
    pub status: FileStatusKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatusKind {
    Untracked,
    Modified,
    Added,
    Deleted,
    Conflicted,
}
```

### 6. Conflict Management

```rust
/// Conflict detection and resolution
pub trait VcsConflicts {
    /// Check if there are any conflicts
    fn has_conflicts(&self) -> Result<bool, VcsError>;
    
    /// List all conflicted files
    fn list_conflicts(&self) -> Result<Vec<ConflictInfo>, VcsError>;
    
    /// Mark a conflict as resolved
    fn resolve_conflict(&self, path: &Path) -> Result<(), VcsError>;
    
    /// Abort ongoing operation (merge/rebase)
    fn abort_operation(&self) -> Result<(), VcsError>;
}

#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub path: String,
    pub operation: ConflictOperation,
    pub sides: ConflictSides,
}

#[derive(Debug, Clone)]
pub enum ConflictOperation {
    Merge,
    Rebase,
    CherryPick,
    Revert,
}

#[derive(Debug, Clone)]
pub struct ConflictSides {
    pub ours: ChangeId,
    pub theirs: ChangeId,
    pub base: Option<ChangeId>,
}
```

### 7. Error Handling

```rust
#[derive(Debug, Error)]
pub enum VcsError {
    #[error("Repository not found: {0}")]
    RepositoryNotFound(String),
    
    #[error("Invalid change ID: {0}")]
    InvalidChangeId(String),
    
    #[error("Branch not found: {0}")]
    BranchNotFound(String),
    
    #[error("Conflict in files: {0}")]
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
}
```

---

## Implementation Strategy

### Phase 1: Core Trait Definition (Week 1)

1. Create `crates/vcs/` new crate with core traits
2. Define all trait interfaces without implementations
3. Add comprehensive documentation and examples
4. Review with team for API ergonomics

### Phase 2: Git Adapter (Week 2-3)

1. Create `GitRepository` struct implementing all traits
2. Wrap existing `GitService` and `GitCli` functionality
3. Add integration tests comparing old vs new API
4. Ensure feature parity with current implementation

### Phase 3: Jujutsu Implementation (Week 4-6)

1. Add `jj-lib` dependency (Jujutsu's Rust library)
2. Implement `JjRepository` for all traits
3. Focus on worktree-free operations
4. Add comprehensive test suite

### Phase 4: Parallel Agent Support (Week 7-8)

1. Design "working copy" abstraction for jj
2. Implement concurrent change creation without worktrees
3. Add agent coordination primitives
4. Test multi-agent scenarios

### Phase 5: Migration & Refinement (Week 9-12)

1. Gradually migrate existing code to use traits
2. Add VCS selection configuration
3. Performance optimization
4. Production testing

---

## Key Implementation Details

### Git Implementation Approach

```rust
pub struct GitRepository {
    path: PathBuf,
    git_service: GitService,
    git_cli: GitCli,
}

impl VcsRepository for GitRepository {
    fn init(path: &Path) -> Result<Self, VcsError> {
        let git_service = GitService::new();
        git_service.initialize_repo_with_main_branch(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            git_service,
            git_cli: GitCli::new(),
        })
    }
    
    // ... other methods wrap GitService/GitCli
}

impl VcsChanges for GitRepository {
    fn create_change(&self, message: &str) -> Result<ChangeId, VcsError> {
        // For git: commit current changes
        self.git_service.commit(&self.path, message)?;
        let head = self.git_service.open_repo(&self.path)?.head()?;
        let oid = head.target().ok_or(VcsError::InvalidChangeId("HEAD".into()))?;
        Ok(ChangeId::new(oid.to_string()))
    }
}
```

### Jujutsu Implementation Approach

```rust
pub struct JjRepository {
    path: PathBuf,
    workspace: jj_lib::workspace::Workspace,
}

impl VcsRepository for JjRepository {
    fn init(path: &Path) -> Result<Self, VcsError> {
        // jj init with git backend
        let workspace = jj_lib::workspace::Workspace::init_internal_git(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            workspace,
        })
    }
}

impl VcsChanges for JjRepository {
    fn create_change(&self, message: &str) -> Result<ChangeId, VcsError> {
        // For jj: create new change without checkout
        let mut tx = self.workspace.start_transaction("create change")?;
        let new_change = tx.new_commit()
            .set_description(message)
            .write()?;
        tx.commit()?;
        Ok(ChangeId::new(new_change.id().hex()))
    }
}
```

### Worktree-Free Parallel Operations with Jujutsu

One of jj's biggest advantages is **no worktree needed** for most operations:

```rust
/// Agent task coordinator for parallel work
pub struct VcsAgentCoordinator<R: VcsRepository + VcsChanges> {
    repo: R,
}

impl<R> VcsAgentCoordinator<R> 
where 
    R: VcsRepository + VcsChanges + VcsBranches 
{
    /// Create a new isolated change for an agent to work on
    pub fn create_agent_task(&self, task_name: &str) -> Result<AgentTask, VcsError> {
        // With jj: just create a new change, no worktree needed
        let base = self.repo.head()?.change_id;
        let change_id = self.repo.create_change(&format!("Task: {}", task_name))?;
        
        Ok(AgentTask {
            change_id,
            base_change: base,
            name: task_name.to_string(),
        })
    }
    
    /// Agent applies changes to their task (no worktree checkout)
    pub fn apply_changes_to_task(
        &self,
        task: &AgentTask,
        changes: Vec<FileChange>,
    ) -> Result<(), VcsError> {
        // With jj: directly modify the change without worktree
        // With git: would need a worktree
        
        // jj implementation:
        // let mut tx = self.repo.start_transaction("apply changes")?;
        // let commit = tx.get_commit(&task.change_id)?;
        // let mut tree_builder = commit.tree().as_ref().builder();
        // for change in changes {
        //     tree_builder.set_file(&change.path, change.content);
        // }
        // let new_tree = tree_builder.write()?;
        // tx.rewrite_commit(&commit).set_tree(new_tree).write()?;
        // tx.commit()?;
        
        Ok(())
    }
}

pub struct AgentTask {
    pub change_id: ChangeId,
    pub base_change: ChangeId,
    pub name: String,
}

pub struct FileChange {
    pub path: PathBuf,
    pub content: Vec<u8>,
}
```

---

## Unified Backend Interface

```rust
/// Main VCS backend that combines all traits
pub trait VcsBackend: 
    VcsRepository 
    + VcsChanges 
    + VcsBranches 
    + VcsRemotes 
    + VcsDiff 
    + VcsConflicts 
{
    /// Get backend type
    fn backend_type(&self) -> VcsBackendType;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsBackendType {
    Git,
    Jujutsu,
}

/// Factory for creating VCS backends
pub struct VcsFactory;

impl VcsFactory {
    /// Create a backend based on configuration
    pub fn create(config: &VcsConfig) -> Result<Box<dyn VcsBackend>, VcsError> {
        match config.backend_type {
            VcsBackendType::Git => Ok(Box::new(GitRepository::open(&config.path)?)),
            VcsBackendType::Jujutsu => Ok(Box::new(JjRepository::open(&config.path)?)),
        }
    }
    
    /// Auto-detect backend from existing repository
    pub fn detect(path: &Path) -> Result<VcsBackendType, VcsError> {
        if path.join(".jj").exists() {
            Ok(VcsBackendType::Jujutsu)
        } else if path.join(".git").exists() {
            Ok(VcsBackendType::Git)
        } else {
            Err(VcsError::RepositoryNotFound(path.display().to_string()))
        }
    }
}

#[derive(Debug, Clone)]
pub struct VcsConfig {
    pub backend_type: VcsBackendType,
    pub path: PathBuf,
}
```

---

## Migration Guide

### Before (Direct Git usage):

```rust
let git_service = GitService::new();
let repo = git_service.open_repo(&path)?;
git_service.commit(&path, "My change")?;
let branches = git_service.get_all_branches(&repo)?;
```

### After (VCS abstraction):

```rust
let vcs = VcsFactory::create(&config)?;
let change_id = vcs.create_change("My change")?;
let branches = vcs.list_branches()?;
```

### Gradual Migration Strategy:

1. **New features**: Use VCS abstraction from day one
2. **Hot paths**: Migrate one module at a time
3. **Tests**: Dual-test both old and new APIs during transition
4. **Compatibility layer**: Keep `GitService` as implementation detail

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    fn test_with_backend<R: VcsBackend>(repo: R) {
        // Test core operations
        let change_id = repo.create_change("Test change").unwrap();
        let info = repo.get_change(&change_id).unwrap();
        assert_eq!(info.description, "Test change");
    }
    
    #[test]
    fn test_git_backend() {
        let temp = tempdir().unwrap();
        let git = GitRepository::init(temp.path()).unwrap();
        test_with_backend(git);
    }
    
    #[test]
    fn test_jj_backend() {
        let temp = tempdir().unwrap();
        let jj = JjRepository::init(temp.path()).unwrap();
        test_with_backend(jj);
    }
}
```

### Integration Tests

- Multi-agent parallel operations
- Git-jj interoperability (jj can read/write git repos)
- Performance benchmarks
- Real-world workflow scenarios

---

## Performance Considerations

### Git Performance

- Continue using CLI for safety-critical operations
- Use libgit2 for read-heavy graph queries
- Worktree operations have filesystem overhead

### Jujutsu Performance

- jj operations are generally faster due to:
  - Optimized Rust implementation
  - No worktree checkout needed
  - Better index structure
- Parallel operations scale better (no worktree conflicts)

### Optimization Strategies

1. **Lazy loading**: Don't materialize full diffs unless needed
2. **Caching**: Cache branch lists, change info
3. **Batch operations**: Group multiple changes into single transaction
4. **Async where possible**: Network operations should be async

---

## Configuration

### Environment Variables

```bash
# Select VCS backend
VIBE_VCS_BACKEND=jj  # or "git"

# For testing
VIBE_VCS_TEST_BOTH=true  # Run tests with both backends
```

### Repository Configuration

```toml
# .vibe/config.toml
[vcs]
backend = "jj"  # or "git"
default_branch = "main"

[vcs.git]
# Git-specific options
worktree_strategy = "persistent"  # or "ephemeral"

[vcs.jj]
# Jujutsu-specific options
colocate_with_git = true  # Keep .git alongside .jj
```

---

## Benefits Summary

### For Existing Git Users
- ✅ No breaking changes - Git continues to work
- ✅ Gradual migration path
- ✅ Same workflows still supported

### For Jujutsu Adoption
- ✅ **No worktree complexity** - Simpler mental model
- ✅ **True parallel workflows** - Multiple agents work simultaneously
- ✅ **Better conflict handling** - Conflicts are first-class objects
- ✅ **Simpler APIs** - No staging area, no detached HEAD
- ✅ **Git interop** - Can still push/pull from Git remotes

### For Development
- ✅ **Type-safe abstractions** - Rust traits catch errors at compile time
- ✅ **Testable** - Mock implementations for testing
- ✅ **Extensible** - Easy to add new VCS backends
- ✅ **Performance** - Can optimize per-backend

---

## Open Questions & Future Work

### Q1: Sparse Checkout Support?
- Git has sparse-checkout for large repos
- jj doesn't need it (no worktree materialization)
- **Decision**: Add as optional trait for Git, skip for jj

### Q2: Worktree Manager Migration?
- Current `worktree_manager.rs` is Git-specific
- With jj, worktrees become unnecessary for most operations
- **Decision**: Keep for Git backend, make optional

### Q3: Authentication?
- Git uses git-credential helpers or SSH keys
- jj uses same mechanisms (via git backend)
- **Decision**: Abstract credential provider trait

### Q4: Hooks Support?
- Git hooks for pre-commit, pre-push
- jj doesn't use git hooks
- **Decision**: VCS-agnostic hook system

### Future Enhancements
1. **Pijul support** - Another modern VCS with different merge semantics
2. **Performance monitoring** - Track VCS operation times
3. **AI-optimized operations** - Batch operations for AI agents
4. **Conflict resolution AI** - Use AI to suggest conflict resolutions

---

## References

- [Jujutsu Documentation](https://github.com/martinvonz/jj)
- [jj-lib Rust API](https://docs.rs/jj-lib/)
- [Git2-rs Documentation](https://docs.rs/git2/)
- [VCS Abstraction Patterns](https://matklad.github.io/2023/09/09/vcs-abstraction.html)

---

## Appendix: Code Structure

```
crates/vcs/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API exports
│   ├── traits.rs           # Core trait definitions
│   ├── types.rs            # Shared types (ChangeId, etc.)
│   ├── error.rs            # Error types
│   ├── backend/
│   │   ├── mod.rs
│   │   ├── git.rs          # Git implementation
│   │   └── jj.rs           # Jujutsu implementation
│   ├── factory.rs          # VcsFactory
│   ├── coordinator.rs      # Agent coordination
│   └── testing.rs          # Test utilities
└── tests/
    ├── integration.rs      # Integration tests
    ├── git_backend.rs      # Git-specific tests
    ├── jj_backend.rs       # Jj-specific tests
    └── parallel_agents.rs  # Multi-agent tests
```

---

## Timeline & Milestones

| Milestone | Duration | Deliverable |
|-----------|----------|-------------|
| Phase 1: Traits | 1 week | Core trait definitions & docs |
| Phase 2: Git Adapter | 2 weeks | Git implementation with tests |
| Phase 3: Jujutsu | 3 weeks | jj implementation with tests |
| Phase 4: Parallel Support | 2 weeks | Agent coordination primitives |
| Phase 5: Migration | 4 weeks | Production-ready with both backends |

**Total: ~12 weeks** to full implementation

---

## Conclusion

This VCS abstraction provides:
1. **Future-proof architecture** - Easy to add new VCS backends
2. **Type-safe APIs** - Compile-time correctness
3. **Minimal disruption** - Existing Git code continues to work
4. **Better scalability** - jj enables true parallel agent workflows
5. **Maintainable** - Clear separation of concerns

The design prioritizes **pragmatism over purity**, keeping Git as first-class citizen while opening door to jj's superior parallel workflow model.
