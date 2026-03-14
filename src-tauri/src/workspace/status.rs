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
            .bind(("ws_id", workspace_id.to_string()))
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
                "UPDATE type::thing('workspace', $ws_id) SET status = $status, updated_at = time::now()",
            )
            .bind(("ws_id", workspace_id.to_string()))
            .bind(("status", target.as_str().to_string()))
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
            .bind(("ws_id", workspace_id.to_string()))
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
        let db: Surreal<Db> = Surreal::new::<Mem>(()).await.unwrap();
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
        sm.transition("ws4", WorkspaceStatus::Backlog)
            .await
            .unwrap();

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
