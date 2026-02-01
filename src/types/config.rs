//! Configuration types for Claude SDK.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

use super::mcp::McpServerConfig;
use super::permission::PermissionMode;
use super::sandbox::SandboxSettings;

/// SDK Beta features.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SdkBeta {
    #[serde(rename = "context-1m-2025-08-07")]
    Context1m20250807,
}

/// Setting source types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingSource {
    User,
    Project,
    Local,
}

/// System prompt preset configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemPromptPreset {
    #[serde(rename = "type")]
    pub preset_type: String,
    pub preset: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append: Option<String>,
}

impl SystemPromptPreset {
    pub fn claude_code() -> Self {
        Self {
            preset_type: "preset".to_string(),
            preset: "claude_code".to_string(),
            append: None,
        }
    }

    pub fn claude_code_with_append(append: impl Into<String>) -> Self {
        Self {
            preset_type: "preset".to_string(),
            preset: "claude_code".to_string(),
            append: Some(append.into()),
        }
    }
}

/// Tools preset configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolsPreset {
    #[serde(rename = "type")]
    pub preset_type: String,
    pub preset: String,
}

impl ToolsPreset {
    pub fn claude_code() -> Self {
        Self {
            preset_type: "preset".to_string(),
            preset: "claude_code".to_string(),
        }
    }
}

/// System prompt configuration - either a string or a preset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SystemPrompt {
    Text(String),
    Preset(SystemPromptPreset),
}

impl From<String> for SystemPrompt {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<&str> for SystemPrompt {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<SystemPromptPreset> for SystemPrompt {
    fn from(preset: SystemPromptPreset) -> Self {
        Self::Preset(preset)
    }
}

/// Tools configuration - either a list of tool names or a preset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Tools {
    List(Vec<String>),
    Preset(ToolsPreset),
}

impl From<Vec<String>> for Tools {
    fn from(tools: Vec<String>) -> Self {
        Self::List(tools)
    }
}

impl From<ToolsPreset> for Tools {
    fn from(preset: ToolsPreset) -> Self {
        Self::Preset(preset)
    }
}

/// MCP servers configuration - either a map or a path to a config file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpServers {
    Map(HashMap<String, McpServerConfig>),
    Path(PathBuf),
    Json(String),
}

impl From<HashMap<String, McpServerConfig>> for McpServers {
    fn from(map: HashMap<String, McpServerConfig>) -> Self {
        Self::Map(map)
    }
}

impl From<PathBuf> for McpServers {
    fn from(path: PathBuf) -> Self {
        Self::Path(path)
    }
}

/// Agent definition configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub description: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl AgentDefinition {
    pub fn new(description: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            prompt: prompt.into(),
            tools: None,
            model: None,
        }
    }

    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// SDK plugin configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SdkPluginConfig {
    #[serde(rename = "type")]
    pub plugin_type: String,
    pub path: String,
}

impl SdkPluginConfig {
    pub fn local(path: impl Into<String>) -> Self {
        Self {
            plugin_type: "local".to_string(),
            path: path.into(),
        }
    }
}

/// Query options for Claude SDK.
#[derive(Debug, Clone, Default)]
pub struct ClaudeAgentOptions {
    /// Base set of tools (list or preset).
    pub tools: Option<Tools>,

    /// Additional tools to allow.
    pub allowed_tools: Vec<String>,

    /// Tools to disallow.
    pub disallowed_tools: Vec<String>,

    /// System prompt (string or preset).
    pub system_prompt: Option<SystemPrompt>,

    /// MCP server configurations.
    pub mcp_servers: Option<McpServers>,

    /// Permission mode controlling tool execution.
    pub permission_mode: Option<PermissionMode>,

    /// Continue a previous conversation.
    pub continue_conversation: bool,

    /// Resume a specific session.
    pub resume: Option<String>,

    /// Maximum number of turns.
    pub max_turns: Option<u32>,

    /// Maximum budget in USD.
    pub max_budget_usd: Option<f64>,

    /// Model to use.
    pub model: Option<String>,

