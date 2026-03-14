//! Smart agent routing rule engine.
//!
//! Routing priority (spec Section 15):
//! 1. User override (explicit agent/model selection)
//! 2. Pipeline step (role preset maps to agent)
//! 3. Keyword classification (task description keywords)
//! 4. Cost tier (role -> model tier mapping)
//! 5. Fallback (defaults from config)
//!
//! Configurable via .orch.toml [routing] table.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Routing decision: which agent and model to use.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutingDecision {
    pub agent_type: String,
    pub model: Option<String>,
    pub role: String,
    /// Which routing signal determined this decision.
    pub decided_by: RoutingSignal,
}

/// Which routing signal produced the decision.
#[derive(Debug, Clone, PartialEq)]
pub enum RoutingSignal {
    UserOverride,
    PipelineStep,
    KeywordClassification,
    CostTier,
    Fallback,
}

/// Per-role routing entry from .orch.toml [routing] table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEntry {
    pub agent: String,
    pub model: Option<String>,
}

/// Routing configuration loaded from .orch.toml.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingConfig {
    /// Role -> agent/model mapping.
    #[serde(flatten)]
    pub roles: HashMap<String, RouteEntry>,
}

/// Default agent and role from [defaults] section.
#[derive(Debug, Clone)]
pub struct DefaultsConfig {
    pub agent: String,
    pub role: String,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            agent: "claude-code".to_string(),
            role: "implementer".to_string(),
        }
    }
}

/// Keyword classification rules.
const KEYWORD_RULES: &[(&[&str], &str)] = &[
    (&["review", "audit", "security", "inspect", "check"], "reviewer"),
    (&["test", "spec", "coverage", "e2e", "unit test"], "tester"),
    (&["fix", "bug", "error", "crash", "broken", "regression"], "fixer"),
    (&["architect", "design", "plan", "rfc", "proposal"], "architect"),
    (&["ship", "deploy", "release", "publish", "ci"], "shipper"),
];

/// The fixer role maps to implementer routing unless explicitly configured.
const FIXER_FALLBACK_ROLE: &str = "implementer";

/// Smart routing engine.
pub struct Router {
    routing: RoutingConfig,
    defaults: DefaultsConfig,
}

impl Router {
    pub fn new(routing: RoutingConfig, defaults: DefaultsConfig) -> Self {
        Self { routing, defaults }
    }

    /// Route a task to an agent/model.
    pub fn route(
        &self,
        user_agent: Option<&str>,
        user_model: Option<&str>,
        pipeline_step_role: Option<&str>,
        task_description: &str,
    ) -> RoutingDecision {
        // Signal 1: User override.
        if user_agent.is_some() || user_model.is_some() {
            let role = pipeline_step_role
                .unwrap_or(&self.defaults.role)
                .to_string();
            return RoutingDecision {
                agent_type: user_agent
                    .unwrap_or(&self.defaults.agent)
                    .to_string(),
                model: user_model.map(|m| m.to_string()).or_else(|| {
                    self.routing
                        .roles
                        .get(&role)
                        .and_then(|e| e.model.clone())
                }),
                role,
                decided_by: RoutingSignal::UserOverride,
            };
        }

        // Signal 2: Pipeline step role.
        if let Some(role) = pipeline_step_role {
            if let Some(entry) = self.routing.roles.get(role) {
                return RoutingDecision {
                    agent_type: entry.agent.clone(),
                    model: entry.model.clone(),
                    role: role.to_string(),
                    decided_by: RoutingSignal::PipelineStep,
                };
            }
            return RoutingDecision {
                agent_type: self.defaults.agent.clone(),
                model: None,
                role: role.to_string(),
                decided_by: RoutingSignal::PipelineStep,
            };
        }

        // Signal 3: Keyword classification.
        let task_lower = task_description.to_lowercase();
        for (keywords, role) in KEYWORD_RULES {
            if keywords.iter().any(|kw| task_lower.contains(kw)) {
                let lookup_role = if *role == "fixer" {
                    if self.routing.roles.contains_key("fixer") {
                        "fixer"
                    } else {
                        FIXER_FALLBACK_ROLE
                    }
                } else {
                    role
                };

                if let Some(entry) = self.routing.roles.get(lookup_role) {
                    return RoutingDecision {
                        agent_type: entry.agent.clone(),
                        model: entry.model.clone(),
                        role: role.to_string(),
                        decided_by: RoutingSignal::KeywordClassification,
                    };
                }
                return RoutingDecision {
                    agent_type: self.defaults.agent.clone(),
                    model: None,
                    role: role.to_string(),
                    decided_by: RoutingSignal::KeywordClassification,
                };
            }
        }

        // Signal 4: Cost tier.
        if let Some(entry) = self.routing.roles.get(&self.defaults.role) {
            return RoutingDecision {
                agent_type: entry.agent.clone(),
                model: entry.model.clone(),
                role: self.defaults.role.clone(),
                decided_by: RoutingSignal::CostTier,
            };
        }

        // Signal 5: Fallback.
        RoutingDecision {
            agent_type: self.defaults.agent.clone(),
            model: None,
            role: self.defaults.role.clone(),
            decided_by: RoutingSignal::Fallback,
        }
    }

