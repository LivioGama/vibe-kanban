# Repository Guidelines

## Project Structure & Module Organization
- `crates/`: Rust workspace crates — `server` (API + bins), `db` (SQLx models/migrations), `executors`, `services`, `utils`, `deployment`, `local-deployment`, `remote`.
- `frontend/`: React + TypeScript app (Vite, Tailwind). Source in `frontend/src`.
- `frontend/src/components/dialogs`: Dialog components for the frontend.
- `remote-frontend/`: Remote deployment frontend.
- `shared/`: Generated TypeScript types (`shared/types.ts`). Do not edit directly.
- `assets/`, `dev_assets_seed/`, `dev_assets/`: Packaged and local dev assets.
- `npx-cli/`: Files published to the npm CLI package.
- `scripts/`: Dev helpers (ports, DB preparation).
- `docs/`: Documentation files.

## Managing Shared Types Between Rust and TypeScript

ts-rs allows you to derive TypeScript types from Rust structs/enums. By annotating your Rust types with #[derive(TS)] and related macros, ts-rs will generate .ts declaration files for those types.
When making changes to the types, you can regenerate them using `pnpm run generate-types`
Do not manually edit shared/types.ts, instead edit crates/server/src/bin/generate_types.rs

## Build, Test, and Development Commands
- Install: `pnpm i`
- Run dev (frontend + backend with ports auto-assigned): `pnpm run dev`
- Backend (watch): `pnpm run backend:dev:watch`
- Frontend (dev): `pnpm run frontend:dev`
- Type checks: `pnpm run check` (frontend) and `pnpm run backend:check` (Rust cargo check)
- Rust tests: `cargo test --workspace`
- Generate TS types from Rust: `pnpm run generate-types` (or `generate-types:check` in CI)
- Prepare SQLx (offline): `pnpm run prepare-db`
- Prepare SQLx (remote package, postgres): `pnpm run remote:prepare-db`
- Local NPX build: `pnpm run build:npx` then `pnpm pack` in `npx-cli/`

## Automated QA
- When testing changes by runnign the application, you should prefer `pnpm run dev:qa` over `pnpm run dev`, which starts the application in a dedicated mode that is optimised for QA testing

## Coding Style & Naming Conventions
- Rust: `rustfmt` enforced (`rustfmt.toml`); group imports by crate; snake_case modules, PascalCase types.
- TypeScript/React: ESLint + Prettier (2 spaces, single quotes, 80 cols). PascalCase components, camelCase vars/functions, kebab-case file names where practical.
- Keep functions small, add `Debug`/`Serialize`/`Deserialize` where useful.

## Testing Guidelines
- Rust: prefer unit tests alongside code (`#[cfg(test)]`), run `cargo test --workspace`. Add tests for new logic and edge cases.
- Frontend: ensure `pnpm run check` and `pnpm run lint` pass. If adding runtime logic, include lightweight tests (e.g., Vitest) in the same directory.

## Security & Config Tips
- Use `.env` for local overrides; never commit secrets. Key envs: `FRONTEND_PORT`, `BACKEND_PORT`, `HOST` 
- Dev ports and assets are managed by `scripts/setup-dev-environment.js`.

## Jujutsu (jj) Version Control

### Change-Based Model

Vibe Kanban supports Jujutsu (jj), a next-generation version control system that uses a **change-based model** instead of Git's branch-based model. Key differences:

- **Working copy is a commit**: Your working directory is always a commit; edits are automatically tracked
- **No staging area**: No need for `git add`; changes are automatically part of the current revision
- **Stable change IDs**: Changes have persistent IDs that survive rebases
- **Branches are ephemeral**: Branches are just labels applied when pushing
- **Non-destructive operations**: Nothing is ever lost; can undo any operation

### Common jj Commands

#### Viewing State
```bash
jj st                    # Status of current change (like git status)
jj log                   # History of changes (like git log)
jj show                  # Show current change details
jj diff                  # Show diff of current change
```

#### Creating & Managing Changes
```bash
jj new                   # Create new change on current
jj new main              # Create new change based on main
jj new -m "description"  # Create with description
jj describe -m "msg"     # Set/update description (like git commit)
jj edit <change-id>      # Switch to different change (like git checkout)
```

#### Syncing with Git Remotes
```bash
jj git fetch                    # Fetch from Git remote
jj git push --branch <name>     # Push current change as Git branch
jj git import                   # Import Git refs into jj
jj git export                   # Export jj changes to Git
jj rebase -d main               # Rebase current change onto main
```

#### Modifying Changes
```bash
jj squash              # Squash current change into parent
jj split               # Split current change into multiple
jj absorb              # Absorb fixes into earlier changes
```

#### Undo Operations
```bash
jj undo                # Undo last operation
jj op log              # View operation log
jj op restore <op-id>  # Restore to specific operation
```

### Simplified Workflows

#### Starting New Work
```bash
# Old Git way:
git checkout main
git pull
git checkout -b feature-branch
# ... make changes ...
git add .
git commit -m "Add feature"

# Simplified jj way:
jj new main -m "Add feature"
# ... make changes (automatically tracked) ...
jj git push --branch feature-branch
```

#### Working on Multiple Tasks
```bash
# Create multiple changes based on main
jj new main -m "Feature A"
# ... work on A ...

jj new main -m "Feature B"
# ... work on B ...

# Switch between them
jj edit <change-a-id>
jj edit <change-b-id>
```

#### Updating After Review
```bash
# Fetch latest, edit change, make fixes
jj git fetch
jj edit <change-id>
# ... make changes ...
jj rebase -d main
jj git push --branch feature --force
```

### Benefits for AI Agent Work

- **Parallel agents**: Each agent works in isolated change without coordination
- **No branch conflicts**: Changes coexist independently
- **Easy context switching**: Jump between tasks instantly
- **Clear history**: Each change is atomic and traceable
- **Automatic tracking**: No need to stage files before switching tasks

### Getting Started with jj

1. Install: `brew install jj` or `cargo install --locked jj-cli`
2. Initialize in repo: `jj init --git` (works alongside existing Git)
3. Start working: `jj new main -m "Task description"`
4. Push when ready: `jj git push --branch task-name`

### Learn More

- [jj Workflow Guide](docs/jj-workflow.md) - Detailed workflow patterns
- [Parallel Agents Guide](docs/parallel-agents.md) - Multi-agent development
- [Migration from Git](docs/migration-from-git.md) - Transition guide
- [Jujutsu Integration](docs/integrations/jujutsu.mdx) - Setup and configuration
