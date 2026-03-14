//! Crash recovery module.
//!
//! On startup, queries SurrealDB for sessions with status='running',
//! checks if their PID is still alive, marks dead ones as 'crashed',
//! and provides context injection for resuming crashed sessions.
//!
//! Spec reference: Section 7.4

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
pub fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
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
        true
    }
}

/// Extract the last N messages from a session log JSONL file for context injection.
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
pub fn build_resume_args(
    _agent_type: &str,
    session_id: &str,
    supports_resume: bool,
) -> Vec<String> {
    if supports_resume {
        vec!["--resume".to_string(), session_id.to_string()]
    } else {
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
    pub async fn scan_and_recover(&self) -> Result<RecoveryScanResult, RecoveryError> {
        let sessions: Vec<serde_json::Value> = self
            .db
            .query(
                "SELECT * FROM session WHERE status = 'running'",
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
                    .bind(("id", session_id.clone()))
                    .await
                    .map_err(|e| RecoveryError::DbError(e.to_string()))?;

                // Clear workspace lock if this session held it.
                if let Some(ref ws_id) = workspace_id {
                    self.db
                        .query(
                            "UPDATE type::thing($ws) SET locked_by = NONE \
                             WHERE locked_by = type::thing($session)",
                        )
                        .bind(("ws", ws_id.clone()))
                        .bind(("session", session_id.clone()))
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
        let pid = std::process::id();
        assert!(is_pid_alive(pid));
    }

    #[test]
    fn test_is_pid_alive_nonexistent() {
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
        assert!(context.contains("Now add refresh tokens."));
        assert!(context.contains("Adding refresh token rotation logic."));
        assert!(context.contains("Created src/auth.rs"));
        assert!(!context.contains("Implement JWT auth"));
    }

    #[test]
    fn test_extract_resume_context_fewer_than_n() {
        let log = r#"{"role":"user","content":"Hello","turn":1}
{"role":"assistant","content":"Hi there","turn":1}
"#;
        let context = extract_resume_context(log, 10);

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
