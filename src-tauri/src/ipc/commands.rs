use crate::config::AppConfig;
use crate::db::queries::Queries;
use crate::db::schema::*;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use tauri::State;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type DbState = Arc<Mutex<Surreal<Db>>>;

// -- Config commands --

#[tauri::command]
pub async fn get_config() -> Result<AppConfig, String> {
    AppConfig::load().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_config(config: AppConfig) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())
}

// -- Repo commands --

#[tauri::command]
pub async fn list_repos(db: State<'_, DbState>) -> Result<Vec<Repo>, String> {
    let db = db.lock().await;
    Queries::new(&db).list_repos().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_repo(
    db: State<'_, DbState>,
    path: String,
    name: String,
    remote_url: Option<String>,
) -> Result<Repo, String> {
    let db = db.lock().await;
    Queries::new(&db)
        .create_repo(&path, &name, remote_url.as_deref())
        .await
        .map_err(|e| e.to_string())
}

// -- Workspace commands --

#[tauri::command]
pub async fn list_workspaces(db: State<'_, DbState>) -> Result<Vec<Workspace>, String> {
    let db = db.lock().await;
    Queries::new(&db).list_workspaces().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_workspaces_by_status(
    db: State<'_, DbState>,
    status: String,
) -> Result<Vec<Workspace>, String> {
    let db = db.lock().await;
    Queries::new(&db)
        .list_workspaces_by_status(&status)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_workspace(
    db: State<'_, DbState>,
    name: String,
    branch: String,
    worktree_path: String,
    repo_id: Option<String>,
) -> Result<Workspace, String> {
    let db = db.lock().await;
    Queries::new(&db)
        .create_workspace(&name, &branch, &worktree_path, repo_id.as_deref().unwrap_or(""))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_workspace_status(
    db: State<'_, DbState>,
    id: String,
    status: String,
) -> Result<(), String> {
    let db = db.lock().await;
    Queries::new(&db)
        .update_workspace_status(&id, &status)
        .await
        .map_err(|e| e.to_string())
}

// -- Session commands --

#[tauri::command]
pub async fn list_sessions(
    db: State<'_, DbState>,
    workspace_id: String,
) -> Result<Vec<Session>, String> {
    let db = db.lock().await;
    Queries::new(&db)
        .list_sessions_for_workspace(&workspace_id)
        .await
        .map_err(|e| e.to_string())
}

// -- Template commands --

#[tauri::command]
pub async fn list_templates(db: State<'_, DbState>) -> Result<Vec<AgentTemplate>, String> {
    let db = db.lock().await;
    Queries::new(&db).list_templates().await.map_err(|e| e.to_string())
}

// -- Preset commands --

#[tauri::command]
pub async fn list_presets(db: State<'_, DbState>) -> Result<Vec<RolePreset>, String> {
    let db = db.lock().await;
    Queries::new(&db).list_presets().await.map_err(|e| e.to_string())
}

// -- Metric commands --

#[tauri::command]
pub async fn get_session_metrics(
    db: State<'_, DbState>,
    session_id: String,
) -> Result<Vec<Metric>, String> {
    let db = db.lock().await;
    Queries::new(&db)
        .get_session_metrics(&session_id)
        .await
        .map_err(|e| e.to_string())
}
