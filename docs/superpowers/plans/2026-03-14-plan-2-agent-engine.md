# Plan 2: Agent Engine — PTY Process Management, Output Parsing, Input Injection

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Rust backend modules that spawn, manage, parse output from, and inject input into CLI-based AI agents via PTY. This is the core engine that makes koompi-orch agent-agnostic — any CLI tool (Claude Code, Codex, Gemini CLI, aider, custom scripts) can be orchestrated through a unified interface.

**Architecture:** The `agent` module sits in the Tauri backend (`src-tauri/src/agent/`). It uses `portable-pty` to spawn child processes in pseudo-terminals, providing full terminal emulation. Output is parsed through pluggable parsers (`json_stream`, `text_markers`, `raw_pty`) into a unified `AgentEvent` enum. Input reaches agents through configurable injection modes (`pty_stdin`, `flag_message`, `file_prompt`). Agent templates and role presets are loaded from SurrealDB (seeded in Plan 1) and can be extended with custom entries. All PTY interactions are abstracted behind traits to enable testing without real process spawning.

**Tech Stack:** Rust, portable-pty 0.8, tokio (async runtime), serde/serde_json (parsing), regex (text_markers), SurrealDB (template/preset storage), thiserror (error types), uuid (temp file naming), chrono (timestamps)

**Spec Reference:** `/home/userx/projects/koompi-orch/docs/superpowers/specs/2026-03-14-koompi-orch-design.md` — Sections 10 (Agent Templates), 11 (Role Presets), 16 (Agent Output Parsing), 17 (Agent Input Protocol)

---

## Chunk 1: Core Types and Configuration

### Task 1: Agent config and shared types (`agent/config.rs`, `agent/mod.rs`)

**Files:**
- Create: `src-tauri/src/agent/config.rs`
- Create: `src-tauri/src/agent/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/agent/config.rs`:
```rust
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
```

Create `src-tauri/src/agent/mod.rs`:
```rust
pub mod config;

pub use config::{AgentConfig, AgentError, AgentEvent, InputMode, OutputMode, PtySize};
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test agent::config::tests`
Expected: FAIL with "can't find crate for `agent`" (module not wired into lib.rs yet)

- [ ] **Step 3: Write minimal implementation**

The implementation is already in Step 1 (types + tests in one file). Wire the module into `src-tauri/src/lib.rs` by adding:
```rust
pub mod agent;
```

- [ ] **Step 4: Run test to verify it passes**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test agent::config::tests`
Expected: PASS — all 8 tests pass

- [ ] **Step 5: Commit**
```bash
scripts/committer "feat(agent): add core types — AgentEvent, AgentConfig, InputMode, OutputMode, AgentError" src-tauri/src/agent/config.rs src-tauri/src/agent/mod.rs src-tauri/src/lib.rs
```

---

## Chunk 2: Output Parsers

### Task 2: JSON stream parser (`agent/parser.rs` — `json_stream`)

**Files:**
- Create: `src-tauri/src/agent/parser.rs`
- Modify: `src-tauri/src/agent/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/agent/parser.rs`:
```rust
use crate::agent::config::{AgentEvent, AgentError, OutputMode};
use regex::Regex;
use std::sync::LazyLock;

/// Trait for output parsers. Each output mode implements this.
pub trait OutputParser: Send + Sync {
    /// Parse a single line of output into zero or more AgentEvents.
    fn parse_line(&mut self, line: &str) -> Vec<AgentEvent>;

    /// Signal that the agent process has exited. Returns any final events.
    fn on_exit(&mut self, exit_code: Option<i32>) -> Vec<AgentEvent>;
}

/// Create a parser for the given output mode.
pub fn create_parser(mode: &OutputMode) -> Box<dyn OutputParser> {
    match mode {
        OutputMode::JsonStream => Box::new(JsonStreamParser::new()),
        OutputMode::TextMarkers => Box::new(TextMarkersParser::new()),
        OutputMode::RawPty => Box::new(RawPtyParser::new()),
    }
}

// ---------------------------------------------------------------------------
// json_stream parser — Claude Code structured JSON output
// ---------------------------------------------------------------------------

/// Parses structured JSON lines from Claude Code.
///
/// Expected line formats:
/// - `{"type":"text","content":"..."}`
/// - `{"type":"tool_use","name":"Read","input":{...}}`
/// - `{"type":"tool_result","name":"Read","output":"..."}`
/// - `{"type":"usage","input_tokens":N,"output_tokens":N}`
/// - `{"type":"error","message":"..."}`
///
/// Lines that are not valid JSON or do not match known types are emitted
/// as `AgentEvent::Text` with the raw content.
pub struct JsonStreamParser {
    /// Buffer for incomplete JSON lines (streaming may split mid-line).
    buffer: String,
}

impl JsonStreamParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    fn try_parse_json(&self, line: &str) -> Option<AgentEvent> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Must start with '{' to be a JSON object
        if !trimmed.starts_with('{') {
            return None;
        }

        let value: serde_json::Value = serde_json::from_str(trimmed).ok()?;
        let obj = value.as_object()?;

        let event_type = obj.get("type")?.as_str()?;

        match event_type {
            "text" => {
                let content = obj.get("content")?.as_str()?.to_string();
                Some(AgentEvent::Text { content })
            }
            "tool_use" => {
                let name = obj.get("name")?.as_str()?.to_string();
                let input = obj.get("input").cloned().unwrap_or(serde_json::Value::Null);
                Some(AgentEvent::ToolUse { name, input })
            }
            "tool_result" => {
                let name = obj.get("name")?.as_str()?.to_string();
                let output = obj.get("output")?.as_str()?.to_string();
                Some(AgentEvent::ToolResult { name, output })
            }
            "usage" => {
                let tokens_in = obj
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let tokens_out = obj
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let cost_usd = obj
                    .get("cost_usd")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                Some(AgentEvent::Usage {
                    tokens_in,
                    tokens_out,
                    cost_usd,
                })
            }
            "error" => {
                let message = obj.get("message")?.as_str()?.to_string();
                Some(AgentEvent::Error { message })
            }
            _ => None,
        }
    }
}

impl OutputParser for JsonStreamParser {
    fn parse_line(&mut self, line: &str) -> Vec<AgentEvent> {
        let mut events = Vec::new();

        // Append to buffer (handles split lines from streaming)
        self.buffer.push_str(line);

        // Try to parse. If it works, clear buffer. If not, check if the line
        // ends with a newline (complete) — if so, emit as raw text and clear.
        if let Some(event) = self.try_parse_json(&self.buffer) {
            events.push(event);
            self.buffer.clear();
        } else if line.ends_with('\n') || line.ends_with('\r') {
            // Complete line that is not JSON — emit as text
            let content = self.buffer.trim().to_string();
            if !content.is_empty() {
                events.push(AgentEvent::Text { content });
            }
            self.buffer.clear();
        }
        // Otherwise keep buffering (incomplete JSON line)

        events
    }

