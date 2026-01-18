# Jujutsu (jj) Workflow Guide

## Understanding Changes vs. Branches

Jujutsu introduces a fundamentally different mental model from Git: **change-based development** instead of branch-based development.

### Git's Branch Model

In Git, you work on branches:
```bash
git checkout -b feature-branch
# Make edits
git add .
git commit -m "Add feature"
git push origin feature-branch
```

Each branch is a pointer to a commit, and you manage a staging area explicitly.

### jj's Change Model

In jj, you work directly with **changes** (revisions):
```bash
jj new
# Make edits (automatically tracked)
jj describe -m "Add feature"
jj git push --branch feature-branch
```

Key differences:
- **No staging area**: Your working copy IS a commit
- **Automatic tracking**: All edits are automatically part of the current change
- **Changes have IDs**: Each change has a unique ID independent of commit hashes
- **Branches are ephemeral**: Branches are just names you assign when pushing

## Core Workflow Concepts

### 1. Working Copy as a Commit

Your working directory is always a commit. When you edit files, you're directly editing a commit:

```bash
# Check current status
jj st

# Your changes are already part of the current revision
# No need to stage or add files
```

### 2. Creating New Changes

Start a new change from the current one:

```bash
# Create a new change on top of current
jj new

# Create a new change based on main
jj new main

# Create a new change with a description
jj new -m "Implement user authentication"
```

### 3. Describing Changes

Add or update the description of your current change:

```bash
# Set description
jj describe -m "Add login form validation"

# Edit description in editor
jj describe
```

### 4. Navigating Between Changes

Switch between different changes:

```bash
# List recent changes
jj log

# Check out a different change
jj edit <change-id>

# Go to a specific change
jj new <change-id>
```

### 5. Syncing with Git Remotes

Keep in sync with Git repositories:

```bash
# Fetch from remote
jj git fetch

# Push current change as a branch
jj git push --branch feature-name

# Push and create PR
jj git push --branch feature-name --remote origin
```

## Common Workflows

### Starting a New Feature

```bash
# Update from main
jj git fetch
jj rebase -d main

# Create new change for feature
jj new main -m "Add user profile page"

# Work on your feature
# ... edit files ...

# Check status
jj st

# Update description if needed
jj describe -m "Add user profile page with avatar upload"

# Push for PR
jj git push --branch add-profile-page
```

### Working on Multiple Features

With jj, you can easily switch between multiple features:

```bash
# Feature 1: Profile page
jj new main -m "Add profile page"
# ... work on feature 1 ...

# Feature 2: Dashboard
jj new main -m "Add dashboard"
# ... work on feature 2 ...

# View your changes
jj log

# Switch back to feature 1
jj edit <feature-1-change-id>
# ... continue work ...

# Switch to feature 2
jj edit <feature-2-change-id>
```

### Updating After Code Review

```bash
# Fetch latest changes
jj git fetch

# Edit the change that needs updates
jj edit <change-id>

# Make your edits
# ... edit files ...

# Amend the description if needed
jj describe

# Rebase if main has moved forward
jj rebase -d main

# Push updated version
jj git push --branch feature-name --force
```

### Squashing Changes

Combine multiple changes into one:

```bash
# Squash current change into parent
jj squash

# Squash specific change into its parent
jj squash -r <change-id>

# Interactive squash (choose what to squash)
jj squash -i
```

### Resolving Conflicts

jj makes conflict resolution more explicit:

```bash
# If rebase creates conflicts
jj rebase -d main

# Conflicts are materialized in working copy
# Edit files to resolve conflicts

# Mark as resolved (automatic)
jj st  # Shows conflicts resolved

# Continue with your work
jj describe -m "Merge with main"
```

## Advanced Workflows

### Parallel Development

Work on dependent changes:

```bash
# Base change
jj new main -m "Add API endpoint"
# ... implement endpoint ...

# Dependent change built on top
jj new -m "Add UI for new endpoint"
# ... implement UI ...

# View stack
jj log

# Push both changes as separate PRs
jj edit <api-change-id>
jj git push --branch api-endpoint

jj edit <ui-change-id>
jj git push --branch ui-for-endpoint
```

### Absorbing Fixes

Fix earlier changes in a stack:

```bash
# You have a stack of changes
jj log

# Make a fix that should go into an earlier change
# ... edit files ...

# Absorb the fix into the appropriate change
jj absorb --from <target-change-id>
```

### Splitting Changes

Split a large change into smaller ones:

```bash
# Create a new change from part of current
jj split

# Interactive mode to select what to split
# Mark files/hunks to move to new change
```

## Integration with Vibe Kanban

### Task-Based Changes

Each Vibe Kanban task can represent a jj change:

1. Create task in Vibe Kanban
2. jj creates a new change for that task
3. Work in the isolated change
4. Push change when ready for review
5. Task tracks the change ID

### Benefits with Multiple Agents

- **Isolated workspaces**: Each agent works in its own change
- **No branch conflicts**: Changes don't interfere with each other
- **Easy context switching**: Jump between agent tasks instantly
- **Parallel work**: Multiple agents can work simultaneously
- **Clear history**: Each change is atomic and traceable

### Commands in Vibe Kanban Context

```bash
# Start working on a task
jj edit <task-change-id>

# Create subtask
jj new -m "Subtask: Add validation"

# Push for review
jj git push --branch task-123-feature

# Sync with latest main
jj git fetch
jj rebase -d main
```

## Best Practices

### Descriptive Change Messages

Always write clear descriptions:
```bash
jj describe -m "Add user authentication

- Implement JWT token generation
- Add login endpoint
- Add middleware for protected routes"
```

### Keep Changes Focused

Each change should be a logical unit:
- One feature or fix per change
- Easy to review and test
- Can be reverted independently

### Sync Frequently

Stay up to date with the main branch:
```bash
# Daily sync routine
jj git fetch
jj rebase -d main
```

### Use Change IDs in Communication

When discussing with team or in PRs:
- Reference change IDs: `@abc123`
- Changes persist across rebases
- More stable than commit hashes

## Comparison Chart

| Task | Git | jj |
|------|-----|-----|
| Start work | `git checkout -b branch` | `jj new -m "description"` |
| See changes | `git status` | `jj st` |
| Save work | `git add . && git commit` | Just edit (automatic) |
| Update message | `git commit --amend` | `jj describe` |
| Switch work | `git checkout other` | `jj edit <change-id>` |
| Update from main | `git pull --rebase main` | `jj git fetch && jj rebase -d main` |
| Push for review | `git push origin branch` | `jj git push --branch name` |
| See history | `git log` | `jj log` |

## Quick Reference

```bash
# View state
jj st                          # Status of current change
jj log                         # History of changes
jj show                        # Details of current change

# Create & edit
jj new                         # New change on current
jj new <base>                  # New change on base
jj describe -m "msg"           # Set description
jj edit <change-id>            # Switch to change

# Sync with Git
jj git fetch                   # Fetch from remote
jj git push --branch <name>    # Push as branch
jj rebase -d <dest>            # Rebase onto destination

# Modify changes
jj squash                      # Squash into parent
jj split                       # Split current change
jj absorb                      # Fix earlier changes

# Undo operations
jj undo                        # Undo last operation
jj op log                      # Operation log
jj op restore <op-id>          # Restore to operation
```

## Learn More

- [Official jj Documentation](https://martinvonz.github.io/jj/)
- [jj Tutorial](https://martinvonz.github.io/jj/latest/tutorial/)
- [Git Comparison](https://martinvonz.github.io/jj/latest/git-comparison/)
- [Jujutsu Integration Guide](integrations/jujutsu.mdx)
