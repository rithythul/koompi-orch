# Plan 3: Workspace and Git Operations

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the workspace lifecycle (worktree management, snapshots, conflict detection, locking, kanban status) and git operations (diff, merge, commit, push, PR creation) modules in the Rust backend.

**Architecture:** The `workspace/` module manages git worktrees via git2-rs, tracks state in SurrealDB, and provides conflict detection across parallel workspaces. The `git/` module wraps git2-rs for diff/merge/commit/push and octocrab for GitHub PR creation. All modules are testable against real temporary git repos created with git2-rs and tempdir.

**Tech Stack:** Rust, git2-rs (libgit2 bindings), octocrab (GitHub API), SurrealDB, tokio, serde, thiserror, tempfile (tests)

**Spec Reference:** `/home/userx/projects/koompi-orch/docs/superpowers/specs/2026-03-14-koompi-orch-design.md` — Sections 6, 7, 7.1-7.7

---

## Chunk 1: Workspace Manager

### Task 1: Create workspace/manager.rs — Worktree lifecycle

**Files:**
- Create: `~/projects/koompi-orch/src-tauri/src/workspace/mod.rs`
- Create: `~/projects/koompi-orch/src-tauri/src/workspace/manager.rs`
- Modify: `~/projects/koompi-orch/src-tauri/src/lib.rs`

- [ ] **Step 1: Write workspace/manager.rs with tests**

Create `src-tauri/src/workspace/manager.rs`:
```rust
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
        let repo = Repository::open(repo_path).map_err(|_| {
            WorktreeError::RepoNotFound(repo_path.display().to_string())
        })?;

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
                repo.branch(branch, &head_commit, false)?
                    .into_reference()
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
        let repo = Repository::open(repo_path).map_err(|_| {
            WorktreeError::RepoNotFound(repo_path.display().to_string())
        })?;

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
        // Create an initial commit (worktrees require at least one commit)
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
            .unwrap();
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
```

- [ ] **Step 2: Create workspace/mod.rs**

Create `src-tauri/src/workspace/mod.rs`:
```rust
pub mod manager;
pub mod snapshot;
pub mod conflict;
pub mod lock;
pub mod status;

pub use manager::{WorktreeManager, WorktreeInfo, WorktreeError};
pub use snapshot::{SnapshotManager, SnapshotError};
pub use conflict::{ConflictDetector, ConflictWarning};
pub use lock::{WorkspaceLock, LockError};
pub use status::{WorkspaceStatus, StatusMachine, StatusError};
```

- [ ] **Step 3: Wire workspace module into lib.rs**

Add to `src-tauri/src/lib.rs`:
```rust
pub mod workspace;
```

- [ ] **Step 4: Run tests**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo test workspace::manager
```

Expected: All 5 tests pass.

- [ ] **Step 5: Commit**

```bash
cd ~/projects/koompi-orch
git add src-tauri/src/workspace/ src-tauri/src/lib.rs
git commit -m "feat: add workspace manager with git worktree lifecycle"
```

---

## Chunk 2: Workspace Snapshots

### Task 2: Create workspace/snapshot.rs — Checkpoint and revert

**Files:**
- Create: `~/projects/koompi-orch/src-tauri/src/workspace/snapshot.rs`

- [ ] **Step 1: Write workspace/snapshot.rs with tests**

Create `src-tauri/src/workspace/snapshot.rs`:
```rust
//! Checkpoint/revert to any agent turn using git commits.
//!
//! Each checkpoint is a commit with a structured message containing
//! the turn number and description. Reverting checks out a specific
//! commit SHA in the worktree.

