# VCS Abstraction Layer

A trait-based abstraction layer for version control systems, supporting both Git and Jujutsu (jj).

## Overview

This crate provides a unified interface for working with different version control systems. It abstracts common VCS operations into clean, composable traits that can be implemented by any VCS backend.

## Design Goals

- **Clean trait interface**: Operations grouped by concern (repository, changes, branches, remotes, diff, conflicts)
- **No worktree complexity**: Especially important for jj's worktree-free operations
- **Parallel agent support**: Multiple agents can work on different changes simultaneously
- **Minimal migration impact**: Existing Git code continues to work
- **Type safety**: Leverage Rust's type system to prevent misuse

## Features

- `git` (default): Git backend using git2-rs
- `jj`: Jujutsu backend (not yet implemented)

## Usage

### Basic Example

```rust
use vcs::{VcsFactory, VcsConfig, VcsBackendType, VcsChanges, VcsBranches};
use std::path::PathBuf;

// Create or open a repository
let config = VcsConfig {
    backend_type: VcsBackendType::Git,
    path: PathBuf::from("/path/to/repo"),
};

let vcs = VcsFactory::create(&config)?;

// Create a change
let change_id = vcs.create_change("My change")?;
println!("Created change: {}", change_id.as_str());

// List branches
let branches = vcs.list_branches()?;
for branch in branches {
    println!("Branch: {} (current: {})", branch.name, branch.is_current);
}
```

### Auto-detecting Repository Type

```rust
use vcs::{VcsFactory, VcsBranches};
use std::path::Path;

let path = Path::new("/path/to/repo");
let vcs = VcsFactory::auto_detect(path)?;

println!("Detected backend: {:?}", vcs.backend_type());
```

### Working with Changes

```rust
use vcs::{VcsChanges, CreateChangeOptions, ChangeFilter};

// Create a change with options
let options = CreateChangeOptions {
    stage_all: true,
    ..Default::default()
};
let change_id = vcs.create_change_with_options("My detailed change", options)?;

// List recent changes
let filter = ChangeFilter {
    limit: Some(10),
    ..Default::default()
};
let changes = vcs.list_changes(filter)?;
```

### Remote Operations

```rust
use vcs::{VcsRemotes, FetchOptions, PushOptions};

// Fetch from remote
vcs.fetch(FetchOptions {
    prune: true,
    ..Default::default()
})?;

// Push to remote
vcs.push(PushOptions {
    force: false,
    ..Default::default()
})?;
```

### Handling Conflicts

```rust
use vcs::VcsConflicts;

if vcs.has_conflicts()? {
    let conflicts = vcs.list_conflicts()?;
    for conflict in conflicts {
        println!("Conflict in: {}", conflict.path);
        println!("Operation: {:?}", conflict.operation);
    }
    
    // Abort the operation
    vcs.abort_operation()?;
}
```

## Architecture

### Trait Hierarchy

```
VcsRepository (base trait)
├── VcsChanges
├── VcsBranches
├── VcsRemotes
├── VcsDiff
└── VcsConflicts

VcsBackend = VcsRepository + VcsChanges + VcsBranches + VcsRemotes + VcsDiff + VcsConflicts
```

### Key Types

- **`ChangeId`**: Unique identifier for a change/commit
- **`HeadInfo`**: Information about the current HEAD
- **`BranchInfo`**: Information about a branch
- **`FileDiff`**: Diff information for a single file
- **`ConflictInfo`**: Information about a conflict

## Git Backend

The Git backend wraps `git2-rs` (libgit2) operations. It follows these principles:

1. **Wraps existing GitService logic**: Reuses proven Git operations
2. **Safe defaults**: Uses CLI for destructive operations when needed
3. **No bare repositories**: Requires working directory

### Git-specific Behavior

- **Changes = Commits**: Creating a change commits the working directory
- **Branches**: Standard Git branches
- **Worktrees**: Git worktrees are used for parallel work

## Jujutsu Backend (Planned)

