-- Add fields to support jj-based parallel agent sessions
-- This enables 5+ parallel sessions without worktree hell

-- Add VCS type to distinguish between git worktrees and jj sessions
ALTER TABLE workspaces ADD COLUMN vcs_type TEXT DEFAULT 'git' CHECK (vcs_type IN ('git', 'jj'));

-- Add jj change ID for jj-based sessions
-- For jj repos, this stores the change ID created for the agent session
-- For git repos, this remains NULL
ALTER TABLE workspaces ADD COLUMN jj_change_id TEXT;

-- Add index for faster lookups by change ID
CREATE INDEX IF NOT EXISTS idx_workspaces_jj_change_id ON workspaces(jj_change_id) WHERE jj_change_id IS NOT NULL;
