//! Git backend implementation for VCS abstraction layer
//!
//! This module wraps git2 operations to implement the VCS traits.

use crate::error::VcsError;
use crate::factory::VcsBackendType;
use crate::traits::*;
use crate::types::*;
use std::path::{Path, PathBuf};

use git2::{BranchType, Repository};

/// Git implementation of VCS backend
pub struct GitRepository {
    path: PathBuf,
    repo: Repository,
}

impl GitRepository {
    /// Create a new GitRepository from an existing git2::Repository
    pub fn from_git2(repo: Repository) -> Result<Self, VcsError> {
        let path = repo
            .workdir()
            .ok_or_else(|| VcsError::InvalidOperation("Bare repositories not supported".into()))?
            .to_path_buf();

        Ok(Self { path, repo })
    }

    /// Get the underlying git2::Repository
    pub fn git2_repo(&self) -> &Repository {
        &self.repo
    }

    /// Convert git2::Oid to ChangeId
    fn oid_to_change_id(oid: git2::Oid) -> ChangeId {
        ChangeId::new(oid.to_string())
    }

    /// Convert ChangeId to git2::Oid
    fn change_id_to_oid(id: &ChangeId) -> Result<git2::Oid, VcsError> {
        git2::Oid::from_str(id.as_str()).map_err(|_| VcsError::InvalidChangeId(id.to_string()))
    }
}

// ============================================================================
// VcsRepository Implementation
// ============================================================================

impl VcsRepository for GitRepository {
    fn init(path: &Path) -> Result<Self, VcsError> {
        if !path.exists() {
            std::fs::create_dir_all(path)?;
        }

        let repo = Repository::init_opts(
            path,
            git2::RepositoryInitOptions::new()
                .initial_head("main")
                .mkdir(true),
        )
        .map_err(VcsError::backend)?;

        // Create initial commit
        {
            let signature = git2::Signature::now("Vibe Kanban", "noreply@vibekanban.com")
                .map_err(VcsError::backend)?;

            let tree_id = {
                let tree_builder = repo.treebuilder(None).map_err(VcsError::backend)?;
                tree_builder.write().map_err(VcsError::backend)?
            };
            let tree = repo.find_tree(tree_id).map_err(VcsError::backend)?;

            repo.commit(
                Some("refs/heads/main"),
                &signature,
                &signature,
                "Initial commit",
                &tree,
                &[],
            )
            .map_err(VcsError::backend)?;

            repo.set_head("refs/heads/main")
                .map_err(VcsError::backend)?;
        }

        Self::from_git2(repo)
    }

    fn open(path: &Path) -> Result<Self, VcsError> {
        let repo = Repository::open(path).map_err(VcsError::backend)?;
        Self::from_git2(repo)
    }

    fn clone(url: &str, path: &Path) -> Result<Self, VcsError> {
        if !path.exists() {
            std::fs::create_dir_all(path)?;
        }

        let repo = Repository::clone(url, path).map_err(VcsError::backend)?;
        Self::from_git2(repo)
    }

    fn work_dir(&self) -> &Path {
        &self.path
    }

    fn is_clean(&self) -> Result<bool, VcsError> {
        // Check for ongoing operations
        if self.repo.state() != git2::RepositoryState::Clean {
            return Ok(false);
        }

        // Check for conflicts
        let index = self.repo.index().map_err(VcsError::backend)?;
        if index.has_conflicts() {
            return Ok(false);
        }

        Ok(true)
    }

    fn head(&self) -> Result<HeadInfo, VcsError> {
        let head = self.repo.head().map_err(VcsError::backend)?;

        let branch = if head.is_branch() {
            head.shorthand().map(String::from)
        } else {
            None
        };

        let oid = head
            .target()
            .ok_or_else(|| VcsError::InvalidOperation("HEAD has no target".into()))?;

        let commit = self.repo.find_commit(oid).map_err(VcsError::backend)?;
        let description = commit
            .summary()
            .unwrap_or("(no message)")
            .to_string();

        Ok(HeadInfo {
            branch,
            change_id: Self::oid_to_change_id(oid),
            description,
        })
    }

    fn is_valid(&self) -> bool {
        // Check if the repository can be accessed
        self.repo.state() == git2::RepositoryState::Clean || !self.repo.is_bare()
    }
}

// ============================================================================
// VcsChanges Implementation
// ============================================================================

