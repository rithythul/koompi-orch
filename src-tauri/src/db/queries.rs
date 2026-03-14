use super::schema::*;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

/// Typed query helpers for common database operations
pub struct Queries<'a> {
    db: &'a Surreal<Db>,
}

impl<'a> Queries<'a> {
    pub fn new(db: &'a Surreal<Db>) -> Self {
        Self { db }
    }

    // -- Repos --

    pub async fn create_repo(&self, path: &str, name: &str, remote_url: Option<&str>) -> Result<Repo, surrealdb::Error> {
        let repo: Option<Repo> = self.db
            .query("CREATE repo SET path = $path, name = $name, remote_url = $remote_url")
            .bind(("path", path.to_string()))
            .bind(("name", name.to_string()))
            .bind(("remote_url", remote_url.map(|s| s.to_string())))
            .await?
            .take(0)?;
        Ok(repo.expect("repo should be created"))
    }

    pub async fn list_repos(&self) -> Result<Vec<Repo>, surrealdb::Error> {
        let repos: Vec<Repo> = self.db
            .query("SELECT * FROM repo ORDER BY name ASC")
            .await?
            .take(0)?;
        Ok(repos)
    }

    // -- Workspaces --

    pub async fn create_workspace(
        &self,
        name: &str,
        branch: &str,
        worktree_path: &str,
        repo_id: &str,
    ) -> Result<Workspace, surrealdb::Error> {
        let ws: Option<Workspace> = self.db
            .query(
                "CREATE workspace SET name = $name, branch = $branch, worktree_path = $path, status = 'backlog'"
            )
            .bind(("name", name.to_string()))
            .bind(("branch", branch.to_string()))
            .bind(("path", worktree_path.to_string()))
            .await?
            .take(0)?;

        // Create the relation
        if let Some(ref workspace) = ws {
            if let Some(ref ws_id) = workspace.id {
                self.db
                    .query("RELATE $ws_id->belongs_to->type::thing('repo', $repo_id)")
                    .bind(("ws_id", ws_id.clone()))
                    .bind(("repo_id", repo_id.to_string()))
                    .await?;
            }
        }

        Ok(ws.expect("workspace should be created"))
    }

    pub async fn list_workspaces(&self) -> Result<Vec<Workspace>, surrealdb::Error> {
        let workspaces: Vec<Workspace> = self.db
            .query("SELECT * FROM workspace ORDER BY updated_at DESC")
            .await?
            .take(0)?;
        Ok(workspaces)
    }

    pub async fn list_workspaces_by_status(&self, status: &str) -> Result<Vec<Workspace>, surrealdb::Error> {
        let workspaces: Vec<Workspace> = self.db
            .query("SELECT * FROM workspace WHERE status = $status ORDER BY updated_at DESC")
            .bind(("status", status.to_string()))
            .await?
            .take(0)?;
        Ok(workspaces)
    }

    pub async fn update_workspace_status(&self, id: &str, status: &str) -> Result<(), surrealdb::Error> {
        self.db
            .query("UPDATE type::thing('workspace', $id) SET status = $status, updated_at = time::now()")
            .bind(("id", id.to_string()))
            .bind(("status", status.to_string()))
            .await?;
        Ok(())
    }

    // -- Sessions --

    pub async fn list_sessions_for_workspace(&self, workspace_id: &str) -> Result<Vec<Session>, surrealdb::Error> {
        let sessions: Vec<Session> = self.db
            .query(
                "SELECT * FROM session WHERE id IN \
                 (SELECT in FROM runs_in WHERE out = type::thing('workspace', $ws_id)).in \
                 ORDER BY started_at DESC"
            )
            .bind(("ws_id", workspace_id.to_string()))
            .await?
            .take(0)?;
        Ok(sessions)
    }

    // -- Templates --

    pub async fn list_templates(&self) -> Result<Vec<AgentTemplate>, surrealdb::Error> {
        let templates: Vec<AgentTemplate> = self.db
            .query("SELECT * FROM agent_template ORDER BY name ASC")
            .await?
            .take(0)?;
        Ok(templates)
    }

    // -- Role Presets --

    pub async fn list_presets(&self) -> Result<Vec<RolePreset>, surrealdb::Error> {
        let presets: Vec<RolePreset> = self.db
            .query("SELECT * FROM role_preset ORDER BY name ASC")
            .await?
            .take(0)?;
        Ok(presets)
    }

    // -- Metrics --

    pub async fn get_session_metrics(&self, session_id: &str) -> Result<Vec<Metric>, surrealdb::Error> {
        let metrics: Vec<Metric> = self.db
            .query(
                "SELECT * FROM metric WHERE id IN \
                 (SELECT in FROM metric_for WHERE out = type::thing('session', $sid)).in \
                 ORDER BY recorded_at ASC"
            )
            .bind(("sid", session_id.to_string()))
            .await?
            .take(0)?;
        Ok(metrics)
    }
}
