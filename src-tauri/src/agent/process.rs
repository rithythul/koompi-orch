use crate::agent::config::{AgentConfig, AgentError, AgentEvent, OutputMode, PtySize};
use crate::agent::parser::create_parser;
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
        Ok(Some(status.exit_code() as i32))
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

    /// Get the process ID (returns the PID even after the process exits).
    pub fn pid(&self) -> Option<u32> {
        self.pid
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
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("expected spawn to fail"),
        };
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