    /// Fallback model.
    pub fallback_model: Option<String>,

    /// Beta features.
    pub betas: Vec<SdkBeta>,

    /// Tool name for permission prompts.
    pub permission_prompt_tool_name: Option<String>,

    /// Working directory.
    pub cwd: Option<PathBuf>,

    /// Path to Claude CLI.
    pub cli_path: Option<PathBuf>,

    /// Settings file path or JSON.
    pub settings: Option<String>,

    /// Additional directories to include.
    pub add_dirs: Vec<PathBuf>,

    /// Environment variables.
    pub env: HashMap<String, String>,

    /// Extra CLI arguments.
    pub extra_args: HashMap<String, Option<String>>,

    /// Maximum buffer size for CLI output.
    pub max_buffer_size: Option<usize>,

    /// User identifier.
    pub user: Option<String>,

    /// Include partial messages in stream.
    pub include_partial_messages: bool,

    /// Fork session when resuming.
    pub fork_session: bool,

    /// Agent definitions.
    pub agents: Option<HashMap<String, AgentDefinition>>,

    /// Setting sources to load.
    pub setting_sources: Option<Vec<SettingSource>>,

    /// Sandbox configuration.
    pub sandbox: Option<SandboxSettings>,

    /// Plugin configurations.
    pub plugins: Vec<SdkPluginConfig>,

    /// Max tokens for thinking blocks.
    pub max_thinking_tokens: Option<u32>,

    /// Output format for structured outputs.
    pub output_format: Option<Value>,

    /// Enable file checkpointing.
    pub enable_file_checkpointing: bool,
}

impl ClaudeAgentOptions {
    /// Create a new builder for ClaudeAgentOptions.
    pub fn builder() -> ClaudeAgentOptionsBuilder {
        ClaudeAgentOptionsBuilder::default()
    }

    /// Create default options.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Builder for ClaudeAgentOptions.
#[derive(Debug, Clone, Default)]
pub struct ClaudeAgentOptionsBuilder {
    options: ClaudeAgentOptions,
}

impl ClaudeAgentOptionsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tools(mut self, tools: impl Into<Tools>) -> Self {
        self.options.tools = Some(tools.into());
        self
    }