The Jujutsu backend will leverage jj's unique features:

1. **No worktree needed**: Changes can be created without checkout
2. **True parallelism**: Multiple agents work on different changes simultaneously
3. **First-class conflicts**: Conflicts are tracked as part of the change
4. **Automatic rebasing**: Changes are automatically rebased on top of each other

### Jujutsu Advantages for AI Agents

- **Worktree-free operations**: No filesystem overhead for creating changes
- **Better conflict handling**: Conflicts don't block other operations
- **Simpler mental model**: No staging area, no detached HEAD
- **Parallel workflows**: Multiple agents can work without coordination

## Migration Guide

### From Direct Git Usage

**Before:**
```rust
let git_service = GitService::new();
let repo = git_service.open_repo(&path)?;
git_service.commit(&path, "My change")?;
let branches = git_service.get_all_branches(&repo)?;
```

**After:**
```rust
use vcs::{VcsFactory, VcsConfig, VcsBackendType, VcsChanges, VcsBranches};

let config = VcsConfig {
    backend_type: VcsBackendType::Git,
    path: path.to_path_buf(),
};
let vcs = VcsFactory::create(&config)?;
let change_id = vcs.create_change("My change")?;
let branches = vcs.list_branches()?;
```

### Gradual Migration Strategy

1. **New features**: Use VCS abstraction from day one
2. **Hot paths**: Migrate one module at a time
3. **Tests**: Dual-test both old and new APIs during transition
4. **Compatibility layer**: Keep `GitService` as implementation detail

## Testing

Run all tests:
```bash
cargo test -p vcs
```

Run tests with output:
```bash
cargo test -p vcs -- --nocapture
```

Test only Git backend:
```bash
cargo test -p vcs --features git
```

## Performance Considerations

### Git Performance
- CLI used for safety-critical operations
- libgit2 used for read-heavy graph queries
- Worktree operations have filesystem overhead

### Jujutsu Performance (Planned)
- Generally faster due to optimized Rust implementation
- No worktree checkout needed for most operations
- Better index structure
- Parallel operations scale better

## Thread Safety

**Important**: The Git backend (`GitRepository`) is `Send` but not `Sync` because `git2::Repository` is not `Sync`. If you need to share a repository across threads, wrap it in `Arc<Mutex<_>>`:

```rust
use std::sync::{Arc, Mutex};

let vcs = VcsFactory::create(&config)?;
let shared_vcs = Arc::new(Mutex::new(vcs));

// Use in multiple threads
let vcs_clone = shared_vcs.clone();
std::thread::spawn(move || {
    let vcs = vcs_clone.lock().unwrap();
    // Use vcs...
});
```

## Error Handling

All operations return `Result<T, VcsError>`. Common error types:

- `RepositoryNotFound`: Repository doesn't exist
- `BranchNotFound`: Branch doesn't exist
- `Conflicts`: Conflicted files
- `DirtyWorkingCopy`: Uncommitted changes
- `OperationInProgress`: Ongoing merge/rebase/etc
- `AuthenticationFailed`: Authentication failed
- `PushRejected`: Push was rejected
- `Backend`: Backend-specific error

## Contributing

When adding a new VCS backend:

1. Implement all traits in `traits.rs`
2. Add feature flag to `Cargo.toml`
3. Update `VcsFactory::create()` to handle new backend
4. Add comprehensive tests
5. Update documentation

## Future Work

- [ ] Complete Jujutsu backend implementation
- [ ] Add async support for network operations
- [ ] Performance benchmarks
- [ ] Sparse checkout abstraction
- [ ] Credential provider abstraction
- [ ] Hook system abstraction
- [ ] Pijul backend support

## License

Same as parent project.

## References

- [Git Documentation](https://git-scm.com/doc)
- [git2-rs Documentation](https://docs.rs/git2/)
- [Jujutsu Documentation](https://github.com/martinvonz/jj)
- [jj-lib Documentation](https://docs.rs/jj-lib/)
