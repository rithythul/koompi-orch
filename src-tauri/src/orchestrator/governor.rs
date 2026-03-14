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
