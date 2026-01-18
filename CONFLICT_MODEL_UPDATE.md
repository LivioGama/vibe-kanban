# Conflict Resolution Update: jj's 3-Way Merge Model

## Overview
Updated conflict handling to align with jj (Jujutsu's) simpler, first-class conflict model.

## Key Changes

### 1. Simplified Conflict Types (`crates/vcs/src/types.rs`)
**Before:**
- `ConflictOperation` enum with 4 variants (Merge, Rebase, CherryPick, Revert)
- ConflictInfo tracked operation type

**After:**
- Removed `ConflictOperation` enum entirely
- ConflictInfo now only tracks path and 3-way merge sides (base, ours, theirs)
- Conflicts are first-class - operation type is less important

### 2. Simplified VCS Trait (`crates/vcs/src/traits.rs`)
**Before:**
```rust
fn ongoing_operation(&self) -> Result<Option<ConflictOperation>, VcsError>;
```

**After:**
- Removed `ongoing_operation()` method
- Focus on conflict detection and resolution, not state tracking

### 3. Removed Git Service Complexity (`crates/services/src/services/git.rs`)
**Before:**
- `ConflictOp` enum (duplicating VCS layer)
- `detect_conflict_op()` method checking multiple state files
- Complex state detection logic

**After:**
- Removed entire `ConflictOp` enum
- Removed `detect_conflict_op()` method (33 lines deleted)
- Simpler conflict handling

### 4. Updated Server Routes (`crates/server/src/routes/task_attempts.rs`)
**Before:**
```rust
pub conflict_op: Option<ConflictOp>,
GitOperationError::MergeConflicts { message, op: ConflictOp::Rebase }
```

**After:**
```rust
// conflict_op field removed from BranchStatus
GitOperationError::MergeConflicts { message }
```

### 5. Updated Git Backend (`crates/vcs/src/backend/git.rs`)
**Before:**
- `list_conflicts()` detected operation type
- `ongoing_operation()` mapped git2 states to ConflictOperation

**After:**
- `list_conflicts()` just returns conflict info with 3-way sides
- `ongoing_operation()` method removed
- Simpler `abort_operation()` implementation

## Benefits

1. **Cleaner UX for Agents**: Conflicts are first-class, not tied to operation state
2. **Less Complexity**: Removed 104 lines, added 40 lines (net -64 lines)
3. **jj Alignment**: Matches jj's model where conflicts can be committed
4. **Simpler Logic**: No need to track operation state across files

## jj's Conflict Model

In jj:
- Conflicts are first-class and can be committed
- `jj resolve` for interactive resolution
- No separate "merge state" vs "rebase state"
- Conflicts tracked in 3-way format (base, left, right)

## Migration Notes

- Frontend needs update: remove `conflict_op` from BranchStatus display
- `ConflictOp` type removed from TypeScript (regenerate types)
- Conflict banners should show "Conflicts present" vs operation-specific messages

## Files Changed

- `crates/vcs/src/types.rs`: Simplified ConflictInfo, removed ConflictOperation
- `crates/vcs/src/traits.rs`: Removed ongoing_operation() from VcsConflicts trait
- `crates/vcs/src/backend/git.rs`: Simplified conflict detection
- `crates/services/src/services/git.rs`: Removed ConflictOp and detect_conflict_op()
- `crates/server/src/routes/task_attempts.rs`: Updated to use simplified model
- `crates/server/src/bin/generate_types.rs`: Removed ConflictOp type export
