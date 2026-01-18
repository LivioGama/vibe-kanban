# Migrating from Git to Jujutsu (jj)

## Introduction

This guide helps you transition from Git to Jujutsu (jj) for use with Vibe Kanban. jj is Git-compatible and can coexist with Git, making migration gradual and risk-free.

## Should You Migrate?

### Consider jj if you:
- Work with multiple coding agents in parallel
- Want simpler, more intuitive version control commands
- Need better conflict resolution workflows
- Desire non-destructive operations (nothing is ever lost)
- Want automatic tracking without staging

### Stay with Git if you:
- Have established Git workflows that work well
- Team is not ready to adopt new tools
- Repository has specific Git-only tooling

**Note**: You can use both! jj and Git can coexist in the same repository.

## Migration Approaches

### Approach 1: Gradual Adoption (Recommended)

Use both Git and jj side-by-side while learning:

1. **Install jj** without removing Git
2. **Initialize jj** in existing Git repos
3. **Use jj for new work** while keeping Git for established workflows
4. **Gradually transition** as you become comfortable

### Approach 2: Full Migration

Switch entirely to jj:

1. **Install jj** and learn core commands
2. **Initialize all repositories** with jj
3. **Use jj exclusively** for new work
4. **Keep Git as fallback** for edge cases

### Approach 3: Coexistence

Use the best tool for each task:

- **jj**: Day-to-day development, local changes, rebasing
- **Git**: CI/CD, hooks, legacy scripts, team collaboration

## Installation

### macOS
```bash
brew install jj
```

### Linux
```bash
brew install jj
# or
pacman -S jj  # Arch Linux
```

### All Platforms
```bash
cargo install --locked jj-cli
```

Verify installation:
```bash
jj --version
```

## Initial Configuration

Create `~/.config/jj/config.toml`:

```toml
[user]
name = "Your Name"
email = "your.email@example.com"

[ui]
diff-editor = "diff"
relative-timestamps = true
color = "auto"

[git]
push-branch-prefix = ""
auto-local-branch = true

[aliases]
st = ["status"]
l = ["log"]
```

## Repository Setup

### Initialize jj in Existing Git Repository

```bash
cd your-git-repo
jj init --git
```

This creates a `.jj` directory alongside `.git`. Both version control systems now work together.

### Verify Setup

```bash
# Check Git status
git status

# Check jj status
jj st

# View Git log
git log

# View jj log
jj log

# They show the same history!
```

## Command Translation

### Common Operations

| Task | Git Command | jj Command |
|------|-------------|------------|
| **Status** | `git status` | `jj st` |
| **History** | `git log` | `jj log` |
| **Create branch** | `git checkout -b feature` | `jj new -m "feature"` |
| **Switch branch** | `git checkout feature` | `jj edit <change-id>` |
| **Stage changes** | `git add file.txt` | *(automatic)* |
| **Commit** | `git commit -m "msg"` | `jj describe -m "msg"` |
| **Amend commit** | `git commit --amend` | `jj describe` |
| **Rebase** | `git rebase main` | `jj rebase -d main` |
| **Fetch** | `git fetch` | `jj git fetch` |
| **Pull** | `git pull --rebase` | `jj git fetch && jj rebase -d main` |
| **Push** | `git push origin branch` | `jj git push --branch name` |
| **Show diff** | `git diff` | `jj diff` |
| **View commit** | `git show <commit>` | `jj show <change>` |
| **Undo last** | `git reset --soft HEAD^` | `jj undo` |
| **Stash** | `git stash` | *(not needed)* |
| **Cherry-pick** | `git cherry-pick <hash>` | `jj rebase -r <change> -d <dest>` |
| **Interactive rebase** | `git rebase -i` | `jj squash / jj split` |

### No Direct Equivalent

Some Git concepts don't exist in jj:

- **Staging area**: Everything in working copy is automatically tracked
- **Stashing**: Just edit different changes directly
- **Detached HEAD**: jj always has a working change
- **Branch checkout**: You edit changes, not branches

## Workflow Migration

