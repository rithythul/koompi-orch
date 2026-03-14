use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur in the agent engine.
#[derive(Debug, Error)]
pub enum AgentError {
    #[error("agent process failed to spawn: {0}")]
    SpawnFailed(String),

    #[error("agent process not running (pid: {0})")]
    ProcessNotRunning(u32),

    #[error("failed to write to agent stdin: {0}")]
    InputFailed(String),

    #[error("output parse error: {0}")]
    ParseError(String),

    #[error("template not found: {0}")]
    TemplateNotFound(String),

    #[error("preset not found: {0}")]
    PresetNotFound(String),

    #[error("database error: {0}")]
    DbError(String),

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("process already running for session {0}")]
    AlreadyRunning(String),

    #[error("unsupported input mode: {0}")]
    UnsupportedInputMode(String),

    #[error("unsupported output mode: {0}")]
    UnsupportedOutputMode(String),
}

/// How the agent receives input from the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InputMode {
    /// Write directly to PTY stdin (most agents).
    PtyStdin,
    /// Spawn a new process per message with the message as a CLI flag.
    FlagMessage,
    /// Write prompt to a temp file and pass the path as an argument.
    FilePrompt,
}

impl InputMode {
    pub fn from_str_loose(s: &str) -> Result<Self, AgentError> {
        match s {
            "pty_stdin" => Ok(Self::PtyStdin),
            "flag_message" => Ok(Self::FlagMessage),
            "file_prompt" => Ok(Self::FilePrompt),
            other => Err(AgentError::UnsupportedInputMode(other.to_string())),
        }
    }
}

/// How the agent's output is parsed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    /// Structured JSON lines (Claude Code).
    JsonStream,
    /// Regex heuristic detection of cost lines, tool blocks, etc.
    TextMarkers,
    /// No parsing — raw PTY passthrough.
    RawPty,
}

impl OutputMode {
    pub fn from_str_loose(s: &str) -> Result<Self, AgentError> {
        match s {
            "json_stream" => Ok(Self::JsonStream),
            "text_markers" => Ok(Self::TextMarkers),
            "raw_pty" => Ok(Self::RawPty),
            other => Err(AgentError::UnsupportedOutputMode(other.to_string())),
        }
    }
}

/// Unified event emitted by all output parsers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// Plain text output from the agent.
    Text { content: String },
    /// Agent is using a tool (e.g., Read, Write, Bash).
    ToolUse { name: String, input: serde_json::Value },
    /// Result returned from a tool invocation.
    ToolResult { name: String, output: String },
    /// Token usage and cost data.
    Usage { tokens_in: u64, tokens_out: u64, cost_usd: f64 },
    /// An error occurred in the agent.
    Error { message: String },
    /// Agent finished its task.
    Completed,
    /// Agent is waiting for user input.
    NeedsInput,
}

/// Runtime configuration for a single agent instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// The command to execute (e.g., "claude", "codex", "gemini").
    pub command: String,
    /// Arguments to pass to the command.
    pub args: Vec<String>,
    /// Environment variables to set for the child process.
    pub env: HashMap<String, String>,
    /// How the agent receives input.
    pub input_mode: InputMode,
    /// How the agent's output is parsed.
    pub output_mode: OutputMode,
    /// Whether the agent supports session resume.
    pub resume_support: bool,
    /// Working directory for the agent process.
    pub working_dir: PathBuf,
    /// Optional message flag name for FlagMessage input mode (e.g., "--message").
    pub message_flag: Option<String>,
    /// Optional print flag for FlagMessage mode (e.g., "--print").
    pub print_flag: Option<String>,
    /// Optional resume flag (e.g., "--resume", "--restore-chat-history").
    pub resume_flag: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            command: "claude".to_string(),
            args: vec!["--dangerously-skip-permissions".to_string()],
            env: HashMap::new(),
            input_mode: InputMode::PtyStdin,
            output_mode: OutputMode::JsonStream,
            resume_support: true,
            working_dir: PathBuf::from("."),
            message_flag: None,
            print_flag: None,
            resume_flag: Some("--resume".to_string()),
        }
    }
}

/// PTY size configuration.
#[derive(Debug, Clone, Copy)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
}

impl Default for PtySize {
    fn default() -> Self {
        Self { rows: 24, cols: 120 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_mode_from_str() {
        assert_eq!(InputMode::from_str_loose("pty_stdin").unwrap(), InputMode::PtyStdin);
        assert_eq!(InputMode::from_str_loose("flag_message").unwrap(), InputMode::FlagMessage);
        assert_eq!(InputMode::from_str_loose("file_prompt").unwrap(), InputMode::FilePrompt);
        assert!(InputMode::from_str_loose("bogus").is_err());
    }

    #[test]
    fn test_output_mode_from_str() {
        assert_eq!(OutputMode::from_str_loose("json_stream").unwrap(), OutputMode::JsonStream);
        assert_eq!(OutputMode::from_str_loose("text_markers").unwrap(), OutputMode::TextMarkers);
        assert_eq!(OutputMode::from_str_loose("raw_pty").unwrap(), OutputMode::RawPty);
        assert!(OutputMode::from_str_loose("bogus").is_err());
    }

    #[test]
    fn test_default_agent_config() {
        let config = AgentConfig::default();
        assert_eq!(config.command, "claude");
        assert_eq!(config.input_mode, InputMode::PtyStdin);
        assert_eq!(config.output_mode, OutputMode::JsonStream);
        assert!(config.resume_support);
        assert_eq!(config.resume_flag, Some("--resume".to_string()));
    }

    #[test]
    fn test_agent_event_serialize() {
        let event = AgentEvent::Text { content: "hello world".to_string() };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("hello world"));

        let event = AgentEvent::Usage { tokens_in: 100, tokens_out: 200, cost_usd: 0.05 };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"usage\""));
        assert!(json.contains("\"tokens_in\":100"));
    }

    #[test]
    fn test_agent_event_deserialize() {
        let json = r#"{"type":"tool_use","name":"Read","input":{"path":"/tmp/file.txt"}}"#;
        let event: AgentEvent = serde_json::from_str(json).unwrap();
        match event {
            AgentEvent::ToolUse { name, input } => {
                assert_eq!(name, "Read");
                assert_eq!(input["path"], "/tmp/file.txt");
            }
            _ => panic!("expected ToolUse"),
        }
    }

    #[test]
    fn test_agent_event_completed_roundtrip() {
        let event = AgentEvent::Completed;
        let json = serde_json::to_string(&event).unwrap();
        let parsed: AgentEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, AgentEvent::Completed);
    }

    #[test]
    fn test_agent_event_needs_input_roundtrip() {
        let event = AgentEvent::NeedsInput;
        let json = serde_json::to_string(&event).unwrap();
        let parsed: AgentEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, AgentEvent::NeedsInput);
    }

    #[test]
    fn test_agent_error_display() {
        let err = AgentError::TemplateNotFound("unknown-agent".to_string());
        assert_eq!(err.to_string(), "template not found: unknown-agent");

        let err = AgentError::SpawnFailed("command not found".to_string());
        assert_eq!(err.to_string(), "agent process failed to spawn: command not found");
    }

    #[test]
    fn test_pty_size_default() {
        let size = PtySize::default();
        assert_eq!(size.rows, 24);
        assert_eq!(size.cols, 120);
    }
}
