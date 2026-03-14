//! Diff generation via git2-rs.
//!
//! Produces structured diff output from worktrees: working directory changes,
//! staged changes, and diff between two commits.

use git2::{DiffOptions, Repository};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiffError {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("commit not found: {0}")]
    CommitNotFound(String),
}

/// A single file's diff information.
#[derive(Debug, Clone)]
pub struct FileDiff {
    /// File path relative to repo root
    pub path: String,
    /// Status: added, modified, deleted, renamed
    pub status: FileStatus,
    /// Unified diff patch text (may be empty for binary files)
    pub patch: String,
    /// Number of lines added
    pub additions: usize,
    /// Number of lines deleted
    pub deletions: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

/// Summary statistics for an entire diff.
#[derive(Debug, Clone)]
pub struct DiffSummary {
    pub files: Vec<FileDiff>,
    pub total_additions: usize,
    pub total_deletions: usize,
    pub files_changed: usize,
}

/// Generate a diff of uncommitted changes (working directory vs HEAD).
pub fn diff_working_dir(worktree_path: &Path) -> Result<DiffSummary, DiffError> {
    let repo = Repository::open(worktree_path)?;
    let head_tree = repo.head()?.peel_to_tree()?;

    let mut opts = DiffOptions::new();
    opts.include_untracked(true);

    // Diff HEAD tree against working directory (includes both staged and unstaged)
    let diff =
        repo.diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut opts))?;
    parse_diff(&diff)
}

/// Generate a diff of staged changes only (index vs HEAD).
pub fn diff_staged(worktree_path: &Path) -> Result<DiffSummary, DiffError> {
    let repo = Repository::open(worktree_path)?;
    let head_tree = repo.head()?.peel_to_tree()?;

    let diff = repo.diff_tree_to_index(Some(&head_tree), None, None)?;
    parse_diff(&diff)
}

/// Generate a diff between two commits.
pub fn diff_commits(
    worktree_path: &Path,
    old_sha: &str,
    new_sha: &str,
) -> Result<DiffSummary, DiffError> {
    let repo = Repository::open(worktree_path)?;

    let old_oid = git2::Oid::from_str(old_sha)
        .map_err(|_| DiffError::CommitNotFound(old_sha.to_string()))?;
    let new_oid = git2::Oid::from_str(new_sha)
        .map_err(|_| DiffError::CommitNotFound(new_sha.to_string()))?;

    let old_tree = repo
        .find_commit(old_oid)
        .map_err(|_| DiffError::CommitNotFound(old_sha.to_string()))?
        .tree()?;
    let new_tree = repo
        .find_commit(new_oid)
        .map_err(|_| DiffError::CommitNotFound(new_sha.to_string()))?
        .tree()?;

    let diff = repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)?;
    parse_diff(&diff)
}

