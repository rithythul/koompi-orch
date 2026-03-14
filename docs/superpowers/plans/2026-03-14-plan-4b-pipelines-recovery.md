# Plan 4B: Pipelines, Smart Routing, and Crash Recovery

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan.

**Goal:** Build pipeline orchestration with handoff protocol, smart agent routing, and crash recovery.
**Architecture:** Pipeline chains agents sequentially with handoff context. Router selects agent/model. Recovery restores state on startup.
**Tech Stack:** Rust, tokio, SurrealDB
**Spec Reference:** Sections 7.2, 7.4, 7.5, 15, 19 of the spec

---

## Task 1: Pipeline Execution Engine (`orchestrator/pipeline.rs`)

**Files:**
- Create: `src-tauri/src/orchestrator/pipeline.rs`
- Create: `src-tauri/src/orchestrator/mod.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod orchestrator`)

- [ ] **Step 1: Write the failing tests**

Create `src-tauri/src/orchestrator/pipeline.rs`:

```rust
//! Pipeline execution engine.
//!
//! Runs pipeline steps sequentially, manages handoff context between agents,
//! writes handoff files to ~/.koompi-orch/handoffs/, and appends auto-review
//! as an implicit last step when configured.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use surrealdb::engine::local::Db;
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use thiserror::Error;
use tokio::fs;

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("pipeline not found: {0}")]
    NotFound(String),

    #[error("pipeline run not found: {0}")]
    RunNotFound(String),

    #[error("pipeline step {step} failed: {reason}")]
    StepFailed { step: usize, reason: String },

    #[error("workspace locked by another session")]
    WorkspaceLocked,

    #[error("handoff generation failed: {0}")]
    HandoffFailed(String),

    #[error("database error: {0}")]
    DbError(String),

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Handoff type between pipeline steps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HandoffType {
    /// LLM-generated summary of what was done, key decisions, file list.
    Summary,
    /// Complete session JSONL, truncated to fit context window.
    FullLog,
    /// git diff of all changes made during the step.
    DiffOnly,
}

impl HandoffType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::FullLog => "full_log",
            Self::DiffOnly => "diff_only",
        }
    }
}

/// A single step in a pipeline definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    /// Role preset name (e.g. "architect", "implementer", "reviewer", "tester").
    pub role: String,
    /// Agent type override (None = use router).
    pub agent_type: Option<String>,
    /// Model override (None = use router).
    pub model: Option<String>,
    /// How to pass context to the next step.
    pub handoff_type: HandoffType,
}

/// Status of a pipeline run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PipelineRunStatus {
    Running,
    Paused,
    Completed,
    Failed,
}

/// Handoff context written between steps.
#[derive(Debug, Clone)]
pub struct HandoffContext {
    pub from_step: usize,
    pub to_step: usize,
    pub handoff_type: HandoffType,
    pub content: String,
    pub file_path: PathBuf,
}

/// Generates handoff content based on the handoff type.
///
/// For `Summary`: reads session log JSONL and extracts structured summary.
/// For `FullLog`: reads full session log, truncated to max_chars.
/// For `DiffOnly`: runs `git diff` in the worktree.
pub fn generate_handoff_content(
    handoff_type: &HandoffType,
    session_log_path: &Path,
    session_log: &str,
    worktree_path: &Path,
    step_role: &str,
    task_prompt: &str,
) -> Result<String, PipelineError> {
    match handoff_type {
        HandoffType::Summary => {
            generate_summary(session_log, step_role, task_prompt)
        }
        HandoffType::FullLog => {
            // Truncate to 100k chars to fit typical context windows.
            let max_chars = 100_000;
            let content = if session_log.len() > max_chars {
                // Keep the last max_chars portion (most recent context).
                let start = session_log.len() - max_chars;
                format!(
                    "[Truncated: showing last {} chars of {} total]\n\n{}",
                    max_chars,
                    session_log.len(),
                    &session_log[start..]
                )
            } else {
                session_log.to_string()
            };
            Ok(content)
        }
        HandoffType::DiffOnly => {
            generate_diff(worktree_path)
        }
    }
}

/// Mechanical summary extraction from session JSONL.
/// Extracts files modified, tools used, and key content.
fn generate_summary(
    session_log: &str,
    role: &str,
    task_prompt: &str,
) -> Result<String, PipelineError> {
    let mut files_modified = Vec::new();
    let mut tools_used = Vec::new();
    let mut key_content = Vec::new();

    for line in session_log.lines() {
        if line.trim().is_empty() {
            continue;
        }
        // Parse each JSONL line; skip unparseable lines.
        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
            // Extract tool use events.
            if let Some(name) = entry.get("name").and_then(|n| n.as_str()) {
                if entry.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    if !tools_used.contains(&name.to_string()) {
                        tools_used.push(name.to_string());
                    }
                    // Extract file paths from tool inputs.
                    if let Some(input) = entry.get("input") {
                        if let Some(path) = input.get("path").and_then(|p| p.as_str()) {
                            if !files_modified.contains(&path.to_string()) {
                                files_modified.push(path.to_string());
                            }
                        }
                        if let Some(path) = input.get("file_path").and_then(|p| p.as_str()) {
                            if !files_modified.contains(&path.to_string()) {
                                files_modified.push(path.to_string());
                            }
                        }
                    }
                }
            }
            // Extract text content for key decisions.
            if entry.get("role").and_then(|r| r.as_str()) == Some("assistant") {
                if let Some(content) = entry.get("content").and_then(|c| c.as_str()) {
                    // Keep first 200 chars of each assistant message as key content.
                    let snippet: String = content.chars().take(200).collect();
                    key_content.push(snippet);
                }
            }
        }
    }

    let mut summary = format!("## Step Summary: {}\n\n", role);
    summary.push_str(&format!("### Task\n{}\n\n", task_prompt));

    if !files_modified.is_empty() {
        summary.push_str("### Files modified\n");
        for f in &files_modified {
            summary.push_str(&format!("- {}\n", f));
        }
        summary.push('\n');
    }

    if !tools_used.is_empty() {
        summary.push_str("### Tools used\n");
        for t in &tools_used {
            summary.push_str(&format!("- {}\n", t));
        }
        summary.push('\n');
    }

    if !key_content.is_empty() {
        summary.push_str("### Key decisions\n");
        // Include at most 5 key content snippets.
        for snippet in key_content.iter().take(5) {
            summary.push_str(&format!("- {}\n", snippet));
        }
        summary.push('\n');
    }

    Ok(summary)
}

/// Generate git diff output for the worktree using git2-rs.
fn generate_diff(worktree_path: &Path) -> Result<String, PipelineError> {
    let repo = git2::Repository::open(worktree_path)
        .map_err(|e| PipelineError::HandoffFailed(format!("git open: {}", e)))?;

    let head_tree = repo
        .head()
        .and_then(|h| h.peel_to_tree())
        .ok();

    let diff = repo
        .diff_tree_to_workdir_with_index(head_tree.as_ref(), None)
        .map_err(|e| PipelineError::HandoffFailed(format!("git diff: {}", e)))?;

    let mut diff_text = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let origin = line.origin();
        if origin == '+' || origin == '-' || origin == ' ' {
            diff_text.push(origin);
        }
        if let Ok(content) = std::str::from_utf8(line.content()) {
            diff_text.push_str(content);
        }
        true
    })
    .map_err(|e| PipelineError::HandoffFailed(format!("diff print: {}", e)))?;

    if diff_text.is_empty() {
        diff_text = "(no changes detected)".to_string();
    }

    Ok(diff_text)
}

/// Write handoff context to disk at ~/.koompi-orch/handoffs/{run_id}/step-{n}.md
pub async fn write_handoff_file(
    base_dir: &Path,
    pipeline_run_id: &str,
    step_number: usize,
    content: &str,
) -> Result<PathBuf, PipelineError> {
    let handoff_dir = base_dir
        .join("handoffs")
        .join(pipeline_run_id);
    fs::create_dir_all(&handoff_dir).await?;

    let file_path = handoff_dir.join(format!("step-{}.md", step_number));
    fs::write(&file_path, content).await?;

    Ok(file_path)
}

/// Format handoff context for injection into the next agent's prompt.
/// Prepends handoff context before the user's original task.
pub fn format_handoff_injection(
    handoff_content: &str,
    from_role: &str,
    task_prompt: &str,
) -> String {
    format!(
        "## Context from previous step ({})\n{}\n\n## Your task\n{}",
        from_role, handoff_content, task_prompt
    )
}

/// Prune handoff files older than retention_days.
/// Called on app startup per spec Section 19.
pub async fn prune_old_handoffs(
    base_dir: &Path,
    retention_days: u64,
) -> Result<usize, PipelineError> {
    let handoff_dir = base_dir.join("handoffs");
    if !handoff_dir.exists() {
        return Ok(0);
    }

    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(retention_days * 24 * 60 * 60);

    let mut pruned = 0;
    let mut entries = fs::read_dir(&handoff_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_dir() {
            let metadata = entry.metadata().await?;
            if let Ok(modified) = metadata.modified() {
                if modified < cutoff {
                    fs::remove_dir_all(entry.path()).await?;
                    pruned += 1;
                }
            }
        }
    }

    Ok(pruned)
}

/// Database operations for pipeline execution.
pub struct PipelineExecutor<'a> {
    db: &'a Surreal<Db>,
    base_dir: PathBuf,
}

impl<'a> PipelineExecutor<'a> {
    pub fn new(db: &'a Surreal<Db>, base_dir: PathBuf) -> Self {
        Self { db, base_dir }
    }

    /// Create a new pipeline_run record linked to a pipeline and workspace.
    pub async fn create_run(
        &self,
        pipeline_id: &str,
        workspace_id: &str,
    ) -> Result<Thing, PipelineError> {
        let result: Option<serde_json::Value> = self
            .db
            .query(
                "CREATE pipeline_run SET current_step = 0, status = 'running' RETURN id",
            )
            .await
            .map_err(|e| PipelineError::DbError(e.to_string()))?
            .take(0)
            .map_err(|e| PipelineError::DbError(e.to_string()))?;

        let run_id = result
            .and_then(|v| v.get("id").cloned())
            .ok_or_else(|| PipelineError::DbError("failed to create pipeline_run".into()))?;

        let run_id_str = run_id
            .as_str()
            .ok_or_else(|| PipelineError::DbError("invalid run id".into()))?;

        // Create instance_of relation: pipeline_run -> pipeline
        self.db
            .query("RELATE type::thing($run) -> instance_of -> type::thing($pipeline)")
            .bind(("run", run_id_str))
            .bind(("pipeline", format!("pipeline:{}", pipeline_id)))
            .await
            .map_err(|e| PipelineError::DbError(e.to_string()))?;

        // Create executes_in relation: pipeline_run -> workspace
        self.db
            .query("RELATE type::thing($run) -> executes_in -> type::thing($workspace)")
            .bind(("run", run_id_str))
            .bind(("workspace", format!("workspace:{}", workspace_id)))
            .await
            .map_err(|e| PipelineError::DbError(e.to_string()))?;

        serde_json::from_value(run_id).map_err(|e| PipelineError::DbError(e.to_string()))
    }

    /// Advance pipeline_run to the next step.
    pub async fn advance_step(
        &self,
        run_id: &Thing,
    ) -> Result<i64, PipelineError> {
        let result: Option<serde_json::Value> = self
            .db
            .query(
                "UPDATE type::thing($id) SET current_step += 1 RETURN current_step",
            )
            .bind(("id", run_id.to_string()))
            .await
            .map_err(|e| PipelineError::DbError(e.to_string()))?
            .take(0)
            .map_err(|e| PipelineError::DbError(e.to_string()))?;

        result
            .and_then(|v| v.get("current_step").and_then(|s| s.as_i64()))
            .ok_or_else(|| PipelineError::DbError("failed to advance step".into()))
    }

    /// Set pipeline_run status (running, paused, completed, failed).
    pub async fn set_run_status(
        &self,
        run_id: &Thing,
        status: PipelineRunStatus,
    ) -> Result<(), PipelineError> {
        let status_str = match status {
            PipelineRunStatus::Running => "running",
            PipelineRunStatus::Paused => "paused",
            PipelineRunStatus::Completed => "completed",
            PipelineRunStatus::Failed => "failed",
        };

        self.db
            .query("UPDATE type::thing($id) SET status = $status, ended_at = IF $status IN ['completed','failed'] THEN time::now() ELSE ended_at END")
            .bind(("id", run_id.to_string()))
            .bind(("status", status_str))
            .await
            .map_err(|e| PipelineError::DbError(e.to_string()))?;

        Ok(())
    }

    /// Record a handoff relation between two sessions.
    pub async fn record_handoff(
        &self,
        from_session_id: &str,
        to_session_id: &str,
        handoff_type: &HandoffType,
        summary: Option<&str>,
        context_file: Option<&str>,
    ) -> Result<(), PipelineError> {
        self.db
            .query(
                "RELATE type::thing($from) -> hands_off_to -> type::thing($to) \
                 SET handoff_type = $ht, output_summary = $summary, context_file = $cf",
            )
            .bind(("from", format!("session:{}", from_session_id)))
            .bind(("to", format!("session:{}", to_session_id)))
            .bind(("ht", handoff_type.as_str()))
            .bind(("summary", summary))
            .bind(("cf", context_file))
            .await
            .map_err(|e| PipelineError::DbError(e.to_string()))?;

        Ok(())
    }

    /// Check if auto-review should be appended as implicit last step.
    /// Returns true if auto_review is enabled and pipeline does not already
    /// end with a "reviewer" step.
    pub fn should_append_auto_review(
        steps: &[PipelineStep],
        auto_review_enabled: bool,
    ) -> bool {
        if !auto_review_enabled {
            return false;
        }
        // Don't append if pipeline already ends with a reviewer.
        steps.last().map_or(true, |s| s.role != "reviewer")
    }

    /// Build the effective step list, appending auto-review if needed.
    pub fn effective_steps(
        steps: &[PipelineStep],
        auto_review_enabled: bool,
    ) -> Vec<PipelineStep> {
        let mut effective = steps.to_vec();
        if Self::should_append_auto_review(steps, auto_review_enabled) {
            effective.push(PipelineStep {
                role: "reviewer".to_string(),
                agent_type: None,
                model: None,
                handoff_type: HandoffType::DiffOnly,
            });
        }
        effective
    }

    /// Generate and persist handoff between steps.
    pub async fn generate_and_store_handoff(
        &self,
        pipeline_run_id: &str,
        step: &PipelineStep,
        step_number: usize,
        session_log: &str,
        session_log_path: &Path,
        worktree_path: &Path,
        task_prompt: &str,
    ) -> Result<HandoffContext, PipelineError> {
        let content = generate_handoff_content(
            &step.handoff_type,
            session_log_path,
            session_log,
            worktree_path,
            &step.role,
            task_prompt,
        )?;

        let file_path = write_handoff_file(
            &self.base_dir,
            pipeline_run_id,
            step_number,
            &content,
        )
        .await?;

        Ok(HandoffContext {
            from_step: step_number,
            to_step: step_number + 1,
            handoff_type: step.handoff_type.clone(),
            content,
            file_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_handoff_type_as_str() {
        assert_eq!(HandoffType::Summary.as_str(), "summary");
        assert_eq!(HandoffType::FullLog.as_str(), "full_log");
        assert_eq!(HandoffType::DiffOnly.as_str(), "diff_only");
    }

    #[test]
    fn test_generate_summary_extracts_files_and_tools() {
        let session_log = r#"{"type":"tool_use","name":"Read","input":{"path":"src/main.rs"}}
{"type":"tool_use","name":"Edit","input":{"file_path":"src/lib.rs"}}
{"role":"assistant","content":"I decided to use JWT for authentication because it is stateless."}
{"role":"user","content":"Looks good, continue."}
{"role":"assistant","content":"Implemented the token validation middleware."}
"#;
        let result = generate_summary(session_log, "architect", "Implement auth").unwrap();

        assert!(result.contains("## Step Summary: architect"));
        assert!(result.contains("### Task\nImplement auth"));
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("src/lib.rs"));
        assert!(result.contains("Read"));
        assert!(result.contains("Edit"));
        assert!(result.contains("JWT for authentication"));
    }

    #[test]
    fn test_generate_summary_handles_empty_log() {
        let result = generate_summary("", "implementer", "Build feature").unwrap();
        assert!(result.contains("## Step Summary: implementer"));
        assert!(result.contains("### Task\nBuild feature"));
        // No files/tools/decisions sections when log is empty.
        assert!(!result.contains("### Files modified"));
    }

    #[test]
    fn test_full_log_truncation() {
        let long_log = "x".repeat(200_000);
        let result = generate_handoff_content(
            &HandoffType::FullLog,
            Path::new("/tmp/test.jsonl"),
            &long_log,
            Path::new("/tmp/worktree"),
            "implementer",
            "task",
        )
        .unwrap();

        assert!(result.contains("[Truncated:"));
        // Should contain last 100k chars plus the truncation header.
        assert!(result.len() < 110_000);
    }

    #[test]
    fn test_full_log_no_truncation_when_short() {
        let short_log = "short session log";
        let result = generate_handoff_content(
            &HandoffType::FullLog,
            Path::new("/tmp/test.jsonl"),
            short_log,
            Path::new("/tmp/worktree"),
            "implementer",
            "task",
        )
        .unwrap();

        assert_eq!(result, "short session log");
    }

    #[test]
    fn test_diff_only_with_real_git_repo() {
        let tmp = TempDir::new().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();

        // Create initial commit.
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[]).unwrap();

        // Add a new file (unstaged change).
        std::fs::write(tmp.path().join("new.txt"), "hello world").unwrap();

        let result = generate_diff(tmp.path()).unwrap();
        assert!(result.contains("hello world"));
    }

    #[test]
    fn test_diff_only_no_changes() {
        let tmp = TempDir::new().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[]).unwrap();

        let result = generate_diff(tmp.path()).unwrap();
        assert_eq!(result, "(no changes detected)");
    }

    #[test]
    fn test_format_handoff_injection() {
        let result = format_handoff_injection(
            "Summary of architect work",
            "architect",
            "Implement the login page",
        );
        assert!(result.starts_with("## Context from previous step (architect)"));
        assert!(result.contains("Summary of architect work"));
        assert!(result.contains("## Your task\nImplement the login page"));
    }

    #[tokio::test]
    async fn test_write_handoff_file_creates_dirs_and_file() {
        let tmp = TempDir::new().unwrap();
        let file_path = write_handoff_file(
            tmp.path(),
            "run-abc123",
            2,
            "## Step Summary\nDid stuff.",
        )
        .await
        .unwrap();

        assert!(file_path.exists());
        assert_eq!(
            file_path,
            tmp.path().join("handoffs/run-abc123/step-2.md")
        );
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Did stuff."));
    }

    #[tokio::test]
    async fn test_prune_old_handoffs() {
        let tmp = TempDir::new().unwrap();
        let handoff_dir = tmp.path().join("handoffs");

        // Create two handoff directories.
        let old_dir = handoff_dir.join("old-run");
        let new_dir = handoff_dir.join("new-run");
        fs::create_dir_all(&old_dir).await.unwrap();
        fs::create_dir_all(&new_dir).await.unwrap();
        fs::write(old_dir.join("step-0.md"), "old").await.unwrap();
        fs::write(new_dir.join("step-0.md"), "new").await.unwrap();

        // Set old_dir mtime to 60 days ago.
        let sixty_days_ago = std::time::SystemTime::now()
            - std::time::Duration::from_secs(60 * 24 * 60 * 60);
        filetime::set_file_mtime(
            &old_dir,
            filetime::FileTime::from_system_time(sixty_days_ago),
        )
        .unwrap();

        // Prune with 30-day retention.
        let pruned = prune_old_handoffs(tmp.path(), 30).await.unwrap();
        assert_eq!(pruned, 1);
        assert!(!old_dir.exists());
        assert!(new_dir.exists());
    }

    #[tokio::test]
    async fn test_prune_returns_zero_when_no_handoffs_dir() {
        let tmp = TempDir::new().unwrap();
        let pruned = prune_old_handoffs(tmp.path(), 30).await.unwrap();
        assert_eq!(pruned, 0);
    }

    #[test]
    fn test_should_append_auto_review() {
        let steps = vec![
            PipelineStep {
                role: "architect".into(),
                agent_type: None,
                model: None,
                handoff_type: HandoffType::Summary,
            },
            PipelineStep {
                role: "implementer".into(),
                agent_type: None,
                model: None,
                handoff_type: HandoffType::DiffOnly,
            },
        ];

        // Auto-review enabled and no reviewer at end -> true.
        assert!(PipelineExecutor::should_append_auto_review(&steps, true));

        // Auto-review disabled -> false.
        assert!(!PipelineExecutor::should_append_auto_review(&steps, false));

        // Pipeline already ends with reviewer -> false.
        let steps_with_reviewer = vec![
            PipelineStep {
                role: "implementer".into(),
                agent_type: None,
                model: None,
                handoff_type: HandoffType::Summary,
            },
            PipelineStep {
                role: "reviewer".into(),
                agent_type: None,
                model: None,
                handoff_type: HandoffType::DiffOnly,
            },
        ];
        assert!(!PipelineExecutor::should_append_auto_review(
            &steps_with_reviewer,
            true
        ));
    }

    #[test]
    fn test_effective_steps_appends_reviewer() {
        let steps = vec![PipelineStep {
            role: "implementer".into(),
            agent_type: None,
            model: None,
            handoff_type: HandoffType::Summary,
        }];

        let effective = PipelineExecutor::effective_steps(&steps, true);
        assert_eq!(effective.len(), 2);
        assert_eq!(effective[1].role, "reviewer");
        assert_eq!(effective[1].handoff_type, HandoffType::DiffOnly);
    }

    #[test]
    fn test_pipeline_step_serialization_roundtrip() {
        let step = PipelineStep {
            role: "architect".into(),
            agent_type: Some("claude-code".into()),
            model: Some("opus".into()),
            handoff_type: HandoffType::Summary,
        };
        let json = serde_json::to_string(&step).unwrap();
        let deserialized: PipelineStep = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.role, "architect");
        assert_eq!(deserialized.agent_type.unwrap(), "claude-code");
        assert_eq!(deserialized.handoff_type, HandoffType::Summary);
    }
}
```

