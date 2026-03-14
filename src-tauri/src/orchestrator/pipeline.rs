//! Pipeline execution engine.
//!
//! Runs pipeline steps sequentially, manages handoff context between agents,
//! writes handoff files to ~/.koompi-orch/handoffs/, and appends auto-review
//! as an implicit last step when configured.

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
pub fn generate_handoff_content(
    handoff_type: &HandoffType,
    _session_log_path: &Path,
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
            let max_chars = 100_000;
            let content = if session_log.len() > max_chars {
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
        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(name) = entry.get("name").and_then(|n| n.as_str()) {
                if entry.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    if !tools_used.contains(&name.to_string()) {
                        tools_used.push(name.to_string());
                    }
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
            if entry.get("role").and_then(|r| r.as_str()) == Some("assistant") {
                if let Some(content) = entry.get("content").and_then(|c| c.as_str()) {
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

    let mut opts = git2::DiffOptions::new();
    opts.include_untracked(true);

    let diff = repo
        .diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut opts))
        .map_err(|e| PipelineError::HandoffFailed(format!("git diff: {}", e)))?;

    let mut diff_text = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        if let Ok(content) = std::str::from_utf8(line.content()) {
            let origin = line.origin();
            match origin {
                '+' | '-' | ' ' | 'H' | '>' | '<' => {
                    if origin == '+' || origin == '-' || origin == ' ' {
                        diff_text.push(origin);
                    }
                    diff_text.push_str(content);
                }
                _ => {
                    diff_text.push_str(content);
                }
            }
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
        _pipeline_id: &str,
        _workspace_id: &str,
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

        serde_json::from_value(serde_json::Value::String(run_id_str.to_string()))
            .map_err(|e| PipelineError::DbError(e.to_string()))
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
        let summary_owned = summary.map(|s| s.to_string());
        let cf_owned = context_file.map(|s| s.to_string());
        self.db
            .query(
                "RELATE type::thing($from) -> hands_off_to -> type::thing($to) \
                 SET handoff_type = $ht, output_summary = $summary, context_file = $cf",
            )
            .bind(("from", format!("session:{}", from_session_id)))
            .bind(("to", format!("session:{}", to_session_id)))
            .bind(("ht", handoff_type.as_str().to_string()))
            .bind(("summary", summary_owned))
            .bind(("cf", cf_owned))
            .await
            .map_err(|e| PipelineError::DbError(e.to_string()))?;

        Ok(())
    }

    /// Check if auto-review should be appended as implicit last step.
    pub fn should_append_auto_review(
        steps: &[PipelineStep],
        auto_review_enabled: bool,
    ) -> bool {
        if !auto_review_enabled {
            return false;
        }
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

        // Create initial commit with a file.
        std::fs::write(tmp.path().join("base.txt"), "base content").unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[]).unwrap();
        }

        // Modify the tracked file (staged changes show in diff).
        std::fs::write(tmp.path().join("base.txt"), "hello world").unwrap();

        let result = generate_diff(tmp.path()).unwrap();
        assert!(result.contains("hello world"));
    }

    #[test]
    fn test_diff_only_no_changes() {
        let tmp = TempDir::new().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[]).unwrap();
        }

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

        assert!(PipelineExecutor::should_append_auto_review(&steps, true));
        assert!(!PipelineExecutor::should_append_auto_review(&steps, false));

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