    fn on_exit(&mut self, exit_code: Option<i32>) -> Vec<AgentEvent> {
        let mut events = Vec::new();

        // Flush any remaining buffer
        if !self.buffer.trim().is_empty() {
            let content = self.buffer.trim().to_string();
            events.push(AgentEvent::Text { content });
            self.buffer.clear();
        }

        match exit_code {
            Some(0) | None => events.push(AgentEvent::Completed),
            Some(code) => events.push(AgentEvent::Error {
                message: format!("agent exited with code {}", code),
            }),
        }

        events
    }
}

// ---------------------------------------------------------------------------
// text_markers parser — Codex, Gemini CLI, aider, etc.
// ---------------------------------------------------------------------------

// Precompiled regex patterns for text_markers parsing.
static COST_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:cost|total|spent|price):\s*\$(\d+\.?\d*)").unwrap()
});

static TOKENS_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:tokens?):\s*([\d,]+)\s*(?:in|input)\s*/?\s*([\d,]+)\s*(?:out|output)")
        .unwrap()
});

static ERROR_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:error|exception|traceback|panic|fatal)\b").unwrap()
});

static TOOL_START_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:│\s*)?(?:Reading|Writing|Editing|Running|Executing|Creating|Deleting)\s+[`'\"]?(\S+)")
        .unwrap()
});

static CODE_FENCE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^```").unwrap()
});

/// Parses plain text output using regex heuristics.
///
/// Detects:
/// - Cost lines: `Cost: $0.05` patterns
/// - Token lines: `Tokens: 1,200 in / 3,400 out` patterns
/// - Tool use: lines starting with action verbs (Reading, Writing, etc.)
/// - Error patterns: error/exception/traceback keywords
/// - Code fences: tracks ``` blocks to avoid false positives inside them
///
/// Unrecognized output passes through as `AgentEvent::Text`.
pub struct TextMarkersParser {
    /// Whether we are currently inside a code fence (``` block).
    in_code_fence: bool,
}

impl TextMarkersParser {
    pub fn new() -> Self {
        Self {
            in_code_fence: false,
        }
    }

    fn parse_tokens(s: &str) -> u64 {
        s.replace(',', "").parse::<u64>().unwrap_or(0)
    }
}

impl OutputParser for TextMarkersParser {
    fn parse_line(&mut self, line: &str) -> Vec<AgentEvent> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return vec![];
        }

        // Track code fences to avoid false positives inside them
        if CODE_FENCE_PATTERN.is_match(trimmed) {
            self.in_code_fence = !self.in_code_fence;
            return vec![AgentEvent::Text {
                content: trimmed.to_string(),
            }];
        }

        // Inside a code fence, pass everything through as text
        if self.in_code_fence {
            return vec![AgentEvent::Text {
                content: trimmed.to_string(),
            }];
        }

        // Check for cost patterns
        if let Some(caps) = COST_PATTERN.captures(trimmed) {
            if let Some(cost_str) = caps.get(1) {
                if let Ok(cost) = cost_str.as_str().parse::<f64>() {
                    return vec![AgentEvent::Usage {
                        tokens_in: 0,
                        tokens_out: 0,
                        cost_usd: cost,
                    }];
                }
            }
        }

        // Check for token usage patterns
        if let Some(caps) = TOKENS_PATTERN.captures(trimmed) {
            let tokens_in = caps
                .get(1)
                .map(|m| Self::parse_tokens(m.as_str()))
                .unwrap_or(0);
            let tokens_out = caps
                .get(2)
                .map(|m| Self::parse_tokens(m.as_str()))
                .unwrap_or(0);
            return vec![AgentEvent::Usage {
                tokens_in,
                tokens_out,
                cost_usd: 0.0,
            }];
        }

        // Check for error patterns
        if ERROR_PATTERN.is_match(trimmed) {
            return vec![AgentEvent::Error {
                message: trimmed.to_string(),
            }];
        }

        // Check for tool use patterns
        if let Some(caps) = TOOL_START_PATTERN.captures(trimmed) {
            let tool_target = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
            return vec![AgentEvent::ToolUse {
                name: trimmed.split_whitespace().next().unwrap_or("Unknown").to_string(),
                input: serde_json::json!({ "target": tool_target }),
            }];
        }

        // Default: pass through as text
        vec![AgentEvent::Text {
            content: trimmed.to_string(),
        }]
    }

    fn on_exit(&mut self, exit_code: Option<i32>) -> Vec<AgentEvent> {
        match exit_code {
            Some(0) | None => vec![AgentEvent::Completed],
            Some(code) => vec![AgentEvent::Error {
                message: format!("agent exited with code {}", code),
            }],
        }
    }
}

// ---------------------------------------------------------------------------
// raw_pty parser — no parsing, passthrough
// ---------------------------------------------------------------------------

/// No-op parser. Passes all output through as `AgentEvent::Text`.
pub struct RawPtyParser;

impl RawPtyParser {
    pub fn new() -> Self {
        Self
    }
}

impl OutputParser for RawPtyParser {
    fn parse_line(&mut self, line: &str) -> Vec<AgentEvent> {
        if line.trim().is_empty() {
            return vec![];
        }
        vec![AgentEvent::Text {
            content: line.to_string(),
        }]
    }