    pub fn allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.options.allowed_tools = tools;
        self
    }

    pub fn disallowed_tools(mut self, tools: Vec<String>) -> Self {
        self.options.disallowed_tools = tools;
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<SystemPrompt>) -> Self {
        self.options.system_prompt = Some(prompt.into());
        self
    }

    pub fn mcp_servers(mut self, servers: impl Into<McpServers>) -> Self {
        self.options.mcp_servers = Some(servers.into());
        self
    }

    pub fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.options.permission_mode = Some(mode);
        self
    }

    pub fn continue_conversation(mut self, continue_conv: bool) -> Self {
        self.options.continue_conversation = continue_conv;
        self
    }

    pub fn resume(mut self, session: impl Into<String>) -> Self {
        self.options.resume = Some(session.into());
        self
    }

    pub fn max_turns(mut self, turns: u32) -> Self {
        self.options.max_turns = Some(turns);
        self
    }

    pub fn max_budget_usd(mut self, budget: f64) -> Self {
        self.options.max_budget_usd = Some(budget);
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.options.model = Some(model.into());
        self
    }

    pub fn fallback_model(mut self, model: impl Into<String>) -> Self {
        self.options.fallback_model = Some(model.into());
        self
    }

    pub fn betas(mut self, betas: Vec<SdkBeta>) -> Self {
        self.options.betas = betas;
        self
    }

    pub fn permission_prompt_tool_name(mut self, name: impl Into<String>) -> Self {
        self.options.permission_prompt_tool_name = Some(name.into());
        self
    }

    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.options.cwd = Some(cwd.into());
        self
    }

    pub fn cli_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.options.cli_path = Some(path.into());
        self
    }

    pub fn settings(mut self, settings: impl Into<String>) -> Self {
        self.options.settings = Some(settings.into());
        self
    }

    pub fn add_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.options.add_dirs = dirs;
        self
    }

    pub fn env(mut self, env: HashMap<String, String>) -> Self {
        self.options.env = env;
        self
    }

    pub fn extra_args(mut self, args: HashMap<String, Option<String>>) -> Self {
        self.options.extra_args = args;
        self
    }

    pub fn max_buffer_size(mut self, size: usize) -> Self {
        self.options.max_buffer_size = Some(size);
        self
    }

    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.options.user = Some(user.into());
        self
    }

    pub fn include_partial_messages(mut self, include: bool) -> Self {
        self.options.include_partial_messages = include;
        self
    }

    pub fn fork_session(mut self, fork: bool) -> Self {
        self.options.fork_session = fork;
        self
    }

    pub fn agents(mut self, agents: HashMap<String, AgentDefinition>) -> Self {
        self.options.agents = Some(agents);
        self
    }

    pub fn setting_sources(mut self, sources: Vec<SettingSource>) -> Self {
        self.options.setting_sources = Some(sources);
        self
    }

    pub fn sandbox(mut self, sandbox: SandboxSettings) -> Self {
        self.options.sandbox = Some(sandbox);
        self
    }

    pub fn plugins(mut self, plugins: Vec<SdkPluginConfig>) -> Self {
        self.options.plugins = plugins;
        self
    }

    pub fn max_thinking_tokens(mut self, tokens: u32) -> Self {
        self.options.max_thinking_tokens = Some(tokens);
        self
    }

    pub fn output_format(mut self, format: Value) -> Self {
        self.options.output_format = Some(format);
        self
    }

    pub fn enable_file_checkpointing(mut self, enable: bool) -> Self {
        self.options.enable_file_checkpointing = enable;
        self
    }

    pub fn build(self) -> ClaudeAgentOptions {
        self.options
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let options = ClaudeAgentOptions::builder()
            .system_prompt("You are helpful")
            .model("claude-3-5-sonnet")
            .max_turns(10)
            .build();

        assert_eq!(
            options.system_prompt,
            Some(SystemPrompt::Text("You are helpful".to_string()))
        );
        assert_eq!(options.model, Some("claude-3-5-sonnet".to_string()));
        assert_eq!(options.max_turns, Some(10));
    }

    #[test]
    fn test_builder_permission_mode() {
        let options = ClaudeAgentOptions::builder()
            .permission_mode(PermissionMode::AcceptEdits)
            .build();

        assert_eq!(options.permission_mode, Some(PermissionMode::AcceptEdits));
    }

    #[test]
    fn test_builder_tools() {
        let options = ClaudeAgentOptions::builder()
            .tools(vec!["Bash".to_string(), "Read".to_string()])
            .build();

        match options.tools {
            Some(Tools::List(tools)) => assert_eq!(tools.len(), 2),
            _ => panic!("Expected tool list"),
        }
    }

    #[test]
    fn test_system_prompt_preset() {
        let preset = SystemPromptPreset::claude_code_with_append("Be concise.");
        let prompt: SystemPrompt = preset.into();

        match prompt {
            SystemPrompt::Preset(p) => {
                assert_eq!(p.preset, "claude_code");
                assert_eq!(p.append, Some("Be concise.".to_string()));
            }
            _ => panic!("Expected preset"),
        }
    }

    #[test]
    fn test_agent_definition() {
        let agent = AgentDefinition::new("Code reviewer", "Review code carefully")
            .with_tools(vec!["Read".to_string()])
            .with_model("claude-3-5-sonnet");

        assert_eq!(agent.description, "Code reviewer");
        assert_eq!(agent.tools, Some(vec!["Read".to_string()]));
        assert_eq!(agent.model, Some("claude-3-5-sonnet".to_string()));
    }

    #[test]
    fn test_sdk_plugin_config() {
        let plugin = SdkPluginConfig::local("/path/to/plugin");
        assert_eq!(plugin.plugin_type, "local");
        assert_eq!(plugin.path, "/path/to/plugin");
    }
}
