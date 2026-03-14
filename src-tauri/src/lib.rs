pub mod agent;
pub mod config;
pub mod db;
pub mod git;
pub mod ipc;
pub mod orchestrator;
pub mod workspace;

use config::AppConfig;
use ipc::agent_commands::*;
use ipc::commands::*;
use ipc::spawner::PtyProcessSpawner;
use orchestrator::Engine;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Emitter;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("koompi_orch=info".parse().unwrap()),
        )
        .init();

    info!("Starting koompi-orch");

    // Load config and ensure directories
    let config = AppConfig::load().expect("Failed to load config");
    config
        .ensure_dirs()
        .expect("Failed to create data directories");

    let max_agents = config.app.max_concurrent_agents as usize;

    // Initialize database synchronously to avoid race with Tauri state
    let db = tauri::async_runtime::block_on(async {
        db::init_db(&config.app.data_dir)
            .await
            .expect("Failed to initialize database")
    });

    let db_state: DbState = Arc::new(Mutex::new(db));

    // Create the orchestration engine with production PTY spawner
    let spawner = Arc::new(PtyProcessSpawner::new());
    let (engine, mut event_rx) = Engine::new(spawner, max_agents);
    let engine_state: EngineState = Arc::new(Mutex::new(engine));

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .manage(db_state)
        .manage(config)
        .manage(engine_state)
        .setup(|app| {
            // Forward engine events to Tauri event system
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    // Emit to frontend: event name = "agent-event"
                    // Payload includes session_id + the agent event
                    let payload = serde_json::json!({
                        "sessionId": event.session_id,
                        "event": event.event,
                    });
                    let _ = handle.emit("agent-event", payload);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Existing commands
            get_config,
            save_config,
            list_repos,
            add_repo,
            list_workspaces,
            list_workspaces_by_status,
            create_workspace,
            update_workspace_status,
            list_sessions,
            list_templates,
            list_presets,
            get_session_metrics,
            // New agent lifecycle commands
            spawn_agent,
            kill_agent,
            get_session_state,
            list_engine_sessions,
            running_agent_count,
            set_setting,
            get_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
