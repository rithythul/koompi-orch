//! Checkpoint/revert to any agent turn using git commits.
//!
//! Each checkpoint is a commit with a structured message containing
//! the turn number and description. Reverting checks out a specific
//! commit SHA in the worktree.

use git2::{Repository, ResetType, Signature};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SnapshotError {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("no changes to checkpoint")]
    NothingToCommit,
    #[error("commit not found: {0}")]
    CommitNotFound(String),
}

/// Info about a created snapshot
#[derive(Debug, Clone)]
pub struct SnapshotInfo {
    pub commit_sha: String,
    pub turn_number: u32,
    pub description: String,
}

/// Manages checkpoint creation and revert operations in a worktree.
pub struct SnapshotManager;

impl SnapshotManager {
    /// Create a checkpoint commit of all current changes in the worktree.
    ///
    /// Stages all modified/added/deleted files, then commits with a message
    /// encoding the turn number: `[checkpoint] turn {n}: {description}`
    pub fn create_checkpoint(
        worktree_path: &Path,
        turn_number: u32,
        description: &str,
    ) -> Result<SnapshotInfo, SnapshotError> {
        let repo = Repository::open(worktree_path)?;

        // Stage all changes (equivalent to git add -A)
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;

        // Check if there are changes to commit
        let tree_oid = index.write_tree()?;
        let tree = repo.find_tree(tree_oid)?;

        let has_changes = if let Ok(head) = repo.head() {
            let head_tree = head.peel_to_tree()?;
            let diff = repo.diff_tree_to_tree(Some(&head_tree), Some(&tree), None)?;
            diff.deltas().len() > 0
        } else {
            true // No HEAD means first commit
        };

        if !has_changes {
            return Err(SnapshotError::NothingToCommit);
        }

        let message = format!("[checkpoint] turn {}: {}", turn_number, description);
        let sig = Signature::now("koompi-orch", "koompi-orch@local")?;

        let commit_oid = if let Ok(head) = repo.head() {
            let parent = head.peel_to_commit()?;
            repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&parent])?
        } else {
            repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[])?
        };

        Ok(SnapshotInfo {
            commit_sha: commit_oid.to_string(),
            turn_number,
            description: description.to_string(),
        })
    }

    /// Revert the worktree to a specific commit SHA.
    ///
    /// Uses a hard reset to move HEAD and working directory to the target commit.
    pub fn revert_to(worktree_path: &Path, commit_sha: &str) -> Result<(), SnapshotError> {
        let repo = Repository::open(worktree_path)?;
        let oid = git2::Oid::from_str(commit_sha)
            .map_err(|_| SnapshotError::CommitNotFound(commit_sha.to_string()))?;
        let commit = repo
            .find_commit(oid)
            .map_err(|_| SnapshotError::CommitNotFound(commit_sha.to_string()))?;
        let obj = commit.into_object();
        repo.reset(&obj, ResetType::Hard, None)?;
        Ok(())
    }

    /// List all checkpoint commits in the worktree (newest first).
    ///
    /// Walks the commit log and filters for messages starting with `[checkpoint]`.
    pub fn list_checkpoints(
        worktree_path: &Path,
    ) -> Result<Vec<SnapshotInfo>, SnapshotError> {
        let repo = Repository::open(worktree_path)?;
        let head = repo.head()?;
        let head_oid = head
            .target()
            .ok_or_else(|| SnapshotError::Git(git2::Error::from_str("HEAD has no target")))?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push(head_oid)?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;

        let mut checkpoints = Vec::new();
        for oid_result in revwalk {
            let oid = oid_result?;
            let commit = repo.find_commit(oid)?;
            let message = commit.message().unwrap_or("");
            if let Some(rest) = message.strip_prefix("[checkpoint] turn ") {
                // Parse "turn {n}: {description}"
                if let Some(colon_pos) = rest.find(':') {
                    let turn_str = &rest[..colon_pos];
                    let desc = rest[colon_pos + 1..].trim();
                    if let Ok(turn) = turn_str.trim().parse::<u32>() {
                        checkpoints.push(SnapshotInfo {
                            commit_sha: oid.to_string(),
                            turn_number: turn,
                            description: desc.to_string(),
                        });
                    }
                }
            }
        }
        Ok(checkpoints)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use std::fs;
    use tempfile::TempDir;

    /// Create a test repo with an initial commit and return the repo + dir.
    fn setup_repo() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = Signature::now("Test", "test@test.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();
        }
        (dir, repo)
    }

    #[test]
    fn test_create_checkpoint() {
        let (dir, _repo) = setup_repo();

        // Create a file so there are changes to commit
        fs::write(dir.path().join("hello.txt"), "hello world").unwrap();

        let info =
            SnapshotManager::create_checkpoint(dir.path(), 1, "implemented auth module").unwrap();

        assert_eq!(info.turn_number, 1);
        assert_eq!(info.description, "implemented auth module");
        assert_eq!(info.commit_sha.len(), 40); // full SHA
    }

    #[test]
    fn test_nothing_to_commit() {
        let (dir, _repo) = setup_repo();

        let result = SnapshotManager::create_checkpoint(dir.path(), 1, "no changes");

        assert!(matches!(result, Err(SnapshotError::NothingToCommit)));
    }

    #[test]
    fn test_revert_to_checkpoint() {
        let (dir, _repo) = setup_repo();

        // Turn 1: create file
        fs::write(dir.path().join("file.txt"), "version 1").unwrap();
        let snap1 = SnapshotManager::create_checkpoint(dir.path(), 1, "v1").unwrap();

        // Turn 2: modify file
        fs::write(dir.path().join("file.txt"), "version 2").unwrap();
        let _snap2 = SnapshotManager::create_checkpoint(dir.path(), 2, "v2").unwrap();

        // Verify current content is v2
        assert_eq!(
            fs::read_to_string(dir.path().join("file.txt")).unwrap(),
            "version 2"
        );

        // Revert to turn 1
        SnapshotManager::revert_to(dir.path(), &snap1.commit_sha).unwrap();

        // Verify content is back to v1
        assert_eq!(
            fs::read_to_string(dir.path().join("file.txt")).unwrap(),
            "version 1"
        );
    }

    #[test]
    fn test_list_checkpoints() {
        let (dir, _repo) = setup_repo();

        fs::write(dir.path().join("a.txt"), "a").unwrap();
        SnapshotManager::create_checkpoint(dir.path(), 1, "first").unwrap();

        fs::write(dir.path().join("b.txt"), "b").unwrap();
        SnapshotManager::create_checkpoint(dir.path(), 2, "second").unwrap();

        fs::write(dir.path().join("c.txt"), "c").unwrap();
        SnapshotManager::create_checkpoint(dir.path(), 3, "third").unwrap();

        let checkpoints = SnapshotManager::list_checkpoints(dir.path()).unwrap();
        assert_eq!(checkpoints.len(), 3);
        // Newest first
        assert_eq!(checkpoints[0].turn_number, 3);
        assert_eq!(checkpoints[1].turn_number, 2);
        assert_eq!(checkpoints[2].turn_number, 1);
    }

    #[test]
    fn test_revert_to_invalid_sha() {
        let (dir, _repo) = setup_repo();
        let result =
            SnapshotManager::revert_to(dir.path(), "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
        assert!(matches!(result, Err(SnapshotError::CommitNotFound(_))));
    }
}
