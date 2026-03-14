//! Production ProcessSpawner bridging Engine to AgentProcess + PortablePtySystem.

use crate::agent::config::{AgentConfig, AgentError, AgentEvent, PtySize};
use crate::agent::process::{AgentProcess, PortablePtySystem};
use crate::orchestrator::engine::{ProcessSpawner, SpawnedProcess};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Real spawner that creates PTY-backed agent processes.
pub struct PtyProcessSpawner {
    pty_system: Arc<PortablePtySystem>,
}

impl PtyProcessSpawner {
    pub fn new() -> Self {
        Self {
            pty_system: Arc::new(PortablePtySystem),
        }
    }
}

impl ProcessSpawner for PtyProcessSpawner {
    fn spawn(
        &self,
        config: AgentConfig,
        size: PtySize,
    ) -> Result<(SpawnedProcess, mpsc::UnboundedReceiver<AgentEvent>), AgentError> {
        let (mut process, event_rx) =
            AgentProcess::spawn(config, self.pty_system.as_ref(), size)?;

        let pid = process.pid().unwrap_or(0);

        // Create a kill channel: when the oneshot fires, kill the process
        let (kill_tx, kill_rx) = tokio::sync::oneshot::channel::<()>();

        // Move the process handle into a thread that waits for the kill signal
        std::thread::Builder::new()
            .name(format!("agent-kill-watcher-{}", pid))
            .spawn(move || {
                // Block until kill signal or sender dropped
                let _ = kill_rx.blocking_recv();
                let _ = process.kill();
            })
            .map_err(|e| AgentError::SpawnFailed(format!("failed to spawn kill watcher: {}", e)))?;

        Ok((
            SpawnedProcess {
                pid,
                kill_tx: Some(kill_tx),
            },
            event_rx,
        ))
    }
}
