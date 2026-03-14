//! IPC commands for agent lifecycle: spawn, kill, send message, list sessions.

use crate::agent::registry::AgentRegistry;
use crate::config::AppConfig;
use crate::ipc::commands::DbState;
use crate::orchestrator::engine::{Engine, SessionState};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

/// Tauri-managed engine state.
pub type EngineState = Arc<Mutex<Engine>>;

/// Serializable session info for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct SessionSnapshot {
    pub session_id: String,
    pub state: String,
}

fn state_str(s: SessionState) -> String {
    match s {
        SessionState::Running => "running",
        SessionState::Paused => "paused",
        SessionState::Completed => "completed",
        SessionState::Failed => "failed",
        SessionState::Killed => "killed",
    }
    .to_string()
}

/// Spawn a new agent session in a workspace directory.
#[tauri::command]
pub async fn spawn_agent(
    engine: State<'_, EngineState>,
    db: State<'_, DbState>,
    agent_type: String,
    workspace_path: String,
    task: Option<String>,
) -> Result<String, String> {
    let db_guard = db.lock().await;

    // Look up the agent template
    let template = AgentRegistry::get_template(&db_guard, &agent_type)
        .await
        .map_err(|e| e.to_string())?;

    let work_dir = PathBuf::from(&workspace_path);
    let mut config = template
        .to_agent_config(work_dir)
        .map_err(|e| e.to_string())?;

    // If a task message is provided and the agent supports flag_message mode,
    // add it as a --message arg for the initial prompt
    if let Some(ref task_msg) = task {
        if let Some(ref msg_flag) = config.message_flag {
            config.args.push(msg_flag.clone());
            config.args.push(task_msg.clone());
        }
        // Also add --print for non-interactive mode if available
        if let Some(ref print_flag) = config.print_flag {
            config.args.push(print_flag.clone());
        }
    }

    drop(db_guard);

    let engine = engine.lock().await;
    let session_id = engine
        .spawn_session(config, PathBuf::from(&workspace_path))
        .await
        .map_err(|e| e.to_string())?;

    Ok(session_id)
}

/// Kill a running agent session.
#[tauri::command]
pub async fn kill_agent(
    engine: State<'_, EngineState>,
    session_id: String,
) -> Result<(), String> {
    let engine = engine.lock().await;
    engine
        .kill_session(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get the state of a specific session.
#[tauri::command]
pub async fn get_session_state(
    engine: State<'_, EngineState>,
    session_id: String,
) -> Result<String, String> {
    let engine = engine.lock().await;
    let state = engine
        .get_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(state_str(state))
}

/// List all active engine sessions (in-memory, not DB).
#[tauri::command]
pub async fn list_engine_sessions(
    engine: State<'_, EngineState>,
) -> Result<Vec<SessionSnapshot>, String> {
    let engine = engine.lock().await;
    let sessions = engine.list_sessions().await;
    Ok(sessions
        .into_iter()
        .map(|(id, state)| SessionSnapshot {
            session_id: id,
            state: state_str(state),
        })
        .collect())
}

/// Get the count of currently running sessions.
#[tauri::command]
pub async fn running_agent_count(
    engine: State<'_, EngineState>,
) -> Result<usize, String> {
    let engine = engine.lock().await;
    Ok(engine.running_count().await)
}

/// Save a single setting key-value to the config file.
#[tauri::command]
pub async fn set_setting(
    config: State<'_, AppConfig>,
    key: String,
    value: String,
) -> Result<(), String> {
    let mut cfg = config.inner().clone();
    match key.as_str() {
        "theme" => cfg.app.theme = value,
        "max_concurrent_agents" => {
            cfg.app.max_concurrent_agents = value
                .parse()
                .map_err(|_| "invalid number".to_string())?;
        }
        "default_agent" => cfg.defaults.agent = value,
        "default_role" => cfg.defaults.role = value,
        "auto_review" => {
            cfg.defaults.auto_review = value.parse().unwrap_or(true);
        }
        "auto_checkpoint" => {
            cfg.defaults.auto_checkpoint = value.parse().unwrap_or(true);
        }
        _ => return Err(format!("unknown setting key: {}", key)),
    }
    cfg.save().map_err(|e| e.to_string())
}

/// Get current settings (returns the full config).
#[tauri::command]
pub async fn get_settings(
    config: State<'_, AppConfig>,
) -> Result<AppConfig, String> {
    Ok(config.inner().clone())
}