    fn on_exit(&mut self, exit_code: Option<i32>) -> Vec<AgentEvent> {
        match exit_code {
            Some(0) | None => vec![AgentEvent::Completed],
            Some(code) => vec![AgentEvent::Error {
                message: format!("agent exited with code {}", code),
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // json_stream tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_json_stream_parse_text() {
        let mut parser = JsonStreamParser::new();
        let events = parser.parse_line("{\"type\":\"text\",\"content\":\"Hello world\"}\n");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Text { content } => assert_eq!(content, "Hello world"),
            other => panic!("expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_json_stream_parse_tool_use() {
        let mut parser = JsonStreamParser::new();
        let line = r#"{"type":"tool_use","name":"Read","input":{"path":"/tmp/test.rs"}}"#;
        let events = parser.parse_line(&format!("{}\n", line));
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::ToolUse { name, input } => {
                assert_eq!(name, "Read");
                assert_eq!(input["path"], "/tmp/test.rs");
            }
            other => panic!("expected ToolUse, got {:?}", other),
        }
    }

    #[test]
    fn test_json_stream_parse_tool_result() {
        let mut parser = JsonStreamParser::new();
        let line = r#"{"type":"tool_result","name":"Bash","output":"success"}"#;
        let events = parser.parse_line(&format!("{}\n", line));
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::ToolResult { name, output } => {
                assert_eq!(name, "Bash");
                assert_eq!(output, "success");
            }
            other => panic!("expected ToolResult, got {:?}", other),
        }
    }

    #[test]
    fn test_json_stream_parse_usage() {
        let mut parser = JsonStreamParser::new();
        let line = r#"{"type":"usage","input_tokens":1500,"output_tokens":3200,"cost_usd":0.08}"#;
        let events = parser.parse_line(&format!("{}\n", line));
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Usage {
                tokens_in,
                tokens_out,
                cost_usd,
            } => {
                assert_eq!(*tokens_in, 1500);
                assert_eq!(*tokens_out, 3200);
                assert!((cost_usd - 0.08).abs() < f64::EPSILON);
            }
            other => panic!("expected Usage, got {:?}", other),
        }
    }

    #[test]
    fn test_json_stream_parse_error() {
        let mut parser = JsonStreamParser::new();
        let line = r#"{"type":"error","message":"rate limit exceeded"}"#;
        let events = parser.parse_line(&format!("{}\n", line));
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Error { message } => assert_eq!(message, "rate limit exceeded"),
            other => panic!("expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_json_stream_non_json_passthrough() {
        let mut parser = JsonStreamParser::new();
        let events = parser.parse_line("This is plain text output\n");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Text { content } => assert_eq!(content, "This is plain text output"),
            other => panic!("expected Text passthrough, got {:?}", other),
        }
    }

    #[test]
    fn test_json_stream_empty_line() {
        let mut parser = JsonStreamParser::new();
        let events = parser.parse_line("\n");
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_json_stream_unknown_type_ignored() {
        let mut parser = JsonStreamParser::new();
        let line = r#"{"type":"unknown_type","data":"something"}"#;
        // Unknown JSON type does not parse to known event; no newline means buffered.
        let events = parser.parse_line(&format!("{}\n", line));
        // Falls through as raw text
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Text { content } => {
                assert!(content.contains("unknown_type"));
            }
            other => panic!("expected Text fallback, got {:?}", other),
        }
    }

    #[test]
    fn test_json_stream_on_exit_success() {
        let mut parser = JsonStreamParser::new();
        let events = parser.on_exit(Some(0));
        assert_eq!(events, vec![AgentEvent::Completed]);
    }

    #[test]
    fn test_json_stream_on_exit_failure() {
        let mut parser = JsonStreamParser::new();
        let events = parser.on_exit(Some(1));
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Error { message } => assert!(message.contains("code 1")),
            other => panic!("expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_json_stream_on_exit_flushes_buffer() {
        let mut parser = JsonStreamParser::new();
        // Feed incomplete line (no newline, not valid JSON)
        parser.parse_line("partial data");
        let events = parser.on_exit(Some(0));
        assert_eq!(events.len(), 2); // flushed text + Completed
        match &events[0] {
            AgentEvent::Text { content } => assert_eq!(content, "partial data"),
            other => panic!("expected Text, got {:?}", other),
        }
        assert_eq!(events[1], AgentEvent::Completed);
    }

    #[test]
    fn test_json_stream_usage_missing_cost() {
        let mut parser = JsonStreamParser::new();
        let line = r#"{"type":"usage","input_tokens":500,"output_tokens":1000}"#;
        let events = parser.parse_line(&format!("{}\n", line));
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Usage { cost_usd, .. } => {
                assert!((cost_usd - 0.0).abs() < f64::EPSILON);
            }
            other => panic!("expected Usage, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // text_markers tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_text_markers_cost_line() {
        let mut parser = TextMarkersParser::new();
        let events = parser.parse_line("Cost: $0.05\n");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Usage { cost_usd, .. } => {
                assert!((cost_usd - 0.05).abs() < f64::EPSILON);
            }
            other => panic!("expected Usage, got {:?}", other),
        }
    }

    #[test]
    fn test_text_markers_cost_total() {
        let mut parser = TextMarkersParser::new();
        let events = parser.parse_line("Total: $1.23\n");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Usage { cost_usd, .. } => {
                assert!((cost_usd - 1.23).abs() < f64::EPSILON);
            }
            other => panic!("expected Usage, got {:?}", other),
        }
    }

    #[test]
    fn test_text_markers_tokens_line() {
        let mut parser = TextMarkersParser::new();
        let events = parser.parse_line("Tokens: 1,200 in / 3,400 out\n");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Usage {
                tokens_in,
                tokens_out,
                ..
            } => {
                assert_eq!(*tokens_in, 1200);
                assert_eq!(*tokens_out, 3400);
            }
            other => panic!("expected Usage, got {:?}", other),
        }
    }

    #[test]
    fn test_text_markers_error_detection() {
        let mut parser = TextMarkersParser::new();
        let events = parser.parse_line("Error: connection refused\n");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Error { message } => {
                assert!(message.contains("Error: connection refused"));
            }
            other => panic!("expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_text_markers_traceback_detection() {
        let mut parser = TextMarkersParser::new();
        let events = parser.parse_line("Traceback (most recent call last):\n");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Error { .. } => {}
            other => panic!("expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_text_markers_tool_use() {
        let mut parser = TextMarkersParser::new();
        let events = parser.parse_line("Reading `/home/user/src/main.rs`\n");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::ToolUse { name, input } => {
                assert_eq!(name, "Reading");
                assert!(input["target"].as_str().unwrap().contains("home"));
            }
            other => panic!("expected ToolUse, got {:?}", other),
        }
    }

    #[test]
    fn test_text_markers_plain_text_passthrough() {
        let mut parser = TextMarkersParser::new();
        let events = parser.parse_line("I'll implement the feature now.\n");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Text { content } => {
                assert!(content.contains("implement the feature"));
            }
            other => panic!("expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_text_markers_code_fence_exclusion() {
        let mut parser = TextMarkersParser::new();

        // Enter code fence
        let events = parser.parse_line("```rust\n");
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], AgentEvent::Text { .. }));

        // Inside code fence — "Error" should NOT trigger error detection
        let events = parser.parse_line("    panic!(\"Error: test failed\");\n");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Text { content } => {
                assert!(content.contains("panic!"));
            }
            other => panic!("expected Text inside fence, got {:?}", other),
        }

        // Close code fence
        let events = parser.parse_line("```\n");
        assert_eq!(events.len(), 1);

        // Outside code fence — "Error" triggers again
        let events = parser.parse_line("Error: actual error\n");
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], AgentEvent::Error { .. }));
    }

    #[test]
    fn test_text_markers_empty_line() {
        let mut parser = TextMarkersParser::new();
        let events = parser.parse_line("\n");
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_text_markers_on_exit() {
        let mut parser = TextMarkersParser::new();
        assert_eq!(parser.on_exit(Some(0)), vec![AgentEvent::Completed]);
        let err_events = parser.on_exit(Some(127));
        match &err_events[0] {
            AgentEvent::Error { message } => assert!(message.contains("127")),
            other => panic!("expected Error, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // raw_pty tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_raw_pty_passthrough() {
        let mut parser = RawPtyParser::new();
        let events = parser.parse_line("anything at all\n");
        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::Text { content } => assert_eq!(content, "anything at all\n"),
            other => panic!("expected Text, got {:?}", other),
        }
    }

    #[test]
    fn test_raw_pty_empty() {
        let mut parser = RawPtyParser::new();
        let events = parser.parse_line("  \n");
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_raw_pty_on_exit() {
        let mut parser = RawPtyParser::new();
        assert_eq!(parser.on_exit(None), vec![AgentEvent::Completed]);
    }

    // -----------------------------------------------------------------------
    // create_parser factory tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_parser_json_stream() {
        let mut parser = create_parser(&OutputMode::JsonStream);
        let events = parser.parse_line("{\"type\":\"text\",\"content\":\"hi\"}\n");
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], AgentEvent::Text { content } if content == "hi"));
    }

    #[test]
    fn test_create_parser_text_markers() {
        let mut parser = create_parser(&OutputMode::TextMarkers);
        let events = parser.parse_line("Cost: $0.10\n");
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], AgentEvent::Usage { .. }));
    }

    #[test]
    fn test_create_parser_raw_pty() {
        let mut parser = create_parser(&OutputMode::RawPty);
        let events = parser.parse_line("raw output\n");
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], AgentEvent::Text { .. }));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test agent::parser::tests`
Expected: FAIL — module not declared in `agent/mod.rs` yet

- [ ] **Step 3: Write minimal implementation**

The implementation is in Step 1. Add `regex` to `Cargo.toml` if not already present:
```toml
regex = "1"
```

Update `src-tauri/src/agent/mod.rs`:
```rust
pub mod config;
pub mod parser;

