//! Core orchestration engine.
//!
//! Manages a set of running agent sessions. Each session binds an AgentProcess
//! (from agent::process) to a workspace worktree (from workspace::manager).
//! The engine spawns agents, tracks their lifecycle via a HashMap, forwards
//! output events, and handles session completion or failure.

use crate::agent::config::{AgentConfig, AgentError, AgentEvent, PtySize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("session already exists: {0}")]
    SessionAlreadyExists(String),

    #[error("agent error: {0}")]
    Agent(#[from] AgentError),

    #[error("engine is at capacity ({0} sessions)")]
    AtCapacity(usize),

    #[error("session {0} is not running")]
    NotRunning(String),

    #[error("governor denied spawn: {0}")]
    GovernorDenied(String),
}

// ---------------------------------------------------------------------------
// Session types
// ---------------------------------------------------------------------------

/// Current lifecycle state of a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Running,
    Paused,
    Completed,
    Failed,
    Killed,
}

/// Metadata for one running (or finished) agent session.
#[derive(Debug)]
pub struct SessionInfo {
    pub session_id: String,
    pub agent_config: AgentConfig,
    pub workspace_path: PathBuf,
    pub state: SessionState,
    pub pid: Option<u32>,
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub total_cost_usd: f64,
}

/// An event emitted by the engine for a specific session.
#[derive(Debug, Clone)]
pub struct EngineEvent {
    pub session_id: String,
    pub event: AgentEvent,
}

// ---------------------------------------------------------------------------
// Trait for process spawning (enables mock injection)
// ---------------------------------------------------------------------------

/// Abstraction over the process-spawning layer so the engine can be tested
/// without real PTY processes.
pub trait ProcessSpawner: Send + Sync {
    /// Spawn a new agent process. Returns a session handle and an event
    /// receiver. The handle allows killing the process; the receiver streams
    /// AgentEvents from the PTY reader thread.
    fn spawn(
        &self,
        config: AgentConfig,
        size: PtySize,
    ) -> Result<(SpawnedProcess, mpsc::UnboundedReceiver<AgentEvent>), AgentError>;
}

/// Handle returned by `ProcessSpawner::spawn`.
pub struct SpawnedProcess {
    pub pid: u32,
    /// Sender that, when dropped or sent to, kills the child process.
    pub kill_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl std::fmt::Debug for SpawnedProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpawnedProcess")
            .field("pid", &self.pid)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// The orchestration engine. Thread-safe, designed to be wrapped in an Arc
/// and shared across Tauri command handlers.
pub struct Engine {
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
    /// Channel that all session events are funneled into.
    event_tx: mpsc::UnboundedSender<EngineEvent>,
    /// Kill senders keyed by session_id — used to signal the background
    /// reader to stop and kill the child.
    kill_handles: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
    spawner: Arc<dyn ProcessSpawner>,
    max_sessions: usize,
}

impl Engine {
    /// Create a new engine.
    ///
    /// * `spawner`      — process spawner (production: wraps AgentProcess)
    /// * `max_sessions` — hard ceiling on concurrent sessions (governor may
    ///                     impose a lower limit)
    pub fn new(
        spawner: Arc<dyn ProcessSpawner>,
        max_sessions: usize,
    ) -> (Self, mpsc::UnboundedReceiver<EngineEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let engine = Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            kill_handles: Arc::new(Mutex::new(HashMap::new())),
            spawner,
            max_sessions,
        };
        (engine, event_rx)
    }

