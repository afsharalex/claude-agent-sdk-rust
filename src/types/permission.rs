//! Permission types for Claude SDK.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Permission modes controlling tool execution behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// CLI prompts for dangerous tools.
    #[default]
    Default,
    /// Auto-accept file edits.
    AcceptEdits,
    /// Plan mode - dry run without execution.
    Plan,
    /// Allow all tools (use with caution).
    BypassPermissions,
}

impl PermissionMode {
    /// Convert to CLI flag value.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::AcceptEdits => "acceptEdits",
            Self::Plan => "plan",
            Self::BypassPermissions => "bypassPermissions",
        }
    }
}

impl std::fmt::Display for PermissionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Destination for permission updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionUpdateDestination {
    UserSettings,
    ProjectSettings,
    LocalSettings,
    Session,
}

/// Permission behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionBehavior {
    Allow,
    Deny,
    Ask,
}

/// Permission rule value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionRuleValue {
    pub tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_content: Option<String>,
}

impl PermissionRuleValue {
    pub fn new(tool_name: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            rule_content: None,
        }
    }

    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.rule_content = Some(content.into());
        self
    }
}

/// Permission update type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionUpdateType {
    AddRules,
    ReplaceRules,
    RemoveRules,
    SetMode,
    AddDirectories,
    RemoveDirectories,
}

/// Permission update configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionUpdate {
    #[serde(rename = "type")]
    pub update_type: PermissionUpdateType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<PermissionRuleValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<PermissionBehavior>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<PermissionMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directories: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<PermissionUpdateDestination>,
}

impl PermissionUpdate {
    /// Create a new add rules update.
    pub fn add_rules(rules: Vec<PermissionRuleValue>, behavior: PermissionBehavior) -> Self {
        Self {
            update_type: PermissionUpdateType::AddRules,
            rules: Some(rules),
            behavior: Some(behavior),
            mode: None,
            directories: None,
            destination: None,
        }
    }

    /// Create a new replace rules update.
    pub fn replace_rules(rules: Vec<PermissionRuleValue>, behavior: PermissionBehavior) -> Self {
        Self {
            update_type: PermissionUpdateType::ReplaceRules,
            rules: Some(rules),
            behavior: Some(behavior),
            mode: None,
            directories: None,
            destination: None,
        }
    }

    /// Create a new remove rules update.
    pub fn remove_rules(rules: Vec<PermissionRuleValue>) -> Self {
        Self {
            update_type: PermissionUpdateType::RemoveRules,
            rules: Some(rules),
            behavior: None,
            mode: None,
            directories: None,
            destination: None,
        }
    }

    /// Create a new set mode update.
    pub fn set_mode(mode: PermissionMode) -> Self {
        Self {
            update_type: PermissionUpdateType::SetMode,
            rules: None,
            behavior: None,
            mode: Some(mode),
            directories: None,
            destination: None,
        }
    }

    /// Create a new add directories update.
    pub fn add_directories(directories: Vec<String>) -> Self {
        Self {
            update_type: PermissionUpdateType::AddDirectories,
            rules: None,
            behavior: None,
            mode: None,
            directories: Some(directories),
            destination: None,
        }
    }

    /// Create a new remove directories update.
    pub fn remove_directories(directories: Vec<String>) -> Self {
        Self {
            update_type: PermissionUpdateType::RemoveDirectories,
            rules: None,
            behavior: None,
            mode: None,
            directories: Some(directories),
            destination: None,
        }
    }

    /// Set the destination for this update.
    pub fn with_destination(mut self, destination: PermissionUpdateDestination) -> Self {
        self.destination = Some(destination);
        self
    }

    /// Convert to dictionary format matching TypeScript control protocol.
    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {
        let mut result = HashMap::new();
        result.insert("type".to_string(), serde_json::json!(self.update_type));

        if let Some(ref dest) = self.destination {
            result.insert("destination".to_string(), serde_json::json!(dest));
        }

        match self.update_type {
            PermissionUpdateType::AddRules
            | PermissionUpdateType::ReplaceRules
            | PermissionUpdateType::RemoveRules => {
                if let Some(ref rules) = self.rules {
                    let rules_json: Vec<serde_json::Value> = rules
                        .iter()
                        .map(|r| {
                            serde_json::json!({
                                "toolName": r.tool_name,
                                "ruleContent": r.rule_content,
                            })
                        })
                        .collect();
                    result.insert("rules".to_string(), serde_json::json!(rules_json));
                }
                if let Some(behavior) = self.behavior {
                    result.insert("behavior".to_string(), serde_json::json!(behavior));
                }
            }
            PermissionUpdateType::SetMode => {
                if let Some(mode) = self.mode {
                    result.insert("mode".to_string(), serde_json::json!(mode));
                }
            }
            PermissionUpdateType::AddDirectories | PermissionUpdateType::RemoveDirectories => {
                if let Some(ref dirs) = self.directories {
                    result.insert("directories".to_string(), serde_json::json!(dirs));
                }
            }
        }

        result
    }
}

