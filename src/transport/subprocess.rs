//! Subprocess transport implementation using Claude Code CLI.

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex;

use crate::error::{ClaudeSDKError, Result};
use crate::types::{
    AgentDefinition, ClaudeAgentOptions, McpServerConfig, McpServers, SdkBeta, SettingSource,
    SystemPrompt, Tools,
};

use super::Transport;

/// SDK version for environment variable.
const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Minimum required Claude Code version.
const MINIMUM_CLAUDE_CODE_VERSION: &str = "2.0.0";

/// Default maximum buffer size (1MB).
const DEFAULT_MAX_BUFFER_SIZE: usize = 1024 * 1024;

/// Subprocess transport using Claude Code CLI.
pub struct SubprocessCLITransport {
    prompt: Option<String>,
    options: ClaudeAgentOptions,
    cli_path: PathBuf,
    cwd: Option<PathBuf>,
    process: Option<Child>,
    stdin: Option<Arc<Mutex<ChildStdin>>>,
    stdout: Option<BufReader<ChildStdout>>,
    ready: bool,
    max_buffer_size: usize,
    is_streaming: bool,
}

impl SubprocessCLITransport {
    /// Create a new subprocess transport with a string prompt.
    pub fn new(prompt: impl Into<String>, options: ClaudeAgentOptions) -> Result<Self> {
        let cli_path = if let Some(ref path) = options.cli_path {
            path.clone()
        } else {
            Self::find_cli()?
        };

        let cwd = options.cwd.clone();
        let max_buffer_size = options.max_buffer_size.unwrap_or(DEFAULT_MAX_BUFFER_SIZE);

        Ok(Self {
            prompt: Some(prompt.into()),
            options,
            cli_path,
            cwd,
            process: None,
            stdin: None,
            stdout: None,
            ready: false,
            max_buffer_size,
            is_streaming: false,
        })
    }

    /// Create a new subprocess transport for streaming mode (no initial prompt).
    pub fn streaming(options: ClaudeAgentOptions) -> Result<Self> {
        let cli_path = if let Some(ref path) = options.cli_path {
            path.clone()
        } else {
            Self::find_cli()?
        };

        let cwd = options.cwd.clone();
        let max_buffer_size = options.max_buffer_size.unwrap_or(DEFAULT_MAX_BUFFER_SIZE);

        Ok(Self {
            prompt: None,
            options,
            cli_path,
            cwd,
            process: None,
            stdin: None,
            stdout: None,
            ready: false,
            max_buffer_size,
            is_streaming: true,
        })
    }

    /// Find Claude Code CLI binary.
    fn find_cli() -> Result<PathBuf> {
        // Check for bundled CLI first (not implemented yet)

        // Fall back to system-wide search
        if let Ok(path) = which::which("claude") {
            return Ok(path);
        }

        // Check common installation locations
        let home = dirs::home_dir();
        let locations = [
            home.as_ref().map(|h| h.join(".npm-global/bin/claude")),
            Some(PathBuf::from("/usr/local/bin/claude")),
            home.as_ref().map(|h| h.join(".local/bin/claude")),
            home.as_ref().map(|h| h.join("node_modules/.bin/claude")),
            home.as_ref().map(|h| h.join(".yarn/bin/claude")),
            home.as_ref().map(|h| h.join(".claude/local/claude")),
        ];

        for location in locations.into_iter().flatten() {
            if location.exists() && location.is_file() {
                return Ok(location);
            }
        }

        Err(ClaudeSDKError::cli_not_found(None))
    }