pub use config::{AgentConfig, AgentError, AgentEvent, InputMode, OutputMode, PtySize};
pub use parser::{create_parser, OutputParser};
```

- [ ] **Step 4: Run test to verify it passes**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test agent::parser::tests`
Expected: PASS — all 24 parser tests pass

- [ ] **Step 5: Commit**
```bash
scripts/committer "feat(agent): add output parsers — json_stream, text_markers, raw_pty" src-tauri/src/agent/parser.rs src-tauri/src/agent/mod.rs src-tauri/Cargo.toml
```

---

## Chunk 3: Input Injection

### Task 3: Input injection module (`agent/input.rs`)

**Files:**
- Create: `src-tauri/src/agent/input.rs`
- Modify: `src-tauri/src/agent/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/agent/input.rs`:
```rust
use crate::agent::config::{AgentConfig, AgentError, InputMode};
use std::io::Write;
use std::path::PathBuf;

/// Trait abstracting PTY stdin writes. Allows mock injection in tests.
pub trait PtyWriter: Send + Sync {
    fn write_all(&mut self, data: &[u8]) -> Result<(), std::io::Error>;
}

/// Real PTY writer wrapping a boxed Write.
pub struct RealPtyWriter {
    writer: Box<dyn Write + Send>,
}

impl RealPtyWriter {
    pub fn new(writer: Box<dyn Write + Send>) -> Self {
        Self { writer }
    }
}

impl PtyWriter for RealPtyWriter {
    fn write_all(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        self.writer.write_all(data)?;
        self.writer.flush()
    }
}

/// Mock PTY writer for testing.
#[cfg(test)]
pub struct MockPtyWriter {
    pub written: Vec<u8>,
}

#[cfg(test)]
impl MockPtyWriter {
    pub fn new() -> Self {
        Self {
            written: Vec::new(),
        }
    }

    pub fn written_string(&self) -> String {
        String::from_utf8_lossy(&self.written).to_string()
    }
}

#[cfg(test)]
impl PtyWriter for MockPtyWriter {
    fn write_all(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        self.written.extend_from_slice(data);
        Ok(())
    }
}

/// Handles sending input to an agent based on its input mode.
pub struct InputInjector;

impl InputInjector {
    /// Send a message to an agent via PTY stdin.
    ///
    /// Writes the message followed by a newline to the PTY.
    pub fn send_pty_stdin(
        writer: &mut dyn PtyWriter,
        message: &str,
    ) -> Result<(), AgentError> {
        let data = format!("{}\n", message);
        writer
            .write_all(data.as_bytes())
            .map_err(|e| AgentError::InputFailed(e.to_string()))
    }

    /// Send a message with role prepended (for first message in a session).
    ///
    /// Format:
    /// ```text
    /// [Role: Architect] Think from first principles, design before coding...
    ///
    /// User task: Implement JWT authentication
    /// ```
    pub fn send_pty_stdin_with_role(
        writer: &mut dyn PtyWriter,
        message: &str,
        role_name: &str,
        role_prompt: &str,
    ) -> Result<(), AgentError> {
        let data = format!(
            "[Role: {}] {}\n\nUser task: {}\n",
            capitalize_first(role_name),
            role_prompt,
            message
        );
        writer
            .write_all(data.as_bytes())
            .map_err(|e| AgentError::InputFailed(e.to_string()))
    }

    /// Build command args for flag_message input mode.
    ///
    /// Returns the full args list including the message flag.
    /// Example: `["--message", "Implement JWT auth", "--print"]`
    pub fn build_flag_message_args(
        config: &AgentConfig,
        message: &str,
    ) -> Result<Vec<String>, AgentError> {
        let msg_flag = config
            .message_flag
            .as_deref()
            .unwrap_or("--message");

        let mut args = config.args.clone();
        args.push(msg_flag.to_string());
        args.push(message.to_string());

        if let Some(print_flag) = &config.print_flag {
            args.push(print_flag.clone());
        }

        Ok(args)
    }

    /// Build command args for file_prompt input mode.
    ///
    /// Writes the prompt to a temp file and returns args including the file path.
    /// The temp file is at `/tmp/koompi-orch-prompt-{uuid}.md`.
    pub fn build_file_prompt_args(
        config: &AgentConfig,
        message: &str,
        role_context: Option<(&str, &str)>,
    ) -> Result<(Vec<String>, PathBuf), AgentError> {
        let file_id = uuid::Uuid::new_v4();
        let temp_path = std::env::temp_dir().join(format!("koompi-orch-prompt-{}.md", file_id));

        let content = if let Some((role_name, role_prompt)) = role_context {
            format!(
                "[Role: {}] {}\n\nUser task: {}\n",
                capitalize_first(role_name),
                role_prompt,
                message,
            )
        } else {
            message.to_string()
        };

        std::fs::write(&temp_path, &content)
            .map_err(|e| AgentError::InputFailed(format!("failed to write prompt file: {}", e)))?;

        let mut args = config.args.clone();
        args.push(temp_path.to_string_lossy().to_string());

        Ok((args, temp_path))
    }

    /// Build command args with handoff context prepended.
    ///
    /// Format:
    /// ```text
    /// ## Context from previous step (architect)
    /// [handoff content here]
    ///
    /// ## Your task
    /// [user's original task description]
    /// ```
    pub fn format_with_handoff(
        message: &str,
        handoff_content: &str,
        previous_role: &str,
    ) -> String {
        format!(
            "## Context from previous step ({})\n{}\n\n## Your task\n{}\n",
            previous_role, handoff_content, message
        )
    }
}

