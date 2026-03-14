use crate::agent::config::{AgentConfig, AgentError};
use std::io::Write;
use std::path::PathBuf;

/// Trait abstracting PTY stdin writes. Allows mock injection in tests.
pub trait PtyWriter: Send {
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
pub(crate) struct MockPtyWriter {
    pub written: Vec<u8>,
}

#[cfg(test)]
impl MockPtyWriter {
    pub(crate) fn new() -> Self {
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
    use crate::agent::config::{InputMode, OutputMode};
    use std::collections::HashMap;

    fn test_config() -> AgentConfig {
        AgentConfig {
            command: "claude".to_string(),
            args: vec!["--dangerously-skip-permissions".to_string()],
            env: HashMap::new(),
            input_mode: InputMode::PtyStdin,
            output_mode: OutputMode::JsonStream,
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
