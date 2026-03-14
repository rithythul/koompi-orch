//! Merge and rebase operations via git2-rs.
//!
//! Provides merge and rebase of one branch into another within a worktree.

use git2::{MergeOptions, Repository, Signature};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MergeError {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("merge conflict in {0} file(s)")]
    Conflict(usize),
    #[error("branch not found: {0}")]
    BranchNotFound(String),
    #[error("nothing to merge: branches are already up to date")]
    AlreadyUpToDate,
}

/// Result of a merge operation.
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// The SHA of the merge commit (or fast-forward target)
    pub commit_sha: String,
    /// Whether it was a fast-forward merge
    pub fast_forward: bool,
}

/// Merge a source branch into the current HEAD of the worktree.
///
/// Attempts fast-forward first; falls back to a merge commit.
/// Returns an error with conflict count if there are unresolvable conflicts.
pub fn merge_branch(
    worktree_path: &Path,
    source_branch: &str,
) -> Result<MergeResult, MergeError> {
    let repo = Repository::open(worktree_path)?;

    let source_ref = repo
        .find_branch(source_branch, git2::BranchType::Local)
        .map_err(|_| MergeError::BranchNotFound(source_branch.to_string()))?;
    let source_commit = source_ref.get().peel_to_commit()?;
    let annotated = repo.find_annotated_commit(source_commit.id())?;

    let analysis = repo.merge_analysis(&[&annotated])?;

    if analysis.0.is_up_to_date() {
        return Err(MergeError::AlreadyUpToDate);
    }

    if analysis.0.is_fast_forward() {
        // Fast-forward
        let target_oid = source_commit.id();
        let mut reference = repo.head()?;
        reference.set_target(target_oid, "fast-forward merge")?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))?;

        return Ok(MergeResult {
            commit_sha: target_oid.to_string(),
            fast_forward: true,
        });
    }

    // Normal merge
    let mut merge_opts = MergeOptions::new();
    repo.merge(&[&annotated], Some(&mut merge_opts), None)?;

    // Check for conflicts
    let index = repo.index()?;
    if index.has_conflicts() {
        let conflict_count = index.conflicts()?.count();
        // Clean up merge state
        repo.cleanup_state()?;
        return Err(MergeError::Conflict(conflict_count));
    }

    // Create merge commit
    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;
    let sig = Signature::now("koompi-orch", "koompi-orch@local")?;
    let head_commit = repo.head()?.peel_to_commit()?;

    let message = format!("Merge branch '{}' into HEAD", source_branch);
    let merge_oid = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &message,
        &tree,
        &[&head_commit, &source_commit],
    )?;

    repo.cleanup_state()?;

    Ok(MergeResult {
        commit_sha: merge_oid.to_string(),
        fast_forward: false,
    })
}

