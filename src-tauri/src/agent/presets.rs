use crate::agent::config::AgentError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

/// How the role's system prompt is injected into the agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InjectionMethod {
    /// Pass via `--system-prompt "..."` flag (if agent supports it).
    Flag,
    /// Set `SYSTEM_PROMPT` environment variable.
    EnvVar,
    /// Write to agent's config file before spawning.
    ConfigFile,
    /// Prepend role instructions to the user's first message (most portable).
    FirstMessage,
}

impl InjectionMethod {
    pub fn from_str_loose(s: &str) -> Result<Self, AgentError> {
        match s {
            "flag" => Ok(Self::Flag),
            "env_var" => Ok(Self::EnvVar),
            "config_file" => Ok(Self::ConfigFile),
            "first_message" => Ok(Self::FirstMessage),
            other => Err(AgentError::PresetNotFound(format!(
                "unknown injection method: {}",
                other
            ))),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Flag => "flag",
            Self::EnvVar => "env_var",
            Self::ConfigFile => "config_file",
            Self::FirstMessage => "first_message",
        }
    }
}

/// A role preset defines the persona/behavior for an agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolePreset {
    pub name: String,
    pub system_prompt: String,
    pub description: String,
    pub injection_method: String,
    pub builtin: bool,
}

impl RolePreset {
    /// Get the parsed injection method.
    pub fn injection(&self) -> Result<InjectionMethod, AgentError> {
        InjectionMethod::from_str_loose(&self.injection_method)
    }

    /// Format the role prompt for first_message injection.
    ///
    /// Returns the message with role context prepended:
    /// ```text
    /// [Role: Architect] Think from first principles...
    ///
    /// User task: <original message>
    /// ```
    pub fn format_first_message(&self, user_message: &str) -> String {
        let capitalized = capitalize_first(&self.name);
        format!(
            "[Role: {}] {}\n\nUser task: {}\n",
            capitalized, self.system_prompt, user_message
        )
    }

    /// Build environment variables for env_var injection.
    pub fn to_env_vars(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();
        env.insert("SYSTEM_PROMPT".to_string(), self.system_prompt.clone());
        env.insert("ROLE_NAME".to_string(), self.name.clone());
        env
    }
}

/// Built-in role presets.
pub fn builtin_presets() -> Vec<RolePreset> {
    vec![
        RolePreset {
            name: "architect".to_string(),
            system_prompt: "Think from first principles, design before coding, consider trade-offs. \
                Outline the architecture, data flow, and component boundaries before any implementation. \
                Ask clarifying questions if the requirements are ambiguous.".to_string(),
            description: "System architect role".to_string(),
            injection_method: "first_message".to_string(),
            builtin: true,
        },
        RolePreset {
            name: "implementer".to_string(),
            system_prompt: "Write production code, follow existing patterns, test as you go. \
                Match the codebase style and conventions. Write clean, maintainable code with \
                appropriate error handling.".to_string(),
            description: "Implementation engineer role".to_string(),
            injection_method: "first_message".to_string(),
            builtin: true,
        },
        RolePreset {
            name: "reviewer".to_string(),
            system_prompt: "Paranoid code review: race conditions, security, N+1 queries, trust boundaries. \
                Check for edge cases, error handling gaps, and potential regressions. \
                Verify test coverage for changed code paths.".to_string(),
            description: "Code reviewer role".to_string(),
            injection_method: "first_message".to_string(),
            builtin: true,
        },
        RolePreset {
            name: "tester".to_string(),
            system_prompt: "Write comprehensive tests: unit, integration, edge cases. \
                Cover happy paths, error paths, boundary conditions, and concurrency scenarios. \
                Ensure tests are deterministic and fast.".to_string(),
            description: "Test engineer role".to_string(),
            injection_method: "first_message".to_string(),
            builtin: true,
        },
        RolePreset {
            name: "shipper".to_string(),
            system_prompt: "Final-mile: sync main, run tests, resolve comments, open PR. \
                Ensure CI passes, changelog is updated, and the PR description is clear. \
                Address any remaining review feedback.".to_string(),
            description: "Release engineer role".to_string(),
            injection_method: "first_message".to_string(),
            builtin: true,
        },
        RolePreset {
            name: "fixer".to_string(),
            system_prompt: "Debug and fix: systematic root cause analysis, minimal changes. \
                Reproduce the issue first, then make the smallest possible fix. \
                Add a regression test for the bug.".to_string(),
            description: "Bug fixer role".to_string(),
            injection_method: "first_message".to_string(),
            builtin: true,
        },
    ]
}

/// Registry for managing role presets.
pub struct PresetRegistry;

impl PresetRegistry {
    /// Get a preset by name. Falls back to builtin presets.
    pub async fn get_preset(
        db: &Surreal<Db>,
        name: &str,
    ) -> Result<RolePreset, AgentError> {
        let results: Vec<RolePreset> = db
            .query("SELECT * FROM role_preset WHERE name = $name LIMIT 1")
            .bind(("name", name.to_string()))
            .await
            .map_err(|e| AgentError::DbError(e.to_string()))?
            .take(0)
            .map_err(|e| AgentError::DbError(e.to_string()))?;

        if let Some(preset) = results.into_iter().next() {
            return Ok(preset);
        }

        builtin_presets()
            .into_iter()
            .find(|p| p.name == name)
            .ok_or_else(|| AgentError::PresetNotFound(name.to_string()))
    }