/// Context information for tool permission callbacks.
#[derive(Debug, Clone, Default)]
pub struct ToolPermissionContext {
    /// Reserved for future abort signal support.
    pub signal: Option<()>,
    /// Permission suggestions from CLI.
    pub suggestions: Vec<PermissionUpdate>,
}

impl ToolPermissionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_suggestions(mut self, suggestions: Vec<PermissionUpdate>) -> Self {
        self.suggestions = suggestions;
        self
    }
}

/// Allow permission result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionResultAllow {
    pub behavior: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_permissions: Option<Vec<PermissionUpdate>>,
}

impl Default for PermissionResultAllow {
    fn default() -> Self {
        Self {
            behavior: "allow".to_string(),
            updated_input: None,
            updated_permissions: None,
        }
    }
}

impl PermissionResultAllow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_updated_input(mut self, input: serde_json::Value) -> Self {
        self.updated_input = Some(input);
        self
    }

    pub fn with_updated_permissions(mut self, permissions: Vec<PermissionUpdate>) -> Self {
        self.updated_permissions = Some(permissions);
        self
    }
}

/// Deny permission result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionResultDeny {
    pub behavior: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub interrupt: bool,
}

impl Default for PermissionResultDeny {
    fn default() -> Self {
        Self {
            behavior: "deny".to_string(),
            message: String::new(),
            interrupt: false,
        }
    }
}

impl PermissionResultDeny {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub fn with_interrupt(mut self, interrupt: bool) -> Self {
        self.interrupt = interrupt;
        self
    }
}

/// Permission result enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PermissionResult {
    Allow(PermissionResultAllow),
    Deny(PermissionResultDeny),
}

impl PermissionResult {
    /// Create an allow result.
    pub fn allow() -> Self {
        Self::Allow(PermissionResultAllow::new())
    }

    /// Create a deny result.
    pub fn deny() -> Self {
        Self::Deny(PermissionResultDeny::new())
    }

    /// Create a deny result with a message.
    pub fn deny_with_message(message: impl Into<String>) -> Self {
        Self::Deny(PermissionResultDeny::new().with_message(message))
    }

    /// Returns true if this is an allow result.
    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow(_))
    }

    /// Returns true if this is a deny result.
    pub fn is_deny(&self) -> bool {
        matches!(self, Self::Deny(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_mode_serde() {
        assert_eq!(PermissionMode::Default.as_str(), "default");
        assert_eq!(PermissionMode::AcceptEdits.as_str(), "acceptEdits");
        assert_eq!(PermissionMode::Plan.as_str(), "plan");
        assert_eq!(
            PermissionMode::BypassPermissions.as_str(),
            "bypassPermissions"
        );
    }

    #[test]
    fn test_permission_update_add_rules() {
        let update = PermissionUpdate::add_rules(
            vec![PermissionRuleValue::new("Bash").with_content("allow all")],
            PermissionBehavior::Allow,
        );
        let dict = update.to_dict();
        assert!(dict.contains_key("rules"));
        assert!(dict.contains_key("behavior"));
    }

    #[test]
    fn test_permission_update_set_mode() {
        let update = PermissionUpdate::set_mode(PermissionMode::AcceptEdits);
        let dict = update.to_dict();
        assert!(dict.contains_key("mode"));
    }

    #[test]
    fn test_permission_result_allow() {
        let result = PermissionResult::allow();
        assert!(result.is_allow());
        assert!(!result.is_deny());
    }

    #[test]
    fn test_permission_result_deny() {
        let result = PermissionResult::deny_with_message("Not allowed");
        assert!(result.is_deny());
        assert!(!result.is_allow());
    }

    #[test]
    fn test_permission_rule_value() {
        let rule = PermissionRuleValue::new("Bash").with_content("allow 'ls' command");
        assert_eq!(rule.tool_name, "Bash");
        assert_eq!(rule.rule_content, Some("allow 'ls' command".to_string()));
    }
}