    /// Classify a task description into a role using keyword rules.
    pub fn classify_role(task_description: &str) -> Option<&'static str> {
        let task_lower = task_description.to_lowercase();
        for (keywords, role) in KEYWORD_RULES {
            if keywords.iter().any(|kw| task_lower.contains(kw)) {
                return Some(role);
            }
        }
        None
    }
}

/// Parse a [routing] TOML table into RoutingConfig.
pub fn parse_routing_config(toml_str: &str) -> Result<RoutingConfig, toml::de::Error> {
    toml::from_str(toml_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_routing_config() -> RoutingConfig {
        let mut roles = HashMap::new();
        roles.insert(
            "architect".into(),
            RouteEntry {
                agent: "claude-code".into(),
                model: Some("opus".into()),
            },
        );
        roles.insert(
            "implementer".into(),
            RouteEntry {
                agent: "claude-code".into(),
                model: Some("sonnet".into()),
            },
        );
        roles.insert(
            "reviewer".into(),
            RouteEntry {
                agent: "claude-code".into(),
                model: Some("opus".into()),
            },
        );
        roles.insert(
            "tester".into(),
            RouteEntry {
                agent: "claude-code".into(),
                model: Some("haiku".into()),
            },
        );
        roles.insert(
            "shipper".into(),
            RouteEntry {
                agent: "claude-code".into(),
                model: Some("haiku".into()),
            },
        );
        RoutingConfig { roles }
    }

    fn test_defaults() -> DefaultsConfig {
        DefaultsConfig::default()
    }

    #[test]
    fn test_user_override_wins_over_everything() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            Some("codex"),
            Some("gpt-5.4"),
            Some("architect"),
            "review this code for security issues",
        );

        assert_eq!(decision.agent_type, "codex");
        assert_eq!(decision.model, Some("gpt-5.4".into()));
        assert_eq!(decision.decided_by, RoutingSignal::UserOverride);
    }

    #[test]
    fn test_user_override_partial_agent_only() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            Some("aider"),
            None,
            None,
            "implement a feature",
        );

        assert_eq!(decision.agent_type, "aider");
        assert_eq!(decision.decided_by, RoutingSignal::UserOverride);
    }

    #[test]
    fn test_user_override_partial_model_only() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            None,
            Some("opus"),
            None,
            "implement a feature",
        );

        assert_eq!(decision.agent_type, "claude-code");
        assert_eq!(decision.model, Some("opus".into()));
        assert_eq!(decision.decided_by, RoutingSignal::UserOverride);
    }

    #[test]
    fn test_pipeline_step_role_routes_correctly() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(None, None, Some("architect"), "do stuff");

        assert_eq!(decision.agent_type, "claude-code");
        assert_eq!(decision.model, Some("opus".into()));
        assert_eq!(decision.role, "architect");
        assert_eq!(decision.decided_by, RoutingSignal::PipelineStep);
    }

    #[test]
    fn test_pipeline_step_unknown_role_uses_defaults() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(None, None, Some("custom_role"), "do stuff");

        assert_eq!(decision.agent_type, "claude-code");
        assert_eq!(decision.model, None);
        assert_eq!(decision.role, "custom_role");
        assert_eq!(decision.decided_by, RoutingSignal::PipelineStep);
    }

    #[test]
    fn test_keyword_classification_reviewer() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            None,
            None,
            None,
            "Review the authentication module for security vulnerabilities",
        );

        assert_eq!(decision.role, "reviewer");
        assert_eq!(decision.model, Some("opus".into()));
        assert_eq!(decision.decided_by, RoutingSignal::KeywordClassification);
    }

    #[test]
    fn test_keyword_classification_tester() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            None,
            None,
            None,
            "Write unit test coverage for the payment module",
        );

        assert_eq!(decision.role, "tester");
        assert_eq!(decision.model, Some("haiku".into()));
        assert_eq!(decision.decided_by, RoutingSignal::KeywordClassification);
    }

    #[test]
    fn test_keyword_classification_fixer_falls_back_to_implementer() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            None,
            None,
            None,
            "Fix the bug in the login flow",
        );

        assert_eq!(decision.role, "fixer");
        assert_eq!(decision.model, Some("sonnet".into()));
        assert_eq!(decision.decided_by, RoutingSignal::KeywordClassification);
    }

    #[test]
    fn test_keyword_classification_architect() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            None,
            None,
            None,
            "Design the new plugin architecture",
        );

        assert_eq!(decision.role, "architect");
        assert_eq!(decision.model, Some("opus".into()));
        assert_eq!(decision.decided_by, RoutingSignal::KeywordClassification);
    }

    #[test]
    fn test_no_keyword_match_falls_through_to_cost_tier() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            None,
            None,
            None,
            "Add a new dashboard component",
        );

        assert_eq!(decision.role, "implementer");
        assert_eq!(decision.model, Some("sonnet".into()));
        assert_eq!(decision.decided_by, RoutingSignal::CostTier);
    }

    #[test]
    fn test_fallback_when_no_routing_config() {
        let router = Router::new(RoutingConfig::default(), test_defaults());

        let decision = router.route(
            None,
            None,
            None,
            "Add a new dashboard component",
        );

        assert_eq!(decision.agent_type, "claude-code");
        assert_eq!(decision.model, None);
        assert_eq!(decision.role, "implementer");
        assert_eq!(decision.decided_by, RoutingSignal::Fallback);
    }

    #[test]
    fn test_classify_role_returns_none_for_unmatched() {
        assert!(Router::classify_role("Add a new feature").is_none());
    }

    #[test]
    fn test_classify_role_returns_correct_roles() {
        assert_eq!(Router::classify_role("review my code"), Some("reviewer"));
        assert_eq!(Router::classify_role("write a test"), Some("tester"));
        assert_eq!(Router::classify_role("fix the crash"), Some("fixer"));
        assert_eq!(Router::classify_role("design the API"), Some("architect"));
        assert_eq!(Router::classify_role("deploy to prod"), Some("shipper"));
    }

    #[test]
    fn test_parse_routing_config_from_toml() {
        let toml_str = r#"
architect = { agent = "claude-code", model = "opus" }
implementer = { agent = "claude-code", model = "sonnet" }
reviewer = { agent = "codex", model = "gpt-5.4" }
tester = { agent = "claude-code", model = "haiku" }
"#;
        let config = parse_routing_config(toml_str).unwrap();
        assert_eq!(config.roles.len(), 4);
        assert_eq!(config.roles["architect"].agent, "claude-code");
        assert_eq!(config.roles["architect"].model, Some("opus".into()));
        assert_eq!(config.roles["reviewer"].agent, "codex");
        assert_eq!(config.roles["reviewer"].model, Some("gpt-5.4".into()));
    }

    #[test]
    fn test_parse_routing_config_model_optional() {
        let toml_str = r#"
implementer = { agent = "aider" }
"#;
        let config = parse_routing_config(toml_str).unwrap();
        assert_eq!(config.roles["implementer"].agent, "aider");
        assert_eq!(config.roles["implementer"].model, None);
    }

    #[test]
    fn test_priority_pipeline_step_beats_keyword() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            None,
            None,
            Some("implementer"),
            "Review the code and fix bugs",
        );

        assert_eq!(decision.role, "implementer");
        assert_eq!(decision.decided_by, RoutingSignal::PipelineStep);
    }

    #[test]
    fn test_priority_user_override_beats_pipeline_step() {
        let router = Router::new(test_routing_config(), test_defaults());

        let decision = router.route(
            Some("aider"),
            None,
            Some("architect"),
            "Design the system",
        );

        assert_eq!(decision.agent_type, "aider");
        assert_eq!(decision.decided_by, RoutingSignal::UserOverride);
    }
}