/// Rebase the current branch onto a target branch.
///
/// Replays commits from the current branch on top of the target branch.
/// Returns the SHA of the final rebased commit.
pub fn rebase_branch(
    worktree_path: &Path,
    onto_branch: &str,
) -> Result<MergeResult, MergeError> {
    let repo = Repository::open(worktree_path)?;

    let onto_ref = repo
        .find_branch(onto_branch, git2::BranchType::Local)
        .map_err(|_| MergeError::BranchNotFound(onto_branch.to_string()))?;
    let onto_commit = onto_ref.get().peel_to_commit()?;
    let onto_annotated = repo.find_annotated_commit(onto_commit.id())?;

    let head_annotated = {
        let head = repo.head()?;
        let oid = head
            .target()
            .ok_or_else(|| git2::Error::from_str("HEAD has no target"))?;
        repo.find_annotated_commit(oid)?
    };

    let mut rebase = repo.rebase(
        Some(&head_annotated),
        Some(&onto_annotated),
        None,
        None,
    )?;

    let sig = Signature::now("koompi-orch", "koompi-orch@local")?;
    let mut last_oid = onto_commit.id();

    while let Some(op) = rebase.next() {
        let _op = op?;
        let index = repo.index()?;
        if index.has_conflicts() {
            let conflict_count = index.conflicts()?.count();
            rebase.abort()?;
            return Err(MergeError::Conflict(conflict_count));
        }
        last_oid = rebase.commit(None, &sig, None)?;
    }

    rebase.finish(None)?;

    Ok(MergeResult {
        commit_sha: last_oid.to_string(),
        fast_forward: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use std::fs;
    use tempfile::TempDir;

    /// Helper to commit all changes in a repo.
    fn commit_all(repo: &Repository, message: &str) -> git2::Oid {
        let sig = Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        if let Ok(head) = repo.head() {
            let parent = head.peel_to_commit().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
                .unwrap()
        } else {
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                .unwrap()
        }
    }

    /// Create a repo with initial commit, then create a feature branch.
    /// Returns the path to the repo dir. Use re-opened repos to avoid borrow issues.
    fn setup_branched_repo(dir: &Path) {
        let repo = Repository::init(dir).unwrap();
        fs::write(dir.join("base.txt"), "base content").unwrap();
        let main_oid = commit_all(&repo, "initial on main");
        let main_commit = repo.find_commit(main_oid).unwrap();
        repo.branch("feature", &main_commit, false).unwrap();
    }

    /// Get the default branch name.
    fn default_branch_name(dir: &Path) -> String {
        let repo = Repository::open(dir).unwrap();
        repo.head().unwrap().shorthand().unwrap().to_string()
    }

    #[test]
    fn test_merge_fast_forward() {
        let dir = TempDir::new().unwrap();
        setup_branched_repo(dir.path());
        let branch = default_branch_name(dir.path());

        // Add a commit on feature branch (single repo handle)
        let repo = Repository::open(dir.path()).unwrap();
        repo.set_head("refs/heads/feature").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        fs::write(dir.path().join("feature.txt"), "feature content").unwrap();
        commit_all(&repo, "feature commit");

        // Switch back to default branch
        repo.set_head(&format!("refs/heads/{}", branch)).unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        drop(repo);

        let result = merge_branch(dir.path(), "feature").unwrap();
        assert!(result.fast_forward);
        assert!(dir.path().join("feature.txt").exists());
    }

    #[test]
    fn test_merge_already_up_to_date() {
        let dir = TempDir::new().unwrap();
        setup_branched_repo(dir.path());

        // feature and main point to the same commit
        let result = merge_branch(dir.path(), "feature");
        assert!(matches!(result, Err(MergeError::AlreadyUpToDate)));
    }

    #[test]
    fn test_merge_branch_not_found() {
        let dir = TempDir::new().unwrap();
        setup_branched_repo(dir.path());

        let result = merge_branch(dir.path(), "nonexistent");
        assert!(matches!(result, Err(MergeError::BranchNotFound(_))));
    }

    #[test]
    fn test_merge_normal_commit() {
        let dir = TempDir::new().unwrap();
        setup_branched_repo(dir.path());
        let branch = default_branch_name(dir.path());

        let repo = Repository::open(dir.path()).unwrap();

        // Add commit on default branch
        fs::write(dir.path().join("main_file.txt"), "main work").unwrap();
        commit_all(&repo, "main commit");

        // Switch to feature branch and add commit
        repo.set_head("refs/heads/feature").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        fs::write(dir.path().join("feature_file.txt"), "feature work").unwrap();
        commit_all(&repo, "feature commit");

        // Switch back to default branch
        repo.set_head(&format!("refs/heads/{}", branch)).unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();
        drop(repo);

        let result = merge_branch(dir.path(), "feature").unwrap();
        assert!(!result.fast_forward);
        assert!(dir.path().join("main_file.txt").exists());
        assert!(dir.path().join("feature_file.txt").exists());
    }
}