/// Capitalize the first character of a string.
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
    use std::collections::HashMap;

    fn test_config() -> AgentConfig {
        AgentConfig {
            command: "claude".to_string(),
            args: vec!["--dangerously-skip-permissions".to_string()],
            env: HashMap::new(),
            input_mode: InputMode::PtyStdin,
            output_mode: crate::agent::config::OutputMode::JsonStream,
            resume_support: false,
            working_dir: PathBuf::from("/tmp"),
            message_flag: Some("--message".to_string()),
            print_flag: Some("--print".to_string()),
            resume_flag: None,
        }
    }

    #[test]
    fn test_send_pty_stdin() {
        let mut writer = MockPtyWriter::new();
        InputInjector::send_pty_stdin(&mut writer, "hello agent").unwrap();
        assert_eq!(writer.written_string(), "hello agent\n");
    }

    #[test]
    fn test_send_pty_stdin_multiline() {
        let mut writer = MockPtyWriter::new();
        InputInjector::send_pty_stdin(&mut writer, "line 1\nline 2").unwrap();
        assert_eq!(writer.written_string(), "line 1\nline 2\n");
    }

    #[test]
    fn test_send_pty_stdin_with_role() {
        let mut writer = MockPtyWriter::new();
        InputInjector::send_pty_stdin_with_role(
            &mut writer,
            "Implement JWT auth",
            "architect",
            "Think from first principles, design before coding",
        )
        .unwrap();

        let written = writer.written_string();
        assert!(written.starts_with("[Role: Architect]"));
        assert!(written.contains("Think from first principles"));
        assert!(written.contains("User task: Implement JWT auth"));
    }

    #[test]
    fn test_build_flag_message_args() {
        let config = test_config();
        let args =
            InputInjector::build_flag_message_args(&config, "Implement JWT auth").unwrap();
        assert_eq!(
            args,
            vec![
                "--dangerously-skip-permissions",
                "--message",
                "Implement JWT auth",
                "--print",
            ]
        );
    }

    #[test]
    fn test_build_flag_message_args_no_print() {
        let mut config = test_config();
        config.print_flag = None;
        let args =
            InputInjector::build_flag_message_args(&config, "hello").unwrap();
        assert_eq!(
            args,
            vec!["--dangerously-skip-permissions", "--message", "hello"]
        );
    }

    #[test]
    fn test_build_flag_message_args_default_flag() {
        let mut config = test_config();
        config.message_flag = None;
        let args =
            InputInjector::build_flag_message_args(&config, "hello").unwrap();
        // Falls back to --message
        assert!(args.contains(&"--message".to_string()));
    }

    #[test]
    fn test_build_file_prompt_args() {
        let config = test_config();
        let (args, temp_path) =
            InputInjector::build_file_prompt_args(&config, "Build a REST API", None).unwrap();

        // Temp file should exist and contain the message
        assert!(temp_path.exists());
        let content = std::fs::read_to_string(&temp_path).unwrap();
        assert_eq!(content, "Build a REST API");

        // Args should include base args + temp file path
        assert_eq!(args.len(), 2); // --dangerously-skip-permissions + path
        assert_eq!(args[0], "--dangerously-skip-permissions");
        assert!(args[1].contains("koompi-orch-prompt-"));

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_build_file_prompt_args_with_role() {
        let config = test_config();
        let (_, temp_path) = InputInjector::build_file_prompt_args(
            &config,
            "Build a REST API",
            Some(("reviewer", "Paranoid code review")),
        )
        .unwrap();

        let content = std::fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("[Role: Reviewer]"));
        assert!(content.contains("Paranoid code review"));
        assert!(content.contains("User task: Build a REST API"));

        // Cleanup
        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_format_with_handoff() {
        let result = InputInjector::format_with_handoff(
            "Write the implementation",
            "Architecture: use a service layer pattern with DI",
            "architect",
        );

        assert!(result.contains("## Context from previous step (architect)"));
        assert!(result.contains("service layer pattern"));
        assert!(result.contains("## Your task"));
        assert!(result.contains("Write the implementation"));
    }

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("architect"), "Architect");
        assert_eq!(capitalize_first("implementer"), "Implementer");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("A"), "A");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test agent::input::tests`
Expected: FAIL — module not declared yet

- [ ] **Step 3: Write minimal implementation**

The implementation is in Step 1. Add `uuid` to `Cargo.toml` if not already present (should be from Plan 1). Update `src-tauri/src/agent/mod.rs`:
```rust
pub mod config;
pub mod input;
pub mod parser;

pub use config::{AgentConfig, AgentError, AgentEvent, InputMode, OutputMode, PtySize};
pub use input::InputInjector;
pub use parser::{create_parser, OutputParser};
```

- [ ] **Step 4: Run test to verify it passes**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test agent::input::tests`
Expected: PASS — all 9 tests pass

- [ ] **Step 5: Commit**
```bash
scripts/committer "feat(agent): add input injection — pty_stdin, flag_message, file_prompt" src-tauri/src/agent/input.rs src-tauri/src/agent/mod.rs
```

---

## Chunk 4: PTY Process Management

### Task 4: PTY-based process management (`agent/process.rs`)

