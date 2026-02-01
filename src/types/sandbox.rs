//! Sandbox configuration types for Claude SDK.

use serde::{Deserialize, Serialize};

/// Network configuration for sandbox.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxNetworkConfig {
    /// Unix socket paths accessible in sandbox (e.g., SSH agents).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unix_sockets: Option<Vec<String>>,

    /// Allow all Unix sockets (less secure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_all_unix_sockets: Option<bool>,

    /// Allow binding to localhost ports (macOS only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_local_binding: Option<bool>,

    /// HTTP proxy port if bringing your own proxy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_proxy_port: Option<u16>,

    /// SOCKS5 proxy port if bringing your own proxy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socks_proxy_port: Option<u16>,
}

impl SandboxNetworkConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_unix_sockets(mut self, sockets: Vec<String>) -> Self {
        self.allow_unix_sockets = Some(sockets);
        self
    }

    pub fn with_all_unix_sockets(mut self, allow: bool) -> Self {
        self.allow_all_unix_sockets = Some(allow);
        self
    }

    pub fn with_local_binding(mut self, allow: bool) -> Self {
        self.allow_local_binding = Some(allow);
        self
    }

    pub fn with_http_proxy(mut self, port: u16) -> Self {
        self.http_proxy_port = Some(port);
        self
    }

    pub fn with_socks_proxy(mut self, port: u16) -> Self {
        self.socks_proxy_port = Some(port);
        self
    }
}

/// Violations to ignore in sandbox.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SandboxIgnoreViolations {
    /// File paths for which violations should be ignored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<Vec<String>>,

    /// Network hosts for which violations should be ignored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<Vec<String>>,
}

impl SandboxIgnoreViolations {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.file = Some(files);
        self
    }

    pub fn with_networks(mut self, networks: Vec<String>) -> Self {
        self.network = Some(networks);
        self
    }
}

/// Sandbox settings configuration.
///
/// This controls how Claude Code sandboxes bash commands for filesystem
/// and network isolation.
///
/// **Important:** Filesystem and network restrictions are configured via permission
/// rules, not via these sandbox settings:
/// - Filesystem read restrictions: Use Read deny rules
/// - Filesystem write restrictions: Use Edit allow/deny rules
/// - Network restrictions: Use WebFetch allow/deny rules
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxSettings {
    /// Enable bash sandboxing (macOS/Linux only). Default: false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Auto-approve bash commands when sandboxed. Default: true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_allow_bash_if_sandboxed: Option<bool>,

    /// Commands that should run outside the sandbox (e.g., ["git", "docker"])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_commands: Option<Vec<String>>,

    /// Allow commands to bypass sandbox via dangerouslyDisableSandbox.
    /// When false, all commands must run sandboxed (or be in excludedCommands). Default: true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unsandboxed_commands: Option<bool>,

    /// Network configuration for sandbox.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<SandboxNetworkConfig>,

    /// Violations to ignore.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_violations: Option<SandboxIgnoreViolations>,

    /// Enable weaker sandbox for unprivileged Docker environments
    /// (Linux only). Reduces security. Default: false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_weaker_nested_sandbox: Option<bool>,
}

impl SandboxSettings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create sandbox settings with sandboxing enabled.
    pub fn enabled() -> Self {
        Self {
            enabled: Some(true),
            ..Default::default()
        }
    }

    /// Create sandbox settings with sandboxing disabled.
    pub fn disabled() -> Self {
        Self {
            enabled: Some(false),
            ..Default::default()
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    pub fn with_auto_allow_bash(mut self, allow: bool) -> Self {
        self.auto_allow_bash_if_sandboxed = Some(allow);
        self
    }

    pub fn with_excluded_commands(mut self, commands: Vec<String>) -> Self {
        self.excluded_commands = Some(commands);
        self
    }

    pub fn with_allow_unsandboxed(mut self, allow: bool) -> Self {
        self.allow_unsandboxed_commands = Some(allow);
        self
    }

    pub fn with_network(mut self, network: SandboxNetworkConfig) -> Self {
        self.network = Some(network);
        self
    }

    pub fn with_ignore_violations(mut self, violations: SandboxIgnoreViolations) -> Self {
        self.ignore_violations = Some(violations);
        self
    }

    pub fn with_weaker_nested_sandbox(mut self, enable: bool) -> Self {
        self.enable_weaker_nested_sandbox = Some(enable);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_settings_serde() {
        let settings = SandboxSettings::enabled()
            .with_auto_allow_bash(true)
            .with_excluded_commands(vec!["docker".to_string()]);

        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"autoAllowBashIfSandboxed\":true"));
        assert!(json.contains("\"excludedCommands\":[\"docker\"]"));

        let parsed: SandboxSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, settings);
    }

    #[test]
    fn test_sandbox_network_config() {
        let network = SandboxNetworkConfig::new()
            .with_unix_sockets(vec!["/var/run/docker.sock".to_string()])
            .with_local_binding(true);

        let json = serde_json::to_string(&network).unwrap();
        assert!(json.contains("docker.sock"));
        assert!(json.contains("\"allowLocalBinding\":true"));
    }

    #[test]
    fn test_sandbox_ignore_violations() {
        let violations = SandboxIgnoreViolations::new()
            .with_files(vec!["/tmp".to_string()])
            .with_networks(vec!["localhost".to_string()]);

        let json = serde_json::to_string(&violations).unwrap();
        assert!(json.contains("\"/tmp\""));
        assert!(json.contains("\"localhost\""));
    }

    #[test]
    fn test_sandbox_settings_full() {
        let settings = SandboxSettings::enabled()
            .with_network(
                SandboxNetworkConfig::new()
                    .with_unix_sockets(vec!["/var/run/docker.sock".to_string()])
                    .with_local_binding(true),
            )
            .with_ignore_violations(
                SandboxIgnoreViolations::new().with_files(vec!["/tmp".to_string()]),
            );

        let json = serde_json::to_string(&settings).unwrap();
        let parsed: SandboxSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, settings);
    }
}