use git2::{Repository, Signature, ResetType};
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
    pub fn revert_to(
        worktree_path: &Path,
        commit_sha: &str,
    ) -> Result<(), SnapshotError> {
        let repo = Repository::open(worktree_path)?;
        let oid = git2::Oid::from_str(commit_sha)
            .map_err(|_| SnapshotError::CommitNotFound(commit_sha.to_string()))?;
        let commit = repo.find_commit(oid)
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
        let head_oid = head.target().ok_or_else(|| {
            SnapshotError::Git(git2::Error::from_str("HEAD has no target"))
        })?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push(head_oid)?;
        revwalk.set_sorting(git2::Sort::TIME)?;

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
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
        (dir, repo)
    }

    #[test]
    fn test_create_checkpoint() {
        let (dir, _repo) = setup_repo();

        // Create a file so there are changes to commit
        fs::write(dir.path().join("hello.txt"), "hello world").unwrap();

        let info = SnapshotManager::create_checkpoint(
            dir.path(),
            1,
            "implemented auth module",
        )
        .unwrap();

        assert_eq!(info.turn_number, 1);
        assert_eq!(info.description, "implemented auth module");
        assert_eq!(info.commit_sha.len(), 40); // full SHA
    }

    #[test]
    fn test_nothing_to_commit() {
        let (dir, _repo) = setup_repo();

        let result = SnapshotManager::create_checkpoint(
            dir.path(),
            1,
            "no changes",
        );

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
        assert_eq!(fs::read_to_string(dir.path().join("file.txt")).unwrap(), "version 2");

        // Revert to turn 1
        SnapshotManager::revert_to(dir.path(), &snap1.commit_sha).unwrap();

        // Verify content is back to v1
        assert_eq!(fs::read_to_string(dir.path().join("file.txt")).unwrap(), "version 1");
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
        let result = SnapshotManager::revert_to(dir.path(), "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
        assert!(matches!(result, Err(SnapshotError::CommitNotFound(_))));
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo test workspace::snapshot
```

Expected: All 5 tests pass.

- [ ] **Step 3: Commit**

```bash
cd ~/projects/koompi-orch
git add src-tauri/src/workspace/snapshot.rs
git commit -m "feat: add workspace snapshots with checkpoint/revert via git commits"
```

---

## Chunk 3: Conflict Detection

### Task 3: Create workspace/conflict.rs — Cross-workspace conflict detection

**Files:**
- Create: `~/projects/koompi-orch/src-tauri/src/workspace/conflict.rs`

- [ ] **Step 1: Write workspace/conflict.rs with tests**

Create `src-tauri/src/workspace/conflict.rs`:
```rust
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
        opts.include_untracked(true)
            .recurse_untracked_dirs(true);

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
            // Create a file and initial commit
            fs::write(repo_dir.join("README.md"), "# test").unwrap();
            let mut index = repo.index().unwrap();
            index
                .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
                .unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();
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

        let warnings =
            ConflictDetector::detect_conflicts(&[wt1, wt2]).unwrap();
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

        let warnings =
            ConflictDetector::detect_conflicts(&[wt1, wt2]).unwrap();
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

        let warnings =
            ConflictDetector::detect_conflicts(&[wt1, wt2]).unwrap();
        assert_eq!(warnings.len(), 2);
        // Sorted alphabetically
        assert_eq!(warnings[0].file_path, "api.rs");
        assert_eq!(warnings[1].file_path, "model.rs");
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo test workspace::conflict
```

Expected: All 5 tests pass.

- [ ] **Step 3: Commit**

```bash
cd ~/projects/koompi-orch
git add src-tauri/src/workspace/conflict.rs
git commit -m "feat: add cross-workspace file conflict detection"
```

---

## Chunk 4: Workspace Lock and Status Machine

### Task 4: Create workspace/lock.rs — Workspace mutual exclusion

**Files:**
- Create: `~/projects/koompi-orch/src-tauri/src/workspace/lock.rs`

- [ ] **Step 1: Write workspace/lock.rs with tests**

Create `src-tauri/src/workspace/lock.rs`:
```rust
//! Workspace mutual exclusion via SurrealDB locked_by field.
//!
//! Each workspace has a `locked_by` field pointing to the active session record.
//! Before spawning an agent, check the lock:
//! - If locked_by = NONE: acquire, proceed
//! - If locked by a running session: reject
//! - If locked by a dead session: clear stale lock, then acquire
//!
//! See spec Section 7.6.

use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LockError {
    #[error("database error: {0}")]
    Db(#[from] surrealdb::Error),
    #[error("workspace {workspace_id} is locked by session {session_id}")]
    AlreadyLocked {
        workspace_id: String,
        session_id: String,
    },
    #[error("workspace not found: {0}")]
    WorkspaceNotFound(String),
}

/// Result of a lock acquisition attempt
#[derive(Debug, Clone, PartialEq)]
pub enum LockResult {
    /// Lock acquired successfully
    Acquired,
    /// Stale lock from a dead session was cleared, then lock acquired
    StaleLockCleared { old_session_id: String },
}

/// Manages workspace mutual exclusion locks stored in SurrealDB.
pub struct WorkspaceLock<'a> {
    db: &'a Surreal<Db>,
}

impl<'a> WorkspaceLock<'a> {
    pub fn new(db: &'a Surreal<Db>) -> Self {
        Self { db }
    }

    /// Try to acquire a lock on a workspace for the given session.
    ///
    /// - If unlocked: sets locked_by to the session record and returns Acquired.
    /// - If locked by a running session: returns AlreadyLocked error.
    /// - If locked by a non-running session (crashed/completed): clears the stale
    ///   lock, acquires, and returns StaleLockCleared.
    pub async fn acquire(
        &self,
        workspace_id: &str,
        session_id: &str,
    ) -> Result<LockResult, LockError> {
        // Fetch the workspace's current locked_by value
        let result: Option<serde_json::Value> = self
            .db
            .query(
                "SELECT locked_by FROM type::thing('workspace', $ws_id)"
            )
            .bind(("ws_id", workspace_id))
            .await?
            .take(0)?;

        let locked_by = result
            .as_ref()
            .and_then(|v| v.get("locked_by"))
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_str().map(|s| s.to_string())
                }
            });

        match locked_by {
            None => {
                // Unlocked — acquire
                self.set_lock(workspace_id, session_id).await?;
                Ok(LockResult::Acquired)
            }
            Some(existing_session_id) => {
                // Check if the holding session is still running
                let session_status: Option<serde_json::Value> = self
                    .db
                    .query(
                        "SELECT status FROM type::thing($sid)"
                    )
                    .bind(("sid", &existing_session_id))
                    .await?
                    .take(0)?;

                let is_running = session_status
                    .as_ref()
                    .and_then(|v| v.get("status"))
                    .and_then(|v| v.as_str())
                    .map(|s| s == "running")
                    .unwrap_or(false);

                if is_running {
                    Err(LockError::AlreadyLocked {
                        workspace_id: workspace_id.to_string(),
                        session_id: existing_session_id,
                    })
                } else {
                    // Stale lock — clear and acquire
                    let old_id = existing_session_id.clone();
                    self.set_lock(workspace_id, session_id).await?;
                    Ok(LockResult::StaleLockCleared {
                        old_session_id: old_id,
                    })
                }
            }
        }
    }

    /// Release the lock on a workspace (set locked_by = NONE).
    pub async fn release(&self, workspace_id: &str) -> Result<(), LockError> {
        self.db
            .query(
                "UPDATE type::thing('workspace', $ws_id) SET locked_by = NONE, updated_at = time::now()"
            )
            .bind(("ws_id", workspace_id))
            .await?;
        Ok(())
    }

    /// Check if a workspace is currently locked.
    pub async fn is_locked(&self, workspace_id: &str) -> Result<Option<String>, LockError> {
        let result: Option<serde_json::Value> = self
            .db
            .query(
                "SELECT locked_by FROM type::thing('workspace', $ws_id)"
            )
            .bind(("ws_id", workspace_id))
            .await?
            .take(0)?;

        let locked_by = result
            .as_ref()
            .and_then(|v| v.get("locked_by"))
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_str().map(|s| s.to_string())
                }
            });

        Ok(locked_by)
    }

    /// Internal: set the locked_by field.
    async fn set_lock(
        &self,
        workspace_id: &str,
        session_id: &str,
    ) -> Result<(), LockError> {
        self.db
            .query(
                "UPDATE type::thing('workspace', $ws_id) SET locked_by = type::thing($sid), updated_at = time::now()"
            )
            .bind(("ws_id", workspace_id))
            .bind(("sid", session_id))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use surrealdb::engine::local::Mem;

    async fn setup_db() -> Surreal<Db> {
        let db = Surreal::new::<Mem>(()).await.unwrap();
        db.use_ns("test").use_db("test").await.unwrap();

        // Apply just the tables we need for testing
        db.query(
            "DEFINE TABLE workspace SCHEMAFULL;
             DEFINE FIELD name ON workspace TYPE string;
             DEFINE FIELD branch ON workspace TYPE string;
             DEFINE FIELD worktree_path ON workspace TYPE string;
             DEFINE FIELD status ON workspace TYPE string;
             DEFINE FIELD locked_by ON workspace TYPE option<record<session>>;
             DEFINE FIELD created_at ON workspace TYPE datetime DEFAULT time::now();
             DEFINE FIELD updated_at ON workspace TYPE datetime DEFAULT time::now();

             DEFINE TABLE session SCHEMAFULL;
             DEFINE FIELD agent_type ON session TYPE string;
             DEFINE FIELD status ON session TYPE string;
             DEFINE FIELD started_at ON session TYPE datetime DEFAULT time::now();
             DEFINE FIELD config ON session TYPE object;",
        )
        .await
        .unwrap();

        db
    }

    async fn create_workspace(db: &Surreal<Db>, id: &str) {
        db.query(&format!(
            "CREATE workspace:{} SET name = 'test', branch = 'main', \
             worktree_path = '/tmp/test', status = 'active'",
            id
        ))
        .await
        .unwrap();
    }

    async fn create_session(db: &Surreal<Db>, id: &str, status: &str) {
        db.query(&format!(
            "CREATE session:{} SET agent_type = 'claude-code', status = '{}', config = {{}}",
            id, status
        ))
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_acquire_unlocked() {
        let db = setup_db().await;
        create_workspace(&db, "ws1").await;
        create_session(&db, "s1", "running").await;

        let lock = WorkspaceLock::new(&db);
        let result = lock.acquire("ws1", "session:s1").await.unwrap();
        assert_eq!(result, LockResult::Acquired);
    }

    #[tokio::test]
    async fn test_acquire_already_locked() {
        let db = setup_db().await;
        create_workspace(&db, "ws2").await;
        create_session(&db, "s2", "running").await;
        create_session(&db, "s3", "running").await;

        let lock = WorkspaceLock::new(&db);
        lock.acquire("ws2", "session:s2").await.unwrap();

        let result = lock.acquire("ws2", "session:s3").await;
        assert!(matches!(result, Err(LockError::AlreadyLocked { .. })));
    }

    #[tokio::test]
    async fn test_acquire_stale_lock() {
        let db = setup_db().await;
        create_workspace(&db, "ws3").await;
        create_session(&db, "s4", "crashed").await;
        create_session(&db, "s5", "running").await;

        let lock = WorkspaceLock::new(&db);
        // First acquire by a session that later crashed
        lock.acquire("ws3", "session:s4").await.unwrap();

        // Update session status to crashed
        db.query("UPDATE session:s4 SET status = 'crashed'")
            .await
            .unwrap();

        // New session should clear stale lock
        let result = lock.acquire("ws3", "session:s5").await.unwrap();
        assert!(matches!(result, LockResult::StaleLockCleared { .. }));
    }

    #[tokio::test]
    async fn test_release_lock() {
        let db = setup_db().await;
        create_workspace(&db, "ws4").await;
        create_session(&db, "s6", "running").await;

        let lock = WorkspaceLock::new(&db);
        lock.acquire("ws4", "session:s6").await.unwrap();

        assert!(lock.is_locked("ws4").await.unwrap().is_some());

        lock.release("ws4").await.unwrap();
        assert!(lock.is_locked("ws4").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_is_locked() {
        let db = setup_db().await;
        create_workspace(&db, "ws5").await;
        create_session(&db, "s7", "running").await;

        let lock = WorkspaceLock::new(&db);
        assert!(lock.is_locked("ws5").await.unwrap().is_none());

        lock.acquire("ws5", "session:s7").await.unwrap();
        assert!(lock.is_locked("ws5").await.unwrap().is_some());
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo test workspace::lock
```

Expected: All 5 tests pass.

- [ ] **Step 3: Commit**

```bash
cd ~/projects/koompi-orch
git add src-tauri/src/workspace/lock.rs
git commit -m "feat: add workspace mutual exclusion locks via SurrealDB"
```

---

### Task 5: Create workspace/status.rs — Kanban state machine

**Files:**
- Create: `~/projects/koompi-orch/src-tauri/src/workspace/status.rs`

- [ ] **Step 1: Write workspace/status.rs with tests**

Create `src-tauri/src/workspace/status.rs`:
```rust
//! Kanban state machine for workspace status transitions.
//!
//! Valid transitions (from spec Section 7.7):
//! - backlog -> active
//! - active -> review
//! - active -> failed
//! - review -> done
//! - review -> active
//! - failed -> active
//! - done -> active
//! - any -> backlog

use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkspaceStatus {
    Backlog,
    Active,
    Review,
    Done,
    Failed,
}

impl WorkspaceStatus {
    /// Parse a status string into the enum.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "backlog" => Some(Self::Backlog),
            "active" => Some(Self::Active),
            "review" => Some(Self::Review),
            "done" => Some(Self::Done),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }

    /// Convert to the database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Backlog => "backlog",
            Self::Active => "active",
            Self::Review => "review",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }

    /// Check if transitioning from `self` to `target` is valid.
    pub fn can_transition_to(&self, target: &WorkspaceStatus) -> bool {
        // Any state -> backlog is always valid (manual move back)
        if *target == WorkspaceStatus::Backlog {
            return true;
        }

        matches!(
            (self, target),
            (Self::Backlog, Self::Active)
                | (Self::Active, Self::Review)
                | (Self::Active, Self::Failed)
                | (Self::Review, Self::Done)
                | (Self::Review, Self::Active)
                | (Self::Failed, Self::Active)
                | (Self::Done, Self::Active)
        )
    }

    /// Return all valid target states from the current status.
    pub fn valid_transitions(&self) -> Vec<WorkspaceStatus> {
        let all = [
            Self::Backlog,
            Self::Active,
            Self::Review,
            Self::Done,
            Self::Failed,
        ];
        all.iter()
            .filter(|target| self.can_transition_to(target))
            .copied()
            .collect()
    }
}

#[derive(Error, Debug)]
pub enum StatusError {
    #[error("database error: {0}")]
    Db(#[from] surrealdb::Error),
    #[error("invalid status transition from '{from}' to '{to}'")]
    InvalidTransition { from: String, to: String },
    #[error("workspace not found: {0}")]
    WorkspaceNotFound(String),
    #[error("unknown status value: {0}")]
    UnknownStatus(String),
}

/// Applies the state machine to workspace status transitions in SurrealDB.
pub struct StatusMachine<'a> {
    db: &'a Surreal<Db>,
}

impl<'a> StatusMachine<'a> {
    pub fn new(db: &'a Surreal<Db>) -> Self {
        Self { db }
    }

    /// Transition a workspace from its current status to a new status.
    ///
    /// Validates the transition against the state machine rules before applying.
    pub async fn transition(
        &self,
        workspace_id: &str,
        target: WorkspaceStatus,
    ) -> Result<(), StatusError> {
        // Get current status
        let result: Option<serde_json::Value> = self
            .db
            .query("SELECT status FROM type::thing('workspace', $ws_id)")
            .bind(("ws_id", workspace_id))
            .await?
            .take(0)?;

        let current_str = result
            .as_ref()
            .and_then(|v| v.get("status"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| StatusError::WorkspaceNotFound(workspace_id.to_string()))?;

        let current = WorkspaceStatus::from_str(current_str)
            .ok_or_else(|| StatusError::UnknownStatus(current_str.to_string()))?;

        if !current.can_transition_to(&target) {
            return Err(StatusError::InvalidTransition {
                from: current.as_str().to_string(),
                to: target.as_str().to_string(),
            });
        }

        self.db
            .query(
                "UPDATE type::thing('workspace', $ws_id) SET status = $status, updated_at = time::now()"
            )
            .bind(("ws_id", workspace_id))
            .bind(("status", target.as_str()))
            .await?;

        Ok(())
    }

    /// Get the current status of a workspace.
    pub async fn get_status(
        &self,
        workspace_id: &str,
    ) -> Result<WorkspaceStatus, StatusError> {
        let result: Option<serde_json::Value> = self
            .db
            .query("SELECT status FROM type::thing('workspace', $ws_id)")
            .bind(("ws_id", workspace_id))
            .await?
            .take(0)?;

        let status_str = result
            .as_ref()
            .and_then(|v| v.get("status"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| StatusError::WorkspaceNotFound(workspace_id.to_string()))?;

        WorkspaceStatus::from_str(status_str)
            .ok_or_else(|| StatusError::UnknownStatus(status_str.to_string()))
    }
}

impl std::fmt::Display for WorkspaceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use surrealdb::engine::local::Mem;

    // --- Pure state machine tests (no DB) ---

    #[test]
    fn test_backlog_to_active() {
        assert!(WorkspaceStatus::Backlog.can_transition_to(&WorkspaceStatus::Active));
    }

    #[test]
    fn test_active_to_review() {
        assert!(WorkspaceStatus::Active.can_transition_to(&WorkspaceStatus::Review));
    }

    #[test]
    fn test_active_to_failed() {
        assert!(WorkspaceStatus::Active.can_transition_to(&WorkspaceStatus::Failed));
    }

    #[test]
    fn test_review_to_done() {
        assert!(WorkspaceStatus::Review.can_transition_to(&WorkspaceStatus::Done));
    }

    #[test]
    fn test_review_to_active() {
        assert!(WorkspaceStatus::Review.can_transition_to(&WorkspaceStatus::Active));
    }

    #[test]
    fn test_failed_to_active() {
        assert!(WorkspaceStatus::Failed.can_transition_to(&WorkspaceStatus::Active));
    }

    #[test]
    fn test_done_to_active() {
        assert!(WorkspaceStatus::Done.can_transition_to(&WorkspaceStatus::Active));
    }

    #[test]
    fn test_any_to_backlog() {
        for status in [
            WorkspaceStatus::Active,
            WorkspaceStatus::Review,
            WorkspaceStatus::Done,
            WorkspaceStatus::Failed,
        ] {
            assert!(
                status.can_transition_to(&WorkspaceStatus::Backlog),
                "{} should transition to backlog",
                status
            );
        }
    }

    #[test]
    fn test_invalid_backlog_to_review() {
        assert!(!WorkspaceStatus::Backlog.can_transition_to(&WorkspaceStatus::Review));
    }

    #[test]
    fn test_invalid_backlog_to_done() {
        assert!(!WorkspaceStatus::Backlog.can_transition_to(&WorkspaceStatus::Done));
    }

    #[test]
    fn test_invalid_backlog_to_failed() {
        assert!(!WorkspaceStatus::Backlog.can_transition_to(&WorkspaceStatus::Failed));
    }

    #[test]
    fn test_invalid_done_to_review() {
        assert!(!WorkspaceStatus::Done.can_transition_to(&WorkspaceStatus::Review));
    }

    #[test]
    fn test_invalid_failed_to_done() {
        assert!(!WorkspaceStatus::Failed.can_transition_to(&WorkspaceStatus::Done));
    }

    #[test]
    fn test_valid_transitions_from_active() {
        let transitions = WorkspaceStatus::Active.valid_transitions();
        assert!(transitions.contains(&WorkspaceStatus::Backlog));
        assert!(transitions.contains(&WorkspaceStatus::Review));
        assert!(transitions.contains(&WorkspaceStatus::Failed));
        assert!(!transitions.contains(&WorkspaceStatus::Done));
    }

    #[test]
    fn test_status_roundtrip() {
        for status in [
            WorkspaceStatus::Backlog,
            WorkspaceStatus::Active,
            WorkspaceStatus::Review,
            WorkspaceStatus::Done,
            WorkspaceStatus::Failed,
        ] {
            assert_eq!(WorkspaceStatus::from_str(status.as_str()), Some(status));
        }
    }

    // --- DB-backed state machine tests ---

    async fn setup_db() -> Surreal<Db> {
        let db = Surreal::new::<Mem>(()).await.unwrap();
        db.use_ns("test").use_db("test").await.unwrap();
        db.query(
            "DEFINE TABLE workspace SCHEMAFULL;
             DEFINE FIELD name ON workspace TYPE string;
             DEFINE FIELD branch ON workspace TYPE string;
             DEFINE FIELD worktree_path ON workspace TYPE string;
             DEFINE FIELD status ON workspace TYPE string;
             DEFINE FIELD locked_by ON workspace TYPE option<record<session>>;
             DEFINE FIELD created_at ON workspace TYPE datetime DEFAULT time::now();
             DEFINE FIELD updated_at ON workspace TYPE datetime DEFAULT time::now();",
        )
        .await
        .unwrap();
        db
    }

    async fn create_workspace(db: &Surreal<Db>, id: &str, status: &str) {
        db.query(&format!(
            "CREATE workspace:{} SET name = 'test', branch = 'main', \
             worktree_path = '/tmp/test', status = '{}'",
            id, status
        ))
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_db_transition_valid() {
        let db = setup_db().await;
        create_workspace(&db, "ws1", "backlog").await;

        let sm = StatusMachine::new(&db);
        sm.transition("ws1", WorkspaceStatus::Active).await.unwrap();

        let status = sm.get_status("ws1").await.unwrap();
        assert_eq!(status, WorkspaceStatus::Active);
    }

    #[tokio::test]
    async fn test_db_transition_invalid() {
        let db = setup_db().await;
        create_workspace(&db, "ws2", "backlog").await;

        let sm = StatusMachine::new(&db);
        let result = sm.transition("ws2", WorkspaceStatus::Done).await;
        assert!(matches!(result, Err(StatusError::InvalidTransition { .. })));
    }

    #[tokio::test]
    async fn test_db_transition_chain() {
        let db = setup_db().await;
        create_workspace(&db, "ws3", "backlog").await;

        let sm = StatusMachine::new(&db);
        sm.transition("ws3", WorkspaceStatus::Active).await.unwrap();
        sm.transition("ws3", WorkspaceStatus::Review).await.unwrap();
        sm.transition("ws3", WorkspaceStatus::Done).await.unwrap();

        let status = sm.get_status("ws3").await.unwrap();
        assert_eq!(status, WorkspaceStatus::Done);
    }

    #[tokio::test]
    async fn test_db_transition_to_backlog_from_any() {
        let db = setup_db().await;
        create_workspace(&db, "ws4", "active").await;

        let sm = StatusMachine::new(&db);
        sm.transition("ws4", WorkspaceStatus::Backlog).await.unwrap();

        let status = sm.get_status("ws4").await.unwrap();
        assert_eq!(status, WorkspaceStatus::Backlog);
    }

    #[tokio::test]
    async fn test_db_workspace_not_found() {
        let db = setup_db().await;
        let sm = StatusMachine::new(&db);
        let result = sm.get_status("nonexistent").await;
        assert!(matches!(result, Err(StatusError::WorkspaceNotFound(_))));
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo test workspace::status
```

Expected: All 21 tests pass (16 pure state machine + 5 DB-backed).

- [ ] **Step 3: Commit**

```bash
cd ~/projects/koompi-orch
git add src-tauri/src/workspace/status.rs
git commit -m "feat: add kanban state machine with validated workspace transitions"
```

---

## Chunk 5: Git Diff

### Task 6: Create git/diff.rs — Diff generation via git2-rs

**Files:**
- Create: `~/projects/koompi-orch/src-tauri/src/git/mod.rs`
- Create: `~/projects/koompi-orch/src-tauri/src/git/diff.rs`
- Modify: `~/projects/koompi-orch/src-tauri/src/lib.rs`

- [ ] **Step 1: Write git/diff.rs with tests**

Create `src-tauri/src/git/diff.rs`:
```rust
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
    let diff = repo.diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut opts))?;
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

    diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
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

        // Find or create the FileDiff entry
        let file_diff = if let Some(fd) = files.iter_mut().find(|f: &&mut FileDiff| f.path == path)
        {
            fd
        } else {
            files.push(FileDiff {
                path: path.clone(),
                status,
                patch: String::new(),
                additions: 0,
                deletions: 0,
            });
            files.last_mut().unwrap()
        };

        // Append patch content
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
                // Hunk header
                file_diff.patch.push_str(content);
            }
            _ => {}
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
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();
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
        assert!(summary.files.iter().any(|f| f.path == "new.txt" && f.status == FileStatus::Added));
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
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo.find_commit(commit1).unwrap();
        let commit2 = repo
            .commit(Some("HEAD"), &sig, &sig, "v2", &tree, &[&parent])
            .unwrap();

        let summary = diff_commits(
            dir.path(),
            &commit1.to_string(),
            &commit2.to_string(),
        )
        .unwrap();

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
```

- [ ] **Step 2: Create git/mod.rs**

Create `src-tauri/src/git/mod.rs`:
```rust
pub mod diff;
pub mod merge;
pub mod commit;
pub mod remote;

pub use diff::{diff_working_dir, diff_staged, diff_commits, DiffSummary, FileDiff, FileStatus, DiffError};
pub use merge::{merge_branch, rebase_branch, MergeResult, MergeError};
pub use commit::{create_commit, push_branch, CommitInfo, CommitError};
pub use remote::{create_pull_request, PullRequestInfo, RemoteError};
```

- [ ] **Step 3: Wire git module into lib.rs**

Add to `src-tauri/src/lib.rs`:
```rust
pub mod git;
```

- [ ] **Step 4: Run tests**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo test git::diff
```

Expected: All 6 tests pass.

- [ ] **Step 5: Commit**

```bash
cd ~/projects/koompi-orch
git add src-tauri/src/git/ src-tauri/src/lib.rs
git commit -m "feat: add git diff generation via git2-rs"
```

---

## Chunk 6: Git Merge and Rebase

### Task 7: Create git/merge.rs — Merge and rebase via git2-rs

**Files:**
- Create: `~/projects/koompi-orch/src-tauri/src/git/merge.rs`

- [ ] **Step 1: Write git/merge.rs with tests**

Create `src-tauri/src/git/merge.rs`:
```rust
//! Merge and rebase operations via git2-rs.
//!
//! Provides merge and rebase of one branch into another within a worktree.

use git2::{AnnotatedCommit, MergeOptions, Repository, Signature};
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
        let oid = head.target().ok_or_else(|| {
            git2::Error::from_str("HEAD has no target")
        })?;
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

    /// Create a repo with initial commit on main, then create a feature branch.
    fn setup_branched_repo(dir: &Path) -> Repository {
        let repo = Repository::init(dir).unwrap();
        let sig = Signature::now("Test", "test@test.com").unwrap();

        // Initial commit on main
        fs::write(dir.join("base.txt"), "base content").unwrap();
        let mut index = repo.index().unwrap();
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let main_commit = repo
            .commit(Some("HEAD"), &sig, &sig, "initial on main", &tree, &[])
            .unwrap();

        // Create feature branch from initial commit
        let commit = repo.find_commit(main_commit).unwrap();
        repo.branch("feature", &commit, false).unwrap();

        repo
    }

    #[test]
    fn test_merge_fast_forward() {
        let dir = TempDir::new().unwrap();
        let repo = setup_branched_repo(dir.path());
        let sig = Signature::now("Test", "test@test.com").unwrap();

        // Add a commit on feature branch
        let feature_ref = repo
            .find_branch("feature", git2::BranchType::Local)
            .unwrap();
        let feature_commit = feature_ref.get().peel_to_commit().unwrap();

        // Switch to feature and commit
        repo.set_head("refs/heads/feature").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();
        fs::write(dir.path().join("feature.txt"), "feature content").unwrap();
        let mut index = repo.index().unwrap();
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "feature commit",
            &tree,
            &[&feature_commit],
        )
        .unwrap();

        // Switch back to main and merge feature (should fast-forward)
        repo.set_head("refs/heads/main").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();

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
        let repo = setup_branched_repo(dir.path());
        let sig = Signature::now("Test", "test@test.com").unwrap();

        // Add commit on main
        let main_head = repo.head().unwrap().peel_to_commit().unwrap();
        fs::write(dir.path().join("main_file.txt"), "main work").unwrap();
        let mut index = repo.index().unwrap();
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "main commit", &tree, &[&main_head])
            .unwrap();

        // Add commit on feature
        let feature_ref = repo
            .find_branch("feature", git2::BranchType::Local)
            .unwrap();
        let feature_commit = feature_ref.get().peel_to_commit().unwrap();
        repo.set_head("refs/heads/feature").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();
        fs::write(dir.path().join("feature_file.txt"), "feature work").unwrap();
        let mut index = repo.index().unwrap();
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "feature commit",
            &tree,
            &[&feature_commit],
        )
        .unwrap();

        // Switch to main and merge feature (diverged, should create merge commit)
        repo.set_head("refs/heads/main").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();

        let result = merge_branch(dir.path(), "feature").unwrap();
        assert!(!result.fast_forward);
        // Both files should exist
        assert!(dir.path().join("main_file.txt").exists());
        assert!(dir.path().join("feature_file.txt").exists());
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo test git::merge
```

Expected: All 4 tests pass.

- [ ] **Step 3: Commit**

```bash
cd ~/projects/koompi-orch
git add src-tauri/src/git/merge.rs
git commit -m "feat: add git merge and rebase operations via git2-rs"
```

---

## Chunk 7: Git Commit and Push

### Task 8: Create git/commit.rs — Commit and push via git2-rs

**Files:**
- Create: `~/projects/koompi-orch/src-tauri/src/git/commit.rs`

- [ ] **Step 1: Write git/commit.rs with tests**

Create `src-tauri/src/git/commit.rs`:
```rust
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
pub fn stage_files(
    worktree_path: &Path,
    file_paths: &[&str],
) -> Result<(), CommitError> {
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
        repo.commit(Some("HEAD"), &author, &committer, message, &tree, &[&parent])?
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
pub fn push_branch(
    worktree_path: &Path,
    remote_name: &str,
) -> Result<(), CommitError> {
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
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();
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
        let commit = repo.find_commit(git2::Oid::from_str(&info.sha).unwrap()).unwrap();
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
```

- [ ] **Step 2: Run tests**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo test git::commit
```

Expected: All 6 tests pass.

- [ ] **Step 3: Commit**

```bash
cd ~/projects/koompi-orch
git add src-tauri/src/git/commit.rs
git commit -m "feat: add git commit and push operations via git2-rs"
```

---

## Chunk 8: Git Remote — PR Creation

### Task 9: Create git/remote.rs — PR creation via octocrab

**Files:**
- Create: `~/projects/koompi-orch/src-tauri/src/git/remote.rs`

- [ ] **Step 1: Write git/remote.rs with tests**

Create `src-tauri/src/git/remote.rs`:
```rust
//! PR creation and remote operations via octocrab (GitHub API).
//!
//! Extracts owner/repo from git remote URL, then uses octocrab to
//! create pull requests and fetch CI status.

use git2::Repository;
use octocrab::Octocrab;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RemoteError {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("github API error: {0}")]
    GitHub(#[from] octocrab::Error),
    #[error("remote not found: {0}")]
    RemoteNotFound(String),
    #[error("cannot parse remote URL: {0}")]
    InvalidRemoteUrl(String),
    #[error("no GitHub token configured")]
    NoToken,
}

/// Info about a created pull request.
#[derive(Debug, Clone)]
pub struct PullRequestInfo {
    pub number: u64,
    pub html_url: String,
    pub title: String,
    pub head_branch: String,
    pub base_branch: String,
}

/// Parsed GitHub remote: owner and repo name.
#[derive(Debug, Clone)]
pub struct GitHubRemote {
    pub owner: String,
    pub repo: String,
}

/// Parse a GitHub remote URL into owner/repo.
///
/// Supports:
/// - `https://github.com/owner/repo.git`
/// - `https://github.com/owner/repo`
/// - `git@github.com:owner/repo.git`
/// - `ssh://git@github.com/owner/repo.git`
pub fn parse_github_remote(url: &str) -> Result<GitHubRemote, RemoteError> {
    // SSH format: git@github.com:owner/repo.git
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let cleaned = rest.trim_end_matches(".git");
        let parts: Vec<&str> = cleaned.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Ok(GitHubRemote {
                owner: parts[0].to_string(),
                repo: parts[1].to_string(),
            });
        }
    }

    // HTTPS format: https://github.com/owner/repo[.git]
    // SSH URL format: ssh://git@github.com/owner/repo[.git]
    if url.contains("github.com") {
        let after_host = url
            .split("github.com")
            .nth(1)
            .ok_or_else(|| RemoteError::InvalidRemoteUrl(url.to_string()))?;
        // Remove leading / or :
        let path = after_host.trim_start_matches('/').trim_start_matches(':');
        let cleaned = path.trim_end_matches(".git");
        let parts: Vec<&str> = cleaned.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Ok(GitHubRemote {
                owner: parts[0].to_string(),
                repo: parts[1].to_string(),
            });
        }
    }

    Err(RemoteError::InvalidRemoteUrl(url.to_string()))
}

/// Extract the GitHub remote info from a repository's remote.
pub fn get_github_remote(
    worktree_path: &Path,
    remote_name: &str,
) -> Result<GitHubRemote, RemoteError> {
    let repo = Repository::open(worktree_path)?;
    let remote = repo
        .find_remote(remote_name)
        .map_err(|_| RemoteError::RemoteNotFound(remote_name.to_string()))?;
    let url = remote
        .url()
        .ok_or_else(|| RemoteError::InvalidRemoteUrl("no URL".to_string()))?;
    parse_github_remote(url)
}

/// Create a pull request on GitHub.
///
/// Requires a GitHub personal access token. The `head` branch must already
/// be pushed to the remote.
pub async fn create_pull_request(
    token: &str,
    owner: &str,
    repo: &str,
    title: &str,
    body: &str,
    head_branch: &str,
    base_branch: &str,
) -> Result<PullRequestInfo, RemoteError> {
    let octocrab = Octocrab::builder()
        .personal_token(token.to_string())
        .build()?;

    let pr = octocrab
        .pulls(owner, repo)
        .create(title, head_branch, base_branch)
        .body(body)
        .send()
        .await?;

    Ok(PullRequestInfo {
        number: pr.number,
        html_url: pr
            .html_url
            .map(|u| u.to_string())
            .unwrap_or_default(),
        title: pr.title.unwrap_or_default().to_string(),
        head_branch: head_branch.to_string(),
        base_branch: base_branch.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use tempfile::TempDir;

    #[test]
    fn test_parse_github_https_url() {
        let remote =
            parse_github_remote("https://github.com/koompi/orch.git").unwrap();
        assert_eq!(remote.owner, "koompi");
        assert_eq!(remote.repo, "orch");
    }

    #[test]
    fn test_parse_github_https_no_git_suffix() {
        let remote =
            parse_github_remote("https://github.com/koompi/orch").unwrap();
        assert_eq!(remote.owner, "koompi");
        assert_eq!(remote.repo, "orch");
    }

    #[test]
    fn test_parse_github_ssh_url() {
        let remote =
            parse_github_remote("git@github.com:koompi/orch.git").unwrap();
        assert_eq!(remote.owner, "koompi");
        assert_eq!(remote.repo, "orch");
    }

    #[test]
    fn test_parse_github_ssh_protocol_url() {
        let remote =
            parse_github_remote("ssh://git@github.com/koompi/orch.git").unwrap();
        assert_eq!(remote.owner, "koompi");
        assert_eq!(remote.repo, "orch");
    }

    #[test]
    fn test_parse_invalid_url() {
        let result = parse_github_remote("https://gitlab.com/user/repo.git");
        assert!(matches!(result, Err(RemoteError::InvalidRemoteUrl(_))));
    }

    #[test]
    fn test_get_github_remote_no_remote() {
        let dir = TempDir::new().unwrap();
        Repository::init(dir.path()).unwrap();
        let result = get_github_remote(dir.path(), "origin");
        assert!(matches!(result, Err(RemoteError::RemoteNotFound(_))));
    }

    #[test]
    fn test_get_github_remote_with_remote() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        repo.remote("origin", "https://github.com/koompi/orch.git")
            .unwrap();

        let remote = get_github_remote(dir.path(), "origin").unwrap();
        assert_eq!(remote.owner, "koompi");
        assert_eq!(remote.repo, "orch");
    }

    // Note: create_pull_request is not unit-tested here because it requires
    // a live GitHub API. It should be tested in integration/e2e tests with
    // a test repo and token, or mocked via a trait abstraction in a future plan.
}
```

- [ ] **Step 2: Run tests**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo test git::remote
```

Expected: All 7 tests pass.

- [ ] **Step 3: Commit**

```bash
cd ~/projects/koompi-orch
git add src-tauri/src/git/remote.rs
git commit -m "feat: add GitHub PR creation via octocrab with remote URL parsing"
```

---

## Chunk 9: Module Wiring and Integration Verification

### Task 10: Wire all modules and verify full build

**Files:**
- Modify: `~/projects/koompi-orch/src-tauri/src/workspace/mod.rs`
- Modify: `~/projects/koompi-orch/src-tauri/src/git/mod.rs`
- Modify: `~/projects/koompi-orch/src-tauri/src/lib.rs`
- Modify: `~/projects/koompi-orch/src-tauri/Cargo.toml`

- [ ] **Step 1: Ensure Cargo.toml has all required dependencies**

Verify `src-tauri/Cargo.toml` includes (add any missing):
```toml
[dependencies]
# ... existing deps from Plan 1 ...
git2 = "0.19"
octocrab = "0.41"
uuid = { version = "1", features = ["v4"] }
thiserror = "2"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Finalize workspace/mod.rs**

Ensure `src-tauri/src/workspace/mod.rs` compiles with all sub-modules:
```rust
pub mod manager;
pub mod snapshot;
pub mod conflict;
pub mod lock;
pub mod status;

pub use manager::{WorktreeManager, WorktreeInfo, WorktreeError};
pub use snapshot::{SnapshotManager, SnapshotInfo, SnapshotError};
pub use conflict::{ConflictDetector, ConflictWarning, ConflictError};
pub use lock::{WorkspaceLock, LockError, LockResult};
pub use status::{WorkspaceStatus, StatusMachine, StatusError};
```

- [ ] **Step 3: Finalize git/mod.rs**

Ensure `src-tauri/src/git/mod.rs` compiles with all sub-modules:
```rust
pub mod diff;
pub mod merge;
pub mod commit;
pub mod remote;

pub use diff::{diff_working_dir, diff_staged, diff_commits, DiffSummary, FileDiff, FileStatus, DiffError};
pub use merge::{merge_branch, rebase_branch, MergeResult, MergeError};
pub use commit::{create_commit, stage_all, stage_files, push_branch, CommitInfo, CommitError};
pub use remote::{create_pull_request, parse_github_remote, get_github_remote, PullRequestInfo, GitHubRemote, RemoteError};
```

- [ ] **Step 4: Ensure lib.rs has both modules**

```rust
pub mod config;
pub mod db;
pub mod workspace;
pub mod git;
```

- [ ] **Step 5: Run full test suite**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo test
```

Expected: All tests pass across all modules (config, db, workspace, git).

- [ ] **Step 6: Run cargo check for compile verification**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo check
```

Expected: Compiles with no errors.

- [ ] **Step 7: Commit**

```bash
cd ~/projects/koompi-orch
git add src-tauri/src/workspace/mod.rs src-tauri/src/git/mod.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "chore: wire workspace and git modules, verify full build"
```