**Files:**
- Create: `src-tauri/src/agent/process.rs`
- Modify: `src-tauri/src/agent/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/agent/process.rs`:
```rust
use crate::agent::config::{AgentConfig, AgentError, AgentEvent, OutputMode, PtySize};
use crate::agent::parser::{create_parser, OutputParser};
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Trait abstracting the PTY system for testability.
/// In production, this wraps `portable_pty`. In tests, it uses mocks.
pub trait PtySystem: Send + Sync {
    /// Spawn a child process in a PTY.
    /// Returns a handle to manage the child.
    fn spawn(
        &self,
        command: &str,
        args: &[String],
        env: &std::collections::HashMap<String, String>,
        working_dir: &std::path::Path,
        size: PtySize,
    ) -> Result<Box<dyn PtyChild>, AgentError>;
}

/// Handle to a running PTY child process.
pub trait PtyChild: Send {
    /// Get a writer to the child's stdin.
    fn take_writer(&mut self) -> Result<Box<dyn Write + Send>, AgentError>;

    /// Get a reader for the child's stdout/stderr (combined via PTY).
    fn take_reader(&mut self) -> Result<Box<dyn Read + Send>, AgentError>;

    /// Get the process ID.
    fn pid(&self) -> u32;

    /// Check if the process is still running.
    fn is_running(&self) -> bool;

    /// Kill the child process.
    fn kill(&mut self) -> Result<(), AgentError>;

    /// Wait for the child to exit. Returns the exit code.
    fn wait(&mut self) -> Result<Option<i32>, AgentError>;
}

/// Production PTY system using portable-pty.
pub struct PortablePtySystem;

impl PtySystem for PortablePtySystem {
    fn spawn(
        &self,
        command: &str,
        args: &[String],
        env: &std::collections::HashMap<String, String>,
        working_dir: &std::path::Path,
        size: PtySize,
    ) -> Result<Box<dyn PtyChild>, AgentError> {
        use portable_pty::{native_pty_system, CommandBuilder, PtySize as PPtySize};

        let pty_system = native_pty_system();
        let pty_size = PPtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system
            .openpty(pty_size)
            .map_err(|e| AgentError::SpawnFailed(format!("failed to open PTY: {}", e)))?;

        let mut cmd = CommandBuilder::new(command);
        cmd.args(args);
        cmd.cwd(working_dir);
        for (key, value) in env {
            cmd.env(key, value);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| AgentError::SpawnFailed(format!("failed to spawn command: {}", e)))?;

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| AgentError::SpawnFailed(format!("failed to clone PTY reader: {}", e)))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| AgentError::SpawnFailed(format!("failed to take PTY writer: {}", e)))?;

        Ok(Box::new(PortablePtyChild {
            child,
            reader: Some(reader),
            writer: Some(writer),
            _master: pair.master,
        }))
    }
}

/// Production PTY child wrapping portable-pty types.
struct PortablePtyChild {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    reader: Option<Box<dyn Read + Send>>,
    writer: Option<Box<dyn Write + Send>>,
    _master: Box<dyn portable_pty::MasterPty + Send>,
}

impl PtyChild for PortablePtyChild {
    fn take_writer(&mut self) -> Result<Box<dyn Write + Send>, AgentError> {
        self.writer
            .take()
            .ok_or_else(|| AgentError::InputFailed("writer already taken".to_string()))
    }

    fn take_reader(&mut self) -> Result<Box<dyn Read + Send>, AgentError> {
        self.reader
            .take()
            .ok_or_else(|| AgentError::InputFailed("reader already taken".to_string()))
    }

    fn pid(&self) -> u32 {
        self.child.process_id().unwrap_or(0)
    }

    fn is_running(&self) -> bool {
        // portable-pty Child does not have a non-blocking poll; defer to wait logic.
        true
    }

    fn kill(&mut self) -> Result<(), AgentError> {
        self.child
            .kill()
            .map_err(|e| AgentError::SpawnFailed(format!("failed to kill process: {}", e)))
    }

    fn wait(&mut self) -> Result<Option<i32>, AgentError> {
        let status = self
            .child
            .wait()
            .map_err(|e| AgentError::SpawnFailed(format!("failed to wait on process: {}", e)))?;
        Ok(status.exit_code().map(|c| c as i32))
    }
}

/// Manages a single agent process lifecycle.
///
/// Spawns the process in a PTY, reads output through a parser, and sends
/// `AgentEvent`s to a channel. Provides methods to inject input and kill
/// the process.
pub struct AgentProcess {
    config: AgentConfig,
    pid: Option<u32>,
    writer: Option<Box<dyn Write + Send>>,
    is_running: Arc<AtomicBool>,
    kill_sender: Option<tokio::sync::oneshot::Sender<()>>,
}

impl AgentProcess {
    /// Spawn a new agent process.
    ///
    /// Returns the process handle and a receiver for AgentEvents.
    pub fn spawn(
        config: AgentConfig,
        pty_system: &dyn PtySystem,
        size: PtySize,
    ) -> Result<(Self, mpsc::UnboundedReceiver<AgentEvent>), AgentError> {
        let mut child = pty_system.spawn(
            &config.command,
            &config.args,
            &config.env,
            &config.working_dir,
            size,
        )?;

        let pid = child.pid();
        let writer = child.take_writer()?;
        let reader = child.take_reader()?;

        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (kill_tx, kill_rx) = tokio::sync::oneshot::channel::<()>();

        let is_running = Arc::new(AtomicBool::new(true));
        let is_running_clone = is_running.clone();
        let output_mode = config.output_mode.clone();

        // Spawn background reader thread (blocking I/O on PTY)
        std::thread::Builder::new()
            .name(format!("agent-pty-reader-{}", pid))
            .spawn(move || {
                Self::reader_thread(
                    reader,
                    child,
                    output_mode,
                    event_tx,
                    kill_rx,
                    is_running_clone,
                );
            })
            .map_err(|e| AgentError::SpawnFailed(format!("failed to spawn reader thread: {}", e)))?;

        let process = AgentProcess {
            config,
            pid: Some(pid),
            writer: Some(writer),
            is_running,
            kill_sender: Some(kill_tx),
        };

        Ok((process, event_rx))
    }

    /// Background thread that reads PTY output line by line and parses it.
    fn reader_thread(
        reader: Box<dyn Read + Send>,
        mut child: Box<dyn PtyChild>,
        output_mode: OutputMode,
        event_tx: mpsc::UnboundedSender<AgentEvent>,
        mut kill_rx: tokio::sync::oneshot::Receiver<()>,
        is_running: Arc<AtomicBool>,
    ) {
        let mut parser = create_parser(&output_mode);
        let buf_reader = BufReader::new(reader);

        for line_result in buf_reader.lines() {
            // Check for kill signal (non-blocking)
            if kill_rx.try_recv().is_ok() {
                let _ = child.kill();
                break;
            }

            match line_result {
                Ok(line) => {
                    let events = parser.parse_line(&format!("{}\n", line));
                    for event in events {
                        if event_tx.send(event).is_err() {
                            // Receiver dropped, stop reading
                            return;
                        }
                    }
                }
                Err(_) => break, // PTY closed
            }
        }

        // Wait for process exit and emit final events
        let exit_code = child.wait().ok().flatten();
        let final_events = parser.on_exit(exit_code);
        for event in final_events {
            let _ = event_tx.send(event);
        }

        is_running.store(false, Ordering::SeqCst);
    }

    /// Get the process ID, if running.
    pub fn pid(&self) -> Option<u32> {
        if self.is_running.load(Ordering::SeqCst) {
            self.pid
        } else {
            None
        }
    }

    /// Check if the agent process is still running.
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Write a message to the agent's PTY stdin.
    pub fn write_stdin(&mut self, message: &str) -> Result<(), AgentError> {
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| AgentError::InputFailed("no writer available".to_string()))?;

        let data = format!("{}\n", message);
        writer
            .write_all(data.as_bytes())
            .map_err(|e| AgentError::InputFailed(e.to_string()))?;
        writer
            .flush()
            .map_err(|e| AgentError::InputFailed(e.to_string()))
    }

    /// Kill the agent process.
    pub fn kill(&mut self) -> Result<(), AgentError> {
        if let Some(kill_tx) = self.kill_sender.take() {
            let _ = kill_tx.send(());
        }
        self.is_running.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Get the agent config.
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::Cursor;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU32;

    // -----------------------------------------------------------------------
    // Mock PTY system for testing
    // -----------------------------------------------------------------------

    /// Mock PTY system that produces predetermined output.
    struct MockPtySystem {
        /// Lines the mock child will produce.
        output_lines: Vec<String>,
        /// Exit code the mock child will return.
        exit_code: Option<i32>,
    }

    impl MockPtySystem {
        fn new(output_lines: Vec<String>, exit_code: Option<i32>) -> Self {
            Self {
                output_lines,
                exit_code,
            }
        }
    }

    static MOCK_PID_COUNTER: AtomicU32 = AtomicU32::new(1000);

    impl PtySystem for MockPtySystem {
        fn spawn(
            &self,
            _command: &str,
            _args: &[String],
            _env: &HashMap<String, String>,
            _working_dir: &std::path::Path,
            _size: PtySize,
        ) -> Result<Box<dyn PtyChild>, AgentError> {
            let output = self.output_lines.join("\n") + "\n";
            let pid = MOCK_PID_COUNTER.fetch_add(1, Ordering::SeqCst);

            Ok(Box::new(MockPtyChild {
                reader: Some(Box::new(Cursor::new(output.into_bytes()))),
                writer: Some(Box::new(Vec::<u8>::new())),
                pid,
                exit_code: self.exit_code,
                killed: false,
            }))
        }
    }

    struct MockPtyChild {
        reader: Option<Box<dyn Read + Send>>,
        writer: Option<Box<dyn Write + Send>>,
        pid: u32,
        exit_code: Option<i32>,
        killed: bool,
    }

    impl PtyChild for MockPtyChild {
        fn take_writer(&mut self) -> Result<Box<dyn Write + Send>, AgentError> {
            self.writer
                .take()
                .ok_or_else(|| AgentError::InputFailed("writer already taken".to_string()))
        }

        fn take_reader(&mut self) -> Result<Box<dyn Read + Send>, AgentError> {
            self.reader
                .take()
                .ok_or_else(|| AgentError::InputFailed("reader already taken".to_string()))
        }

        fn pid(&self) -> u32 {
            self.pid
        }

        fn is_running(&self) -> bool {
            !self.killed
        }

        fn kill(&mut self) -> Result<(), AgentError> {
            self.killed = true;
            Ok(())
        }

        fn wait(&mut self) -> Result<Option<i32>, AgentError> {
            Ok(self.exit_code)
        }
    }

    /// Failing mock PTY system.
    struct FailingPtySystem;

    impl PtySystem for FailingPtySystem {
        fn spawn(
            &self,
            _command: &str,
            _args: &[String],
            _env: &HashMap<String, String>,
            _working_dir: &std::path::Path,
            _size: PtySize,
        ) -> Result<Box<dyn PtyChild>, AgentError> {
            Err(AgentError::SpawnFailed("command not found".to_string()))
        }
    }

    fn default_test_config() -> AgentConfig {
        AgentConfig {
            command: "echo".to_string(),
            args: vec![],
            env: HashMap::new(),
            input_mode: crate::agent::config::InputMode::PtyStdin,
            output_mode: OutputMode::JsonStream,
            resume_support: false,
            working_dir: PathBuf::from("/tmp"),
            message_flag: None,
            print_flag: None,
            resume_flag: None,
        }
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_spawn_and_receive_events() {
        let pty = MockPtySystem::new(
            vec![
                r#"{"type":"text","content":"Hello from agent"}"#.to_string(),
                r#"{"type":"usage","input_tokens":100,"output_tokens":200,"cost_usd":0.01}"#
                    .to_string(),
            ],
            Some(0),
        );

        let config = default_test_config();
        let (process, mut event_rx) = AgentProcess::spawn(config, &pty, PtySize::default()).unwrap();

        assert!(process.pid().is_some());

        let mut events = Vec::new();
        while let Some(event) = event_rx.recv().await {
            events.push(event);
        }

        // Should have: Text, Usage, Completed
        assert!(events.len() >= 2, "got {} events: {:?}", events.len(), events);
        assert!(
            events.iter().any(|e| matches!(e, AgentEvent::Text { content } if content == "Hello from agent")),
            "missing Text event in {:?}",
            events
        );
        assert!(
            events.iter().any(|e| matches!(e, AgentEvent::Completed)),
            "missing Completed event in {:?}",
            events
        );
    }

    #[tokio::test]
    async fn test_spawn_with_text_markers() {
        let pty = MockPtySystem::new(
            vec!["Cost: $0.42".to_string(), "All done!".to_string()],
            Some(0),
        );

        let mut config = default_test_config();
        config.output_mode = OutputMode::TextMarkers;

        let (_process, mut event_rx) =
            AgentProcess::spawn(config, &pty, PtySize::default()).unwrap();

        let mut events = Vec::new();
        while let Some(event) = event_rx.recv().await {
            events.push(event);
        }

        assert!(
            events.iter().any(|e| matches!(e, AgentEvent::Usage { cost_usd, .. } if (*cost_usd - 0.42).abs() < f64::EPSILON)),
            "missing Usage event in {:?}",
            events
        );
    }

    #[tokio::test]
    async fn test_spawn_with_raw_pty() {
        let pty = MockPtySystem::new(
            vec!["raw output line".to_string()],
            Some(0),
        );

        let mut config = default_test_config();
        config.output_mode = OutputMode::RawPty;

        let (_process, mut event_rx) =
            AgentProcess::spawn(config, &pty, PtySize::default()).unwrap();

        let mut events = Vec::new();
        while let Some(event) = event_rx.recv().await {
            events.push(event);
        }

        assert!(
            events.iter().any(|e| matches!(e, AgentEvent::Text { .. })),
            "missing Text event in {:?}",
            events
        );
        assert!(
            events.iter().any(|e| matches!(e, AgentEvent::Completed)),
            "missing Completed in {:?}",
            events
        );
    }

    #[tokio::test]
    async fn test_spawn_failure() {
        let pty = FailingPtySystem;
        let config = default_test_config();
        let result = AgentProcess::spawn(config, &pty, PtySize::default());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("command not found"));
    }

    #[tokio::test]
    async fn test_spawn_nonzero_exit() {
        let pty = MockPtySystem::new(
            vec![r#"{"type":"text","content":"partial work"}"#.to_string()],
            Some(1),
        );

        let config = default_test_config();
        let (_process, mut event_rx) =
            AgentProcess::spawn(config, &pty, PtySize::default()).unwrap();

        let mut events = Vec::new();
        while let Some(event) = event_rx.recv().await {
            events.push(event);
        }

        assert!(
            events.iter().any(|e| matches!(e, AgentEvent::Error { message } if message.contains("code 1"))),
            "missing Error event for nonzero exit in {:?}",
            events
        );
    }

    #[tokio::test]
    async fn test_kill_process() {
        let pty = MockPtySystem::new(vec![], Some(0));
        let config = default_test_config();
        let (mut process, _event_rx) =
            AgentProcess::spawn(config, &pty, PtySize::default()).unwrap();

        process.kill().unwrap();
        // After kill, is_running should be false
        assert!(!process.is_running());
    }

    #[tokio::test]
    async fn test_process_config_accessor() {
        let pty = MockPtySystem::new(vec![], Some(0));
        let config = default_test_config();
        let (process, _event_rx) =
            AgentProcess::spawn(config, &pty, PtySize::default()).unwrap();

        assert_eq!(process.config().command, "echo");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test agent::process::tests`
