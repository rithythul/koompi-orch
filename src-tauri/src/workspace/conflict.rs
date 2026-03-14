//! Cross-workspace file conflict detection.
//!
//! Every 5 seconds, compares modified file lists across active worktrees
//! in the same repo. If any file path appears in 2+ worktrees, emits a
//! conflict warning. Uses git2-rs `statuses()` for ~5ms per worktree.

use git2::{Repository, StatusOptions};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConflictError {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// A warning about files modified in multiple worktrees simultaneously.
#[derive(Debug, Clone)]
pub struct ConflictWarning {
    /// File path (relative to repo root) that is modified in multiple worktrees
    pub file_path: String,
    /// Worktree paths that have this file modified
    pub worktree_paths: Vec<PathBuf>,
}

/// Detects file conflicts across worktrees in the same repository.
pub struct ConflictDetector;

impl ConflictDetector {
    /// Get the set of modified file paths (uncommitted changes) in a worktree.
    ///
    /// Includes staged and unstaged modifications, new files, and deletions.
    pub fn modified_files(worktree_path: &Path) -> Result<HashSet<String>, ConflictError> {
        let repo = Repository::open(worktree_path)?;
        let mut opts = StatusOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(true);

        let statuses = repo.statuses(Some(&mut opts))?;
        let mut files = HashSet::new();

        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                files.insert(path.to_string());
            }
        }

        Ok(files)
    }

    /// Check a group of worktrees (all belonging to the same repo) for conflicts.
    ///
    /// Returns a list of warnings for any file that is modified in 2+ worktrees.
    pub fn detect_conflicts(
        worktree_paths: &[PathBuf],
    ) -> Result<Vec<ConflictWarning>, ConflictError> {
        // Map: file_path -> list of worktree paths that modify it
        let mut file_to_worktrees: HashMap<String, Vec<PathBuf>> = HashMap::new();

        for wt_path in worktree_paths {
            let modified = Self::modified_files(wt_path)?;
            for file in modified {
                file_to_worktrees
                    .entry(file)
                    .or_default()
                    .push(wt_path.clone());
            }
        }

        // Filter to files appearing in 2+ worktrees
        let mut warnings: Vec<ConflictWarning> = file_to_worktrees
            .into_iter()
            .filter(|(_, wts)| wts.len() >= 2)
            .map(|(file_path, worktree_paths)| ConflictWarning {
                file_path,
                worktree_paths,
            })
            .collect();

        // Sort by file path for deterministic output
        warnings.sort_by(|a, b| a.file_path.cmp(&b.file_path));
        Ok(warnings)
    }
}

/// Background conflict detection loop. Runs every `interval` and calls
/// `on_conflict` when overlapping files are found.
///
/// `workspace_groups` returns current active worktrees grouped by repo name.
/// This function is meant to be spawned as a tokio task.
pub async fn conflict_detection_loop<F, G>(
    interval: std::time::Duration,
    mut workspace_groups: G,
    mut on_conflict: F,
) where
    F: FnMut(Vec<ConflictWarning>) + Send,
    G: FnMut() -> Vec<Vec<PathBuf>> + Send,
{
    loop {
        tokio::time::sleep(interval).await;

        let groups = workspace_groups();
        for group in groups {
            if group.len() < 2 {
                continue;
            }
            match ConflictDetector::detect_conflicts(&group) {
                Ok(warnings) if !warnings.is_empty() => {
                    on_conflict(warnings);
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("Conflict detection error: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use std::fs;
    use tempfile::TempDir;

    /// Create a test repo with an initial commit, then create a worktree.
    fn setup_repo_with_worktree(
        repo_dir: &Path,
        worktree_dir: &Path,
        branch: &str,
    ) -> PathBuf {
        let repo = if repo_dir.join(".git").exists() {
            Repository::open(repo_dir).unwrap()
        } else {
            let repo = Repository::init(repo_dir).unwrap();
            let sig = Signature::now("Test", "test@test.com").unwrap();
            fs::write(repo_dir.join("README.md"), "# test").unwrap();
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
        };

        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let branch_ref = repo
            .branch(branch, &head, false)
            .unwrap()
            .into_reference();

        let wt_path = worktree_dir.join(branch);
        let mut opts = git2::WorktreeAddOptions::new();
        opts.reference(Some(&branch_ref));
        repo.worktree(branch, &wt_path, Some(&opts)).unwrap();

        wt_path
    }

    #[test]
    fn test_modified_files_empty() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = Signature::now("Test", "test@test.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        let files = ConflictDetector::modified_files(dir.path()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_modified_files_with_changes() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = Signature::now("Test", "test@test.com").unwrap();

        // Initial commit with one file
        fs::write(dir.path().join("existing.txt"), "original").unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        // Modify existing and add new
        fs::write(dir.path().join("existing.txt"), "modified").unwrap();
        fs::write(dir.path().join("new_file.txt"), "new content").unwrap();

        let files = ConflictDetector::modified_files(dir.path()).unwrap();
        assert!(files.contains("existing.txt"));
        assert!(files.contains("new_file.txt"));
    }

    #[test]
    fn test_detect_no_conflicts() {
        let repo_dir = TempDir::new().unwrap();
        let wt_base = TempDir::new().unwrap();

        let wt1 = setup_repo_with_worktree(repo_dir.path(), wt_base.path(), "branch-a");
        let wt2 = setup_repo_with_worktree(repo_dir.path(), wt_base.path(), "branch-b");

        // Different files modified in each worktree
        fs::write(wt1.join("file_a.txt"), "content a").unwrap();
        fs::write(wt2.join("file_b.txt"), "content b").unwrap();

        let warnings = ConflictDetector::detect_conflicts(&[wt1, wt2]).unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_detect_conflicts() {
        let repo_dir = TempDir::new().unwrap();
        let wt_base = TempDir::new().unwrap();

        let wt1 = setup_repo_with_worktree(repo_dir.path(), wt_base.path(), "branch-c");
        let wt2 = setup_repo_with_worktree(repo_dir.path(), wt_base.path(), "branch-d");

        // Same file modified in both worktrees
        fs::write(wt1.join("shared.rs"), "impl A {}").unwrap();
        fs::write(wt2.join("shared.rs"), "impl B {}").unwrap();

        let warnings = ConflictDetector::detect_conflicts(&[wt1, wt2]).unwrap();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].file_path, "shared.rs");
        assert_eq!(warnings[0].worktree_paths.len(), 2);
    }

    #[test]
    fn test_detect_multiple_conflicts() {
        let repo_dir = TempDir::new().unwrap();
        let wt_base = TempDir::new().unwrap();

        let wt1 = setup_repo_with_worktree(repo_dir.path(), wt_base.path(), "branch-e");
        let wt2 = setup_repo_with_worktree(repo_dir.path(), wt_base.path(), "branch-f");

        // Two files modified in both
        fs::write(wt1.join("api.rs"), "v1").unwrap();
        fs::write(wt1.join("model.rs"), "v1").unwrap();
        fs::write(wt2.join("api.rs"), "v2").unwrap();
        fs::write(wt2.join("model.rs"), "v2").unwrap();
        // One file only in wt1 (no conflict)
        fs::write(wt1.join("unique.rs"), "unique").unwrap();

        let warnings = ConflictDetector::detect_conflicts(&[wt1, wt2]).unwrap();
        assert_eq!(warnings.len(), 2);
        // Sorted alphabetically
        assert_eq!(warnings[0].file_path, "api.rs");
        assert_eq!(warnings[1].file_path, "model.rs");
    }
}
