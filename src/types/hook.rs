//! Hook types for Claude SDK.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Supported hook event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    UserPromptSubmit,
    Stop,
    SubagentStop,
    PreCompact,
}

impl HookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::PostToolUseFailure => "PostToolUseFailure",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::Stop => "Stop",
            Self::SubagentStop => "SubagentStop",
            Self::PreCompact => "PreCompact",
        }
    }
}

impl std::fmt::Display for HookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Base hook input fields present across many hook events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaseHookInput {
    pub session_id: String,
    pub transcript_path: String,
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
}

/// Input data for PreToolUse hook events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreToolUseHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    pub hook_event_name: String,
    pub tool_name: String,
    pub tool_input: Value,
}

/// Input data for PostToolUse hook events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostToolUseHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    pub hook_event_name: String,
    pub tool_name: String,
    pub tool_input: Value,
    pub tool_response: Value,
}

/// Input data for PostToolUseFailure hook events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostToolUseFailureHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    pub hook_event_name: String,
    pub tool_name: String,
    pub tool_input: Value,
    pub tool_use_id: String,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_interrupt: Option<bool>,
}

/// Input data for UserPromptSubmit hook events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserPromptSubmitHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    pub hook_event_name: String,
    pub prompt: String,
}

/// Input data for Stop hook events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StopHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    pub hook_event_name: String,
    pub stop_hook_active: bool,
}

/// Input data for SubagentStop hook events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubagentStopHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    pub hook_event_name: String,
    pub stop_hook_active: bool,
}

/// Trigger type for PreCompact events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PreCompactTrigger {
    Manual,
    Auto,
}

/// Input data for PreCompact hook events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreCompactHookInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    pub hook_event_name: String,
    pub trigger: PreCompactTrigger,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_instructions: Option<String>,
}

/// Union type for all hook inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookInput {
    PreToolUse(PreToolUseHookInput),
    PostToolUse(PostToolUseHookInput),
    PostToolUseFailure(PostToolUseFailureHookInput),
    UserPromptSubmit(UserPromptSubmitHookInput),
    Stop(StopHookInput),
    SubagentStop(SubagentStopHookInput),
    PreCompact(PreCompactHookInput),
}

impl HookInput {
    /// Get the hook event name.
    pub fn hook_event_name(&self) -> &str {
        match self {
            Self::PreToolUse(input) => &input.hook_event_name,
            Self::PostToolUse(input) => &input.hook_event_name,
            Self::PostToolUseFailure(input) => &input.hook_event_name,
            Self::UserPromptSubmit(input) => &input.hook_event_name,
            Self::Stop(input) => &input.hook_event_name,
            Self::SubagentStop(input) => &input.hook_event_name,
            Self::PreCompact(input) => &input.hook_event_name,
        }
    }

    /// Get the base input.
    pub fn base(&self) -> &BaseHookInput {
        match self {
            Self::PreToolUse(input) => &input.base,
            Self::PostToolUse(input) => &input.base,
            Self::PostToolUseFailure(input) => &input.base,
            Self::UserPromptSubmit(input) => &input.base,
            Self::Stop(input) => &input.base,
            Self::SubagentStop(input) => &input.base,
            Self::PreCompact(input) => &input.base,
        }
    }
}

/// Permission decision for PreToolUse hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookPermissionDecision {
    Allow,
    Deny,
    Ask,
}

/// Hook-specific output for PreToolUse events.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreToolUseHookSpecificOutput {
    pub hook_event_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_decision: Option<HookPermissionDecision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_decision_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<Value>,
}

impl PreToolUseHookSpecificOutput {
    pub fn new() -> Self {
        Self {
            hook_event_name: "PreToolUse".to_string(),
            ..Default::default()
        }
    }
}

/// Hook-specific output for PostToolUse events.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostToolUseHookSpecificOutput {
    pub hook_event_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

impl PostToolUseHookSpecificOutput {
    pub fn new() -> Self {
        Self {
            hook_event_name: "PostToolUse".to_string(),
            additional_context: None,
        }
    }
}

/// Hook-specific output for PostToolUseFailure events.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostToolUseFailureHookSpecificOutput {
    pub hook_event_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

impl PostToolUseFailureHookSpecificOutput {
    pub fn new() -> Self {
        Self {
            hook_event_name: "PostToolUseFailure".to_string(),
            additional_context: None,
        }
    }
}

/// Hook-specific output for UserPromptSubmit events.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPromptSubmitHookSpecificOutput {
    pub hook_event_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

impl UserPromptSubmitHookSpecificOutput {
    pub fn new() -> Self {
        Self {
            hook_event_name: "UserPromptSubmit".to_string(),
            additional_context: None,
        }
    }
}

