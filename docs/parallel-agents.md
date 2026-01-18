# Parallel Agent Development Guide

## Overview

Vibe Kanban with Jujutsu (jj) enables true parallel development where multiple AI coding agents can work simultaneously on different tasks without conflicts or coordination overhead.

## Why Parallel Agents Work Better with jj

### Traditional Git Challenges

With Git, parallel agent work faces issues:
- **Branch conflicts**: Agents creating branches with same names
- **Merge conflicts**: Multiple agents modifying shared files
- **Coordination overhead**: Need to manage who works on what
- **Stale branches**: Agents working on outdated code

### jj Advantages

With jj, parallel work is natural:
- **Change isolation**: Each agent works in isolated change
- **No branch management**: Changes exist independently
- **Automatic conflict detection**: Conflicts discovered early
- **Easy rebase**: Changes can be rebased independently
- **Stable identifiers**: Change IDs persist across operations

## Setting Up Parallel Agents

### Prerequisites

1. Install Jujutsu:
```bash
brew install jj
# or
cargo install --locked jj-cli
```

2. Initialize jj in your repository:
```bash
jj init --git
jj git fetch
```

3. Configure Vibe Kanban to use jj (automatic detection)

### Architecture

```
Main Branch (main)
    ├─ Agent 1: Feature A (change @abc123)
    ├─ Agent 2: Feature B (change @def456)
    ├─ Agent 3: Bug Fix  (change @ghi789)
    └─ Agent 4: Refactor (change @jkl012)
```

Each agent works independently in its own change, all based on main.

## Workflows

### 1. Basic Parallel Workflow

#### Create Tasks for Multiple Agents

In Vibe Kanban:
1. Create multiple tasks
2. Assign different agents to each task
3. Start all tasks simultaneously

Behind the scenes:
```bash
# Agent 1 starts
jj new main -m "Feature A: Add user dashboard"
# ... work in change @abc123 ...

# Agent 2 starts (parallel)
jj new main -m "Feature B: Add notifications"
# ... work in change @def456 ...

# Agent 3 starts (parallel)
jj new main -m "Fix login redirect bug"
# ... work in change @ghi789 ...
```

#### Check Status

View all parallel work:
```bash
jj log -r 'main..@'

# Shows:
# @  ghi789 Agent 3 Fix login redirect bug
# │ ○  def456 Agent 2 Feature B: Add notifications
# ├─╯
# │ ○  abc123 Agent 1 Feature A: Add user dashboard
# ├─╯
# ○  main
```

#### Push for Review

Each agent pushes independently:
```bash
# Agent 1
jj edit @abc123
jj git push --branch feature-a

# Agent 2
jj edit @def456
jj git push --branch feature-b

# Agent 3
jj edit @ghi789
jj git push --branch fix-login
```

### 2. Dependent Tasks Workflow

When tasks build on each other:

```bash
# Agent 1: Base API
jj new main -m "Add REST API endpoints"
# ... implement API in change @api123 ...

# Agent 2: UI using the API (builds on Agent 1)
jj new @api123 -m "Add UI for new API"
# ... implement UI in change @ui456 ...

# View dependency
jj log
# Shows:
# @  ui456 Agent 2 Add UI for new API
# ○  api123 Agent 1 Add REST API endpoints
# ○  main
```

Push both as separate PRs:
```bash
# Push base change
jj edit @api123
jj git push --branch api-endpoints

# Push dependent change
jj edit @ui456
jj git push --branch api-ui
```

### 3. Handling Conflicts

When agents modify the same files:

```bash
# Agent 1 pushes first
jj edit @abc123
jj git push --branch feature-a
# Merged to main

# Agent 2 rebases onto updated main
jj edit @def456
jj git fetch
jj rebase -d main

# If conflicts occur:
# - jj materializes conflicts in working copy
# - Agent resolves conflicts
# - Continue work automatically

jj describe -m "Feature B: Add notifications (resolved conflicts)"
jj git push --branch feature-b
```

### 4. Coordinated Landing

Landing multiple related changes in sequence:

```bash
# Three agents complete related features
jj log
# @  zzz789 Agent 3 Add settings UI
# ○  yyy456 Agent 2 Add settings API
# ○  xxx123 Agent 1 Add settings model
# ○  main

# Land in order: model → API → UI
jj edit @xxx123
jj git push --branch settings-model
# PR merged

jj edit @yyy456
jj rebase -d main  # Rebase onto merged model
jj git push --branch settings-api
# PR merged

jj edit @zzz789
jj rebase -d main  # Rebase onto merged API
jj git push --branch settings-ui
# PR merged
```

## Advanced Patterns

### Fan-Out Pattern

One agent creates base, multiple agents build on it:

```bash
# Agent 1: Core infrastructure
jj new main -m "Add plugin system"
# ... implement base ...
# Change @base000

# Multiple agents build on base (parallel)
jj new @base000 -m "Add auth plugin"    # Agent 2 → @auth111
jj new @base000 -m "Add logging plugin" # Agent 3 → @log222
jj new @base000 -m "Add cache plugin"   # Agent 4 → @cache333

# Topology:
#      auth111
#      │
#      ├─ log222
#      │
# base000
#      │
#      └─ cache333
```

### Review Cycles

Updating changes based on review:

```bash
# Agent receives review feedback
jj edit <change-id>

# Make requested changes
# Files automatically tracked

# Amend description with review notes
jj describe -m "Feature: Add dashboard

Addressed review feedback:
- Fixed error handling
- Added tests
- Updated documentation"

# Push update
jj git push --branch feature-dashboard --force
```

### Cross-Agent Dependencies

When Agent B needs Agent A's work:

```bash
# Agent A working on @aaa111
jj describe -m "Add API client"

# Agent B needs it
jj new @aaa111 -m "Add API tests"
# Work in @bbb222 depends on @aaa111

# Agent A makes changes
jj edit @aaa111
# ... update API ...

# Agent B rebases to get updates
jj edit @bbb222
jj rebase -d @aaa111
# Gets latest from Agent A
```

## Best Practices

### 1. Clear Task Boundaries

Define tasks with minimal overlap:
- Separate features into independent units
- Minimize shared file modifications
- Use clear interfaces between components

### 2. Frequent Syncing

Keep changes up to date:
```bash
# Daily sync routine for each agent
jj git fetch
jj rebase -d main
```

### 3. Descriptive Changes

Use clear, detailed descriptions:
```bash
jj describe -m "Feature: User authentication

Agent: Claude
Task: #123
Files: auth.rs, auth_test.rs, routes.rs
Status: Ready for review"
```

### 4. Communication

Track dependencies:
- Note which changes depend on others
- Update descriptions with dependencies
- Use change IDs in agent communication

### 5. Testing Before Landing

Ensure changes work together:
```bash
# Test dependent changes together
jj edit @feature2
jj rebase -d @feature1
# Run tests
# Both features work together
```

## Troubleshooting

### Issue: Agents Creating Conflicting Changes

**Problem**: Multiple agents modify the same code

**Solution**:
```bash
# Identify conflict
jj log

# Rebase one change onto the other
jj edit <later-change>
jj rebase -d <earlier-change>

# Resolve conflicts
# jj shows conflicts in working copy
# Edit files to resolve

# Continue
jj st  # Verify conflicts resolved
```

### Issue: Lost Changes

**Problem**: Can't find an agent's work

**Solution**:
```bash
# View all recent changes
jj log -r 'all()'

# Search by description
jj log -r 'description(Agent-Name)'

# Restore if needed
jj new <change-id>
```

### Issue: Complex Rebase

**Problem**: Rebase creates many conflicts

**Solution**:
```bash
# Abort rebase
jj undo

# Try rebasing incrementally
jj rebase -d <intermediate-commit>
# Resolve conflicts
jj rebase -d main
# Resolve remaining conflicts
```

## Monitoring Multiple Agents

### Dashboard View

In Vibe Kanban:
- View all agent tasks
- See change IDs for each task
- Track status and dependencies
- Monitor for conflicts

### Command Line Monitoring

```bash
# View all active changes
jj log -r 'main..'

# View changes by agent (if using descriptions)
jj log -r 'description(Agent-1)'

# See what's ready to push
jj log -r 'main.. & description(ready)'

# Check for conflicts
jj log -r 'main.. & conflict()'
```

## Example: Four Agents Working in Parallel

```bash
# Setup
jj git fetch

# Agent 1: Frontend feature
jj new main -m "[Agent-1] Add profile page"
# @profile111

# Agent 2: Backend API
jj new main -m "[Agent-2] Add user API"
# @userapi222

# Agent 3: Database migration
jj new main -m "[Agent-3] Add user fields"
# @dbmig333

# Agent 4: Tests
jj new main -m "[Agent-4] Add integration tests"
# @tests444

# View parallel work
jj log
# Shows all four changes based on main

# Agent 3 finishes first
jj edit @dbmig333
jj git push --branch db-migration
# Merged to main

# Other agents rebase
jj edit @profile111
jj rebase -d main

jj edit @userapi222
jj rebase -d main

jj edit @tests444
jj rebase -d main

# All continue working in sync
```

## Performance Considerations

### Scaling to Many Agents

- **5-10 agents**: Works seamlessly with standard workflow
- **10-20 agents**: Consider organizing into teams/areas
- **20+ agents**: Use topic branches as intermediate bases

### Resource Management

- Each agent's change is lightweight
- jj operations are fast even with many changes
- Disk space is minimal (shared Git objects)

## Learn More

- [jj Workflow Guide](jj-workflow.md)
- [Migration from Git](migration-from-git.md)
- [Jujutsu Documentation](https://martinvonz.github.io/jj/)
