# Plan 4A: Orchestration Engine and Resource Governor

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the core orchestrator that spawns and monitors agents, plus cost/resource guardrails.
**Architecture:** Engine manages agent lifecycles via agent::process and workspace::manager. Governor enforces concurrency and cost limits.
**Tech Stack:** Rust, tokio, SurrealDB
**Spec Reference:** Sections 7.1, 15a of the spec

---

## Chunk 1: Orchestration Engine

### Task 1: Core engine — spawn, monitor, kill agents (`orchestrator/engine.rs`, `orchestrator/mod.rs`)

**Files:**
- Create: `src-tauri/src/orchestrator/engine.rs`
- Create: `src-tauri/src/orchestrator/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/orchestrator/engine.rs`:
```rust
//! Core orchestration engine.
//!
//! Manages a set of running agent sessions. Each session binds an AgentProcess
//! (from agent::process) to a workspace worktree (from workspace::manager).
//! The engine spawns agents, tracks their lifecycle via a HashMap, forwards
//! output events, and handles session completion or failure.

use crate::agent::config::{AgentConfig, AgentError, AgentEvent, PtySize};
use crate::agent::process::PtySystem;
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
#[cfg_attr(test, mockall::automock)]
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
```

Create `src-tauri/src/orchestrator/mod.rs`:
```rust
pub mod engine;

pub use engine::{Engine, EngineError, EngineEvent, ProcessSpawner, SessionInfo, SessionState, SpawnedProcess};
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test orchestrator::engine::tests`
Expected: FAIL with "can't find crate for `orchestrator`" (module not wired into lib.rs yet)

- [ ] **Step 3: Wire into lib.rs**

Add to `src-tauri/src/lib.rs`:
```rust
pub mod orchestrator;
```

- [ ] **Step 4: Run test to verify it passes**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test orchestrator::engine::tests`
Expected: PASS — all 11 tests pass

- [ ] **Step 5: Commit**
```bash
scripts/committer "feat(orchestrator): add core engine — spawn, monitor, kill agent sessions" src-tauri/src/orchestrator/engine.rs src-tauri/src/orchestrator/mod.rs src-tauri/src/lib.rs
```

---

## Chunk 2: Resource Governor

### Task 2: Cost and concurrency governor (`orchestrator/governor.rs`)

**Files:**
- Create: `src-tauri/src/orchestrator/governor.rs`
- Modify: `src-tauri/src/orchestrator/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/orchestrator/governor.rs`:
```rust
//! Resource governor: enforces concurrency limits, tracks cost per session,
//! pauses agents when cost limits are reached, warns at configurable
//! percentage thresholds.
//!
//! On Unix, pause/resume uses SIGSTOP/SIGCONT. On Windows, the governor
//! records the "should pause" state but delegates actual suspension to the
//! engine (portable-pty does not expose NtSuspendProcess).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum GovernorError {
    #[error("session not tracked: {0}")]
    SessionNotTracked(String),

    #[error("concurrency limit reached ({0}/{0})")]
    ConcurrencyLimitReached(usize),

    #[error("failed to send signal to pid {pid}: {reason}")]
    SignalFailed { pid: u32, reason: String },
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Cost and concurrency limits loaded from config.toml / .orch.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernorConfig {
    /// Max concurrent running agents.
    pub max_concurrent_agents: usize,
    /// Max cost in USD per individual session. 0.0 = unlimited.
    pub max_cost_per_session_usd: f64,
    /// Max total tokens per session. 0 = unlimited.
    pub max_tokens_per_session: u64,
    /// Max cost in USD across all sessions in a pipeline. 0.0 = unlimited.
    pub max_cost_per_pipeline_usd: f64,
    /// Percentage of limit at which to emit a warning (0..100).
    pub warn_at_percent: u8,
}

impl Default for GovernorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_agents: 10,
            max_cost_per_session_usd: 10.0,
            max_tokens_per_session: 500_000,
            max_cost_per_pipeline_usd: 50.0,
            warn_at_percent: 80,
        }
    }
}