/// Union type for hook-specific outputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookSpecificOutput {
    PreToolUse(PreToolUseHookSpecificOutput),
    PostToolUse(PostToolUseHookSpecificOutput),
    PostToolUseFailure(PostToolUseFailureHookSpecificOutput),
    UserPromptSubmit(UserPromptSubmitHookSpecificOutput),
}

/// Hook JSON output for synchronous hooks.
///
/// Note: The Python SDK uses `async_` and `continue_` to avoid keyword conflicts.
/// In Rust, we use `is_async` and `should_continue` instead.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookJSONOutput {
    /// Whether to continue after hook execution.
    #[serde(rename = "continue", skip_serializing_if = "Option::is_none")]
    pub should_continue: Option<bool>,

    /// Whether this is an async hook.
    #[serde(rename = "async", skip_serializing_if = "Option::is_none")]
    pub is_async: Option<bool>,

    /// Timeout for async operations in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub async_timeout: Option<i64>,

    /// Hide stdout from transcript mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,

    /// Message shown when continue is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,

    /// Decision - set to "block" to indicate blocking behavior.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,

    /// Warning message displayed to the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,

    /// Feedback message for Claude about the decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Hook-specific outputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_specific_output: Option<HookSpecificOutput>,
}

impl HookJSONOutput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_continue(mut self, should_continue: bool) -> Self {
        self.should_continue = Some(should_continue);
        self
    }

    pub fn with_async(mut self, is_async: bool, timeout: Option<i64>) -> Self {
        self.is_async = Some(is_async);
        self.async_timeout = timeout;
        self
    }

    pub fn with_stop_reason(mut self, reason: impl Into<String>) -> Self {
        self.should_continue = Some(false);
        self.stop_reason = Some(reason.into());
        self
    }

    pub fn with_decision(mut self, decision: impl Into<String>) -> Self {
        self.decision = Some(decision.into());
        self
    }

    pub fn with_hook_specific_output(mut self, output: HookSpecificOutput) -> Self {
        self.hook_specific_output = Some(output);
        self
    }
}

/// Context information for hook callbacks.
#[derive(Debug, Clone, Default)]
pub struct HookContext {
    /// Reserved for future abort signal support.
    pub signal: Option<()>,
}

impl HookContext {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Type alias for hook callback function.
pub type HookCallbackFn = Arc<
    dyn Fn(
            HookInput,
            Option<String>,
            HookContext,
        ) -> Pin<Box<dyn Future<Output = HookJSONOutput> + Send>>
        + Send
        + Sync,
>;

/// Hook matcher configuration.
#[derive(Clone)]
pub struct HookMatcher {
    /// Pattern to match tool names (e.g., "Bash" or "Write|MultiEdit|Edit").
    pub matcher: Option<String>,
    /// List of hook callback functions.
    pub hooks: Vec<HookCallbackFn>,
    /// Timeout in seconds for all hooks in this matcher (default: 60).
    pub timeout: Option<f64>,
}

impl std::fmt::Debug for HookMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookMatcher")
            .field("matcher", &self.matcher)
            .field("hooks_count", &self.hooks.len())
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl Default for HookMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl HookMatcher {
    pub fn new() -> Self {
        Self {
            matcher: None,
            hooks: Vec::new(),
            timeout: None,
        }
    }

    pub fn with_matcher(mut self, matcher: impl Into<String>) -> Self {
        self.matcher = Some(matcher.into());
        self
    }

    pub fn with_hook(mut self, hook: HookCallbackFn) -> Self {
        self.hooks.push(hook);
        self
    }

    pub fn with_timeout(mut self, timeout: f64) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_display() {
        assert_eq!(HookEvent::PreToolUse.as_str(), "PreToolUse");
        assert_eq!(HookEvent::PostToolUse.as_str(), "PostToolUse");
        assert_eq!(HookEvent::Stop.as_str(), "Stop");
    }

    #[test]
    fn test_hook_json_output_serde() {
        let output = HookJSONOutput::new()
            .with_continue(true)
            .with_decision("allow");

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"continue\":true"));
        assert!(json.contains("\"decision\":\"allow\""));
    }

    #[test]
    fn test_hook_json_output_async() {
        let output = HookJSONOutput::new().with_async(true, Some(5000));
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"async\":true"));
        assert!(json.contains("\"asyncTimeout\":5000"));
    }

    #[test]
    fn test_hook_matcher_builder() {
        let matcher = HookMatcher::new().with_matcher("Bash").with_timeout(30.0);

        assert_eq!(matcher.matcher, Some("Bash".to_string()));
        assert_eq!(matcher.timeout, Some(30.0));
        assert!(matcher.hooks.is_empty());
    }

    #[test]
    fn test_pre_tool_use_hook_specific_output() {
        let output = PreToolUseHookSpecificOutput::new();
        assert_eq!(output.hook_event_name, "PreToolUse");
    }
}
