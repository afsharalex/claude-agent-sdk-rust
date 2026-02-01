//! Configuration types for Claude SDK.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use super::hook::{HookEvent, HookMatcher};
use super::mcp::McpServerConfig;
use super::permission::{PermissionMode, PermissionResult, ToolPermissionContext};
use super::sandbox::SandboxSettings;

/// Type alias for the tool permission callback function.
///
/// This callback is invoked when a tool requests permission to execute.
/// The function receives the tool name, input, and context, and should
/// return a permission result indicating whether to allow or deny.
pub type CanUseToolFn = Arc<
    dyn Fn(
            String,
            Value,
            ToolPermissionContext,
        ) -> Pin<Box<dyn Future<Output = PermissionResult> + Send>>
        + Send
        + Sync,
>;

/// Type alias for stderr callback function.
///
/// This callback is invoked when stderr output is received from the CLI process.
pub type StderrCallbackFn = Arc<dyn Fn(String) + Send + Sync>;

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

    /// Callback for tool permission decisions.
    ///
    /// When set, this callback is invoked for each tool execution request
    /// to determine whether the tool should be allowed or denied.
    pub can_use_tool: Option<CanUseToolFn>,

    /// Hook configurations for various SDK events.
    ///
    /// Maps hook events to their matchers and callbacks.
    pub hooks: HashMap<HookEvent, Vec<HookMatcher>>,

    /// Callback for stderr output from the CLI.
    ///
    /// When set, stderr output from the Claude CLI process will be
    /// passed to this callback instead of being inherited.
    pub stderr: Option<StderrCallbackFn>,

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

impl std::fmt::Debug for ClaudeAgentOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeAgentOptions")
            .field("tools", &self.tools)
            .field("allowed_tools", &self.allowed_tools)
            .field("disallowed_tools", &self.disallowed_tools)
            .field("system_prompt", &self.system_prompt)
            .field("mcp_servers", &self.mcp_servers)
            .field("permission_mode", &self.permission_mode)
            .field("can_use_tool", &self.can_use_tool.is_some())
            .field("hooks", &self.hooks)
            .field("stderr", &self.stderr.is_some())
            .field("continue_conversation", &self.continue_conversation)
            .field("resume", &self.resume)
            .field("max_turns", &self.max_turns)
            .field("max_budget_usd", &self.max_budget_usd)
            .field("model", &self.model)
            .field("fallback_model", &self.fallback_model)
            .field("betas", &self.betas)
            .field(
                "permission_prompt_tool_name",
                &self.permission_prompt_tool_name,
            )
            .field("cwd", &self.cwd)
            .field("cli_path", &self.cli_path)
            .field("settings", &self.settings)
            .field("add_dirs", &self.add_dirs)
            .field("env", &self.env)
            .field("extra_args", &self.extra_args)
            .field("max_buffer_size", &self.max_buffer_size)
            .field("user", &self.user)
            .field("include_partial_messages", &self.include_partial_messages)
            .field("fork_session", &self.fork_session)
            .field("agents", &self.agents)
            .field("setting_sources", &self.setting_sources)
            .field("sandbox", &self.sandbox)
            .field("plugins", &self.plugins)
            .field("max_thinking_tokens", &self.max_thinking_tokens)
            .field("output_format", &self.output_format)
            .field("enable_file_checkpointing", &self.enable_file_checkpointing)
            .finish()
    }
}