impl VcsChanges for GitRepository {
    fn create_change(&self, message: &str) -> Result<ChangeId, VcsError> {
        self.create_change_with_options(message, CreateChangeOptions::default())
    }

    fn create_change_with_options(
        &self,
        message: &str,
        options: CreateChangeOptions,
    ) -> Result<ChangeId, VcsError> {
        // Stage changes if requested
        if options.stage_all {
            let mut index = self.repo.index().map_err(VcsError::backend)?;
            index
                .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
                .map_err(VcsError::backend)?;
            index.write().map_err(VcsError::backend)?;
        }

        let mut index = self.repo.index().map_err(VcsError::backend)?;
        let tree_oid = index.write_tree().map_err(VcsError::backend)?;
        let tree = self.repo.find_tree(tree_oid).map_err(VcsError::backend)?;

        // Get parent commits
        let parents: Vec<git2::Commit> = if options.parents.is_empty() {
            // Use HEAD as parent
            if let Ok(head) = self.repo.head() {
                if let Some(oid) = head.target() {
                    if let Ok(commit) = self.repo.find_commit(oid) {
                        vec![commit]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        } else {
            options
                .parents
                .iter()
                .filter_map(|id| {
                    Self::change_id_to_oid(id)
                        .ok()
                        .and_then(|oid| self.repo.find_commit(oid).ok())
                })
                .collect()
        };

        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

        // Get or create signature
        let signature = self
            .repo
            .signature()
            .or_else(|_| git2::Signature::now("Vibe Kanban", "noreply@vibekanban.com"))
            .map_err(VcsError::backend)?;

        let oid = self
            .repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &parent_refs,
            )
            .map_err(VcsError::backend)?;

        Ok(Self::oid_to_change_id(oid))
    }

    fn amend_change(&self, message: Option<&str>) -> Result<(), VcsError> {
        let head = self.repo.head().map_err(VcsError::backend)?;
        let head_commit = head
            .peel_to_commit()
            .map_err(VcsError::backend)?;

        let _signature = self
            .repo
            .signature()
            .or_else(|_| {
                git2::Signature::now("Vibe Kanban", "noreply@vibekanban.com")
            })
            .map_err(VcsError::backend)?;

        let mut index = self.repo.index().map_err(VcsError::backend)?;
        let tree_oid = index.write_tree().map_err(VcsError::backend)?;
        let tree = self.repo.find_tree(tree_oid).map_err(VcsError::backend)?;

        let message = message.unwrap_or_else(|| {
            head_commit.message().unwrap_or("Amended commit")
        });

        head_commit
            .amend(
                Some("HEAD"),
                None,
                None,
                None,
                Some(message),
                Some(&tree),
            )
            .map_err(VcsError::backend)?;

        Ok(())
    }

    fn get_change(&self, id: &ChangeId) -> Result<ChangeInfo, VcsError> {
        let oid = Self::change_id_to_oid(id)?;
        let commit = self.repo.find_commit(oid).map_err(VcsError::backend)?;

        let parent_ids = commit
            .parent_ids()
            .map(Self::oid_to_change_id)
            .collect();

        let timestamp = chrono::DateTime::from_timestamp(commit.time().seconds(), 0)
            .ok_or_else(|| VcsError::InvalidOperation("Invalid timestamp".into()))?;

        let author = commit.author();
        let author_name = author.name().unwrap_or("unknown").to_string();

        Ok(ChangeInfo {
            id: id.clone(),
            parent_ids,
            author: author_name,
            timestamp,
            description: commit.message().unwrap_or("").to_string(),
            is_empty: commit.parent_count() > 0
                && commit
                    .tree_id()
                    == commit
                        .parent(0)
                        .ok()
                        .map(|p| p.tree_id())
                        .unwrap_or(commit.tree_id()),
        })
    }

    fn list_changes(&self, filter: ChangeFilter) -> Result<Vec<ChangeInfo>, VcsError> {
        let mut revwalk = self.repo.revwalk().map_err(VcsError::backend)?;
        revwalk.set_sorting(git2::Sort::TIME).map_err(VcsError::backend)?;

        // Start from the appropriate reference
        if let Some(branch) = &filter.branch {
            let reference = self
                .repo
                .find_branch(branch, BranchType::Local)
                .map_err(VcsError::backend)?;
            let target = reference
                .get()
                .target()
                .ok_or_else(|| VcsError::BranchNotFound(branch.clone()))?;
            revwalk.push(target).map_err(VcsError::backend)?;
        } else {
            revwalk.push_head().map_err(VcsError::backend)?;
        }

        let mut changes = Vec::new();
        let mut count = 0;

        for oid in revwalk {
            let oid = oid.map_err(VcsError::backend)?;
            let commit = self.repo.find_commit(oid).map_err(VcsError::backend)?;

            // Apply filters
            if let Some(author) = &filter.author {
                if commit.author().name() != Some(author.as_str()) {
                    continue;
                }
            }

            if let Some(since) = filter.since {
                let commit_time = chrono::DateTime::from_timestamp(commit.time().seconds(), 0)
                    .ok_or_else(|| VcsError::InvalidOperation("Invalid timestamp".into()))?;
                if commit_time < since {
                    break;
                }
            }

            let change_id = Self::oid_to_change_id(oid);
            changes.push(self.get_change(&change_id)?);

            count += 1;
            if let Some(limit) = filter.limit {
                if count >= limit {
                    break;
                }
            }
        }

        Ok(changes)
    }

    fn abandon_change(&self, id: &ChangeId) -> Result<(), VcsError> {
        // For Git, we can't easily "abandon" a commit, but we can reset
        // This is a destructive operation
        let oid = Self::change_id_to_oid(id)?;
        let commit = self.repo.find_commit(oid).map_err(VcsError::backend)?;

        // Reset to parent
        if commit.parent_count() > 0 {
            let parent = commit.parent(0).map_err(VcsError::backend)?;
            self.repo
                .reset(
                    parent.as_object(),
                    git2::ResetType::Mixed,
                    None,
                )
                .map_err(VcsError::backend)?;
        }

        Ok(())
    }

    fn change_exists(&self, id: &ChangeId) -> Result<bool, VcsError> {
        let oid = Self::change_id_to_oid(id)?;
        Ok(self.repo.find_commit(oid).is_ok())
    }
}

// ============================================================================
// VcsBranches Implementation
// ============================================================================

impl VcsBranches for GitRepository {
    fn create_branch(&self, name: &str, base: Option<&ChangeId>) -> Result<(), VcsError> {
        let commit = if let Some(base_id) = base {
            let oid = Self::change_id_to_oid(base_id)?;
            self.repo.find_commit(oid).map_err(VcsError::backend)?
        } else {
            let head = self.repo.head().map_err(VcsError::backend)?;
            head.peel_to_commit().map_err(VcsError::backend)?
        };

        self.repo
            .branch(name, &commit, false)
            .map_err(VcsError::backend)?;

        Ok(())
    }

    fn delete_branch(&self, name: &str) -> Result<(), VcsError> {
        let mut branch = self
            .repo
            .find_branch(name, BranchType::Local)
            .map_err(VcsError::backend)?;

        branch.delete().map_err(VcsError::backend)?;
        Ok(())
    }

    fn rename_branch(&self, old_name: &str, new_name: &str) -> Result<(), VcsError> {
        let mut branch = self
            .repo
            .find_branch(old_name, BranchType::Local)
            .map_err(VcsError::backend)?;

        branch
            .rename(new_name, false)
            .map_err(VcsError::backend)?;
        Ok(())
    }

    fn list_branches(&self) -> Result<Vec<BranchInfo>, VcsError> {
        let branches = self.repo.branches(None).map_err(VcsError::backend)?;

        let mut result = Vec::new();

        for branch_result in branches {
            let (branch, branch_type) = branch_result.map_err(VcsError::backend)?;

            let name = branch
                .name()
                .map_err(VcsError::backend)?
                .ok_or_else(|| VcsError::InvalidOperation("Invalid branch name".into()))?
                .to_string();

            let reference = branch.get();
            let oid = reference
                .target()
                .ok_or_else(|| VcsError::InvalidOperation("Branch has no target".into()))?;

            let commit = self.repo.find_commit(oid).map_err(VcsError::backend)?;
            let timestamp = chrono::DateTime::from_timestamp(commit.time().seconds(), 0)
                .ok_or_else(|| VcsError::InvalidOperation("Invalid timestamp".into()))?;

            result.push(BranchInfo {
                name,
                change_id: Self::oid_to_change_id(oid),
                is_current: branch.is_head(),
                is_remote: branch_type == BranchType::Remote,
                last_updated: timestamp,
            });
        }

        Ok(result)
    }

    fn current_branch(&self) -> Result<Option<String>, VcsError> {
        let head = self.repo.head().map_err(VcsError::backend)?;

        if head.is_branch() {
            Ok(head.shorthand().map(String::from))
        } else {
            Ok(None)
        }
    }

    fn switch_to(&self, target: &BranchOrChange) -> Result<(), VcsError> {
        match target {
            BranchOrChange::Branch(branch_name) => {
                let (obj, reference) = self
                    .repo
                    .revparse_ext(branch_name)
                    .map_err(VcsError::backend)?;

                self.repo
                    .checkout_tree(&obj, None)
                    .map_err(VcsError::backend)?;

                match reference {
                    Some(gref) => {
                        self.repo
                            .set_head(gref.name().ok_or_else(|| {
                                VcsError::InvalidOperation("Invalid reference name".into())
                            })?)
                            .map_err(VcsError::backend)?;
                    }
                    None => {
                        self.repo
                            .set_head_detached(obj.id())
                            .map_err(VcsError::backend)?;
                    }
                }
            }
            BranchOrChange::Change(change_id) => {
                let oid = Self::change_id_to_oid(change_id)?;
                let commit = self.repo.find_commit(oid).map_err(VcsError::backend)?;

                self.repo
                    .checkout_tree(commit.as_object(), None)
                    .map_err(VcsError::backend)?;

                self.repo
                    .set_head_detached(oid)
                    .map_err(VcsError::backend)?;
            }
        }

        Ok(())
    }

    fn branch_exists(&self, name: &str) -> Result<bool, VcsError> {
        Ok(self
            .repo
            .find_branch(name, BranchType::Local)
            .is_ok())
    }

    fn is_branch_name_valid(&self, name: &str) -> bool {
        git2::Branch::name_is_valid(name).unwrap_or(false)
    }
}

// ============================================================================
// VcsRemotes Implementation
// ============================================================================

impl VcsRemotes for GitRepository {
    fn fetch(&self, options: FetchOptions) -> Result<(), VcsError> {
        let remote_name = options.remote.as_deref().unwrap_or("origin");
        let mut remote = self
            .repo
            .find_remote(remote_name)
            .map_err(VcsError::backend)?;

        let mut fetch_options = git2::FetchOptions::new();
        if options.prune {
            fetch_options.prune(git2::FetchPrune::On);
        }

        remote
            .fetch(&[] as &[&str], Some(&mut fetch_options), None)
            .map_err(VcsError::backend)?;

        Ok(())
    }

    fn push(&self, options: PushOptions) -> Result<(), VcsError> {
        let remote_name = options.remote.as_deref().unwrap_or("origin");
        let mut remote = self
            .repo
            .find_remote(remote_name)
            .map_err(VcsError::backend)?;

        let branch = if let Some(b) = options.branch {
            b
        } else {
            self.current_branch()?
                .ok_or_else(|| VcsError::InvalidOperation("Not on a branch".into()))?
        };

        let refspec = format!("refs/heads/{}", branch);
        let mut push_options = git2::PushOptions::new();

        remote
            .push(&[&refspec], Some(&mut push_options))
            .map_err(|e| VcsError::PushRejected(e.to_string()))?;

        Ok(())
    }

    fn remote_branch_exists(&self, remote: &str, branch: &str) -> Result<bool, VcsError> {
        let remote_branch = format!("{}/{}", remote, branch);
        Ok(self
            .repo
            .find_branch(&remote_branch, BranchType::Remote)
            .is_ok())
    }

    fn get_remote_url(&self, name: &str) -> Result<String, VcsError> {
        let remote = self.repo.find_remote(name).map_err(VcsError::backend)?;

        remote
            .url()
            .ok_or_else(|| VcsError::InvalidOperation("Remote has no URL".into()))
            .map(String::from)
    }

    fn set_remote_url(&self, name: &str, url: &str) -> Result<(), VcsError> {
        self.repo
            .remote_set_url(name, url)
            .map_err(VcsError::backend)?;
        Ok(())
    }

    fn list_remotes(&self) -> Result<Vec<String>, VcsError> {
        let remotes = self.repo.remotes().map_err(VcsError::backend)?;

        Ok(remotes
            .iter()
            .filter_map(|r| r.map(String::from))
            .collect())
    }
}

// ============================================================================
// VcsDiff Implementation
// ============================================================================

impl VcsDiff for GitRepository {
    fn diff_changes(&self, from: &ChangeId, to: &ChangeId) -> Result<Vec<FileDiff>, VcsError> {
        let from_oid = Self::change_id_to_oid(from)?;
        let to_oid = Self::change_id_to_oid(to)?;

        let from_commit = self.repo.find_commit(from_oid).map_err(VcsError::backend)?;
        let to_commit = self.repo.find_commit(to_oid).map_err(VcsError::backend)?;

        let from_tree = from_commit.tree().map_err(VcsError::backend)?;
        let to_tree = to_commit.tree().map_err(VcsError::backend)?;

        let diff = self
            .repo
            .diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)
            .map_err(VcsError::backend)?;

        self.convert_diff_to_file_diffs(&diff)
    }

    fn diff_uncommitted(&self) -> Result<Vec<FileDiff>, VcsError> {
        let head = self.repo.head().map_err(VcsError::backend)?;
        let head_commit = head.peel_to_commit().map_err(VcsError::backend)?;
        let head_tree = head_commit.tree().map_err(VcsError::backend)?;

        let diff = self
            .repo
            .diff_tree_to_workdir_with_index(Some(&head_tree), None)
            .map_err(VcsError::backend)?;

        self.convert_diff_to_file_diffs(&diff)
    }

    fn status(&self) -> Result<Vec<FileStatus>, VcsError> {
        let statuses = self
            .repo
            .statuses(None)
            .map_err(VcsError::backend)?;

        let mut result = Vec::new();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let status_flags = entry.status();

            let status = if status_flags.is_conflicted() {
                FileStatusKind::Conflicted
            } else if status_flags.is_wt_new() {
                FileStatusKind::Untracked
            } else if status_flags.is_wt_modified() || status_flags.is_index_modified() {
                FileStatusKind::Modified
            } else if status_flags.is_wt_deleted() || status_flags.is_index_deleted() {
                FileStatusKind::Deleted
            } else if status_flags.is_index_new() {
                FileStatusKind::Added
            } else {
                continue;
            };

            result.push(FileStatus { path, status });
        }

        Ok(result)
    }