/// Parse a git2 Diff into our DiffSummary structure.
fn parse_diff(diff: &git2::Diff) -> Result<DiffSummary, DiffError> {
    let stats = diff.stats()?;
    let mut files = Vec::new();

    // First pass: collect all file entries from deltas
    for delta_idx in 0..diff.deltas().len() {
        let delta = diff.get_delta(delta_idx).unwrap();
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .and_then(|p| p.to_str())
            .unwrap_or("<unknown>")
            .to_string();

        let status = match delta.status() {
            git2::Delta::Added | git2::Delta::Untracked => FileStatus::Added,
            git2::Delta::Deleted => FileStatus::Deleted,
            git2::Delta::Renamed => FileStatus::Renamed,
            _ => FileStatus::Modified,
        };

        files.push(FileDiff {
            path,
            status,
            patch: String::new(),
            additions: 0,
            deletions: 0,
        });
    }

    // Second pass: collect patch content
    diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .and_then(|p| p.to_str())
            .unwrap_or("<unknown>");

        if let Some(file_diff) = files.iter_mut().find(|f| f.path == path) {
            let content = std::str::from_utf8(line.content()).unwrap_or("");
            match line.origin() {
                '+' => {
                    file_diff.patch.push('+');
                    file_diff.patch.push_str(content);
                    file_diff.additions += 1;
                }
                '-' => {
                    file_diff.patch.push('-');
                    file_diff.patch.push_str(content);
                    file_diff.deletions += 1;
                }
                ' ' => {
                    file_diff.patch.push(' ');
                    file_diff.patch.push_str(content);
                }
                'H' => {
                    file_diff.patch.push_str(content);
                }
                _ => {}
            }
        }

        true
    })?;

    Ok(DiffSummary {
        total_additions: stats.insertions(),
        total_deletions: stats.deletions(),
        files_changed: stats.files_changed(),
        files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use std::fs;
    use tempfile::TempDir;

    fn create_repo_with_file(dir: &Path, filename: &str, content: &str) -> Repository {
        let repo = Repository::init(dir).unwrap();
        fs::write(dir.join(filename), content).unwrap();
        let sig = Signature::now("Test", "test@test.com").unwrap();
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
    fn test_diff_working_dir_no_changes() {
        let dir = TempDir::new().unwrap();
        create_repo_with_file(dir.path(), "file.txt", "content");

        let summary = diff_working_dir(dir.path()).unwrap();
        assert_eq!(summary.files_changed, 0);
        assert!(summary.files.is_empty());
    }

    #[test]
    fn test_diff_working_dir_with_modifications() {
        let dir = TempDir::new().unwrap();
        create_repo_with_file(dir.path(), "file.txt", "line1\nline2\n");

        // Modify the file
        fs::write(dir.path().join("file.txt"), "line1\nmodified\nline3\n").unwrap();

        let summary = diff_working_dir(dir.path()).unwrap();
        assert_eq!(summary.files_changed, 1);
        assert!(summary.total_additions > 0);
        assert!(summary.total_deletions > 0);
        assert_eq!(summary.files[0].path, "file.txt");
        assert_eq!(summary.files[0].status, FileStatus::Modified);
    }

    #[test]
    fn test_diff_working_dir_new_file() {
        let dir = TempDir::new().unwrap();
        create_repo_with_file(dir.path(), "existing.txt", "exists");

        // Add a new file
        fs::write(dir.path().join("new.txt"), "new content").unwrap();

        let summary = diff_working_dir(dir.path()).unwrap();
        assert!(summary
            .files
            .iter()
            .any(|f| f.path == "new.txt" && f.status == FileStatus::Added));
    }

    #[test]
    fn test_diff_working_dir_deleted_file() {
        let dir = TempDir::new().unwrap();
        create_repo_with_file(dir.path(), "to_delete.txt", "delete me");

        fs::remove_file(dir.path().join("to_delete.txt")).unwrap();

        let summary = diff_working_dir(dir.path()).unwrap();
        assert!(summary
            .files
            .iter()
            .any(|f| f.path == "to_delete.txt" && f.status == FileStatus::Deleted));
    }

    #[test]
    fn test_diff_commits() {
        let dir = TempDir::new().unwrap();
        let repo = create_repo_with_file(dir.path(), "file.txt", "version 1\n");
        let commit1 = repo.head().unwrap().target().unwrap();

        // Make a second commit
        fs::write(dir.path().join("file.txt"), "version 2\n").unwrap();
        let sig = Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo.find_commit(commit1).unwrap();
        let commit2 = repo
            .commit(Some("HEAD"), &sig, &sig, "v2", &tree, &[&parent])
            .unwrap();

        let summary =
            diff_commits(dir.path(), &commit1.to_string(), &commit2.to_string()).unwrap();

        assert_eq!(summary.files_changed, 1);
        assert_eq!(summary.files[0].path, "file.txt");
    }

    #[test]
    fn test_diff_commits_invalid_sha() {
        let dir = TempDir::new().unwrap();
        create_repo_with_file(dir.path(), "f.txt", "x");

        let result = diff_commits(dir.path(), "invalid", "alsobad");
        assert!(matches!(result, Err(DiffError::CommitNotFound(_))));
    }
}
