use crate::agent::config::{AgentEvent, OutputMode};
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
    Regex::new(r#"^(?:│\s*)?(?:Reading|Writing|Editing|Running|Executing|Creating|Deleting)\s+[`'"]?(\S+)"#)
        .unwrap()
});

static CODE_FENCE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^```"#).unwrap()
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