    fn has_uncommitted_changes(&self) -> Result<bool, VcsError> {
        let statuses = self
            .repo
            .statuses(None)
            .map_err(VcsError::backend)?;

        Ok(!statuses.is_empty())
    }
}

impl GitRepository {
    fn convert_diff_to_file_diffs(&self, diff: &git2::Diff) -> Result<Vec<FileDiff>, VcsError> {
        let mut file_diffs = Vec::new();

        for delta in diff.deltas() {
            let old_file = delta.old_file();
            let new_file = delta.new_file();

            let path = new_file
                .path()
                .or_else(|| old_file.path())
                .and_then(|p| p.to_str())
                .ok_or_else(|| VcsError::InvalidOperation("Invalid file path".into()))?
                .to_string();

            let old_path = if delta.status() == git2::Delta::Renamed {
                old_file.path().and_then(|p| p.to_str()).map(String::from)
            } else {
                None
            };

            let change_type = match delta.status() {
                git2::Delta::Added => FileChangeType::Added,
                git2::Delta::Modified => FileChangeType::Modified,
                git2::Delta::Deleted => FileChangeType::Deleted,
                git2::Delta::Renamed => FileChangeType::Renamed,
                git2::Delta::Copied => FileChangeType::Copied,
                _ => continue,
            };

            file_diffs.push(FileDiff {
                path,
                old_path,
                change_type,
                additions: 0, // TODO: Compute from patches
                deletions: 0, // TODO: Compute from patches
                content: None, // TODO: Add content extraction
            });
        }

        Ok(file_diffs)
    }
}