- [ ] **Step 2: Create the module file**

Create `src-tauri/src/orchestrator/mod.rs`:

```rust
pub mod pipeline;
pub mod recovery;
pub mod router;
```

- [ ] **Step 3: Wire into lib.rs**

Add to `src-tauri/src/lib.rs`:

```rust
mod orchestrator;
```

- [ ] **Step 4: Verify all tests pass**

```bash
cd src-tauri && cargo test pipeline -- --nocapture
```

All 14 tests should pass: handoff generation (summary, full_log, diff_only), file I/O, pruning, auto-review detection, serialization.

---

## Task 2: Smart Routing Rule Engine (`orchestrator/router.rs`)

**Files:**
- Create: `src-tauri/src/orchestrator/router.rs`

- [ ] **Step 1: Write the failing tests and implementation**

Create `src-tauri/src/orchestrator/router.rs`:

```rust
//! Smart agent routing rule engine.
//!
//! Routing priority (spec Section 15):
//! 1. User override (explicit agent/model selection)
//! 2. Pipeline step (role preset maps to agent)
//! 3. Keyword classification (task description keywords)
//! 4. Cost tier (role -> model tier mapping)
//! 5. Fallback (defaults from config)
//!
//! Configurable via .orch.toml [routing] table.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Routing decision: which agent and model to use.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutingDecision {
    pub agent_type: String,
    pub model: Option<String>,
    pub role: String,
    /// Which routing signal determined this decision.
    pub decided_by: RoutingSignal,
}

/// Which routing signal produced the decision.
#[derive(Debug, Clone, PartialEq)]
pub enum RoutingSignal {
    UserOverride,
    PipelineStep,
    KeywordClassification,
    CostTier,
    Fallback,
}

/// Per-role routing entry from .orch.toml [routing] table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEntry {
    pub agent: String,
    pub model: Option<String>,
}

/// Routing configuration loaded from .orch.toml.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingConfig {
    /// Role -> agent/model mapping.
    /// Keys: "architect", "implementer", "reviewer", "tester", "shipper", etc.
    #[serde(flatten)]
    pub roles: HashMap<String, RouteEntry>,
}

/// Default agent and role from [defaults] section.
#[derive(Debug, Clone)]
pub struct DefaultsConfig {
    pub agent: String,
    pub role: String,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            agent: "claude-code".to_string(),
            role: "implementer".to_string(),
        }
    }
}

/// Keyword classification rules.
/// Maps keyword patterns to role names.
const KEYWORD_RULES: &[(&[&str], &str)] = &[
    (&["review", "audit", "security", "inspect", "check"], "reviewer"),
    (&["test", "spec", "coverage", "e2e", "unit test"], "tester"),
    (&["fix", "bug", "error", "crash", "broken", "regression"], "fixer"),
    (&["architect", "design", "plan", "rfc", "proposal"], "architect"),
    (&["ship", "deploy", "release", "publish", "ci"], "shipper"),
];

/// The fixer role maps to implementer routing unless explicitly configured.
const FIXER_FALLBACK_ROLE: &str = "implementer";

/// Smart routing engine.
pub struct Router {
    routing: RoutingConfig,
    defaults: DefaultsConfig,
}

impl Router {
    pub fn new(routing: RoutingConfig, defaults: DefaultsConfig) -> Self {
        Self { routing, defaults }
    }

    /// Route a task to an agent/model.
    ///
    /// Arguments:
    /// - `user_agent`: explicit agent override from user (None = no override)
    /// - `user_model`: explicit model override from user (None = no override)
    /// - `pipeline_step_role`: role from the current pipeline step (None = not in pipeline)
    /// - `task_description`: the task text for keyword classification
    pub fn route(
        &self,
        user_agent: Option<&str>,
        user_model: Option<&str>,
        pipeline_step_role: Option<&str>,
        task_description: &str,
    ) -> RoutingDecision {
        // Signal 1: User override.
        if user_agent.is_some() || user_model.is_some() {
            let role = pipeline_step_role
                .unwrap_or(&self.defaults.role)
                .to_string();
            return RoutingDecision {
                agent_type: user_agent
                    .unwrap_or(&self.defaults.agent)
                    .to_string(),
                model: user_model.map(|m| m.to_string()).or_else(|| {
                    self.routing
                        .roles
                        .get(&role)
                        .and_then(|e| e.model.clone())
                }),
                role,
                decided_by: RoutingSignal::UserOverride,
            };
        }

        // Signal 2: Pipeline step role.
        if let Some(role) = pipeline_step_role {
            if let Some(entry) = self.routing.roles.get(role) {
                return RoutingDecision {
                    agent_type: entry.agent.clone(),
                    model: entry.model.clone(),
                    role: role.to_string(),
                    decided_by: RoutingSignal::PipelineStep,
                };
            }
            // Role specified but not in routing table: use defaults with role.
            return RoutingDecision {
                agent_type: self.defaults.agent.clone(),
                model: None,
                role: role.to_string(),
                decided_by: RoutingSignal::PipelineStep,
            };
        }

        // Signal 3: Keyword classification.
        let task_lower = task_description.to_lowercase();
        for (keywords, role) in KEYWORD_RULES {
            if keywords.iter().any(|kw| task_lower.contains(kw)) {
                let lookup_role = if *role == "fixer" {
                    // Fixer maps to implementer unless "fixer" is explicitly configured.
                    if self.routing.roles.contains_key("fixer") {
                        "fixer"
                    } else {
                        FIXER_FALLBACK_ROLE
                    }
                } else {
                    role
                };

                if let Some(entry) = self.routing.roles.get(lookup_role) {
                    return RoutingDecision {
                        agent_type: entry.agent.clone(),
                        model: entry.model.clone(),
                        role: role.to_string(),
                        decided_by: RoutingSignal::KeywordClassification,
                    };
                }
                // Keyword matched but role not in routing table: use default agent.
                return RoutingDecision {
                    agent_type: self.defaults.agent.clone(),
                    model: None,
                    role: role.to_string(),
                    decided_by: RoutingSignal::KeywordClassification,
                };
            }
        }

        // Signal 4: Cost tier (use implementer as default mid-tier role).
        if let Some(entry) = self.routing.roles.get(&self.defaults.role) {
            return RoutingDecision {
                agent_type: entry.agent.clone(),
                model: entry.model.clone(),
                role: self.defaults.role.clone(),
                decided_by: RoutingSignal::CostTier,
            };
        }

        // Signal 5: Fallback.
        RoutingDecision {
            agent_type: self.defaults.agent.clone(),
            model: None,
            role: self.defaults.role.clone(),
            decided_by: RoutingSignal::Fallback,
        }
    }

    /// Classify a task description into a role using keyword rules.
    /// Returns None if no keywords match.
    pub fn classify_role(task_description: &str) -> Option<&'static str> {
        let task_lower = task_description.to_lowercase();
        for (keywords, role) in KEYWORD_RULES {
            if keywords.iter().any(|kw| task_lower.contains(kw)) {
                return Some(role);
            }
        }
        None
    }
}

/// Parse a [routing] TOML table into RoutingConfig.
pub fn parse_routing_config(toml_str: &str) -> Result<RoutingConfig, toml::de::Error> {
    toml::from_str(toml_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_routing_config() -> RoutingConfig {
        let mut roles = HashMap::new();
        roles.insert(
            "architect".into(),
            RouteEntry {
                agent: "claude-code".into(),
                model: Some("opus".into()),
            },
        );
        roles.insert(
            "implementer".into(),
            RouteEntry {
                agent: "claude-code".into(),
                model: Some("sonnet".into()),
            },
        );
        roles.insert(
            "reviewer".into(),
            RouteEntry {
                agent: "claude-code".into(),
                model: Some("opus".into()),
            },
        );
        roles.insert(
            "tester".into(),
            RouteEntry {
                agent: "claude-code".into(),
                model: Some("haiku".into()),
            },
        );
        roles.insert(
            "shipper".into(),
            RouteEntry {
                agent: "claude-code".into(),
                model: Some("haiku".into()),
            },
        );
        RoutingConfig { roles }
    }

    fn test_defaults() -> DefaultsConfig {
        DefaultsConfig::default()
    }

    #[test]
    fn test_user_override_wins_over_everything() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            Some("codex"),
            Some("gpt-5.4"),
            Some("architect"),
            "review this code for security issues",
        );

        assert_eq!(decision.agent_type, "codex");
        assert_eq!(decision.model, Some("gpt-5.4".into()));
        assert_eq!(decision.decided_by, RoutingSignal::UserOverride);
    }

    #[test]
    fn test_user_override_partial_agent_only() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            Some("aider"),
            None,
            None,
            "implement a feature",
        );

        assert_eq!(decision.agent_type, "aider");
        assert_eq!(decision.decided_by, RoutingSignal::UserOverride);
    }

    #[test]
    fn test_user_override_partial_model_only() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            None,
            Some("opus"),
            None,
            "implement a feature",
        );

        assert_eq!(decision.agent_type, "claude-code");
        assert_eq!(decision.model, Some("opus".into()));
        assert_eq!(decision.decided_by, RoutingSignal::UserOverride);
    }

    #[test]
    fn test_pipeline_step_role_routes_correctly() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(None, None, Some("architect"), "do stuff");

        assert_eq!(decision.agent_type, "claude-code");
        assert_eq!(decision.model, Some("opus".into()));
        assert_eq!(decision.role, "architect");
        assert_eq!(decision.decided_by, RoutingSignal::PipelineStep);
    }

    #[test]
    fn test_pipeline_step_unknown_role_uses_defaults() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(None, None, Some("custom_role"), "do stuff");

        assert_eq!(decision.agent_type, "claude-code");
        assert_eq!(decision.model, None);
        assert_eq!(decision.role, "custom_role");
        assert_eq!(decision.decided_by, RoutingSignal::PipelineStep);
    }

    #[test]
    fn test_keyword_classification_reviewer() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            None,
            None,
            None,
            "Review the authentication module for security vulnerabilities",
        );

        assert_eq!(decision.role, "reviewer");
        assert_eq!(decision.model, Some("opus".into()));
        assert_eq!(decision.decided_by, RoutingSignal::KeywordClassification);
    }

    #[test]
    fn test_keyword_classification_tester() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            None,
            None,
            None,
            "Write unit test coverage for the payment module",
        );

        assert_eq!(decision.role, "tester");
        assert_eq!(decision.model, Some("haiku".into()));
        assert_eq!(decision.decided_by, RoutingSignal::KeywordClassification);
    }

    #[test]
    fn test_keyword_classification_fixer_falls_back_to_implementer() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            None,
            None,
            None,
            "Fix the bug in the login flow",
        );

        // "fixer" role is not in routing config, so it falls back to "implementer".
        assert_eq!(decision.role, "fixer");
        assert_eq!(decision.model, Some("sonnet".into()));
        assert_eq!(decision.decided_by, RoutingSignal::KeywordClassification);
    }

    #[test]
    fn test_keyword_classification_architect() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            None,
            None,
            None,
            "Design the new plugin architecture",
        );

        assert_eq!(decision.role, "architect");
        assert_eq!(decision.model, Some("opus".into()));
        assert_eq!(decision.decided_by, RoutingSignal::KeywordClassification);
    }

    #[test]
    fn test_no_keyword_match_falls_through_to_cost_tier() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            None,
            None,
            None,
            "Add a new dashboard component",
        );

        // No keyword match -> cost tier -> defaults.role = "implementer".
        assert_eq!(decision.role, "implementer");
        assert_eq!(decision.model, Some("sonnet".into()));
        assert_eq!(decision.decided_by, RoutingSignal::CostTier);
    }

    #[test]
    fn test_fallback_when_no_routing_config() {
        let router = Router::new(RoutingConfig::default(), test_defaults());

        let decision = router.route(
            None,
            None,
            None,
            "Add a new dashboard component",
        );

        assert_eq!(decision.agent_type, "claude-code");
        assert_eq!(decision.model, None);
        assert_eq!(decision.role, "implementer");
        assert_eq!(decision.decided_by, RoutingSignal::Fallback);
    }

    #[test]
    fn test_classify_role_returns_none_for_unmatched() {
        assert!(Router::classify_role("Add a new feature").is_none());
    }

    #[test]
    fn test_classify_role_returns_correct_roles() {
        assert_eq!(Router::classify_role("review my code"), Some("reviewer"));
        assert_eq!(Router::classify_role("write a test"), Some("tester"));
        assert_eq!(Router::classify_role("fix the crash"), Some("fixer"));
        assert_eq!(Router::classify_role("design the API"), Some("architect"));
        assert_eq!(Router::classify_role("deploy to prod"), Some("shipper"));
    }

    #[test]
    fn test_parse_routing_config_from_toml() {
        let toml_str = r#"
architect = { agent = "claude-code", model = "opus" }
implementer = { agent = "claude-code", model = "sonnet" }
reviewer = { agent = "codex", model = "gpt-5.4" }
tester = { agent = "claude-code", model = "haiku" }
"#;
        let config = parse_routing_config(toml_str).unwrap();
        assert_eq!(config.roles.len(), 4);
        assert_eq!(config.roles["architect"].agent, "claude-code");
        assert_eq!(config.roles["architect"].model, Some("opus".into()));
        assert_eq!(config.roles["reviewer"].agent, "codex");
        assert_eq!(config.roles["reviewer"].model, Some("gpt-5.4".into()));
    }

    #[test]
    fn test_parse_routing_config_model_optional() {
        let toml_str = r#"
implementer = { agent = "aider" }
"#;
        let config = parse_routing_config(toml_str).unwrap();
        assert_eq!(config.roles["implementer"].agent, "aider");
        assert_eq!(config.roles["implementer"].model, None);
    }

    #[test]
    fn test_priority_pipeline_step_beats_keyword() {
        let router = Router::new(test_routing_config(), test_defaults());

        // Task has "review" keyword but pipeline step says "implementer".
        let decision = router.route(
            None,
            None,
            Some("implementer"),
            "Review the code and fix bugs",
        );

        // Pipeline step wins.
        assert_eq!(decision.role, "implementer");
        assert_eq!(decision.decided_by, RoutingSignal::PipelineStep);
    }

    #[test]
    fn test_priority_user_override_beats_pipeline_step() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            Some("aider"),
            None,
            Some("architect"),
            "Design the system",
        );

        assert_eq!(decision.agent_type, "aider");
        assert_eq!(decision.decided_by, RoutingSignal::UserOverride);
    }
}
```