### Git Workflow: Feature Branch

```bash
# Git
git checkout main
git pull
git checkout -b feature-x
# ... make changes ...
git add .
git commit -m "Add feature X"
git push origin feature-x
```

### jj Equivalent: Change-Based

```bash
# jj
jj git fetch
jj new main -m "Add feature X"
# ... make changes (automatically tracked) ...
jj describe -m "Add feature X"
jj git push --branch feature-x
```

### Git Workflow: Multiple Features

```bash
# Git
git checkout main
git checkout -b feature-a
# ... work on A ...
git add . && git commit -m "Feature A"

git checkout main
git checkout -b feature-b
# ... work on B ...
git add . && git commit -m "Feature B"

git checkout feature-a
# ... continue A ...
```

### jj Equivalent: Parallel Changes

```bash
# jj
jj new main -m "Feature A"
# ... work on A ...

jj new main -m "Feature B"
# ... work on B ...

jj edit <feature-a-change-id>
# ... continue A ...
```

## Key Concept Changes

### 1. No Staging Area

**Git**: Explicit staging with `git add`
```bash
git add file.txt
git commit -m "Update file"
```

**jj**: Automatic tracking
```bash
# Just edit files
jj describe -m "Update file"
```

### 2. Working Copy is a Commit

**Git**: Working directory is separate from commits
```bash
git status  # Shows "unstaged changes"
```

**jj**: Working copy IS the current commit
```bash
jj st  # Shows changes in current revision
```

### 3. Changes Have Identities

**Git**: Commits identified by SHA hashes (change with rebases)
```bash
git show abc123def456...
```

**jj**: Changes have stable IDs
```bash
jj show @abc123  # ID persists across rebases
```

### 4. Branches Are Just Labels

**Git**: Branches are first-class citizens
```bash
git checkout feature-branch
```

**jj**: Branches are names applied when pushing
```bash
jj git push --branch feature-branch
# Branch created on demand
```

## Handling Common Scenarios

### Scenario 1: In Progress Work

**Git**: Need to stash before switching branches
```bash
git stash
git checkout other-branch
# ... work ...
git checkout original-branch
git stash pop
```

**jj**: Just switch between changes
```bash
jj edit <other-change-id>
# ... work ...
jj edit <original-change-id>
# Your changes are still there
```

### Scenario 2: Fixing Earlier Commit

**Git**: Interactive rebase
```bash
git rebase -i HEAD~3
# Mark commit as 'edit'
# Make changes
git add .
git commit --amend
git rebase --continue
```

**jj**: Direct editing
```bash
jj edit <earlier-change-id>
# Make changes (automatic)
jj describe -m "Updated description"
```

### Scenario 3: Conflicts During Rebase

**Git**: Step-by-step resolution
```bash
git rebase main
# CONFLICT
# Fix conflicts
git add .
git rebase --continue
```

**jj**: Materialized conflicts
```bash
jj rebase -d main
# Conflicts shown in working copy as special markers
# Fix conflicts
jj st  # Automatically resolved
```

### Scenario 4: Undo Mistakes

**Git**: Various reset options
```bash
git reset --soft HEAD^    # Undo commit, keep changes
git reset --hard HEAD^    # Discard everything
```

**jj**: Simple undo
```bash
jj undo  # Undo last operation
jj op log  # See all operations
jj op restore <op-id>  # Restore to any point
```

## Team Collaboration

### If Team Uses Git

You can use jj locally while team uses Git:

```bash
# You (using jj)
jj new main -m "Feature"
# ... work ...
jj git push --branch feature

# Team (using Git)
git checkout -b feature
git pull origin feature
# ... review ...
git push
```

Your jj changes appear as normal Git commits to the team.

### Converting Team to jj

Gradual team migration:

1. **Phase 1**: Individual experimentation
   - Each developer installs jj
   - Use jj locally, Git for pushing
   
2. **Phase 2**: jj for new features
   - New work uses jj workflows
   - Legacy branches stay in Git
   
3. **Phase 3**: Full adoption
   - Team uses jj primarily
   - Git used only for CI/CD integration

