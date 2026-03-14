//! Git worktree lifecycle management.
//!
//! Worktree paths: ~/.koompi-orch/worktrees/{repo_name}/{branch}-{workspace_id}
//! Uses git2-rs for all git operations (no shelling out).

use git2::{Repository, WorktreeAddOptions};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum WorktreeError {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("repository not found at path: {0}")]
    RepoNotFound(String),
    #[error("worktree not found: {0}")]
    WorktreeNotFound(String),
    #[error("branch does not exist: {0}")]
    BranchNotFound(String),
}

/// Info about a created worktree
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub workspace_id: String,
    pub worktree_path: PathBuf,
    pub branch: String,
    pub repo_name: String,
}

/// Manages git worktree creation, listing, and cleanup.
pub struct WorktreeManager {
    /// Base directory for worktrees (e.g. ~/.koompi-orch/worktrees)
    base_dir: PathBuf,
}

impl WorktreeManager {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Create a new worktree for the given repo and branch.
    ///
    /// The worktree is placed at:
    ///   {base_dir}/{repo_name}/{branch}-{workspace_id}
    ///
    /// If the branch does not exist locally, it is created from HEAD.
    pub fn create_worktree(
        &self,
        repo_path: &Path,
        repo_name: &str,
        branch: &str,
    ) -> Result<WorktreeInfo, WorktreeError> {
        let repo = Repository::open(repo_path)
            .map_err(|_| WorktreeError::RepoNotFound(repo_path.display().to_string()))?;

        let workspace_id = Uuid::new_v4().to_string()[..8].to_string();
        let worktree_name = format!("{}-{}", branch, workspace_id);
        let worktree_dir = self.base_dir.join(repo_name);
        std::fs::create_dir_all(&worktree_dir)?;
        let worktree_path = worktree_dir.join(&worktree_name);

        // Ensure the branch exists; create from HEAD if it does not
        let branch_ref = match repo.find_branch(branch, git2::BranchType::Local) {
            Ok(b) => b.into_reference(),
            Err(_) => {
                // Create branch from HEAD
                let head_commit = repo.head()?.peel_to_commit()?;
                repo.branch(branch, &head_commit, false)?.into_reference()
            }
        };

        let mut opts = WorktreeAddOptions::new();
        opts.reference(Some(&branch_ref));

        repo.worktree(&worktree_name, &worktree_path, Some(&opts))?;

        Ok(WorktreeInfo {
            workspace_id,
            worktree_path,
            branch: branch.to_string(),
            repo_name: repo_name.to_string(),
        })
    }

