-- Add VCS backend selection to repos table
-- Defaults to 'git' for compatibility
ALTER TABLE repos ADD COLUMN vcs_backend TEXT NOT NULL DEFAULT 'git' CHECK (vcs_backend IN ('git', 'jj'));