// ---------------------------------------------------------------------------
// Governor decisions
// ---------------------------------------------------------------------------

/// Action the governor instructs the engine to take.
#[derive(Debug, Clone, PartialEq)]
pub enum GovernorAction {
    /// Everything within limits, proceed normally.
    Allow,
    /// Cost/token usage is approaching the limit — emit a user notification.
    Warn {
        session_id: String,
        message: String,
        percent: u8,
    },
    /// Limit has been hit — the engine should pause (SIGSTOP) this agent.
    Pause {
        session_id: String,
        reason: String,
    },
}

// ---------------------------------------------------------------------------
// Per-session usage tracker
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct SessionUsage {
    tokens_in: u64,
    tokens_out: u64,
    cost_usd: f64,
    pid: u32,
    paused: bool,
    /// Track whether we already emitted a warning for this session.
    warned: bool,
}

// ---------------------------------------------------------------------------
// Governor
// ---------------------------------------------------------------------------

/// The resource governor tracks usage per session and decides when to warn
/// or pause agents.
pub struct Governor {
    config: GovernorConfig,
    sessions: HashMap<String, SessionUsage>,
}

impl Governor {
    pub fn new(config: GovernorConfig) -> Self {
        Self {
            config,
            sessions: HashMap::new(),
        }
    }

    /// Register a new session with the governor.
    pub fn track_session(&mut self, session_id: &str, pid: u32) {
        self.sessions.insert(
            session_id.to_string(),
            SessionUsage {
                tokens_in: 0,
                tokens_out: 0,
                cost_usd: 0.0,
                pid,
                paused: false,
                warned: false,
            },
        );
    }

    /// Remove a session from tracking (completed/killed).
    pub fn untrack_session(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
    }

    /// Check whether a new session is allowed to spawn (concurrency check).
    pub fn can_spawn(&self) -> bool {
        let active = self
            .sessions
            .values()
            .filter(|s| !s.paused)
            .count();
        active < self.config.max_concurrent_agents
    }

    /// Number of currently active (non-paused) sessions.
    pub fn active_count(&self) -> usize {
        self.sessions.values().filter(|s| !s.paused).count()
    }

    /// Record a usage update for a session and return the governor's decision.
    ///
    /// The engine calls this each time it receives a `Usage` event from an
    /// agent. The governor accumulates totals and checks against limits.
    pub fn record_usage(
        &mut self,
        session_id: &str,
        tokens_in: u64,
        tokens_out: u64,
        cost_usd: f64,
    ) -> Result<GovernorAction, GovernorError> {
        let usage = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| GovernorError::SessionNotTracked(session_id.to_string()))?;

        usage.tokens_in += tokens_in;
        usage.tokens_out += tokens_out;
        usage.cost_usd += cost_usd;

        let total_tokens = usage.tokens_in + usage.tokens_out;

        // Check cost limit
        if self.config.max_cost_per_session_usd > 0.0
            && usage.cost_usd >= self.config.max_cost_per_session_usd
        {
            usage.paused = true;
            return Ok(GovernorAction::Pause {
                session_id: session_id.to_string(),
                reason: format!(
                    "cost limit reached (${:.2}/${:.2})",
                    usage.cost_usd, self.config.max_cost_per_session_usd
                ),
            });
        }

        // Check token limit
        if self.config.max_tokens_per_session > 0
            && total_tokens >= self.config.max_tokens_per_session
        {
            usage.paused = true;
            return Ok(GovernorAction::Pause {
                session_id: session_id.to_string(),
                reason: format!(
                    "token limit reached ({}/{})",
                    total_tokens, self.config.max_tokens_per_session
                ),
            });
        }