    /// Build the CLI command with all arguments.
    fn build_command(&self) -> Vec<String> {
        let mut cmd = vec![
            self.cli_path.to_string_lossy().to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
        ];

        // System prompt
        match &self.options.system_prompt {
            None => {
                cmd.extend(["--system-prompt".to_string(), String::new()]);
            }
            Some(SystemPrompt::Text(text)) => {
                cmd.extend(["--system-prompt".to_string(), text.clone()]);
            }
            Some(SystemPrompt::Preset(preset)) => {
                if preset.preset_type == "preset" {
                    if let Some(ref append) = preset.append {
                        cmd.extend(["--append-system-prompt".to_string(), append.clone()]);
                    }
                }
            }
        }

        // Tools
        if let Some(ref tools) = self.options.tools {
            match tools {
                Tools::List(list) => {
                    if list.is_empty() {
                        cmd.extend(["--tools".to_string(), String::new()]);
                    } else {
                        cmd.extend(["--tools".to_string(), list.join(",")]);
                    }
                }
                Tools::Preset(_) => {
                    cmd.extend(["--tools".to_string(), "default".to_string()]);
                }
            }
        }

        // Allowed tools
        if !self.options.allowed_tools.is_empty() {
            cmd.extend([
                "--allowedTools".to_string(),
                self.options.allowed_tools.join(","),
            ]);
        }

        // Max turns
        if let Some(max_turns) = self.options.max_turns {
            cmd.extend(["--max-turns".to_string(), max_turns.to_string()]);
        }

        // Max budget
        if let Some(budget) = self.options.max_budget_usd {
            cmd.extend(["--max-budget-usd".to_string(), budget.to_string()]);
        }

        // Disallowed tools
        if !self.options.disallowed_tools.is_empty() {
            cmd.extend([
                "--disallowedTools".to_string(),
                self.options.disallowed_tools.join(","),
            ]);
        }

        // Model
        if let Some(ref model) = self.options.model {
            cmd.extend(["--model".to_string(), model.clone()]);
        }

        // Fallback model
        if let Some(ref model) = self.options.fallback_model {
            cmd.extend(["--fallback-model".to_string(), model.clone()]);
        }

        // Betas
        if !self.options.betas.is_empty() {
            let betas: Vec<String> = self
                .options
                .betas
                .iter()
                .map(|b| match b {
                    SdkBeta::Context1m20250807 => "context-1m-2025-08-07".to_string(),
                })
                .collect();
            cmd.extend(["--betas".to_string(), betas.join(",")]);
        }

        // Permission prompt tool name
        if let Some(ref name) = self.options.permission_prompt_tool_name {
            cmd.extend(["--permission-prompt-tool".to_string(), name.clone()]);
        }

        // Permission mode
        if let Some(mode) = self.options.permission_mode {
            cmd.extend(["--permission-mode".to_string(), mode.to_string()]);
        }

        // Continue conversation
        if self.options.continue_conversation {
            cmd.push("--continue".to_string());
        }

        // Resume session
        if let Some(ref session) = self.options.resume {
            cmd.extend(["--resume".to_string(), session.clone()]);
        }

        // Settings
        if let Some(ref settings) = self.build_settings_value() {
            cmd.extend(["--settings".to_string(), settings.clone()]);
        }

        // Add directories
        for dir in &self.options.add_dirs {
            cmd.extend(["--add-dir".to_string(), dir.to_string_lossy().to_string()]);
        }

        // MCP servers
        if let Some(ref mcp_servers) = self.options.mcp_servers {
            match mcp_servers {
                McpServers::Map(map) => {
                    if !map.is_empty() {
                        // Filter out SDK server instances
                        let servers_for_cli: HashMap<String, &McpServerConfig> = map
                            .iter()
                            .filter(|(_, config)| !config.is_sdk())
                            .map(|(k, v)| (k.clone(), v))
                            .collect();

                        if !servers_for_cli.is_empty() {
                            let config = serde_json::json!({ "mcpServers": servers_for_cli });
                            cmd.extend(["--mcp-config".to_string(), config.to_string()]);
                        }
                    }
                }
                McpServers::Path(path) => {
                    cmd.extend([
                        "--mcp-config".to_string(),
                        path.to_string_lossy().to_string(),
                    ]);
                }
                McpServers::Json(json) => {
                    cmd.extend(["--mcp-config".to_string(), json.clone()]);
                }
            }
        }

        // Include partial messages
        if self.options.include_partial_messages {
            cmd.push("--include-partial-messages".to_string());
        }

        // Fork session
        if self.options.fork_session {
            cmd.push("--fork-session".to_string());
        }

        // Agents
        if let Some(ref agents) = self.options.agents {
            let agents_json = self.serialize_agents(agents);
            cmd.extend(["--agents".to_string(), agents_json]);
        }

        // Setting sources
        let sources = if let Some(ref sources) = self.options.setting_sources {
            sources
                .iter()
                .map(|s| match s {
                    SettingSource::User => "user",
                    SettingSource::Project => "project",
                    SettingSource::Local => "local",
                })
                .collect::<Vec<_>>()
                .join(",")
        } else {
            String::new()
        };
        cmd.extend(["--setting-sources".to_string(), sources]);

        // Plugins
        for plugin in &self.options.plugins {
            if plugin.plugin_type == "local" {
                cmd.extend(["--plugin-dir".to_string(), plugin.path.clone()]);
            }
        }

        // Extra args
        for (flag, value) in &self.options.extra_args {
            if let Some(val) = value {
                cmd.extend([format!("--{}", flag), val.clone()]);
            } else {
                cmd.push(format!("--{}", flag));
            }
        }

        // Max thinking tokens
        if let Some(tokens) = self.options.max_thinking_tokens {
            cmd.extend(["--max-thinking-tokens".to_string(), tokens.to_string()]);
        }

        // Output format (JSON schema)
        if let Some(ref format) = self.options.output_format {
            if let Some(schema) = format.get("schema") {
                if format.get("type") == Some(&serde_json::json!("json_schema")) {
                    cmd.extend(["--json-schema".to_string(), schema.to_string()]);
                }
            }
        }

        // Prompt handling
        if self.is_streaming {
            cmd.extend(["--input-format".to_string(), "stream-json".to_string()]);
        } else if let Some(ref prompt) = self.prompt {
            cmd.extend(["--print".to_string(), "--".to_string(), prompt.clone()]);
        }

        cmd
    }