impl Clone for ClaudeAgentOptions {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
            allowed_tools: self.allowed_tools.clone(),
            disallowed_tools: self.disallowed_tools.clone(),
            system_prompt: self.system_prompt.clone(),
            mcp_servers: self.mcp_servers.clone(),
            permission_mode: self.permission_mode,
            can_use_tool: self.can_use_tool.clone(),
            hooks: self.hooks.clone(),
            stderr: self.stderr.clone(),
            continue_conversation: self.continue_conversation,
            resume: self.resume.clone(),
            max_turns: self.max_turns,
            max_budget_usd: self.max_budget_usd,
            model: self.model.clone(),
            fallback_model: self.fallback_model.clone(),
            betas: self.betas.clone(),
            permission_prompt_tool_name: self.permission_prompt_tool_name.clone(),
            cwd: self.cwd.clone(),
            cli_path: self.cli_path.clone(),
            settings: self.settings.clone(),
            add_dirs: self.add_dirs.clone(),
            env: self.env.clone(),
            extra_args: self.extra_args.clone(),
            max_buffer_size: self.max_buffer_size,
            user: self.user.clone(),
            include_partial_messages: self.include_partial_messages,
            fork_session: self.fork_session,
            agents: self.agents.clone(),
            setting_sources: self.setting_sources.clone(),
            sandbox: self.sandbox.clone(),
            plugins: self.plugins.clone(),
            max_thinking_tokens: self.max_thinking_tokens,
            output_format: self.output_format.clone(),
            enable_file_checkpointing: self.enable_file_checkpointing,
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for ClaudeAgentOptions {
    fn default() -> Self {
        Self {
            tools: None,
            allowed_tools: Vec::new(),
            disallowed_tools: Vec::new(),
            system_prompt: None,
            mcp_servers: None,
            permission_mode: None,
            can_use_tool: None,
            hooks: HashMap::new(),
            stderr: None,
            continue_conversation: false,
            resume: None,
            max_turns: None,
            max_budget_usd: None,
            model: None,
            fallback_model: None,
            betas: Vec::new(),
            permission_prompt_tool_name: None,
            cwd: None,
            cli_path: None,
            settings: None,
            add_dirs: Vec::new(),
            env: HashMap::new(),
            extra_args: HashMap::new(),
            max_buffer_size: None,
            user: None,
            include_partial_messages: false,
            fork_session: false,
            agents: None,
            setting_sources: None,
            sandbox: None,
            plugins: Vec::new(),
            max_thinking_tokens: None,
            output_format: None,
            enable_file_checkpointing: false,
        }
    }
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
pub struct ClaudeAgentOptionsBuilder {
    options: ClaudeAgentOptions,
}

impl std::fmt::Debug for ClaudeAgentOptionsBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeAgentOptionsBuilder")
            .field("options", &self.options)
            .finish()
    }
}