Expected: FAIL — module not declared yet

- [ ] **Step 3: Write minimal implementation**

The implementation is in Step 1. Update `src-tauri/src/agent/mod.rs`:
```rust
pub mod config;
pub mod input;
pub mod parser;
pub mod process;

pub use config::{AgentConfig, AgentError, AgentEvent, InputMode, OutputMode, PtySize};
pub use input::InputInjector;
pub use parser::{create_parser, OutputParser};
pub use process::{AgentProcess, PtyChild, PtySystem, PortablePtySystem};
```

- [ ] **Step 4: Run test to verify it passes**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test agent::process::tests`
Expected: PASS — all 7 tests pass

- [ ] **Step 5: Commit**
```bash
scripts/committer "feat(agent): add PTY process management with mock-based tests" src-tauri/src/agent/process.rs src-tauri/src/agent/mod.rs
```

---

## Chunk 5: Agent Registry and Presets

### Task 5: Agent template registry (`agent/registry.rs`)

**Files:**
- Create: `src-tauri/src/agent/registry.rs`
- Modify: `src-tauri/src/agent/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/agent/registry.rs`:
```rust
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
            .bind(("name", name))
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
        .bind(("name", &template.name))
        .bind(("command", &template.command))
        .bind(("args", &template.default_args))
        .bind(("env", &template.env))
        .bind(("input_mode", &template.input_mode))
        .bind(("output_mode", &template.output_mode))
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
            .bind(("name", name))
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
        // Run the migration to create tables (reuse from db module)
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
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test agent::registry::tests`
Expected: FAIL — module not declared

- [ ] **Step 3: Write minimal implementation**

The implementation is in Step 1. Update `src-tauri/src/agent/mod.rs`:
```rust
pub mod config;
pub mod input;
pub mod parser;
pub mod process;
pub mod registry;