    /// Build settings value, merging sandbox settings if provided.
    fn build_settings_value(&self) -> Option<String> {
        let has_settings = self.options.settings.is_some();
        let has_sandbox = self.options.sandbox.is_some();

        if !has_settings && !has_sandbox {
            return None;
        }

        // If only settings path and no sandbox, pass through as-is
        if has_settings && !has_sandbox {
            return self.options.settings.clone();
        }

        // If we have sandbox settings, we need to merge into a JSON object
        let mut settings_obj: serde_json::Map<String, Value> = serde_json::Map::new();

        if let Some(ref settings_str) = self.options.settings {
            let trimmed = settings_str.trim();
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                // Parse JSON string
                if let Ok(parsed) = serde_json::from_str::<serde_json::Map<String, Value>>(trimmed)
                {
                    settings_obj = parsed;
                }
            } else {
                // It's a file path - read and parse
                if let Ok(content) = std::fs::read_to_string(trimmed) {
                    if let Ok(parsed) =
                        serde_json::from_str::<serde_json::Map<String, Value>>(&content)
                    {
                        settings_obj = parsed;
                    }
                }
            }
        }

        // Merge sandbox settings
        if let Some(ref sandbox) = self.options.sandbox {
            settings_obj.insert(
                "sandbox".to_string(),
                serde_json::to_value(sandbox).unwrap_or_default(),
            );
        }