impl Clone for ClaudeAgentOptionsBuilder {
    fn clone(&self) -> Self {
        Self {
            options: self.options.clone(),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for ClaudeAgentOptionsBuilder {
    fn default() -> Self {
        Self {
            options: ClaudeAgentOptions::default(),
        }
    }
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

    /// Set the can_use_tool callback for tool permission decisions.
    ///
    /// This callback is invoked when a tool requests permission to execute.
    /// The function receives the tool name, input, and context, and should
    /// return a permission result indicating whether to allow or deny.
    pub fn can_use_tool(mut self, callback: CanUseToolFn) -> Self {
        self.options.can_use_tool = Some(callback);
        self
    }

    /// Set hook configurations for SDK events.
    ///
    /// Maps hook events to their matchers and callbacks.
    pub fn hooks(mut self, hooks: HashMap<HookEvent, Vec<HookMatcher>>) -> Self {
        self.options.hooks = hooks;
        self
    }

    /// Add a hook matcher for a specific event.
    ///
    /// This is a convenience method for adding individual hook matchers
    /// without replacing the entire hooks map.
    pub fn add_hook(mut self, event: HookEvent, matcher: HookMatcher) -> Self {
        self.options.hooks.entry(event).or_default().push(matcher);
        self
    }

    /// Set the stderr callback.
    ///
    /// When set, stderr output from the Claude CLI process will be
    /// passed to this callback instead of being inherited.
    pub fn stderr(mut self, callback: StderrCallbackFn) -> Self {
        self.options.stderr = Some(callback);
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

    #[test]
    fn test_builder_cwd() {
        let options = ClaudeAgentOptions::builder()
            .cwd("/home/user/project")
            .build();
        assert_eq!(
            options.cwd,
            Some(std::path::PathBuf::from("/home/user/project"))
        );
    }

    #[test]
    fn test_builder_cli_path() {
        let options = ClaudeAgentOptions::builder()
            .cli_path("/usr/local/bin/claude")
            .build();
        assert_eq!(
            options.cli_path,
            Some(std::path::PathBuf::from("/usr/local/bin/claude"))
        );
    }

    #[test]
    fn test_builder_allowed_tools() {
        let options = ClaudeAgentOptions::builder()
            .allowed_tools(vec!["Bash".to_string(), "Read".to_string()])
            .build();
        assert_eq!(options.allowed_tools, vec!["Bash", "Read"]);
    }

    #[test]
    fn test_builder_disallowed_tools() {
        let options = ClaudeAgentOptions::builder()
            .disallowed_tools(vec!["Write".to_string()])
            .build();
        assert_eq!(options.disallowed_tools, vec!["Write"]);
    }

    #[test]
    fn test_builder_mcp_servers_map() {
        use crate::types::McpServerConfig;
        let mut servers = HashMap::new();
        servers.insert(
            "test-server".to_string(),
            McpServerConfig::stdio_with_args("npx", vec!["test-server".to_string()]),
        );
        let options = ClaudeAgentOptions::builder()
            .mcp_servers(servers.clone())
            .build();
        match options.mcp_servers {
            Some(McpServers::Map(m)) => assert_eq!(m.len(), 1),
            _ => panic!("Expected map"),
        }
    }

    #[test]
    fn test_builder_mcp_servers_path() {
        let path = std::path::PathBuf::from("/path/to/mcp.json");
        let options = ClaudeAgentOptions::builder()
            .mcp_servers(path.clone())
            .build();
        match options.mcp_servers {
            Some(McpServers::Path(p)) => assert_eq!(p, path),
            _ => panic!("Expected path"),
        }
    }

    #[test]
    fn test_builder_resume_session() {
        let options = ClaudeAgentOptions::builder().resume("session-123").build();
        assert_eq!(options.resume, Some("session-123".to_string()));
    }

    #[test]
    fn test_builder_continue_conversation() {
        let options = ClaudeAgentOptions::builder()
            .continue_conversation(true)
            .build();
        assert!(options.continue_conversation);
    }

    #[test]
    fn test_builder_agents() {
        let mut agents = HashMap::new();
        agents.insert(
            "reviewer".to_string(),
            AgentDefinition::new("Code reviewer", "Review code carefully"),
        );
        let options = ClaudeAgentOptions::builder().agents(agents).build();
        assert!(options.agents.is_some());
        assert_eq!(options.agents.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_builder_setting_sources() {
        let options = ClaudeAgentOptions::builder()
            .setting_sources(vec![SettingSource::User, SettingSource::Project])
            .build();
        assert_eq!(
            options.setting_sources,
            Some(vec![SettingSource::User, SettingSource::Project])
        );
    }

    #[test]
    fn test_builder_sandbox() {
        use crate::types::SandboxSettings;
        let sandbox = SandboxSettings::default();
        let options = ClaudeAgentOptions::builder().sandbox(sandbox).build();
        assert!(options.sandbox.is_some());
    }

    #[test]
    fn test_builder_plugins() {
        let plugins = vec![SdkPluginConfig::local("/path/to/plugin")];
        let options = ClaudeAgentOptions::builder().plugins(plugins).build();
        assert_eq!(options.plugins.len(), 1);
    }

    #[test]
    fn test_builder_max_thinking_tokens() {
        let options = ClaudeAgentOptions::builder()
            .max_thinking_tokens(1000)
            .build();
        assert_eq!(options.max_thinking_tokens, Some(1000));
    }

    #[test]
    fn test_builder_output_format() {
        let format = serde_json::json!({"type": "json"});
        let options = ClaudeAgentOptions::builder()
            .output_format(format.clone())
            .build();
        assert_eq!(options.output_format, Some(format));
    }

    #[test]
    fn test_builder_enable_file_checkpointing() {
        let options = ClaudeAgentOptions::builder()
            .enable_file_checkpointing(true)
            .build();
        assert!(options.enable_file_checkpointing);
    }

    #[test]
    fn test_builder_max_budget_usd() {
        let options = ClaudeAgentOptions::builder().max_budget_usd(10.0).build();
        assert_eq!(options.max_budget_usd, Some(10.0));
    }

    #[test]
    fn test_builder_fallback_model() {
        let options = ClaudeAgentOptions::builder()
            .fallback_model("claude-3-haiku")
            .build();
        assert_eq!(options.fallback_model, Some("claude-3-haiku".to_string()));
    }

    #[test]
    fn test_builder_betas() {
        let options = ClaudeAgentOptions::builder()
            .betas(vec![SdkBeta::Context1m20250807])
            .build();
        assert_eq!(options.betas, vec![SdkBeta::Context1m20250807]);
    }

    #[test]
    fn test_builder_permission_prompt_tool_name() {
        let options = ClaudeAgentOptions::builder()
            .permission_prompt_tool_name("my-tool")
            .build();
        assert_eq!(
            options.permission_prompt_tool_name,
            Some("my-tool".to_string())
        );
    }

    #[test]
    fn test_builder_settings() {
        let options = ClaudeAgentOptions::builder()
            .settings(r#"{"key": "value"}"#)
            .build();
        assert_eq!(options.settings, Some(r#"{"key": "value"}"#.to_string()));
    }

    #[test]
    fn test_builder_add_dirs() {
        let dirs = vec![
            std::path::PathBuf::from("/dir1"),
            std::path::PathBuf::from("/dir2"),
        ];
        let options = ClaudeAgentOptions::builder().add_dirs(dirs.clone()).build();
        assert_eq!(options.add_dirs, dirs);
    }

    #[test]
    fn test_builder_env() {
        let mut env = HashMap::new();
        env.insert("KEY".to_string(), "VALUE".to_string());
        let options = ClaudeAgentOptions::builder().env(env.clone()).build();
        assert_eq!(options.env, env);
    }

    #[test]
    fn test_builder_extra_args() {
        let mut args = HashMap::new();
        args.insert("--flag".to_string(), Some("value".to_string()));
        args.insert("--bool-flag".to_string(), None);
        let options = ClaudeAgentOptions::builder()
            .extra_args(args.clone())
            .build();
        assert_eq!(options.extra_args, args);
    }

    #[test]
    fn test_builder_max_buffer_size() {
        let options = ClaudeAgentOptions::builder()
            .max_buffer_size(1024 * 1024)
            .build();
        assert_eq!(options.max_buffer_size, Some(1024 * 1024));
    }

    #[test]
    fn test_builder_user() {
        let options = ClaudeAgentOptions::builder().user("test-user").build();
        assert_eq!(options.user, Some("test-user".to_string()));
    }

    #[test]
    fn test_builder_include_partial_messages() {
        let options = ClaudeAgentOptions::builder()
            .include_partial_messages(true)
            .build();
        assert!(options.include_partial_messages);
    }

    #[test]
    fn test_builder_fork_session() {
        let options = ClaudeAgentOptions::builder().fork_session(true).build();
        assert!(options.fork_session);
    }

    #[test]
    fn test_system_prompt_from_string() {
        let prompt: SystemPrompt = "You are helpful".into();
        match prompt {
            SystemPrompt::Text(text) => assert_eq!(text, "You are helpful"),
            _ => panic!("Expected text"),
        }
    }

    #[test]
    fn test_system_prompt_preset_claude_code() {
        let preset = SystemPromptPreset::claude_code();
        assert_eq!(preset.preset, "claude_code");
        assert!(preset.append.is_none());
    }

    #[test]
    fn test_tools_from_list() {
        let tools: Tools = vec!["Bash".to_string()].into();
        match tools {
            Tools::List(list) => assert_eq!(list, vec!["Bash"]),
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_tools_preset() {
        let preset = ToolsPreset::claude_code();
        let tools: Tools = preset.into();
        match tools {
            Tools::Preset(p) => assert_eq!(p.preset, "claude_code"),
            _ => panic!("Expected preset"),
        }
    }

    #[test]
    fn test_setting_source_serde() {
        let source = SettingSource::User;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"user\"");

        let source = SettingSource::Project;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"project\"");

        let source = SettingSource::Local;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"local\"");
    }

    #[test]
    fn test_sdk_beta_serde() {
        let beta = SdkBeta::Context1m20250807;
        let json = serde_json::to_string(&beta).unwrap();
        assert_eq!(json, "\"context-1m-2025-08-07\"");
    }

    #[test]
    fn test_options_clone() {
        let options = ClaudeAgentOptions::builder()
            .model("claude-3-5-sonnet")
            .max_turns(10)
            .build();
        let cloned = options.clone();
        assert_eq!(cloned.model, options.model);
        assert_eq!(cloned.max_turns, options.max_turns);
    }

    #[test]
    fn test_options_debug() {
        let options = ClaudeAgentOptions::builder()
            .model("claude-3-5-sonnet")
            .build();
        let debug_str = format!("{:?}", options);
        assert!(debug_str.contains("claude-3-5-sonnet"));
    }

    #[test]
    fn test_builder_clone() {
        let builder = ClaudeAgentOptions::builder().model("claude-3-5-sonnet");
        let cloned = builder.clone();
        let options = cloned.max_turns(5).build();
        assert_eq!(options.model, Some("claude-3-5-sonnet".to_string()));
        assert_eq!(options.max_turns, Some(5));
    }

    #[test]
    fn test_builder_new() {
        let builder = ClaudeAgentOptionsBuilder::new();
        let options = builder.build();
        assert!(options.model.is_none());
    }

    #[test]
    fn test_options_default() {
        let options = ClaudeAgentOptions::default();
        assert!(options.model.is_none());
        assert!(options.tools.is_none());
        assert!(!options.continue_conversation);
    }

    #[test]
    fn test_agent_definition_without_optional_fields() {
        let agent = AgentDefinition::new("Test agent", "Test prompt");
        assert_eq!(agent.description, "Test agent");
        assert_eq!(agent.prompt, "Test prompt");
        assert!(agent.tools.is_none());
        assert!(agent.model.is_none());
    }
}