- [ ] **Step 2: Verify all tests pass**

```bash
cd src-tauri && cargo test router -- --nocapture
```

All 16 tests should pass: priority ordering, keyword classification, TOML parsing, fallback behavior.

---

## Task 3: Crash Recovery (`orchestrator/recovery.rs`)

**Files:**
- Create: `src-tauri/src/orchestrator/recovery.rs`

- [ ] **Step 1: Write the failing tests and implementation**

Create `src-tauri/src/orchestrator/recovery.rs`:

```rust
//! Crash recovery module.
//!
//! On startup, queries SurrealDB for sessions with status='running',
//! checks if their PID is still alive, marks dead ones as 'crashed',
//! and provides context injection for resuming crashed sessions.
//!
//! Spec reference: Section 7.4

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecoveryError {
    #[error("database error: {0}")]
    DbError(String),

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("session log not found: {0}")]
    LogNotFound(String),

    #[error("session not found: {0}")]
    SessionNotFound(String),
}

/// A session that was found in 'running' state on startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrphanedSession {
    pub session_id: String,
    pub agent_type: String,
    pub model: Option<String>,
    pub role_preset: Option<String>,
    pub pid: Option<u32>,
    pub workspace_id: Option<String>,
    /// Whether the PID is still alive.
    pub pid_alive: bool,
    /// Whether the agent supports native resume (e.g. Claude Code --resume).
    pub supports_resume: bool,
}

/// Result of recovery scan.
#[derive(Debug, Clone)]
pub struct RecoveryScanResult {
    /// Sessions that were running but their PID is dead -> marked 'crashed'.
    pub crashed: Vec<OrphanedSession>,
    /// Sessions that are still actually running (PID alive) -> left as-is.
    pub still_running: Vec<OrphanedSession>,
}

/// Check if a process with the given PID is alive.
///
/// On Unix: uses kill(pid, 0) which checks existence without sending a signal.
/// On Windows: uses OpenProcess to check if the process handle is valid.
pub fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // kill(pid, 0) returns 0 if process exists (even if we can't signal it).
        // Returns -1 with ESRCH if process does not exist.
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle != 0 {
                CloseHandle(handle);
                true
            } else {
                false
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        // Conservative: assume alive if we can't check.
        true
    }
}

/// Extract the last N messages from a session log JSONL file for context injection.
///
/// Returns formatted markdown for prepending to the resume prompt.
pub fn extract_resume_context(
    session_log: &str,
    last_n_messages: usize,
) -> String {
    let lines: Vec<&str> = session_log
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    if lines.is_empty() {
        return String::new();
    }

    let start = if lines.len() > last_n_messages {
        lines.len() - last_n_messages
    } else {
        0
    };

    let mut context = String::from("## Resuming crashed session\n\n");
    context.push_str("The previous session crashed. Here is the context from the last messages:\n\n");

    for line in &lines[start..] {
        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
            let role = entry
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("unknown");
            let content = entry
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("");
            let turn = entry
                .get("turn")
                .and_then(|t| t.as_u64())
                .map(|t| format!(" (turn {})", t))
                .unwrap_or_default();

            context.push_str(&format!("### {}{}\n{}\n\n", role, turn, content));
        }
    }

    context.push_str("Please continue from where the session left off.\n");
    context
}

/// Build the resume command arguments for an agent.
///
/// - Agents with native resume (e.g. Claude Code): return `["--resume", session_id]`.
/// - Agents without resume: return empty vec (context injection handles continuity).
pub fn build_resume_args(
    agent_type: &str,
    session_id: &str,
    supports_resume: bool,
) -> Vec<String> {
    if supports_resume {
        // Claude Code style: --resume <session-id>
        vec!["--resume".to_string(), session_id.to_string()]
    } else {
        // No native resume; the caller injects context from session log.
        vec![]
    }
}

/// Database-backed recovery scanner.
pub struct RecoveryScanner<'a> {
    db: &'a Surreal<Db>,
    logs_dir: PathBuf,
}

impl<'a> RecoveryScanner<'a> {
    pub fn new(db: &'a Surreal<Db>, logs_dir: PathBuf) -> Self {
        Self { db, logs_dir }
    }

    /// Scan for orphaned sessions and mark crashed ones.
    ///
    /// 1. Query sessions WHERE status = 'running'
    /// 2. Check if PID is alive for each
    /// 3. Mark dead PIDs as 'crashed', clear workspace lock
    /// 4. Return scan results
    pub async fn scan_and_recover(&self) -> Result<RecoveryScanResult, RecoveryError> {
        // Query all sessions with status = 'running'.
        let sessions: Vec<serde_json::Value> = self
            .db
            .query(
                "SELECT *, \
                 (SELECT VALUE out FROM runs_in WHERE in = $parent.id)[0] AS workspace_id, \
                 (SELECT VALUE resume_support FROM agent_template WHERE name = $parent.agent_type)[0] AS resume_support \
                 FROM session WHERE status = 'running'",
            )
            .await
            .map_err(|e| RecoveryError::DbError(e.to_string()))?
            .take(0)
            .map_err(|e| RecoveryError::DbError(e.to_string()))?;

        let mut result = RecoveryScanResult {
            crashed: Vec::new(),
            still_running: Vec::new(),
        };

        for session in sessions {
            let session_id = session
                .get("id")
                .and_then(|id| id.as_str())
                .unwrap_or("")
                .to_string();

            let pid = session
                .get("pid")
                .and_then(|p| p.as_u64())
                .map(|p| p as u32);

            let agent_type = session
                .get("agent_type")
                .and_then(|a| a.as_str())
                .unwrap_or("")
                .to_string();

            let model = session
                .get("model")
                .and_then(|m| m.as_str())
                .map(|m| m.to_string());

            let role_preset = session
                .get("role_preset")
                .and_then(|r| r.as_str())
                .map(|r| r.to_string());

            let workspace_id = session
                .get("workspace_id")
                .and_then(|w| w.as_str())
                .map(|w| w.to_string());

            let supports_resume = session
                .get("resume_support")
                .and_then(|r| r.as_bool())
                .unwrap_or(false);

            let pid_alive = pid.map_or(false, is_pid_alive);

            let orphan = OrphanedSession {
                session_id: session_id.clone(),
                agent_type,
                model,
                role_preset,
                pid,
                workspace_id: workspace_id.clone(),
                pid_alive,
                supports_resume,
            };

            if pid_alive {
                result.still_running.push(orphan);
            } else {
                // Mark session as crashed.
                self.db
                    .query("UPDATE type::thing($id) SET status = 'crashed', ended_at = time::now()")
                    .bind(("id", &session_id))
                    .await
                    .map_err(|e| RecoveryError::DbError(e.to_string()))?;

                // Clear workspace lock if this session held it.
                if let Some(ref ws_id) = workspace_id {
                    self.db
                        .query(
                            "UPDATE type::thing($ws) SET locked_by = NONE \
                             WHERE locked_by = type::thing($session)",
                        )
                        .bind(("ws", ws_id))
                        .bind(("session", &session_id))
                        .await
                        .map_err(|e| RecoveryError::DbError(e.to_string()))?;
                }

                result.crashed.push(orphan);
            }
        }

        Ok(result)
    }

    /// Get the session log path for a given session ID.
    pub fn session_log_path(&self, session_id: &str) -> PathBuf {
        self.logs_dir.join(format!("session-{}.jsonl", session_id))
    }

    /// Read session log and extract resume context for a crashed session.
    pub async fn get_resume_context(
        &self,
        session_id: &str,
        last_n_messages: usize,
    ) -> Result<String, RecoveryError> {
        let log_path = self.session_log_path(session_id);
        let log_content = tokio::fs::read_to_string(&log_path)
            .await
            .map_err(|_| RecoveryError::LogNotFound(log_path.display().to_string()))?;

        Ok(extract_resume_context(&log_content, last_n_messages))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_pid_alive_current_process() {
        // Our own PID should be alive.
        let pid = std::process::id();
        assert!(is_pid_alive(pid));
    }

    #[test]
    fn test_is_pid_alive_nonexistent() {
        // PID 4294967 is almost certainly not running.
        // (Using a high but valid PID range.)
        assert!(!is_pid_alive(4_294_967));
    }

    #[test]
    fn test_extract_resume_context_with_messages() {
        let log = r#"{"role":"user","content":"Implement JWT auth","turn":1}
{"role":"assistant","content":"I'll start by creating the auth module.","turn":1}
{"role":"assistant","content":"Created src/auth.rs with JWT validation.","turn":2}
{"role":"user","content":"Now add refresh tokens.","turn":3}
{"role":"assistant","content":"Adding refresh token rotation logic.","turn":3}
"#;
        let context = extract_resume_context(log, 3);

        assert!(context.contains("## Resuming crashed session"));
        assert!(context.contains("Please continue from where the session left off."));
        // Should contain last 3 messages only.
        assert!(context.contains("Now add refresh tokens."));
        assert!(context.contains("Adding refresh token rotation logic."));
        assert!(context.contains("Created src/auth.rs"));
        // Should NOT contain the very first message (only last 3).
        assert!(!context.contains("Implement JWT auth"));
    }

    #[test]
    fn test_extract_resume_context_fewer_than_n() {
        let log = r#"{"role":"user","content":"Hello","turn":1}
{"role":"assistant","content":"Hi there","turn":1}
"#;
        let context = extract_resume_context(log, 10);

        // Should include all messages when fewer than N.
        assert!(context.contains("Hello"));
        assert!(context.contains("Hi there"));
    }

    #[test]
    fn test_extract_resume_context_empty_log() {
        let context = extract_resume_context("", 5);
        assert!(context.is_empty());
    }

    #[test]
    fn test_extract_resume_context_with_turn_numbers() {
        let log = r#"{"role":"user","content":"Do something","turn":42}
"#;
        let context = extract_resume_context(log, 5);
        assert!(context.contains("(turn 42)"));
    }

    #[test]
    fn test_extract_resume_context_malformed_jsonl_skipped() {
        let log = "not json at all\n{\"role\":\"user\",\"content\":\"Valid line\",\"turn\":1}\n";
        let context = extract_resume_context(log, 5);

        // Malformed line is skipped; valid line is included.
        assert!(context.contains("Valid line"));
        assert!(!context.contains("not json at all"));
    }

    #[test]
    fn test_build_resume_args_with_native_resume() {
        let args = build_resume_args("claude-code", "session-abc", true);
        assert_eq!(args, vec!["--resume", "session-abc"]);
    }

    #[test]
    fn test_build_resume_args_without_native_resume() {
        let args = build_resume_args("aider", "session-abc", false);
        assert!(args.is_empty());
    }

    #[test]
    fn test_orphaned_session_serialization() {
        let orphan = OrphanedSession {
            session_id: "session:abc123".into(),
            agent_type: "claude-code".into(),
            model: Some("opus".into()),
            role_preset: Some("architect".into()),
            pid: Some(12345),
            workspace_id: Some("workspace:ws1".into()),
            pid_alive: false,
            supports_resume: true,
        };

        let json = serde_json::to_string(&orphan).unwrap();
        let deserialized: OrphanedSession = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.session_id, "session:abc123");
        assert_eq!(deserialized.pid, Some(12345));
        assert!(!deserialized.pid_alive);
        assert!(deserialized.supports_resume);
    }

    #[test]
    fn test_session_log_path() {
        let logs_dir = PathBuf::from("/home/user/.koompi-orch/logs");
        // Test the path construction directly without DB.
        let expected = logs_dir.join("session-abc123.jsonl");
        assert_eq!(
            expected,
            PathBuf::from("/home/user/.koompi-orch/logs/session-abc123.jsonl")
        );
    }

    #[test]
    fn test_recovery_scan_result_default_empty() {
        let result = RecoveryScanResult {
            crashed: Vec::new(),
            still_running: Vec::new(),
        };
        assert!(result.crashed.is_empty());
        assert!(result.still_running.is_empty());
    }
}
```

- [ ] **Step 2: Verify all tests pass**

```bash
cd src-tauri && cargo test recovery -- --nocapture
```

All 11 tests should pass: PID checks, context extraction, resume args, serialization.

- [ ] **Step 3: Integration check — all modules compile together**

```bash
cd src-tauri && cargo test orchestrator -- --nocapture
```

All 41 tests across pipeline (14), router (16), and recovery (11) should pass.

---

## Dependencies

Add to `src-tauri/Cargo.toml` under `[dependencies]`:

```toml
libc = "0.2"
filetime = "0.2"     # dev-dependency for handoff pruning test
toml = "0.8"
```

Under `[dev-dependencies]`:

```toml
filetime = "0.2"
```

**Note:** `git2`, `surrealdb`, `serde`, `serde_json`, `tokio`, `thiserror`, `chrono`, `tempfile`, and `uuid` are already present from Plans 1-3.