        Some(serde_json::to_string(&settings_obj).unwrap_or_default())
    }

    /// Serialize agents to JSON.
    fn serialize_agents(&self, agents: &HashMap<String, AgentDefinition>) -> String {
        let agents_map: HashMap<String, Value> = agents
            .iter()
            .map(|(name, def)| {
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "description".to_string(),
                    Value::String(def.description.clone()),
                );
                obj.insert("prompt".to_string(), Value::String(def.prompt.clone()));
                if let Some(ref tools) = def.tools {
                    obj.insert("tools".to_string(), serde_json::json!(tools));
                }
                if let Some(ref model) = def.model {
                    obj.insert("model".to_string(), Value::String(model.clone()));
                }
                (name.clone(), Value::Object(obj))
            })
            .collect();
        serde_json::to_string(&agents_map).unwrap_or_default()
    }

    /// Check Claude Code version.
    async fn check_version(&self) -> Result<()> {
        if env::var("CLAUDE_AGENT_SDK_SKIP_VERSION_CHECK").is_ok() {
            return Ok(());
        }

        let output = tokio::process::Command::new(&self.cli_path)
            .arg("-v")
            .output()
            .await;

        if let Ok(output) = output {
            if let Ok(version_str) = String::from_utf8(output.stdout) {
                let version_str = version_str.trim();
                if let Some(version) = version_str.split_whitespace().next() {
                    if Self::version_compare(version, MINIMUM_CLAUDE_CODE_VERSION) < 0 {
                        tracing::warn!(
                            "Claude Code version {} is unsupported. Minimum required: {}",
                            version,
                            MINIMUM_CLAUDE_CODE_VERSION
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Compare version strings.
    fn version_compare(v1: &str, v2: &str) -> i32 {
        let parse_version =
            |v: &str| -> Vec<i32> { v.split('.').filter_map(|s| s.parse::<i32>().ok()).collect() };

        let v1_parts = parse_version(v1);
        let v2_parts = parse_version(v2);

        for i in 0..std::cmp::max(v1_parts.len(), v2_parts.len()) {
            let p1 = v1_parts.get(i).copied().unwrap_or(0);
            let p2 = v2_parts.get(i).copied().unwrap_or(0);
            if p1 < p2 {
                return -1;
            }
            if p1 > p2 {
                return 1;
            }
        }
        0
    }
}

#[async_trait]
impl Transport for SubprocessCLITransport {
    async fn connect(&mut self) -> Result<()> {
        if self.process.is_some() {
            return Ok(());
        }

        self.check_version().await?;

        let cmd = self.build_command();
        let program = &cmd[0];
        let args = &cmd[1..];

        // Build environment
        let mut env_vars: HashMap<String, String> = env::vars().collect();
        env_vars.extend(self.options.env.clone());
        env_vars.insert("CLAUDE_CODE_ENTRYPOINT".to_string(), "sdk-rust".to_string());
        env_vars.insert(
            "CLAUDE_AGENT_SDK_VERSION".to_string(),
            SDK_VERSION.to_string(),
        );

        if self.options.enable_file_checkpointing {
            env_vars.insert(
                "CLAUDE_CODE_ENABLE_SDK_FILE_CHECKPOINTING".to_string(),
                "true".to_string(),
            );
        }

        if let Some(ref cwd) = self.cwd {
            env_vars.insert("PWD".to_string(), cwd.to_string_lossy().to_string());
        }

        let mut command = tokio::process::Command::new(program);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .envs(&env_vars);

        if let Some(ref cwd) = self.cwd {
            command.current_dir(cwd);
        }

        let mut child = command.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ClaudeSDKError::cli_not_found(Some(program.clone()))
            } else {
                ClaudeSDKError::CLIConnection(format!("Failed to start Claude Code: {}", e))
            }
        })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ClaudeSDKError::CLIConnection("Failed to capture stdout".to_string()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ClaudeSDKError::CLIConnection("Failed to capture stdin".to_string()))?;

        self.process = Some(child);
        self.stdin = Some(Arc::new(Mutex::new(stdin)));
        self.stdout = Some(BufReader::new(stdout));
        self.ready = true;

        // If not streaming mode, close stdin immediately
        if !self.is_streaming {
            self.end_input().await?;
        }

        Ok(())
    }

    async fn write(&mut self, data: &str) -> Result<()> {
        if !self.ready {
            return Err(ClaudeSDKError::CLIConnection(
                "Transport is not ready for writing".to_string(),
            ));
        }

        let stdin = self
            .stdin
            .as_ref()
            .ok_or_else(|| ClaudeSDKError::CLIConnection("No stdin available".to_string()))?;

        let mut guard = stdin.lock().await;
        guard.write_all(data.as_bytes()).await.map_err(|e| {
            ClaudeSDKError::CLIConnection(format!("Failed to write to process stdin: {}", e))
        })?;
        guard
            .flush()
            .await
            .map_err(|e| ClaudeSDKError::CLIConnection(format!("Failed to flush stdin: {}", e)))?;

        Ok(())
    }

    fn read_messages(&mut self) -> Pin<Box<dyn Stream<Item = Result<Value>> + Send + '_>> {
        let stdout = self.stdout.take();
        let max_buffer_size = self.max_buffer_size;

        Box::pin(async_stream::try_stream! {
            let mut stdout = stdout.ok_or_else(|| {
                ClaudeSDKError::CLIConnection("Not connected".to_string())
            })?;

            let mut json_buffer = String::new();
            let mut line = String::new();

            loop {
                line.clear();
                let bytes_read = stdout.read_line(&mut line).await.map_err(|e| {
                    ClaudeSDKError::CLIConnection(format!("Failed to read from stdout: {}", e))
                })?;

                if bytes_read == 0 {
                    break; // EOF
                }

                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Accumulate partial JSON
                json_buffer.push_str(trimmed);

                if json_buffer.len() > max_buffer_size {
                    let len = json_buffer.len();
                    json_buffer.clear();
                    Err(ClaudeSDKError::CLIConnection(format!(
                        "JSON message exceeded maximum buffer size of {} bytes (got {})",
                        max_buffer_size, len
                    )))?;
                }

                // Try to parse
                match serde_json::from_str::<Value>(&json_buffer) {
                    Ok(data) => {
                        json_buffer.clear();
                        yield data;
                    }
                    Err(_) => {
                        // Keep accumulating
                        continue;
                    }
                }
            }
        })
    }

    async fn close(&mut self) -> Result<()> {
        self.ready = false;

        // Close stdin
        if let Some(stdin) = self.stdin.take() {
            drop(stdin);
        }

        // Terminate process
        if let Some(mut process) = self.process.take() {
            let _ = process.kill().await;
            let _ = process.wait().await;
        }

        self.stdout = None;

        Ok(())
    }

    fn is_ready(&self) -> bool {
        self.ready
    }

    async fn end_input(&mut self) -> Result<()> {
        if let Some(stdin) = self.stdin.take() {
            drop(stdin);
        }
        Ok(())
    }
}

impl Drop for SubprocessCLITransport {
    fn drop(&mut self) {
        // Process cleanup is handled asynchronously, but we try to ensure
        // the process is killed when dropped
        if let Some(ref mut process) = self.process {
            let _ = process.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compare() {
        assert_eq!(SubprocessCLITransport::version_compare("2.0.0", "2.0.0"), 0);
        assert_eq!(SubprocessCLITransport::version_compare("2.1.0", "2.0.0"), 1);
        assert_eq!(
            SubprocessCLITransport::version_compare("1.9.0", "2.0.0"),
            -1
        );
        assert_eq!(SubprocessCLITransport::version_compare("2.0.1", "2.0.0"), 1);
    }

    #[test]
    fn test_build_command_basic() {
        let options = ClaudeAgentOptions::builder()
            .system_prompt("Be helpful")
            .model("claude-3-5-sonnet")
            .build();

        let transport = SubprocessCLITransport {
            prompt: Some("Hello".to_string()),
            options,
            cli_path: PathBuf::from("/usr/bin/claude"),
            cwd: None,
            process: None,
            stdin: None,
            stdout: None,
            ready: false,
            max_buffer_size: DEFAULT_MAX_BUFFER_SIZE,
            is_streaming: false,
        };

        let cmd = transport.build_command();

        assert!(cmd.contains(&"--output-format".to_string()));
        assert!(cmd.contains(&"stream-json".to_string()));
        assert!(cmd.contains(&"--system-prompt".to_string()));
        assert!(cmd.contains(&"Be helpful".to_string()));
        assert!(cmd.contains(&"--model".to_string()));
        assert!(cmd.contains(&"claude-3-5-sonnet".to_string()));
        assert!(cmd.contains(&"--print".to_string()));
        assert!(cmd.contains(&"Hello".to_string()));
    }

    #[test]
    fn test_build_command_streaming() {
        let options = ClaudeAgentOptions::new();

        let transport = SubprocessCLITransport {
            prompt: None,
            options,
            cli_path: PathBuf::from("/usr/bin/claude"),
            cwd: None,
            process: None,
            stdin: None,
            stdout: None,
            ready: false,
            max_buffer_size: DEFAULT_MAX_BUFFER_SIZE,
            is_streaming: true,
        };

        let cmd = transport.build_command();

        assert!(cmd.contains(&"--input-format".to_string()));
        assert!(cmd.contains(&"stream-json".to_string()));
        assert!(!cmd.contains(&"--print".to_string()));
    }

    #[test]
    fn test_build_command_with_tools() {
        let options = ClaudeAgentOptions::builder()
            .tools(vec!["Bash".to_string(), "Read".to_string()])
            .allowed_tools(vec!["Write".to_string()])
            .disallowed_tools(vec!["WebFetch".to_string()])
            .build();

        let transport = SubprocessCLITransport {
            prompt: Some("test".to_string()),
            options,
            cli_path: PathBuf::from("/usr/bin/claude"),
            cwd: None,
            process: None,
            stdin: None,
            stdout: None,
            ready: false,
            max_buffer_size: DEFAULT_MAX_BUFFER_SIZE,
            is_streaming: false,
        };

        let cmd = transport.build_command();

        assert!(cmd.contains(&"--tools".to_string()));
        assert!(cmd.contains(&"Bash,Read".to_string()));
        assert!(cmd.contains(&"--allowedTools".to_string()));
        assert!(cmd.contains(&"--disallowedTools".to_string()));
    }

    #[test]
    fn test_build_command_with_limits() {
        let options = ClaudeAgentOptions::builder()
            .max_turns(10)
            .max_budget_usd(5.0)
            .max_thinking_tokens(1000)
            .build();

        let transport = SubprocessCLITransport {
            prompt: Some("test".to_string()),
            options,
            cli_path: PathBuf::from("/usr/bin/claude"),
            cwd: None,
            process: None,
            stdin: None,
            stdout: None,
            ready: false,
            max_buffer_size: DEFAULT_MAX_BUFFER_SIZE,
            is_streaming: false,
        };

        let cmd = transport.build_command();

        assert!(cmd.contains(&"--max-turns".to_string()));
        assert!(cmd.contains(&"10".to_string()));
        assert!(cmd.contains(&"--max-budget-usd".to_string()));
        assert!(cmd.contains(&"5".to_string()));
        assert!(cmd.contains(&"--max-thinking-tokens".to_string()));
        assert!(cmd.contains(&"1000".to_string()));
    }

    #[test]
    fn test_version_compare_edge_cases() {
        // Same versions
        assert_eq!(SubprocessCLITransport::version_compare("1.0.0", "1.0.0"), 0);
        assert_eq!(SubprocessCLITransport::version_compare("0.0.1", "0.0.1"), 0);

        // Different number of components
        assert_eq!(SubprocessCLITransport::version_compare("1.0", "1.0.0"), 0);
        assert_eq!(SubprocessCLITransport::version_compare("1", "1.0.0"), 0);
        assert_eq!(SubprocessCLITransport::version_compare("2", "1.9.9"), 1);

        // Early vs later versions
        assert_eq!(
            SubprocessCLITransport::version_compare("0.1.0", "1.0.0"),
            -1
        );
        assert_eq!(
            SubprocessCLITransport::version_compare("10.0.0", "2.0.0"),
            1
        );
    }

    #[test]
    fn test_build_command_with_permission_mode() {
        let options = ClaudeAgentOptions::builder()
            .permission_mode(crate::types::PermissionMode::AcceptEdits)
            .build();

        let transport = SubprocessCLITransport {
            prompt: Some("test".to_string()),
            options,
            cli_path: PathBuf::from("/usr/bin/claude"),
            cwd: None,
            process: None,
            stdin: None,
            stdout: None,
            ready: false,
            max_buffer_size: DEFAULT_MAX_BUFFER_SIZE,
            is_streaming: false,
        };

        let cmd = transport.build_command();

        assert!(cmd.contains(&"--permission-mode".to_string()));
        assert!(cmd.contains(&"acceptEdits".to_string()));
    }

    #[test]
    fn test_build_command_with_resume_session() {
        let options = ClaudeAgentOptions::builder().resume("session-123").build();

        let transport = SubprocessCLITransport {
            prompt: Some("test".to_string()),
            options,
            cli_path: PathBuf::from("/usr/bin/claude"),
            cwd: None,
            process: None,
            stdin: None,
            stdout: None,
            ready: false,
            max_buffer_size: DEFAULT_MAX_BUFFER_SIZE,
            is_streaming: false,
        };

        let cmd = transport.build_command();

        assert!(cmd.contains(&"--resume".to_string()));
        assert!(cmd.contains(&"session-123".to_string()));
    }

    #[test]
    fn test_build_command_with_continue_conversation() {
        let options = ClaudeAgentOptions::builder()
            .continue_conversation(true)
            .build();

        let transport = SubprocessCLITransport {
            prompt: Some("test".to_string()),
            options,
            cli_path: PathBuf::from("/usr/bin/claude"),
            cwd: None,
            process: None,
            stdin: None,
            stdout: None,
            ready: false,
            max_buffer_size: DEFAULT_MAX_BUFFER_SIZE,
            is_streaming: false,
        };

        let cmd = transport.build_command();

        assert!(cmd.contains(&"--continue".to_string()));
    }

    #[test]
    fn test_transport_is_ready_initially_false() {
        let options = ClaudeAgentOptions::new();
        let transport = SubprocessCLITransport {
            prompt: Some("test".to_string()),
            options,
            cli_path: PathBuf::from("/usr/bin/claude"),
            cwd: None,
            process: None,
            stdin: None,
            stdout: None,
            ready: false,
            max_buffer_size: DEFAULT_MAX_BUFFER_SIZE,
            is_streaming: false,
        };

        assert!(!transport.is_ready());
    }

    #[test]
    fn test_build_command_with_cwd() {
        let options = ClaudeAgentOptions::builder().cwd("/some/path").build();

        let transport = SubprocessCLITransport {
            prompt: Some("test".to_string()),
            options,
            cli_path: PathBuf::from("/usr/bin/claude"),
            cwd: Some(PathBuf::from("/some/path")),
            process: None,
            stdin: None,
            stdout: None,
            ready: false,
            max_buffer_size: DEFAULT_MAX_BUFFER_SIZE,
            is_streaming: false,
        };

        assert!(transport.cwd.is_some());
        assert_eq!(transport.cwd.as_ref().unwrap().to_str(), Some("/some/path"));
    }

    #[test]
    fn test_build_command_with_fallback_model() {
        let options = ClaudeAgentOptions::builder()
            .fallback_model("claude-3-5-haiku")
            .build();

        let transport = SubprocessCLITransport {
            prompt: Some("test".to_string()),
            options,
            cli_path: PathBuf::from("/usr/bin/claude"),
            cwd: None,
            process: None,
            stdin: None,
            stdout: None,
            ready: false,
            max_buffer_size: DEFAULT_MAX_BUFFER_SIZE,
            is_streaming: false,
        };

        let cmd = transport.build_command();

        assert!(cmd.contains(&"--fallback-model".to_string()));
        assert!(cmd.contains(&"claude-3-5-haiku".to_string()));
    }

    #[test]
    fn test_build_command_with_max_buffer_size() {
        let options = ClaudeAgentOptions::builder()
            .max_buffer_size(1024 * 1024)
            .build();

        let transport = SubprocessCLITransport {
            prompt: Some("test".to_string()),
            options: options.clone(),
            cli_path: PathBuf::from("/usr/bin/claude"),
            cwd: None,
            process: None,
            stdin: None,
            stdout: None,
            ready: false,
            max_buffer_size: options.max_buffer_size.unwrap_or(DEFAULT_MAX_BUFFER_SIZE),
            is_streaming: false,
        };

        assert_eq!(transport.max_buffer_size, 1024 * 1024);
    }
}
