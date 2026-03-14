use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app: AppSettings,
    pub defaults: DefaultSettings,
    pub notifications: NotificationSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,
    pub data_dir: PathBuf,
    pub max_concurrent_agents: u32,
    #[serde(default = "default_handoff_retention")]
    pub handoff_retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultSettings {
    pub agent: String,
    pub role: String,
    pub auto_review: bool,
    pub auto_checkpoint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub agent_completed: bool,
    pub agent_failed: bool,
    pub agent_needs_input: bool,
    pub ci_status: bool,
}

fn default_handoff_retention() -> u32 {
    30
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app: AppSettings {
                theme: "dark".to_string(),
                data_dir: dirs::home_dir()
                    .unwrap_or_default()
                    .join(".koompi-orch"),
                max_concurrent_agents: 10,
                handoff_retention_days: 30,
            },
            defaults: DefaultSettings {
                agent: "claude-code".to_string(),
                role: "implementer".to_string(),
                auto_review: true,
                auto_checkpoint: true,
            },
            notifications: NotificationSettings {
                agent_completed: true,
                agent_failed: true,
                agent_needs_input: true,
                ci_status: true,
            },
        }
    }
}

impl AppConfig {
    /// Load config from global path, falling back to defaults
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".koompi-orch")
            .join("config.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: AppConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save config to global path
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = self.app.data_dir.join("config.toml");
        std::fs::create_dir_all(config_path.parent().unwrap())?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Ensure data directories exist
    pub fn ensure_dirs(&self) -> Result<(), Box<dyn std::error::Error>> {
        let dirs = [
            self.app.data_dir.clone(),
            self.app.data_dir.join("db"),
            self.app.data_dir.join("worktrees"),
            self.app.data_dir.join("plugins"),
            self.app.data_dir.join("logs"),
            self.app.data_dir.join("handoffs"),
        ];
        for dir in &dirs {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.app.theme, "dark");
        assert_eq!(config.app.max_concurrent_agents, 10);
        assert_eq!(config.defaults.agent, "claude-code");
        assert!(config.defaults.auto_review);
    }

    #[test]
    fn test_roundtrip_toml() {
        let config = AppConfig::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.app.theme, config.app.theme);
        assert_eq!(deserialized.defaults.agent, config.defaults.agent);
    }

    #[test]
    fn test_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let mut config = AppConfig::default();
        config.app.data_dir = tmp.path().to_path_buf();
        config.save().unwrap();

        let loaded_content = fs::read_to_string(tmp.path().join("config.toml")).unwrap();
        let loaded: AppConfig = toml::from_str(&loaded_content).unwrap();
        assert_eq!(loaded.app.theme, "dark");
    }

    #[test]
    fn test_ensure_dirs() {
        let tmp = TempDir::new().unwrap();
        let mut config = AppConfig::default();
        config.app.data_dir = tmp.path().to_path_buf();
        config.ensure_dirs().unwrap();

        assert!(tmp.path().join("db").exists());
        assert!(tmp.path().join("worktrees").exists());
        assert!(tmp.path().join("plugins").exists());
        assert!(tmp.path().join("logs").exists());
        assert!(tmp.path().join("handoffs").exists());
    }
}