// ============================================================================
// VcsConflicts Implementation
// ============================================================================

impl VcsConflicts for GitRepository {
    fn has_conflicts(&self) -> Result<bool, VcsError> {
        let index = self.repo.index().map_err(VcsError::backend)?;
        Ok(index.has_conflicts())
    }

    fn list_conflicts(&self) -> Result<Vec<ConflictInfo>, VcsError> {
        let index = self.repo.index().map_err(VcsError::backend)?;

        if !index.has_conflicts() {
            return Ok(Vec::new());
        }

        let operation = self.ongoing_operation()?.unwrap_or(ConflictOperation::Merge);

        let mut conflicts = Vec::new();
        let conflict_iter = index.conflicts().map_err(VcsError::backend)?;

        for conflict in conflict_iter {
            let conflict = conflict.map_err(VcsError::backend)?;

            if let Some(ours) = conflict.our {
                if let Some(theirs) = conflict.their {
                    let path = String::from_utf8_lossy(&ours.path).to_string();

                    let sides = ConflictSides {
                        ours: Self::oid_to_change_id(ours.id),
                        theirs: Self::oid_to_change_id(theirs.id),
                        base: conflict.ancestor.map(|a| Self::oid_to_change_id(a.id)),
                    };

                    conflicts.push(ConflictInfo {
                        path,
                        operation,
                        sides,
                    });
                }
            }
        }

        Ok(conflicts)
    }

