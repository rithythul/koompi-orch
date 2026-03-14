use crate::agent::config::{AgentConfig, AgentError, InputMode, OutputMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

/// An agent template defines how to spawn and interact with a particular CLI agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    pub name: String,
    pub command: String,
    pub default_args: Vec<String>,
    pub env: Option<HashMap<String, String>>,
    pub input_mode: String,
    pub output_mode: String,
    pub resume_support: bool,
    pub builtin: bool,
    /// For flag_message mode: the flag name (e.g., "--message").
    pub message_flag: Option<String>,
    /// For flag_message mode: print flag (e.g., "--print").
    pub print_flag: Option<String>,
    /// Resume flag (e.g., "--resume").
    pub resume_flag: Option<String>,
}

impl AgentTemplate {
    /// Convert this template into an AgentConfig for a given working directory.
    pub fn to_agent_config(&self, working_dir: PathBuf) -> Result<AgentConfig, AgentError> {
        Ok(AgentConfig {
            command: self.command.clone(),
            args: self.default_args.clone(),
            env: self.env.clone().unwrap_or_default(),
            input_mode: InputMode::from_str_loose(&self.input_mode)?,
            output_mode: OutputMode::from_str_loose(&self.output_mode)?,
            resume_support: self.resume_support,
            working_dir,
            message_flag: self.message_flag.clone(),
            print_flag: self.print_flag.clone(),
            resume_flag: self.resume_flag.clone(),
        })
    }
}

/// Built-in agent templates, used when seeding the database or as fallback.
pub fn builtin_templates() -> Vec<AgentTemplate> {
    vec![
        AgentTemplate {
            name: "claude-code".to_string(),
            command: "claude".to_string(),
            default_args: vec!["--dangerously-skip-permissions".to_string()],
            env: None,
            input_mode: "pty_stdin".to_string(),
            output_mode: "json_stream".to_string(),
            resume_support: true,
            builtin: true,
            message_flag: Some("--message".to_string()),
            print_flag: Some("--print".to_string()),
            resume_flag: Some("--resume".to_string()),
        },
        AgentTemplate {
            name: "codex".to_string(),
            command: "codex".to_string(),
            default_args: vec![],
            env: None,
            input_mode: "pty_stdin".to_string(),
            output_mode: "text_markers".to_string(),
            resume_support: false,
            builtin: true,
            message_flag: None,
            print_flag: None,
            resume_flag: None,
        },
        AgentTemplate {
            name: "gemini-cli".to_string(),
            command: "gemini".to_string(),
            default_args: vec![],
            env: None,
            input_mode: "pty_stdin".to_string(),
            output_mode: "text_markers".to_string(),
            resume_support: false,
            builtin: true,
            message_flag: None,
            print_flag: None,
            resume_flag: None,
        },
        AgentTemplate {
            name: "aider".to_string(),
            command: "aider".to_string(),
            default_args: vec!["--no-auto-commits".to_string()],
            env: None,
            input_mode: "pty_stdin".to_string(),
            output_mode: "text_markers".to_string(),
            resume_support: true,
            builtin: true,
            message_flag: None,
            print_flag: None,
            resume_flag: Some("--restore-chat-history".to_string()),
        },
    ]
}

/// Registry for managing agent templates. Loads from SurrealDB with
/// in-memory builtin fallback.
pub struct AgentRegistry;

impl AgentRegistry {
    /// Get a template by name from the database.
    /// Falls back to builtin templates if not found in DB.
    pub async fn get_template(
        db: &Surreal<Db>,
        name: &str,
    ) -> Result<AgentTemplate, AgentError> {
        // Try DB first
        let results: Vec<AgentTemplate> = db
            .query("SELECT * FROM agent_template WHERE name = $name LIMIT 1")
            .bind(("name", name.to_string()))
            .await
            .map_err(|e| AgentError::DbError(e.to_string()))?
            .take(0)
            .map_err(|e| AgentError::DbError(e.to_string()))?;

        if let Some(template) = results.into_iter().next() {
            return Ok(template);
        }

        // Fallback to builtins
        builtin_templates()
            .into_iter()
            .find(|t| t.name == name)
            .ok_or_else(|| AgentError::TemplateNotFound(name.to_string()))
    }

