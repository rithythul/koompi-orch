pub mod agent;
pub mod config;
pub mod db;
pub mod ipc;

use config::AppConfig;
use ipc::commands::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("koompi_orch=info".parse().unwrap()))
        .init();

    info!("Starting koompi-orch");

    // Load config and ensure directories
    let config = AppConfig::load().expect("Failed to load config");
    config.ensure_dirs().expect("Failed to create data directories");

    // Initialize database synchronously to avoid race with Tauri state
    let db = tauri::async_runtime::block_on(async {
        db::init_db(&config.app.data_dir)
            .await
            .expect("Failed to initialize database")
    });

    let db_state: DbState = Arc::new(Mutex::new(db));

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .manage(db_state)
        .manage(config)
        .invoke_handler(tauri::generate_handler![
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