pub use config::{AgentConfig, AgentError, AgentEvent, InputMode, OutputMode, PtySize};
pub use input::InputInjector;
pub use parser::{create_parser, OutputParser};
pub use process::{AgentProcess, PtyChild, PtySystem, PortablePtySystem};
pub use registry::{AgentRegistry, AgentTemplate, builtin_templates};
```

- [ ] **Step 4: Run test to verify it passes**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test agent::registry::tests`
Expected: PASS — all 10 tests pass

- [ ] **Step 5: Commit**
```bash
scripts/committer "feat(agent): add template registry with DB + builtin fallback" src-tauri/src/agent/registry.rs src-tauri/src/agent/mod.rs
```

---

### Task 6: Role preset loading (`agent/presets.rs`)

**Files:**
- Create: `src-tauri/src/agent/presets.rs`
- Modify: `src-tauri/src/agent/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/agent/presets.rs`:
```rust
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
            .bind(("name", name))
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
        .bind(("name", &preset.name))
        .bind(("prompt", &preset.system_prompt))
        .bind(("desc", &preset.description))
        .bind(("method", &preset.injection_method))
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
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test agent::presets::tests`
Expected: FAIL — module not declared

- [ ] **Step 3: Write minimal implementation**

The implementation is in Step 1. Update `src-tauri/src/agent/mod.rs` to the final form:
```rust
pub mod config;
pub mod input;
pub mod parser;
pub mod presets;
pub mod process;
pub mod registry;

pub use config::{AgentConfig, AgentError, AgentEvent, InputMode, OutputMode, PtySize};
pub use input::InputInjector;
pub use parser::{create_parser, OutputParser};
pub use presets::{PresetRegistry, RolePreset, builtin_presets, InjectionMethod};
pub use process::{AgentProcess, PtyChild, PtySystem, PortablePtySystem};
pub use registry::{AgentRegistry, AgentTemplate, builtin_templates};
```

- [ ] **Step 4: Run test to verify it passes**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test agent::presets::tests`
Expected: PASS — all 11 tests pass

- [ ] **Step 5: Commit**
```bash
scripts/committer "feat(agent): add role preset registry with injection methods" src-tauri/src/agent/presets.rs src-tauri/src/agent/mod.rs
```

---

## Chunk 6: Integration Verification

### Task 7: Full module integration test

**Files:**
- No new files — tests added to `src-tauri/src/agent/mod.rs`

- [ ] **Step 1: Write the failing test**

Add to the bottom of `src-tauri/src/agent/mod.rs`:
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{Cursor, Read, Write};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

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
            "    println!(\"Error: not really\");\n",  // should NOT be detected as error
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
        let text_count = all_events.iter().filter(|e| matches!(e, AgentEvent::Text { .. })).count();
        let tool_count = all_events.iter().filter(|e| matches!(e, AgentEvent::ToolUse { .. })).count();
        let usage_count = all_events.iter().filter(|e| matches!(e, AgentEvent::Usage { .. })).count();
        let error_count = all_events.iter().filter(|e| matches!(e, AgentEvent::Error { .. })).count();

        assert!(text_count >= 3, "expected at least 3 text events, got {}", text_count);
        assert_eq!(tool_count, 1, "expected 1 tool use (Reading)");
        assert_eq!(usage_count, 2, "expected 2 usage events (tokens + cost)");
        assert_eq!(error_count, 0, "error inside code fence should be suppressed");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test agent::integration_tests`
Expected: FAIL — the `MockPtyWriter` is `#[cfg(test)]` in `input.rs` but needs `pub(crate)` visibility

- [ ] **Step 3: Write minimal implementation**

In `src-tauri/src/agent/input.rs`, change the mock visibility:
```rust
#[cfg(test)]
pub(crate) struct MockPtyWriter {
```

And the `impl` block:
```rust
#[cfg(test)]
impl MockPtyWriter {
    pub(crate) fn new() -> Self {
```

Then also make `TextMarkersParser` in `parser.rs` visible:
```rust
pub struct TextMarkersParser {
```
(It should already be `pub` from Step 1.)

- [ ] **Step 4: Run test to verify it passes**
Run: `cd ~/projects/koompi-orch/src-tauri && cargo test agent::integration_tests`
Expected: PASS — all 3 integration tests pass

- [ ] **Step 5: Commit**
```bash
scripts/committer "test(agent): add integration tests for template-to-process pipeline" src-tauri/src/agent/mod.rs src-tauri/src/agent/input.rs
```

---

## Final Verification

- [ ] **Run all agent tests**
```bash
cd ~/projects/koompi-orch/src-tauri && cargo test agent
```
Expected: All tests pass (approximately 52 tests across 6 modules).

- [ ] **Type check the entire crate**
```bash
cd ~/projects/koompi-orch/src-tauri && cargo check
```
Expected: Compiles with no errors.

---

## Summary

| Task | Module | Tests |
|------|--------|-------|
| 1 | `agent/config.rs` — AgentEvent, AgentConfig, InputMode, OutputMode, AgentError | 8 |
| 2 | `agent/parser.rs` — json_stream, text_markers, raw_pty parsers | 24 |
| 3 | `agent/input.rs` — PTY stdin, flag_message, file_prompt injection | 9 |
| 4 | `agent/process.rs` — PTY process spawn, read, kill with mock system | 7 |
| 5 | `agent/registry.rs` — Agent template CRUD with DB + builtin fallback | 10 |
| 6 | `agent/presets.rs` — Role preset registry with injection methods | 11 |
| 7 | Integration tests — end-to-end pipeline verification | 3 |
| **Total** | | **~72** |