    /// List all worktrees for a given repo name.
    pub fn list_worktrees(&self, repo_name: &str) -> Result<Vec<PathBuf>, WorktreeError> {
        let repo_dir = self.base_dir.join(repo_name);
        if !repo_dir.exists() {
            return Ok(vec![]);
        }

        let mut worktrees = Vec::new();
        for entry in std::fs::read_dir(&repo_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                worktrees.push(entry.path());
            }
        }
        Ok(worktrees)
    }

    /// Remove a worktree by its path. Prunes the worktree from the parent repo.
    pub fn remove_worktree(
        &self,
        repo_path: &Path,
        worktree_path: &Path,
    ) -> Result<(), WorktreeError> {
        let repo = Repository::open(repo_path)
            .map_err(|_| WorktreeError::RepoNotFound(repo_path.display().to_string()))?;

        // Find the worktree name from the directory name
        let worktree_name = worktree_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                WorktreeError::WorktreeNotFound(worktree_path.display().to_string())
            })?;

        // Prune the worktree reference from git
        if let Ok(wt) = repo.find_worktree(worktree_name) {
            // Validate and prune: locked worktrees are pruned forcefully
            wt.prune(Some(
                git2::WorktreePruneOptions::new()
                    .working_tree(true)
                    .valid(true)
                    .locked(true),
            ))?;
        }

        // Remove the directory on disk
        if worktree_path.exists() {
            std::fs::remove_dir_all(worktree_path)?;
        }

        Ok(())
    }

    /// Clean up all worktrees for a repo (used when removing a repo).
    pub fn cleanup_all(&self, repo_path: &Path, repo_name: &str) -> Result<(), WorktreeError> {
        let worktrees = self.list_worktrees(repo_name)?;
        for wt_path in worktrees {
            self.remove_worktree(repo_path, &wt_path)?;
        }
        // Remove the repo directory under worktrees base
        let repo_dir = self.base_dir.join(repo_name);
        if repo_dir.exists() {
            std::fs::remove_dir_all(&repo_dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use tempfile::TempDir;

    /// Create a real git repo with an initial commit so worktrees work.
    fn create_test_repo(dir: &Path) -> Repository {
        let repo = Repository::init(dir).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
                .unwrap();
        }
        repo
    }

    #[test]
    fn test_create_worktree() {
        let repo_dir = TempDir::new().unwrap();
        let worktree_base = TempDir::new().unwrap();
        let _repo = create_test_repo(repo_dir.path());

        let manager = WorktreeManager::new(worktree_base.path().to_path_buf());
        let info = manager
            .create_worktree(repo_dir.path(), "test-repo", "feat-auth")
            .unwrap();

        assert_eq!(info.branch, "feat-auth");
        assert_eq!(info.repo_name, "test-repo");
        assert!(info.worktree_path.exists());
        assert_eq!(info.workspace_id.len(), 8);
    }

    #[test]
    fn test_list_worktrees() {
        let repo_dir = TempDir::new().unwrap();
        let worktree_base = TempDir::new().unwrap();
        let _repo = create_test_repo(repo_dir.path());

        let manager = WorktreeManager::new(worktree_base.path().to_path_buf());

        // Initially empty
        let list = manager.list_worktrees("test-repo").unwrap();
        assert!(list.is_empty());

        // Create two worktrees
        manager
            .create_worktree(repo_dir.path(), "test-repo", "branch-a")
            .unwrap();
        manager
            .create_worktree(repo_dir.path(), "test-repo", "branch-b")
            .unwrap();

        let list = manager.list_worktrees("test-repo").unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_remove_worktree() {
        let repo_dir = TempDir::new().unwrap();
        let worktree_base = TempDir::new().unwrap();
        let _repo = create_test_repo(repo_dir.path());

        let manager = WorktreeManager::new(worktree_base.path().to_path_buf());
        let info = manager
            .create_worktree(repo_dir.path(), "test-repo", "feat-remove")
            .unwrap();

        assert!(info.worktree_path.exists());
        manager
            .remove_worktree(repo_dir.path(), &info.worktree_path)
            .unwrap();
        assert!(!info.worktree_path.exists());
    }

    #[test]
    fn test_cleanup_all() {
        let repo_dir = TempDir::new().unwrap();
        let worktree_base = TempDir::new().unwrap();
        let _repo = create_test_repo(repo_dir.path());

        let manager = WorktreeManager::new(worktree_base.path().to_path_buf());
        manager
            .create_worktree(repo_dir.path(), "test-repo", "a")
            .unwrap();
        manager
            .create_worktree(repo_dir.path(), "test-repo", "b")
            .unwrap();

        manager
            .cleanup_all(repo_dir.path(), "test-repo")
            .unwrap();

        assert!(!worktree_base.path().join("test-repo").exists());
    }

    #[test]
    fn test_create_worktree_existing_branch() {
        let repo_dir = TempDir::new().unwrap();
        let worktree_base = TempDir::new().unwrap();
        let repo = create_test_repo(repo_dir.path());

        // Pre-create the branch
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("existing-branch", &head, false).unwrap();

        let manager = WorktreeManager::new(worktree_base.path().to_path_buf());
        let info = manager
            .create_worktree(repo_dir.path(), "test-repo", "existing-branch")
            .unwrap();

        assert_eq!(info.branch, "existing-branch");
        assert!(info.worktree_path.exists());
    }
}