## Troubleshooting Migration

### Issue: Git and jj Out of Sync

**Problem**: jj doesn't see Git commits

**Solution**:
```bash
jj git import
```

### Issue: jj Changes Not in Git

**Problem**: Git doesn't see jj changes

**Solution**:
```bash
jj git export
```

### Issue: Forgot to Import/Export

**Problem**: Changes missing after switching tools

**Solution**:
```bash
# Full sync
jj git import
jj git export
```

### Issue: Existing Git Workflows Break

**Problem**: Scripts/tools rely on Git

**Solution**:
- Keep using Git for those specific workflows
- Use jj for daily development
- Export before running Git-dependent tools

### Issue: Branch Names Confusing

**Problem**: jj doesn't use branches the same way

**Solution**:
- Think "changes" not "branches"
- Use descriptive change messages
- Create Git branches only when pushing

## Learning Resources

### Gradual Learning Path

1. **Week 1**: Basic commands
   - `jj st`, `jj log`, `jj new`, `jj describe`
   
2. **Week 2**: Navigation
   - `jj edit`, `jj rebase`, `jj squash`
   
3. **Week 3**: Git integration
   - `jj git fetch`, `jj git push`, `jj git import/export`
   
4. **Week 4**: Advanced features
   - `jj split`, `jj absorb`, `jj op log`

### Practice Repository

Create a test repo to practice:

```bash
mkdir jj-practice
cd jj-practice
git init
jj init --git
echo "test" > file.txt
jj describe -m "Initial commit"
jj git push --branch main

# Practice workflows without risk
```

### Comparison Table for Learning

| Git Habit | jj Equivalent | Why Different |
|-----------|---------------|---------------|
| Check out branch | Edit change | Changes, not branches |
| Stage files | Just edit | Automatic tracking |
| Commit | Describe | Working copy is commit |
| Stash | Edit other change | Can work on multiple changes |
| Rebase interactive | Squash/split | Direct change manipulation |
| Cherry-pick | Rebase change | Change-based, not commit-based |

## Using with Vibe Kanban

### Setup

1. Install jj: `brew install jj`
2. Initialize in repo: `jj init --git`
3. Start Vibe Kanban: `npx vibe-kanban`

### Workflow

1. Create task in Vibe Kanban
2. jj automatically creates change for task
3. Work in isolated change
4. Push when ready: `jj git push --branch task-name`
5. Track progress in Vibe Kanban

### Benefits

- Each agent task has isolated change
- No branch name conflicts
- Easy parallel work
- Clear change history
- Simplified code review

## When to Use Git vs jj

### Use Git for:
- CI/CD pipelines (for now)
- Legacy scripts and tools
- Git hooks (until jj supports them)
- Submodules (jj support coming)
- LFS (large file storage)

### Use jj for:
- Daily development work
- Local branch management
- Rebasing and history editing
- Parallel feature development
- Working with multiple agents
- Conflict resolution

## Summary Checklist

- [ ] Install jj
- [ ] Configure `~/.config/jj/config.toml`
- [ ] Initialize jj in repositories: `jj init --git`
- [ ] Learn basic commands: `st`, `log`, `new`, `describe`
- [ ] Practice workflows in test repository
- [ ] Use jj for new features
- [ ] Keep Git as fallback
- [ ] Gradually expand jj usage
- [ ] Share experience with team

## Next Steps

1. Read [jj Workflow Guide](jj-workflow.md)
2. Review [Parallel Agents Guide](parallel-agents.md)
3. Check [Jujutsu Integration](integrations/jujutsu.mdx)
4. Join [Vibe Kanban Discord](https://discord.gg/AC4nwVtJM3) for help

## Additional Resources

- [Official jj Tutorial](https://martinvonz.github.io/jj/latest/tutorial/)
- [Git Comparison](https://martinvonz.github.io/jj/latest/git-comparison/)
- [jj Documentation](https://martinvonz.github.io/jj/)
- [GitHub Integration](https://docs.jj-vcs.dev/latest/github/)