    fn resolve_conflict(&self, path: &Path) -> Result<(), VcsError> {
        let mut index = self.repo.index().map_err(VcsError::backend)?;

        // Add the file to mark it as resolved
        index
            .add_path(path)
            .map_err(VcsError::backend)?;

        index.write().map_err(VcsError::backend)?;
        Ok(())
    }

    fn abort_operation(&self) -> Result<(), VcsError> {
        match self.repo.state() {
            git2::RepositoryState::Merge => {
                self.repo.cleanup_state().map_err(VcsError::backend)?;
            }
            git2::RepositoryState::Rebase
            | git2::RepositoryState::RebaseInteractive
            | git2::RepositoryState::RebaseMerge => {
                // Git doesn't provide a direct API to abort rebase via libgit2
                // This would typically require CLI: git rebase --abort
                return Err(VcsError::InvalidOperation(
                    "Rebase abort requires CLI".into(),
                ));
            }
            git2::RepositoryState::CherryPick | git2::RepositoryState::CherryPickSequence => {
                self.repo.cleanup_state().map_err(VcsError::backend)?;
            }
            git2::RepositoryState::Revert | git2::RepositoryState::RevertSequence => {
                self.repo.cleanup_state().map_err(VcsError::backend)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn ongoing_operation(&self) -> Result<Option<ConflictOperation>, VcsError> {
        match self.repo.state() {
            git2::RepositoryState::Merge => Ok(Some(ConflictOperation::Merge)),
            git2::RepositoryState::Rebase
            | git2::RepositoryState::RebaseInteractive
            | git2::RepositoryState::RebaseMerge => Ok(Some(ConflictOperation::Rebase)),
            git2::RepositoryState::CherryPick | git2::RepositoryState::CherryPickSequence => {
                Ok(Some(ConflictOperation::CherryPick))
            }
            git2::RepositoryState::Revert | git2::RepositoryState::RevertSequence => {
                Ok(Some(ConflictOperation::Revert))
            }
            _ => Ok(None),
        }
    }
}

// ============================================================================
// VcsBackend Implementation
// ============================================================================

impl VcsBackend for GitRepository {
    fn backend_type(&self) -> VcsBackendType {
        VcsBackendType::Git
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, GitRepository) {
        let temp = TempDir::new().unwrap();
        let repo = GitRepository::init(temp.path()).unwrap();
        (temp, repo)
    }

    #[test]
    fn test_init_repository() {
        let (_temp, repo) = setup_test_repo();
        assert!(repo.is_valid());
    }

    #[test]
    fn test_head() {
        let (_temp, repo) = setup_test_repo();
        let head = repo.head().unwrap();
        assert_eq!(head.branch, Some("main".to_string()));
        assert_eq!(head.description, "Initial commit");
    }

    #[test]
    fn test_is_clean() {
        let (_temp, repo) = setup_test_repo();
        assert!(repo.is_clean().unwrap());
    }

    #[test]
    fn test_list_branches() {
        let (_temp, repo) = setup_test_repo();
        let branches = repo.list_branches().unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");
        assert!(branches[0].is_current);
    }

    #[test]
    fn test_create_branch() {
        let (_temp, repo) = setup_test_repo();
        repo.create_branch("feature", None).unwrap();

        let branches = repo.list_branches().unwrap();
        assert_eq!(branches.len(), 2);

        let feature_branch = branches.iter().find(|b| b.name == "feature");
        assert!(feature_branch.is_some());
    }

    #[test]
    fn test_branch_name_validation() {
        let (_temp, repo) = setup_test_repo();
        assert!(repo.is_branch_name_valid("feature/test"));
        assert!(repo.is_branch_name_valid("feature-123"));
        assert!(!repo.is_branch_name_valid("feature..test"));
        assert!(!repo.is_branch_name_valid(""));
    }
}
