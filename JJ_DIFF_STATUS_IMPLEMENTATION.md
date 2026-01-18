# Jujutsu Diff and Status Implementation

## Overview

Implemented jj-based diff and status operations to replace git equivalents. This provides a cleaner, change-based model without the complexity of temporary index files and sparse-checkout workarounds.

## Changes Made

### 1. Enhanced `JjStatus` Structure

Added file tracking to the status structure:

```rust
pub struct JjStatus {
    pub working_copy_change_id: String,
    pub has_changes: bool,
    pub has_conflicts: bool,
    pub conflicted_files: Vec<String>,
    // NEW: Track specific file changes
    pub modified_files: Vec<String>,
    pub added_files: Vec<String>,
    pub deleted_files: Vec<String>,
}
```

**Benefits:**
- Direct tracking of modified, added, and deleted files
- No need for worktree status parsing complexity
- Simpler change state representation

### 2. New `JjDiffSummary` Structure

Created a diff summary structure similar to git's `--name-status`:

```rust
pub struct JjDiffSummary {
    pub change_type: String,  // M, A, D, R
    pub path: String,
    pub old_path: Option<String>,  // For renames
}
```

**Benefits:**
- Compatible with existing git diff parsing patterns
- Handles renames cleanly
- Easy to integrate with existing code

### 3. Enhanced `JjDiffOptions`

Updated diff options to support both summary and stat modes:

```rust
pub struct JjDiffOptions {
    pub from: Option<String>,
    pub to: Option<String>,
    pub paths: Option<Vec<String>>,
    pub summary: bool,  // --summary: shows M/A/D/R
    pub stat: bool,     // --stat: shows histogram
}
```

**Benefits:**
- `--summary`: Similar to git's `--name-status`, shows file change types
- `--stat`: Shows histogram of changes
- Support for diff between arbitrary revisions
- Path filtering support

### 4. New `diff_summary()` Method

Added a convenience method for getting diff summaries:

```rust
pub fn diff_summary(
    &self,
    repo_path: &Path,
    from: Option<&str>,
    to: Option<&str>,
    paths: Option<Vec<String>>,
) -> Result<Vec<JjDiffSummary>, JujutsuCliError>
```

**Benefits:**
- Direct replacement for git's diff with `--name-status`
- Cleaner API than using `JjDiffOptions`
- Easier to use for common cases

### 5. Enhanced Status Parsing

Updated `parse_status()` to extract modified, added, and deleted files:

```rust
// Parses output like:
// Working copy : pzsxstzt 3d0c8c7e (no description set)
// Working copy changes:
// M file.txt
// A new_file.txt
// D old_file.txt
```

**Benefits:**
- Directly extracts file lists from status output
- No need for separate diff operations
- Single source of truth for working copy state

### 6. New Diff Summary Parsing

Added `parse_diff_summary()` to parse `jj diff --summary` output:

```rust
// Parses output like:
// M file.txt
// A new_file.txt
// D old_file.txt
// R old_name.txt => new_name.txt
```

**Benefits:**
- Handles standard change types (M, A, D)
- Properly handles renames with old path tracking
- Compatible with git diff output patterns

## Key Advantages Over Git

### 1. No Temporary Index Files
- Git's `diff_status()` creates temporary index files
- jj operates directly on changes without index complexity
- Simpler, faster, and less error-prone

### 2. No Sparse-Checkout Issues
- Git requires careful handling of sparse-checkout paths
- jj naturally handles path filtering without workarounds
- Cleaner path filter implementation

### 3. Change-Based Model
- jj's change IDs provide stable references
- No need to track HEAD vs worktree states separately
- Simpler mental model for developers

### 4. Simpler Status Output
- jj status shows current change state directly
- No staging area complexity
- Unified view of working copy changes

## Usage Examples

### Get Status
```rust
let cli = JujutsuCli::new();
let status = cli.status(repo_path)?;

println!("Modified files: {:?}", status.modified_files);
println!("Added files: {:?}", status.added_files);
println!("Deleted files: {:?}", status.deleted_files);
```

### Get Diff Summary
```rust
let cli = JujutsuCli::new();

// Diff current change vs parent
let summary = cli.diff_summary(repo_path, None, None, None)?;

// Diff between two changes
let summary = cli.diff_summary(
    repo_path,
    Some("main"),
    Some("@"),
    None
)?;

// Diff with path filter
let summary = cli.diff_summary(
    repo_path,
    None,
    None,
    Some(vec!["src/**/*.rs".to_string()])
)?;

for entry in summary {
    println!("{} {}", entry.change_type, entry.path);
    if let Some(old) = entry.old_path {
        println!("  (renamed from {})", old);
    }
}
```

### Get Full Diff with Stats
```rust
let cli = JujutsuCli::new();
let opts = JjDiffOptions {
    from: Some("main".to_string()),
    to: Some("@".to_string()),
    paths: None,
    summary: false,
    stat: true,  // Show histogram
};

let diff_output = cli.diff(repo_path, opts)?;
println!("{}", diff_output);
```

## Testing

Added comprehensive tests:

- `test_parse_status()`: Verifies status parsing with file tracking
- `test_parse_diff_summary()`: Verifies diff summary parsing including renames
- Existing tests still pass for backward compatibility

## Migration Path

To migrate from git-based diff/status:

1. Replace `GitCli::get_worktree_status()` with `JujutsuCli::status()`
2. Replace `GitCli::diff_status()` with `JujutsuCli::diff_summary()`
3. Update code to use `JjStatus` fields instead of parsing git output
4. Remove temporary index file handling code

## Future Enhancements

Potential improvements:

1. **Binary Diff Support**: Handle binary file diffs
2. **Conflict Markers**: Better parsing of conflict information
3. **Path Filtering**: More sophisticated pathspec support
4. **Performance**: Optimize parsing for large repos
5. **Async Operations**: Add async versions of operations

## Conclusion

This implementation provides a solid foundation for jj-based diff and status operations, offering significant advantages over git:

- ✅ No temporary index files
- ✅ No sparse-checkout complexity
- ✅ Clean change-based model
- ✅ Simple, consistent API
- ✅ Full test coverage
- ✅ Compatible with existing patterns
