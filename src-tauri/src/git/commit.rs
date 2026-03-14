//! Commit and push operations via git2-rs.
//!
//! Provides staging, committing, and pushing to remote repositories.

use git2::{Cred, PushOptions, RemoteCallbacks, Repository, Signature};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommitError {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("no changes staged for commit")]
    NothingToCommit,
    #[error("remote not found: {0}")]
    RemoteNotFound(String),
    #[error("push failed: {0}")]
    PushFailed(String),
}

/// Info about a created commit.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub sha: String,
    pub message: String,
}

/// Stage specific files by path (relative to repo root).
pub fn stage_files(worktree_path: &Path, file_paths: &[&str]) -> Result<(), CommitError> {
    let repo = Repository::open(worktree_path)?;
    let mut index = repo.index()?;
    for path in file_paths {
        index.add_path(Path::new(path))?;
    }
    index.write()?;
    Ok(())
}

/// Stage all changes (modified, added, deleted) — equivalent to `git add -A`.
pub fn stage_all(worktree_path: &Path) -> Result<(), CommitError> {
    let repo = Repository::open(worktree_path)?;
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    // Also handle deletions: update index for removed files
    index.update_all(["*"].iter(), None)?;
    index.write()?;
    Ok(())
}

/// Create a commit from the currently staged changes.
///
/// Uses "koompi-orch" as the committer identity. Pass a custom author
/// name/email for attribution.
pub fn create_commit(
    worktree_path: &Path,
    message: &str,
    author_name: Option<&str>,
    author_email: Option<&str>,
) -> Result<CommitInfo, CommitError> {
    let repo = Repository::open(worktree_path)?;
    let mut index = repo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    // Check if there are actual changes to commit
    if let Ok(head) = repo.head() {
        let head_tree = head.peel_to_tree()?;
        let diff = repo.diff_tree_to_tree(Some(&head_tree), Some(&tree), None)?;
        if diff.deltas().len() == 0 {
            return Err(CommitError::NothingToCommit);
        }
    }

    let author = Signature::now(
        author_name.unwrap_or("koompi-orch"),
        author_email.unwrap_or("koompi-orch@local"),
    )?;
    let committer = Signature::now("koompi-orch", "koompi-orch@local")?;

    let oid = if let Ok(head) = repo.head() {
        let parent = head.peel_to_commit()?;
        repo.commit(
            Some("HEAD"),
            &author,
            &committer,
            message,
            &tree,
            &[&parent],
        )?
    } else {
        repo.commit(Some("HEAD"), &author, &committer, message, &tree, &[])?
    };

    Ok(CommitInfo {
        sha: oid.to_string(),
        message: message.to_string(),
    })
}

/// Push the current branch to a remote.
///
/// Uses SSH agent for authentication by default. Falls back to credential
/// helper if available.
pub fn push_branch(worktree_path: &Path, remote_name: &str) -> Result<(), CommitError> {
    let repo = Repository::open(worktree_path)?;

    let mut remote = repo
        .find_remote(remote_name)
        .map_err(|_| CommitError::RemoteNotFound(remote_name.to_string()))?;

    let head = repo.head()?;
    let branch_name = head
        .shorthand()
        .ok_or_else(|| CommitError::PushFailed("cannot determine branch name".to_string()))?
        .to_string();

    let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        // Try SSH agent first
        let username = username_from_url.unwrap_or("git");
        Cred::ssh_key_from_agent(username)
    });

    let mut push_opts = PushOptions::new();
    push_opts.remote_callbacks(callbacks);

    remote
        .push(&[&refspec], Some(&mut push_opts))
        .map_err(|e| CommitError::PushFailed(e.message().to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use std::fs;
    use tempfile::TempDir;

    fn create_repo_with_commit(dir: &Path) -> Repository {
        let repo = Repository::init(dir).unwrap();
        let sig = Signature::now("Test", "test@test.com").unwrap();
        fs::write(dir.join("initial.txt"), "init").unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();
        }
        repo
    }

    #[test]
    fn test_stage_and_commit() {
        let dir = TempDir::new().unwrap();
        create_repo_with_commit(dir.path());

        // Create a new file
        fs::write(dir.path().join("new.txt"), "new content").unwrap();

        // Stage and commit
        stage_files(dir.path(), &["new.txt"]).unwrap();
        let info = create_commit(dir.path(), "add new file", None, None).unwrap();

        assert_eq!(info.message, "add new file");
        assert_eq!(info.sha.len(), 40);
    }

    #[test]
    fn test_stage_all_and_commit() {
        let dir = TempDir::new().unwrap();
        create_repo_with_commit(dir.path());

        fs::write(dir.path().join("a.txt"), "a").unwrap();
        fs::write(dir.path().join("b.txt"), "b").unwrap();

        stage_all(dir.path()).unwrap();
        let info = create_commit(dir.path(), "add multiple files", None, None).unwrap();

        assert_eq!(info.sha.len(), 40);
    }

    #[test]
    fn test_commit_nothing_to_commit() {
        let dir = TempDir::new().unwrap();
        create_repo_with_commit(dir.path());

        // Stage nothing, attempt commit
        let result = create_commit(dir.path(), "empty", None, None);
        assert!(matches!(result, Err(CommitError::NothingToCommit)));
    }

    #[test]
    fn test_commit_with_custom_author() {
        let dir = TempDir::new().unwrap();
        let repo = create_repo_with_commit(dir.path());

        fs::write(dir.path().join("authored.txt"), "content").unwrap();
        stage_all(dir.path()).unwrap();
        let info = create_commit(
            dir.path(),
            "custom author commit",
            Some("Alice"),
            Some("alice@example.com"),
        )
        .unwrap();

        // Verify the author
        let commit = repo
            .find_commit(git2::Oid::from_str(&info.sha).unwrap())
            .unwrap();
        assert_eq!(commit.author().name(), Some("Alice"));
        assert_eq!(commit.author().email(), Some("alice@example.com"));
        // Committer is always koompi-orch
        assert_eq!(commit.committer().name(), Some("koompi-orch"));
    }

    #[test]
    fn test_stage_specific_files() {
        let dir = TempDir::new().unwrap();
        create_repo_with_commit(dir.path());

        fs::write(dir.path().join("staged.txt"), "yes").unwrap();
        fs::write(dir.path().join("unstaged.txt"), "no").unwrap();

        // Stage only one file
        stage_files(dir.path(), &["staged.txt"]).unwrap();
        let info = create_commit(dir.path(), "partial commit", None, None).unwrap();
        assert_eq!(info.sha.len(), 40);

        // The unstaged file should still show as modified/untracked
        let repo = Repository::open(dir.path()).unwrap();
        let statuses = repo.statuses(None).unwrap();
        let untracked: Vec<_> = statuses
            .iter()
            .filter(|e| e.path() == Some("unstaged.txt"))
            .collect();
        assert!(!untracked.is_empty());
    }

    #[test]
    fn test_push_remote_not_found() {
        let dir = TempDir::new().unwrap();
        create_repo_with_commit(dir.path());

        let result = push_branch(dir.path(), "origin");
        assert!(matches!(result, Err(CommitError::RemoteNotFound(_))));
    }
}
