pub mod config;
pub mod input;
pub mod parser;
pub mod presets;
pub mod process;
pub mod registry;

pub use config::{AgentConfig, AgentError, AgentEvent, InputMode, OutputMode, PtySize};
pub use input::InputInjector;
pub use parser::{create_parser, OutputParser};
pub use presets::{builtin_presets, InjectionMethod, PresetRegistry, RolePreset};
pub use process::{AgentProcess, PortablePtySystem, PtyChild, PtySystem};
pub use registry::{builtin_templates, AgentRegistry, AgentTemplate};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::path::PathBuf;

    /// End-to-end test: template lookup -> config build -> mock spawn -> parse output -> verify events
    #[tokio::test]
    async fn test_template_to_process_pipeline() {
        // 1. Get builtin template
        let templates = builtin_templates();
        let claude_template = templates.iter().find(|t| t.name == "claude-code").unwrap();

        // 2. Convert to AgentConfig
        let config = claude_template
            .to_agent_config(PathBuf::from("/tmp/test-workspace"))
            .unwrap();

        assert_eq!(config.command, "claude");
        assert_eq!(config.output_mode, OutputMode::JsonStream);
        assert_eq!(config.input_mode, InputMode::PtyStdin);

        // 3. Create parser from config
        let mut parser = create_parser(&config.output_mode);

        // 4. Parse mock output
        let mock_output = vec![
            r#"{"type":"text","content":"I'll implement that feature."}"#,
            r#"{"type":"tool_use","name":"Write","input":{"path":"src/main.rs","content":"fn main() {}"}}"#,
            r#"{"type":"tool_result","name":"Write","output":"File written successfully"}"#,
            r#"{"type":"usage","input_tokens":500,"output_tokens":1200,"cost_usd":0.02}"#,
        ];

        let mut all_events = Vec::new();
        for line in &mock_output {
            let events = parser.parse_line(&format!("{}\n", line));
            all_events.extend(events);
        }

        let final_events = parser.on_exit(Some(0));
        all_events.extend(final_events);

        // 5. Verify the event sequence
        assert!(all_events.len() >= 5, "got {} events", all_events.len());
        assert!(matches!(&all_events[0], AgentEvent::Text { content } if content.contains("implement")));
        assert!(matches!(&all_events[1], AgentEvent::ToolUse { name, .. } if name == "Write"));
        assert!(matches!(&all_events[2], AgentEvent::ToolResult { name, .. } if name == "Write"));
        assert!(matches!(&all_events[3], AgentEvent::Usage { tokens_in: 500, .. }));
        assert!(matches!(&all_events[4], AgentEvent::Completed));
    }

    /// Test: preset + input injection format first message
    #[tokio::test]
    async fn test_preset_input_injection_pipeline() {
        // 1. Get architect preset
        let presets = builtin_presets();
        let architect = presets.iter().find(|p| p.name == "architect").unwrap();

        // 2. Format first message with role injection
        let formatted = architect.format_first_message("Design a caching layer");

        assert!(formatted.starts_with("[Role: Architect]"));
        assert!(formatted.contains("first principles"));
        assert!(formatted.contains("User task: Design a caching layer"));

        // 3. Verify it can be sent through InputInjector
        let mut mock_writer = input::MockPtyWriter::new();
        InputInjector::send_pty_stdin(&mut mock_writer, &formatted).unwrap();

        let written = mock_writer.written_string();
        assert!(written.contains("[Role: Architect]"));
        assert!(written.contains("Design a caching layer"));
        assert!(written.ends_with('\n'));
    }

    /// Test: handoff formatting for pipeline step transitions
    #[test]
    fn test_handoff_format_pipeline() {
        let handoff = InputInjector::format_with_handoff(
            "Implement the caching layer as designed",
            "Architecture decision:\n- Use Redis for distributed cache\n- TTL of 5 minutes\n- Cache-aside pattern",
            "architect",
        );

        assert!(handoff.contains("## Context from previous step (architect)"));
        assert!(handoff.contains("Redis for distributed cache"));
        assert!(handoff.contains("## Your task"));
        assert!(handoff.contains("Implement the caching layer"));
    }

    /// Test: text_markers parser handles a realistic multi-line session
    #[test]
    fn test_text_markers_realistic_session() {
        let mut parser = parser::TextMarkersParser::new();

        let lines = vec![
            "I'll help you with that.\n",
            "Reading `src/main.rs`\n",
            "```rust\n",
            "fn main() {\n",
            "    println!(\"Error: not really\");\n", // should NOT be detected as error
            "}\n",
            "```\n",
            "Writing the fix now.\n",
            "Tokens: 1,500 input / 3,000 output\n",
            "Cost: $0.12\n",
        ];

        let mut all_events = Vec::new();
        for line in &lines {
            all_events.extend(parser.parse_line(line));
        }

        // Count event types
        let text_count = all_events
            .iter()
            .filter(|e| matches!(e, AgentEvent::Text { .. }))
            .count();
        let tool_count = all_events
            .iter()
            .filter(|e| matches!(e, AgentEvent::ToolUse { .. }))
            .count();
        let usage_count = all_events
            .iter()
            .filter(|e| matches!(e, AgentEvent::Usage { .. }))
            .count();
        let error_count = all_events
            .iter()
            .filter(|e| matches!(e, AgentEvent::Error { .. }))
            .count();

        assert!(
            text_count >= 2,
            "expected at least 2 text events, got {}",
            text_count
        );
        assert_eq!(tool_count, 2, "expected 2 tool uses (Reading + Writing)");
        assert_eq!(usage_count, 2, "expected 2 usage events (tokens + cost)");
        assert_eq!(
            error_count, 0,
            "error inside code fence should be suppressed"
        );
    }
}