    /// Spawn an agent in the given workspace directory.
    ///
    /// Returns the generated session ID. The engine starts a background task
    /// that reads events from the agent and forwards them through the
    /// unified `EngineEvent` channel.
    pub async fn spawn_session(
        &self,
        config: AgentConfig,
        workspace_path: PathBuf,
    ) -> Result<String, EngineError> {
        // Capacity check
        let sessions = self.sessions.read().await;
        let running = sessions
            .values()
            .filter(|s| s.state == SessionState::Running)
            .count();
        if running >= self.max_sessions {
            return Err(EngineError::AtCapacity(self.max_sessions));
        }
        drop(sessions);

        let session_id = Uuid::new_v4().to_string()[..8].to_string();

        // Spawn via the injected spawner
        let (spawned, mut event_rx) = self
            .spawner
            .spawn(config.clone(), PtySize::default())?;

        let info = SessionInfo {
            session_id: session_id.clone(),
            agent_config: config,
            workspace_path,
            state: SessionState::Running,
            pid: Some(spawned.pid),
            total_tokens_in: 0,
            total_tokens_out: 0,
            total_cost_usd: 0.0,
        };

        self.sessions.write().await.insert(session_id.clone(), info);

        if let Some(kill_tx) = spawned.kill_tx {
            self.kill_handles
                .lock()
                .await
                .insert(session_id.clone(), kill_tx);
        }

        // Background task: forward events and update session state
        let sessions_ref = self.sessions.clone();
        let event_tx = self.event_tx.clone();
        let sid = session_id.clone();

        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                // Update accumulators on Usage events
                if let AgentEvent::Usage {
                    tokens_in,
                    tokens_out,
                    cost_usd,
                } = &event
                {
                    let mut sessions = sessions_ref.write().await;
                    if let Some(info) = sessions.get_mut(&sid) {
                        info.total_tokens_in += tokens_in;
                        info.total_tokens_out += tokens_out;
                        info.total_cost_usd += cost_usd;
                    }
                }

                // Detect terminal events
                let is_terminal = matches!(
                    &event,
                    AgentEvent::Completed | AgentEvent::Error { .. }
                );

                let _ = event_tx.send(EngineEvent {
                    session_id: sid.clone(),
                    event: event.clone(),
                });

                if is_terminal {
                    let mut sessions = sessions_ref.write().await;
                    if let Some(info) = sessions.get_mut(&sid) {
                        info.state = match &event {
                            AgentEvent::Completed => SessionState::Completed,
                            AgentEvent::Error { .. } => SessionState::Failed,
                            _ => unreachable!(),
                        };
                    }
                    break;
                }
            }
        });

        Ok(session_id)
    }

    /// Kill a running session.
    pub async fn kill_session(&self, session_id: &str) -> Result<(), EngineError> {
        // Send the kill signal
        let kill_tx = self
            .kill_handles
            .lock()
            .await
            .remove(session_id);

        match kill_tx {
            Some(tx) => {
                let _ = tx.send(());
            }
            None => {
                // No kill handle — check if the session even exists
                let sessions = self.sessions.read().await;
                if !sessions.contains_key(session_id) {
                    return Err(EngineError::SessionNotFound(session_id.to_string()));
                }
            }
        }

        // Mark as killed
        let mut sessions = self.sessions.write().await;
        if let Some(info) = sessions.get_mut(session_id) {
            info.state = SessionState::Killed;
        }

        Ok(())
    }

    /// Get a snapshot of all sessions.
    pub async fn list_sessions(&self) -> Vec<(String, SessionState)> {
        let sessions = self.sessions.read().await;
        sessions
            .iter()
            .map(|(id, info)| (id.clone(), info.state))
            .collect()
    }

    /// Get details for a single session.
    pub async fn get_session(&self, session_id: &str) -> Result<SessionState, EngineError> {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .map(|s| s.state)
            .ok_or_else(|| EngineError::SessionNotFound(session_id.to_string()))
    }

    /// Number of currently running sessions.
    pub async fn running_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| s.state == SessionState::Running)
            .count()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::config::{AgentConfig, AgentEvent, InputMode, OutputMode};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tokio::sync::mpsc;

    /// A fake spawner that returns a controllable event channel and records
    /// spawn calls. Does not spawn real processes.
    struct FakeSpawner {
        /// Pre-loaded events that will be sent when spawn() is called.
        events: Arc<Mutex<Vec<AgentEvent>>>,
        /// Counter of spawn calls.
        spawn_count: Arc<std::sync::atomic::AtomicUsize>,
        /// If true, spawn() returns an error.
        fail: bool,
    }

    impl FakeSpawner {
        fn new(events: Vec<AgentEvent>) -> Self {
            Self {
                events: Arc::new(Mutex::new(events)),
                spawn_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                events: Arc::new(Mutex::new(vec![])),
                spawn_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                fail: true,
            }
        }

        fn call_count(&self) -> usize {
            self.spawn_count
                .load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    impl ProcessSpawner for FakeSpawner {
        fn spawn(
            &self,
            _config: AgentConfig,
            _size: PtySize,
        ) -> Result<(SpawnedProcess, mpsc::UnboundedReceiver<AgentEvent>), AgentError> {
            self.spawn_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            if self.fail {
                return Err(AgentError::SpawnFailed(
                    "fake spawn failure".to_string(),
                ));
            }

            let (tx, rx) = mpsc::unbounded_channel();
            let (kill_tx, _kill_rx) = tokio::sync::oneshot::channel();

            // Send pre-loaded events in background
            let events = self.events.clone();
            tokio::spawn(async move {
                let events = events.lock().await;
                for event in events.iter() {
                    if tx.send(event.clone()).is_err() {
                        break;
                    }
                }
            });

            Ok((
                SpawnedProcess {
                    pid: 42,
                    kill_tx: Some(kill_tx),
                },
                rx,
            ))
        }
    }

    fn test_config() -> AgentConfig {
        AgentConfig {
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            env: HashMap::new(),
            input_mode: InputMode::PtyStdin,
            output_mode: OutputMode::JsonStream,
            resume_support: false,
            working_dir: PathBuf::from("/tmp"),
            message_flag: None,
            print_flag: None,
            resume_flag: None,
        }
    }

    #[tokio::test]
    async fn test_spawn_session_returns_session_id() {
        let spawner = Arc::new(FakeSpawner::new(vec![AgentEvent::Completed]));
        let (engine, _event_rx) = Engine::new(spawner.clone(), 10);

        let session_id = engine
            .spawn_session(test_config(), PathBuf::from("/tmp/ws"))
            .await
            .unwrap();

        assert_eq!(session_id.len(), 8);
        assert_eq!(spawner.call_count(), 1);
    }

    #[tokio::test]
    async fn test_spawn_session_forwards_events() {
        let events = vec![
            AgentEvent::Text {
                content: "hello".to_string(),
            },
            AgentEvent::Usage {
                tokens_in: 100,
                tokens_out: 200,
                cost_usd: 0.05,
            },
            AgentEvent::Completed,
        ];
        let spawner = Arc::new(FakeSpawner::new(events));
        let (engine, mut event_rx) = Engine::new(spawner, 10);

        let sid = engine
            .spawn_session(test_config(), PathBuf::from("/tmp/ws"))
            .await
            .unwrap();

        // Collect forwarded events
        let mut received = Vec::new();
        while let Some(eng_event) = event_rx.recv().await {
            assert_eq!(eng_event.session_id, sid);
            received.push(eng_event.event);
            if matches!(received.last(), Some(AgentEvent::Completed)) {
                break;
            }
        }

        assert_eq!(received.len(), 3);
        assert!(matches!(received[0], AgentEvent::Text { .. }));
        assert!(matches!(received[1], AgentEvent::Usage { .. }));
        assert!(matches!(received[2], AgentEvent::Completed));
    }

    #[tokio::test]
    async fn test_session_state_transitions_to_completed() {
        let spawner = Arc::new(FakeSpawner::new(vec![AgentEvent::Completed]));
        let (engine, mut event_rx) = Engine::new(spawner, 10);

        let sid = engine
            .spawn_session(test_config(), PathBuf::from("/tmp/ws"))
            .await
            .unwrap();

        // Drain events until completed
        while let Some(e) = event_rx.recv().await {
            if matches!(e.event, AgentEvent::Completed) {
                break;
            }
        }

        // Allow the background task to update state
        tokio::task::yield_now().await;

        let state = engine.get_session(&sid).await.unwrap();
        assert_eq!(state, SessionState::Completed);
    }

    #[tokio::test]
    async fn test_session_state_transitions_to_failed() {
        let spawner = Arc::new(FakeSpawner::new(vec![AgentEvent::Error {
            message: "boom".to_string(),
        }]));
        let (engine, mut event_rx) = Engine::new(spawner, 10);

        let sid = engine
            .spawn_session(test_config(), PathBuf::from("/tmp/ws"))
            .await
            .unwrap();

        while let Some(e) = event_rx.recv().await {
            if matches!(e.event, AgentEvent::Error { .. }) {
                break;
            }
        }

        tokio::task::yield_now().await;

        let state = engine.get_session(&sid).await.unwrap();
        assert_eq!(state, SessionState::Failed);
    }

    #[tokio::test]
    async fn test_usage_accumulation() {
        let events = vec![
            AgentEvent::Usage {
                tokens_in: 100,
                tokens_out: 50,
                cost_usd: 0.01,
            },
            AgentEvent::Usage {
                tokens_in: 200,
                tokens_out: 100,
                cost_usd: 0.02,
            },
            AgentEvent::Completed,
        ];
        let spawner = Arc::new(FakeSpawner::new(events));
        let (engine, mut event_rx) = Engine::new(spawner, 10);

        let sid = engine
            .spawn_session(test_config(), PathBuf::from("/tmp/ws"))
            .await
            .unwrap();

        // Drain all events
        while let Some(e) = event_rx.recv().await {
            if matches!(e.event, AgentEvent::Completed) {
                break;
            }
        }

        tokio::task::yield_now().await;

        // Verify accumulated usage
        let sessions = engine.sessions.read().await;
        let info = sessions.get(&sid).unwrap();
        assert_eq!(info.total_tokens_in, 300);
        assert_eq!(info.total_tokens_out, 150);
        assert!((info.total_cost_usd - 0.03).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_capacity_enforcement() {
        let spawner = Arc::new(FakeSpawner::new(vec![])); // no Completed => stays Running
        let (engine, _event_rx) = Engine::new(spawner, 2);

        // Fill to capacity
        engine
            .spawn_session(test_config(), PathBuf::from("/tmp/ws1"))
            .await
            .unwrap();
        engine
            .spawn_session(test_config(), PathBuf::from("/tmp/ws2"))
            .await
            .unwrap();

        // Third should fail
        let result = engine
            .spawn_session(test_config(), PathBuf::from("/tmp/ws3"))
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), EngineError::AtCapacity(2)));
    }

    #[tokio::test]
    async fn test_kill_session() {
        let spawner = Arc::new(FakeSpawner::new(vec![]));
        let (engine, _event_rx) = Engine::new(spawner, 10);

        let sid = engine
            .spawn_session(test_config(), PathBuf::from("/tmp/ws"))
            .await
            .unwrap();

        engine.kill_session(&sid).await.unwrap();

        let state = engine.get_session(&sid).await.unwrap();
        assert_eq!(state, SessionState::Killed);
    }

    #[tokio::test]
    async fn test_kill_nonexistent_session() {
        let spawner = Arc::new(FakeSpawner::new(vec![]));
        let (engine, _event_rx) = Engine::new(spawner, 10);

        let result = engine.kill_session("no-such-id").await;
        assert!(matches!(
            result.unwrap_err(),
            EngineError::SessionNotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let spawner = Arc::new(FakeSpawner::new(vec![]));
        let (engine, _event_rx) = Engine::new(spawner, 10);

        let sid1 = engine
            .spawn_session(test_config(), PathBuf::from("/tmp/ws1"))
            .await
            .unwrap();
        let sid2 = engine
            .spawn_session(test_config(), PathBuf::from("/tmp/ws2"))
            .await
            .unwrap();

        let sessions = engine.list_sessions().await;
        assert_eq!(sessions.len(), 2);

        let ids: Vec<&String> = sessions.iter().map(|(id, _)| id).collect();
        assert!(ids.contains(&&sid1));
        assert!(ids.contains(&&sid2));
    }

    #[tokio::test]
    async fn test_running_count() {
        let spawner = Arc::new(FakeSpawner::new(vec![]));
        let (engine, _event_rx) = Engine::new(spawner, 10);

        assert_eq!(engine.running_count().await, 0);

        engine
            .spawn_session(test_config(), PathBuf::from("/tmp/ws1"))
            .await
            .unwrap();

        assert_eq!(engine.running_count().await, 1);
    }

    #[tokio::test]
    async fn test_spawn_failure_propagates() {
        let spawner = Arc::new(FakeSpawner::failing());
        let (engine, _event_rx) = Engine::new(spawner, 10);

        let result = engine
            .spawn_session(test_config(), PathBuf::from("/tmp/ws"))
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), EngineError::Agent(_)));
        assert_eq!(engine.running_count().await, 0);
    }

    #[tokio::test]
    async fn test_get_session_not_found() {
        let spawner = Arc::new(FakeSpawner::new(vec![]));
        let (engine, _event_rx) = Engine::new(spawner, 10);

        let result = engine.get_session("nonexistent").await;
        assert!(matches!(
            result.unwrap_err(),
            EngineError::SessionNotFound(_)
        ));
    }
}