    /// List all presets (DB + builtins merged).
    pub async fn list_presets(
        db: &Surreal<Db>,
    ) -> Result<Vec<RolePreset>, AgentError> {
        let db_presets: Vec<RolePreset> = db
            .query("SELECT * FROM role_preset ORDER BY name ASC")
            .await
            .map_err(|e| AgentError::DbError(e.to_string()))?
            .take(0)
            .map_err(|e| AgentError::DbError(e.to_string()))?;

        let mut presets_map: HashMap<String, RolePreset> = HashMap::new();

        for p in builtin_presets() {
            presets_map.insert(p.name.clone(), p);
        }

        for p in db_presets {
            presets_map.insert(p.name.clone(), p);
        }

        let mut result: Vec<RolePreset> = presets_map.into_values().collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    /// Register a custom preset.
    pub async fn register_custom(
        db: &Surreal<Db>,
        preset: &RolePreset,
    ) -> Result<(), AgentError> {
        db.query(
            "CREATE role_preset SET \
             name = $name, system_prompt = $prompt, description = $desc, \
             injection_method = $method, builtin = false",
        )
        .bind(("name", preset.name.to_string()))
        .bind(("prompt", preset.system_prompt.to_string()))
        .bind(("desc", preset.description.to_string()))
        .bind(("method", preset.injection_method.to_string()))
        .await
        .map_err(|e| AgentError::DbError(e.to_string()))?;

        Ok(())
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
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
    fn test_builtin_presets_count() {
        assert_eq!(builtin_presets().len(), 6);
    }

    #[test]
    fn test_builtin_preset_names() {
        let presets = builtin_presets();
        let names: Vec<&str> = presets.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"architect"));
        assert!(names.contains(&"implementer"));
        assert!(names.contains(&"reviewer"));
        assert!(names.contains(&"tester"));
        assert!(names.contains(&"shipper"));
        assert!(names.contains(&"fixer"));
    }

    #[test]
    fn test_injection_method_from_str() {
        assert_eq!(
            InjectionMethod::from_str_loose("first_message").unwrap(),
            InjectionMethod::FirstMessage
        );
        assert_eq!(
            InjectionMethod::from_str_loose("flag").unwrap(),
            InjectionMethod::Flag
        );
        assert_eq!(
            InjectionMethod::from_str_loose("env_var").unwrap(),
            InjectionMethod::EnvVar
        );
        assert_eq!(
            InjectionMethod::from_str_loose("config_file").unwrap(),
            InjectionMethod::ConfigFile
        );
        assert!(InjectionMethod::from_str_loose("bogus").is_err());
    }

    #[test]
    fn test_injection_method_roundtrip() {
        for method in &[
            InjectionMethod::Flag,
            InjectionMethod::EnvVar,
            InjectionMethod::ConfigFile,
            InjectionMethod::FirstMessage,
        ] {
            let s = method.as_str();
            let parsed = InjectionMethod::from_str_loose(s).unwrap();
            assert_eq!(&parsed, method);
        }
    }

    #[test]
    fn test_format_first_message() {
        let preset = &builtin_presets()[0]; // architect
        let formatted = preset.format_first_message("Build a REST API");
        assert!(formatted.starts_with("[Role: Architect]"));
        assert!(formatted.contains("first principles"));
        assert!(formatted.contains("User task: Build a REST API"));
    }

    #[test]
    fn test_to_env_vars() {
        let preset = &builtin_presets()[1]; // implementer
        let env = preset.to_env_vars();
        assert!(env.contains_key("SYSTEM_PROMPT"));
        assert!(env.contains_key("ROLE_NAME"));
        assert_eq!(env["ROLE_NAME"], "implementer");
        assert!(env["SYSTEM_PROMPT"].contains("production code"));
    }

    #[tokio::test]
    async fn test_get_preset_builtin_fallback() {
        let db = setup_db().await;
        let preset = PresetRegistry::get_preset(&db, "architect").await.unwrap();
        assert_eq!(preset.name, "architect");
        assert!(preset.system_prompt.contains("first principles"));
    }

    #[tokio::test]
    async fn test_get_preset_not_found() {
        let db = setup_db().await;
        let result = PresetRegistry::get_preset(&db, "nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_presets() {
        let db = setup_db().await;
        let presets = PresetRegistry::list_presets(&db).await.unwrap();
        assert_eq!(presets.len(), 6);
        // Should be sorted alphabetically
        assert_eq!(presets[0].name, "architect");
        assert_eq!(presets[1].name, "fixer");
        assert_eq!(presets[2].name, "implementer");
    }

    #[tokio::test]
    async fn test_register_custom_preset() {
        let db = setup_db().await;

        let custom = RolePreset {
            name: "debugger".to_string(),
            system_prompt: "Use systematic debugging: reproduce, bisect, fix, verify.".to_string(),
            description: "Advanced debugger role".to_string(),
            injection_method: "first_message".to_string(),
            builtin: false,
        };

        PresetRegistry::register_custom(&db, &custom).await.unwrap();

        let fetched = PresetRegistry::get_preset(&db, "debugger").await.unwrap();
        assert_eq!(fetched.name, "debugger");
        assert!(fetched.system_prompt.contains("systematic debugging"));
    }

    #[tokio::test]
    async fn test_list_presets_with_custom() {
        let db = setup_db().await;

        let custom = RolePreset {
            name: "alpha-tester".to_string(),
            system_prompt: "Test alpha features.".to_string(),
            description: "Alpha tester".to_string(),
            injection_method: "env_var".to_string(),
            builtin: false,
        };
        PresetRegistry::register_custom(&db, &custom).await.unwrap();

        let presets = PresetRegistry::list_presets(&db).await.unwrap();
        assert_eq!(presets.len(), 7); // 6 builtins + 1 custom
        assert!(presets.iter().any(|p| p.name == "alpha-tester"));
    }
}
