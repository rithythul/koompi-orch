//! Workspace mutual exclusion via SurrealDB locked_by field.
//!
//! Each workspace has a `locked_by` field storing the session ID as a string.
//! Before spawning an agent, check the lock:
//! - If locked_by = NONE: acquire, proceed
//! - If locked by a running session: reject
//! - If locked by a dead session: clear stale lock, then acquire

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

#[derive(serde::Deserialize)]
struct LockedByRecord {
    locked_by: Option<String>,
}

#[derive(serde::Deserialize)]
struct StatusRecord {
    status: String,
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
    pub async fn acquire(
        &self,
        workspace_id: &str,
        session_id: &str,
    ) -> Result<LockResult, LockError> {
        let result: Vec<LockedByRecord> = self
            .db
            .query("SELECT locked_by FROM type::thing('workspace', $ws_id)")
            .bind(("ws_id", workspace_id.to_string()))
            .await?
            .take(0)?;

        let locked_by = result.first().and_then(|r| r.locked_by.clone());

        match locked_by {
            None => {
                self.set_lock(workspace_id, session_id).await?;
                Ok(LockResult::Acquired)
            }
            Some(existing_session_id) => {
                // Check if the holding session is still running
                // Parse session table and id from "session:xyz" format
                let session_results: Vec<StatusRecord> = self
                    .db
                    .query(&format!(
                        "SELECT status FROM {}",
                        existing_session_id
                    ))
                    .await?
                    .take(0)?;

                let is_running = session_results
                    .first()
                    .map(|r| r.status == "running")
                    .unwrap_or(false);

                if is_running {
                    Err(LockError::AlreadyLocked {
                        workspace_id: workspace_id.to_string(),
                        session_id: existing_session_id,
                    })
                } else {
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
            .query("UPDATE type::thing('workspace', $ws_id) SET locked_by = NONE, updated_at = time::now()")
            .bind(("ws_id", workspace_id.to_string()))
            .await?;
        Ok(())
    }

    /// Check if a workspace is currently locked.
    pub async fn is_locked(&self, workspace_id: &str) -> Result<Option<String>, LockError> {
        let result: Vec<LockedByRecord> = self
            .db
            .query("SELECT locked_by FROM type::thing('workspace', $ws_id)")
            .bind(("ws_id", workspace_id.to_string()))
            .await?
            .take(0)?;

        Ok(result.first().and_then(|r| r.locked_by.clone()))
    }

    /// Internal: set the locked_by field as a string.
    async fn set_lock(
        &self,
        workspace_id: &str,
        session_id: &str,
    ) -> Result<(), LockError> {
        self.db
            .query("UPDATE type::thing('workspace', $ws_id) SET locked_by = $sid, updated_at = time::now()")
            .bind(("ws_id", workspace_id.to_string()))
            .bind(("sid", session_id.to_string()))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use surrealdb::engine::local::Mem;

    async fn setup_db() -> Surreal<Db> {
        let db: Surreal<Db> = Surreal::new::<Mem>(()).await.unwrap();
        db.use_ns("test").use_db("test").await.unwrap();

        db.query(
            "DEFINE TABLE workspace SCHEMAFULL;
             DEFINE FIELD name ON workspace TYPE string;
             DEFINE FIELD branch ON workspace TYPE string;
             DEFINE FIELD worktree_path ON workspace TYPE string;
             DEFINE FIELD status ON workspace TYPE string;
             DEFINE FIELD locked_by ON workspace TYPE option<string>;
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
        create_session(&db, "s4", "running").await;
        create_session(&db, "s5", "running").await;

        let lock = WorkspaceLock::new(&db);
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