        // Check warning threshold (only once)
        if !usage.warned && self.config.warn_at_percent > 0 {
            let warn_threshold = self.config.warn_at_percent as f64 / 100.0;

            let cost_pct = if self.config.max_cost_per_session_usd > 0.0 {
                usage.cost_usd / self.config.max_cost_per_session_usd
            } else {
                0.0
            };

            let token_pct = if self.config.max_tokens_per_session > 0 {
                total_tokens as f64 / self.config.max_tokens_per_session as f64
            } else {
                0.0
            };

            let max_pct = cost_pct.max(token_pct);
            if max_pct >= warn_threshold {
                usage.warned = true;
                let percent = (max_pct * 100.0).round() as u8;
                return Ok(GovernorAction::Warn {
                    session_id: session_id.to_string(),
                    message: format!(
                        "session at {}% of limit (${:.2} cost, {} tokens)",
                        percent, usage.cost_usd, total_tokens
                    ),
                    percent,
                });
            }
        }

        Ok(GovernorAction::Allow)
    }

    /// Mark a session as resumed (un-paused). Called after the user manually
    /// resumes a paused agent.
    pub fn resume_session(
        &mut self,
        session_id: &str,
    ) -> Result<(), GovernorError> {
        let usage = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| GovernorError::SessionNotTracked(session_id.to_string()))?;
        usage.paused = false;
        Ok(())
    }

    /// Check if a specific session is paused by the governor.
    pub fn is_paused(&self, session_id: &str) -> Result<bool, GovernorError> {
        self.sessions
            .get(session_id)
            .map(|s| s.paused)
            .ok_or_else(|| GovernorError::SessionNotTracked(session_id.to_string()))
    }

    /// Get the PID of a tracked session (used by the engine to send
    /// SIGSTOP/SIGCONT).
    pub fn session_pid(&self, session_id: &str) -> Result<u32, GovernorError> {
        self.sessions
            .get(session_id)
            .map(|s| s.pid)
            .ok_or_else(|| GovernorError::SessionNotTracked(session_id.to_string()))
    }

    /// Pause a process via SIGSTOP (Unix only).
    /// On non-Unix platforms, this is a no-op; the caller should use
    /// platform-specific suspension.
    #[cfg(unix)]
    pub fn send_stop(pid: u32) -> Result<(), GovernorError> {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;
        kill(Pid::from_raw(pid as i32), Signal::SIGSTOP).map_err(|e| {
            GovernorError::SignalFailed {
                pid,
                reason: e.to_string(),
            }
        })
    }

    /// Resume a process via SIGCONT (Unix only).
    #[cfg(unix)]
    pub fn send_cont(pid: u32) -> Result<(), GovernorError> {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;
        kill(Pid::from_raw(pid as i32), Signal::SIGCONT).map_err(|e| {
            GovernorError::SignalFailed {
                pid,
                reason: e.to_string(),
            }
        })
    }

    /// No-op on non-Unix.
    #[cfg(not(unix))]
    pub fn send_stop(_pid: u32) -> Result<(), GovernorError> {
        Ok(())
    }

    /// No-op on non-Unix.
    #[cfg(not(unix))]
    pub fn send_cont(_pid: u32) -> Result<(), GovernorError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> GovernorConfig {
        GovernorConfig {
            max_concurrent_agents: 3,
            max_cost_per_session_usd: 10.0,
            max_tokens_per_session: 1_000,
            max_cost_per_pipeline_usd: 50.0,
            warn_at_percent: 80,
        }
    }

    #[test]
    fn test_track_and_can_spawn() {
        let mut gov = Governor::new(default_config());
        assert!(gov.can_spawn());
        assert_eq!(gov.active_count(), 0);

        gov.track_session("s1", 100);
        gov.track_session("s2", 101);
        gov.track_session("s3", 102);

        assert_eq!(gov.active_count(), 3);
        assert!(!gov.can_spawn()); // at capacity
    }

    #[test]
    fn test_untrack_frees_slot() {
        let mut gov = Governor::new(default_config());
        gov.track_session("s1", 100);
        gov.track_session("s2", 101);
        gov.track_session("s3", 102);

        assert!(!gov.can_spawn());

        gov.untrack_session("s2");
        assert!(gov.can_spawn());
        assert_eq!(gov.active_count(), 2);
    }

    #[test]
    fn test_record_usage_allow() {
        let mut gov = Governor::new(default_config());
        gov.track_session("s1", 100);

        let action = gov.record_usage("s1", 10, 5, 0.01).unwrap();
        assert_eq!(action, GovernorAction::Allow);
    }

    #[test]
    fn test_record_usage_cost_warning() {
        let mut gov = Governor::new(default_config());
        gov.track_session("s1", 100);

        // Push to 80% of $10 limit = $8.00
        let action = gov.record_usage("s1", 100, 50, 8.00).unwrap();
        match action {
            GovernorAction::Warn {
                session_id,
                percent,
                ..
            } => {
                assert_eq!(session_id, "s1");
                assert_eq!(percent, 80);
            }
            other => panic!("expected Warn, got {:?}", other),
        }
    }

    #[test]
    fn test_record_usage_cost_pause() {
        let mut gov = Governor::new(default_config());
        gov.track_session("s1", 100);

        // Hit the $10 limit
        let action = gov.record_usage("s1", 500, 500, 10.00).unwrap();
        match action {
            GovernorAction::Pause {
                session_id,
                reason,
            } => {
                assert_eq!(session_id, "s1");
                assert!(reason.contains("cost limit"));
                assert!(reason.contains("$10.00/$10.00"));
            }
            other => panic!("expected Pause, got {:?}", other),
        }

        // Session should be marked as paused
        assert!(gov.is_paused("s1").unwrap());
    }

    #[test]
    fn test_record_usage_token_pause() {
        let mut gov = Governor::new(default_config());
        gov.track_session("s1", 100);

        // Hit 1000 token limit (600 in + 500 out = 1100)
        let action = gov.record_usage("s1", 600, 500, 0.01).unwrap();
        match action {
            GovernorAction::Pause {
                session_id,
                reason,
            } => {
                assert_eq!(session_id, "s1");
                assert!(reason.contains("token limit"));
            }
            other => panic!("expected Pause, got {:?}", other),
        }
    }

    #[test]
    fn test_warning_fires_only_once() {
        let mut gov = Governor::new(default_config());
        gov.track_session("s1", 100);

        // First call at 80% threshold => Warn
        let action = gov.record_usage("s1", 100, 50, 8.50).unwrap();
        assert!(matches!(action, GovernorAction::Warn { .. }));

        // Second call still under pause limit => Allow (warning already fired)
        let action = gov.record_usage("s1", 10, 5, 0.50).unwrap();
        assert_eq!(action, GovernorAction::Allow);
    }

    #[test]
    fn test_cost_takes_priority_over_warning() {
        let mut gov = Governor::new(default_config());
        gov.track_session("s1", 100);

        // Exceed cost limit in one shot (skips warning)
        let action = gov.record_usage("s1", 100, 50, 15.00).unwrap();
        assert!(matches!(action, GovernorAction::Pause { .. }));
    }

    #[test]
    fn test_paused_session_does_not_count_as_active() {
        let mut gov = Governor::new(default_config());
        gov.track_session("s1", 100);
        gov.track_session("s2", 101);
        gov.track_session("s3", 102);

        assert_eq!(gov.active_count(), 3);
        assert!(!gov.can_spawn());

        // Pause s2 via cost limit
        let _ = gov.record_usage("s2", 500, 500, 10.00);

        assert_eq!(gov.active_count(), 2);
        assert!(gov.can_spawn());
    }

    #[test]
    fn test_resume_session() {
        let mut gov = Governor::new(default_config());
        gov.track_session("s1", 100);

        // Pause it
        let _ = gov.record_usage("s1", 500, 500, 10.00);
        assert!(gov.is_paused("s1").unwrap());

        // Resume it
        gov.resume_session("s1").unwrap();
        assert!(!gov.is_paused("s1").unwrap());
    }

    #[test]
    fn test_session_pid() {
        let mut gov = Governor::new(default_config());
        gov.track_session("s1", 12345);

        assert_eq!(gov.session_pid("s1").unwrap(), 12345);
    }

    #[test]
    fn test_untracked_session_errors() {
        let mut gov = Governor::new(default_config());

        assert!(matches!(
            gov.record_usage("nope", 1, 1, 0.01).unwrap_err(),
            GovernorError::SessionNotTracked(_)
        ));
        assert!(matches!(
            gov.is_paused("nope").unwrap_err(),
            GovernorError::SessionNotTracked(_)
        ));
        assert!(matches!(
            gov.resume_session("nope").unwrap_err(),
            GovernorError::SessionNotTracked(_)
        ));
        assert!(matches!(
            gov.session_pid("nope").unwrap_err(),
            GovernorError::SessionNotTracked(_)
        ));
    }

    #[test]
    fn test_unlimited_cost_skips_pause() {
        let config = GovernorConfig {
            max_cost_per_session_usd: 0.0, // unlimited
            max_tokens_per_session: 0,      // unlimited
            warn_at_percent: 80,
            ..default_config()
        };
        let mut gov = Governor::new(config);
        gov.track_session("s1", 100);

        // Huge usage should not trigger pause
        let action = gov.record_usage("s1", 999_999, 999_999, 999.99).unwrap();
        assert_eq!(action, GovernorAction::Allow);
    }

    #[test]
    fn test_default_config() {
        let config = GovernorConfig::default();
        assert_eq!(config.max_concurrent_agents, 10);
        assert!((config.max_cost_per_session_usd - 10.0).abs() < f64::EPSILON);
        assert_eq!(config.max_tokens_per_session, 500_000);
        assert!((config.max_cost_per_pipeline_usd - 50.0).abs() < f64::EPSILON);
        assert_eq!(config.warn_at_percent, 80);
    }

    #[test]
    fn test_token_warning_threshold() {
        let config = GovernorConfig {
            max_cost_per_session_usd: 0.0, // unlimited cost
            max_tokens_per_session: 1_000,
            warn_at_percent: 80,
            ..default_config()
        };
        let mut gov = Governor::new(config);
        gov.track_session("s1", 100);

        // 800/1000 tokens = 80% => should warn
        let action = gov.record_usage("s1", 500, 300, 0.0).unwrap();
        match action {
            GovernorAction::Warn { percent, .. } => {
                assert_eq!(percent, 80);
            }
            other => panic!("expected Warn, got {:?}", other),
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test orchestrator::governor::tests`
Expected: FAIL — module not wired into `orchestrator/mod.rs` yet

- [ ] **Step 3: Wire governor into mod.rs**

Update `src-tauri/src/orchestrator/mod.rs`:
```rust
pub mod engine;
pub mod governor;

pub use engine::{Engine, EngineError, EngineEvent, ProcessSpawner, SessionInfo, SessionState, SpawnedProcess};
pub use governor::{Governor, GovernorAction, GovernorConfig, GovernorError};
```

- [ ] **Step 4: Run test to verify it passes**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test orchestrator::governor::tests`
Expected: PASS — all 15 tests pass

- [ ] **Step 5: Commit**
```bash
scripts/committer "feat(orchestrator): add resource governor — concurrency, cost, and token limit enforcement" src-tauri/src/orchestrator/governor.rs src-tauri/src/orchestrator/mod.rs
```

---

## Summary

| Chunk | Module | Tests |
|-------|--------|-------|
| 1 | `orchestrator/engine.rs` — session spawn, monitor, kill, event forwarding | 11 |
| 2 | `orchestrator/governor.rs` — concurrency cap, cost/token limits, warn/pause | 15 |
| **Total** | | **26** |

**Dependencies from prior plans:**
- `agent::config` — `AgentConfig`, `AgentEvent`, `AgentError`, `PtySize` (Plan 2, Chunk 1)
- `agent::process` — `AgentProcess`, `PtySystem`, `PtyChild` (Plan 2, Chunk 4)
- `workspace::manager` — `WorktreeManager`, `WorktreeInfo` (Plan 3, Chunk 1)

**What Plan 4B will cover:**
- `orchestrator/pipeline.rs` — chained multi-agent workflows with handoff
- `orchestrator/recovery.rs` — crash recovery and session restore from SurrealDB