    /// List all templates (DB + builtins merged, DB overrides builtins).
    pub async fn list_templates(
        db: &Surreal<Db>,
    ) -> Result<Vec<AgentTemplate>, AgentError> {
        let db_templates: Vec<AgentTemplate> = db
            .query("SELECT * FROM agent_template ORDER BY name ASC")
            .await
            .map_err(|e| AgentError::DbError(e.to_string()))?
            .take(0)
            .map_err(|e| AgentError::DbError(e.to_string()))?;

        let mut templates_map: HashMap<String, AgentTemplate> = HashMap::new();

        // Add builtins first
        for t in builtin_templates() {
            templates_map.insert(t.name.clone(), t);
        }

        // DB templates override builtins
        for t in db_templates {
            templates_map.insert(t.name.clone(), t);
        }

        let mut result: Vec<AgentTemplate> = templates_map.into_values().collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    /// Register a custom template in the database.
    pub async fn register_custom(
        db: &Surreal<Db>,
        template: &AgentTemplate,
    ) -> Result<(), AgentError> {
        db.query(
            "CREATE agent_template SET \
             name = $name, command = $command, default_args = $args, \
             env = $env, input_mode = $input_mode, output_mode = $output_mode, \
             resume_support = $resume, builtin = false",
        )
        .bind(("name", template.name.to_string()))
        .bind(("command", template.command.to_string()))
        .bind(("args", template.default_args.clone()))
        .bind(("env", template.env.clone()))
        .bind(("input_mode", template.input_mode.to_string()))
        .bind(("output_mode", template.output_mode.to_string()))
        .bind(("resume", template.resume_support))
        .await
        .map_err(|e| AgentError::DbError(e.to_string()))?;

        Ok(())
    }

    /// Delete a custom template from the database. Cannot delete builtins.
    pub async fn delete_custom(
        db: &Surreal<Db>,
        name: &str,
    ) -> Result<(), AgentError> {
        // Check if it's a builtin
        if builtin_templates().iter().any(|t| t.name == name) {
            return Err(AgentError::TemplateNotFound(format!(
                "cannot delete builtin template: {}",
                name
            )));
        }

        db.query("DELETE FROM agent_template WHERE name = $name AND builtin = false")
            .bind(("name", name.to_string()))
            .await
            .map_err(|e| AgentError::DbError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use surrealdb::engine::local::Mem;

    async fn setup_db() -> Surreal<Db> {
        let db = Surreal::new::<Mem>(()).await.unwrap();
        db.use_ns("test").use_db("test").await.unwrap();
        crate::db::migrate::run_migrations(&db).await.unwrap();
        db
    }

    #[test]
    fn test_builtin_templates_count() {
        let templates = builtin_templates();
        assert_eq!(templates.len(), 4);
    }

    #[test]
    fn test_builtin_template_names() {
        let templates = builtin_templates();
        let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"claude-code"));
        assert!(names.contains(&"codex"));
        assert!(names.contains(&"gemini-cli"));
        assert!(names.contains(&"aider"));
    }

    #[test]
    fn test_template_to_agent_config() {
        let templates = builtin_templates();
        let claude = templates.iter().find(|t| t.name == "claude-code").unwrap();
        let config = claude.to_agent_config(PathBuf::from("/tmp/workspace")).unwrap();

        assert_eq!(config.command, "claude");
        assert_eq!(config.input_mode, InputMode::PtyStdin);
        assert_eq!(config.output_mode, OutputMode::JsonStream);
        assert!(config.resume_support);
        assert_eq!(config.working_dir, PathBuf::from("/tmp/workspace"));
        assert_eq!(config.args, vec!["--dangerously-skip-permissions"]);
    }

    #[test]
    fn test_template_to_config_invalid_mode() {
        let mut template = builtin_templates()[0].clone();
        template.input_mode = "invalid_mode".to_string();
        let result = template.to_agent_config(PathBuf::from("/tmp"));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_template_builtin_fallback() {
        let db = setup_db().await;
        // No templates seeded in DB — should fall back to builtin
        let template = AgentRegistry::get_template(&db, "claude-code").await.unwrap();
        assert_eq!(template.name, "claude-code");
        assert_eq!(template.command, "claude");
    }

    #[tokio::test]
    async fn test_get_template_not_found() {
        let db = setup_db().await;
        let result = AgentRegistry::get_template(&db, "nonexistent-agent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("template not found"));
    }

    #[tokio::test]
    async fn test_list_templates_builtins() {
        let db = setup_db().await;
        let templates = AgentRegistry::list_templates(&db).await.unwrap();
        assert_eq!(templates.len(), 4);
        // Should be sorted alphabetically
        assert_eq!(templates[0].name, "aider");
        assert_eq!(templates[1].name, "claude-code");
        assert_eq!(templates[2].name, "codex");
        assert_eq!(templates[3].name, "gemini-cli");
    }

    #[tokio::test]
    async fn test_register_and_get_custom_template() {
        let db = setup_db().await;

        let custom = AgentTemplate {
            name: "my-custom-agent".to_string(),
            command: "/usr/local/bin/my-agent".to_string(),
            default_args: vec!["--verbose".to_string()],
            env: Some(HashMap::from([("API_KEY".to_string(), "test".to_string())])),
            input_mode: "pty_stdin".to_string(),
            output_mode: "raw_pty".to_string(),
            resume_support: false,
            builtin: false,
            message_flag: None,
            print_flag: None,
            resume_flag: None,
        };

        AgentRegistry::register_custom(&db, &custom).await.unwrap();

        let fetched = AgentRegistry::get_template(&db, "my-custom-agent").await.unwrap();
        assert_eq!(fetched.command, "/usr/local/bin/my-agent");
        assert_eq!(fetched.output_mode, "raw_pty");
    }

    #[tokio::test]
    async fn test_list_templates_with_custom() {
        let db = setup_db().await;

        let custom = AgentTemplate {
            name: "beta-agent".to_string(),
            command: "beta".to_string(),
            default_args: vec![],
            env: None,
            input_mode: "file_prompt".to_string(),
            output_mode: "text_markers".to_string(),
            resume_support: false,
            builtin: false,
            message_flag: None,
            print_flag: None,
            resume_flag: None,
        };
        AgentRegistry::register_custom(&db, &custom).await.unwrap();

        let templates = AgentRegistry::list_templates(&db).await.unwrap();
        assert_eq!(templates.len(), 5); // 4 builtins + 1 custom
        assert!(templates.iter().any(|t| t.name == "beta-agent"));
    }

    #[tokio::test]
    async fn test_delete_custom_template() {
        let db = setup_db().await;

        let custom = AgentTemplate {
            name: "temp-agent".to_string(),
            command: "temp".to_string(),
            default_args: vec![],
            env: None,
            input_mode: "pty_stdin".to_string(),
            output_mode: "raw_pty".to_string(),
            resume_support: false,
            builtin: false,
            message_flag: None,
            print_flag: None,
            resume_flag: None,
        };
        AgentRegistry::register_custom(&db, &custom).await.unwrap();

        // Verify it exists
        AgentRegistry::get_template(&db, "temp-agent").await.unwrap();

        // Delete it
        AgentRegistry::delete_custom(&db, "temp-agent").await.unwrap();

        // Should fall back to not found (no builtin with that name)
        let result = AgentRegistry::get_template(&db, "temp-agent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cannot_delete_builtin() {
        let db = setup_db().await;
        let result = AgentRegistry::delete_custom(&db, "claude-code").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot delete builtin"));
    }
}
