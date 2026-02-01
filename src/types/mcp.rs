//! MCP (Model Context Protocol) server configuration types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP stdio server configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpStdioServerConfig {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub config_type: Option<String>,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

impl McpStdioServerConfig {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            config_type: Some("stdio".to_string()),
            command: command.into(),
            args: None,
            env: None,
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = Some(args);
        self
    }

    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = Some(env);
        self
    }
}

/// MCP SSE server configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpSSEServerConfig {
    #[serde(rename = "type")]
    pub config_type: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

impl McpSSEServerConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            config_type: "sse".to_string(),
            url: url.into(),
            headers: None,
        }
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }
}

/// MCP HTTP server configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpHttpServerConfig {
    #[serde(rename = "type")]
    pub config_type: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

impl McpHttpServerConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            config_type: "http".to_string(),
            url: url.into(),
            headers: None,
        }
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }
}

/// SDK MCP server configuration.
///
/// Note: In Rust, we don't have direct support for in-process MCP servers
/// like the Python SDK. This is a placeholder for potential future support.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpSdkServerConfig {
    #[serde(rename = "type")]
    pub config_type: String,
    pub name: String,
    // Instance field is not serialized - it would contain a trait object in a full implementation
}

impl McpSdkServerConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config_type: "sdk".to_string(),
            name: name.into(),
        }
    }
}

/// MCP server configuration enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpServerConfig {
    Stdio {
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        env: Option<HashMap<String, String>>,
    },
    #[serde(rename = "sse")]
    SSE {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
    Http {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
    Sdk {
        name: String,
    },
}

impl McpServerConfig {
    /// Create a new stdio MCP server config.
    pub fn stdio(command: impl Into<String>) -> Self {
        Self::Stdio {
            command: command.into(),
            args: None,
            env: None,
        }
    }

    /// Create a new stdio MCP server config with args.
    pub fn stdio_with_args(command: impl Into<String>, args: Vec<String>) -> Self {
        Self::Stdio {
            command: command.into(),
            args: Some(args),
            env: None,
        }
    }

    /// Create a new SSE MCP server config.
    pub fn sse(url: impl Into<String>) -> Self {
        Self::SSE {
            url: url.into(),
            headers: None,
        }
    }

    /// Create a new HTTP MCP server config.
    pub fn http(url: impl Into<String>) -> Self {
        Self::Http {
            url: url.into(),
            headers: None,
        }
    }

    /// Create a new SDK MCP server config.
    pub fn sdk(name: impl Into<String>) -> Self {
        Self::Sdk { name: name.into() }
    }

    /// Check if this is a stdio config.
    pub fn is_stdio(&self) -> bool {
        matches!(self, Self::Stdio { .. })
    }

    /// Check if this is an SSE config.
    pub fn is_sse(&self) -> bool {
        matches!(self, Self::SSE { .. })
    }

    /// Check if this is an HTTP config.
    pub fn is_http(&self) -> bool {
        matches!(self, Self::Http { .. })
    }

    /// Check if this is an SDK config.
    pub fn is_sdk(&self) -> bool {
        matches!(self, Self::Sdk { .. })
    }
}

impl From<McpStdioServerConfig> for McpServerConfig {
    fn from(config: McpStdioServerConfig) -> Self {
        Self::Stdio {
            command: config.command,
            args: config.args,
            env: config.env,
        }
    }
}

impl From<McpSSEServerConfig> for McpServerConfig {
    fn from(config: McpSSEServerConfig) -> Self {
        Self::SSE {
            url: config.url,
            headers: config.headers,
        }
    }
}

impl From<McpHttpServerConfig> for McpServerConfig {
    fn from(config: McpHttpServerConfig) -> Self {
        Self::Http {
            url: config.url,
            headers: config.headers,
        }
    }
}

impl From<McpSdkServerConfig> for McpServerConfig {
    fn from(config: McpSdkServerConfig) -> Self {
        Self::Sdk { name: config.name }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdio_config_serde() {
        let config = McpServerConfig::stdio_with_args("node", vec!["server.js".to_string()]);
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"type\":\"stdio\""));
        assert!(json.contains("\"command\":\"node\""));

        let parsed: McpServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, config);
    }

    #[test]
    fn test_sse_config_serde() {
        let config = McpServerConfig::sse("http://localhost:3000/sse");
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"type\":\"sse\""));
        assert!(json.contains("\"url\":\"http://localhost:3000/sse\""));
    }

    #[test]
    fn test_http_config_serde() {
        let config = McpServerConfig::http("http://localhost:3000/mcp");
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"type\":\"http\""));
    }

    #[test]
    fn test_sdk_config_serde() {
        let config = McpServerConfig::sdk("my-server");
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"type\":\"sdk\""));
        assert!(json.contains("\"name\":\"my-server\""));
    }

    #[test]
    fn test_config_type_checks() {
        assert!(McpServerConfig::stdio("cmd").is_stdio());
        assert!(McpServerConfig::sse("url").is_sse());
        assert!(McpServerConfig::http("url").is_http());
        assert!(McpServerConfig::sdk("name").is_sdk());
    }

    #[test]
    fn test_from_conversions() {
        let stdio = McpStdioServerConfig::new("node");
        let config: McpServerConfig = stdio.into();
        assert!(config.is_stdio());

        let sse = McpSSEServerConfig::new("http://example.com");
        let config: McpServerConfig = sse.into();
        assert!(config.is_sse());
    }

    #[test]
    fn test_stdio_config_with_args() {
        let config = McpStdioServerConfig::new("npx").with_args(vec!["server.js".to_string()]);
        assert_eq!(config.command, "npx");
        assert_eq!(config.args, Some(vec!["server.js".to_string()]));
    }

    #[test]
    fn test_stdio_config_with_env() {
        let mut env = HashMap::new();
        env.insert("NODE_ENV".to_string(), "production".to_string());
        let config = McpStdioServerConfig::new("node").with_env(env.clone());
        assert_eq!(config.env, Some(env));
    }

    #[test]
    fn test_sse_config_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());
        let config =
            McpSSEServerConfig::new("http://example.com/sse").with_headers(headers.clone());
        assert_eq!(config.headers, Some(headers));
    }

    #[test]
    fn test_http_config_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        let config =
            McpHttpServerConfig::new("http://example.com/mcp").with_headers(headers.clone());
        assert_eq!(config.headers, Some(headers));
    }

    #[test]
    fn test_sdk_server_config() {
        let config = McpSdkServerConfig::new("my-sdk-server");
        assert_eq!(config.config_type, "sdk");
        assert_eq!(config.name, "my-sdk-server");
    }

    #[test]
    fn test_from_http_config() {
        let http = McpHttpServerConfig::new("http://example.com");
        let config: McpServerConfig = http.into();
        assert!(config.is_http());
    }

    #[test]
    fn test_from_sdk_config() {
        let sdk = McpSdkServerConfig::new("test");
        let config: McpServerConfig = sdk.into();
        assert!(config.is_sdk());
    }

    #[test]
    fn test_config_type_checks_negative() {
        let stdio = McpServerConfig::stdio("cmd");
        assert!(!stdio.is_sse());
        assert!(!stdio.is_http());
        assert!(!stdio.is_sdk());

        let sse = McpServerConfig::sse("url");
        assert!(!sse.is_stdio());
        assert!(!sse.is_http());
        assert!(!sse.is_sdk());

        let http = McpServerConfig::http("url");
        assert!(!http.is_stdio());
        assert!(!http.is_sse());
        assert!(!http.is_sdk());

        let sdk = McpServerConfig::sdk("name");
        assert!(!sdk.is_stdio());
        assert!(!sdk.is_sse());
        assert!(!sdk.is_http());
    }

    #[test]
    fn test_stdio_config_deserialization() {
        let json = r#"{"type":"stdio","command":"node","args":["server.js"]}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        assert!(config.is_stdio());
        match config {
            McpServerConfig::Stdio { command, args, .. } => {
                assert_eq!(command, "node");
                assert_eq!(args, Some(vec!["server.js".to_string()]));
            }
            _ => panic!("Expected Stdio config"),
        }
    }

    #[test]
    fn test_sse_config_deserialization() {
        let json = r#"{"type":"sse","url":"http://localhost:3000"}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        assert!(config.is_sse());
    }

    #[test]
    fn test_http_config_new() {
        let config = McpHttpServerConfig::new("http://localhost:8080/mcp");
        assert_eq!(config.config_type, "http");
        assert_eq!(config.url, "http://localhost:8080/mcp");
        assert!(config.headers.is_none());
    }

    #[test]
    fn test_sse_config_new() {
        let config = McpSSEServerConfig::new("http://localhost:3000/events");
        assert_eq!(config.config_type, "sse");
        assert_eq!(config.url, "http://localhost:3000/events");
        assert!(config.headers.is_none());
    }

    #[test]
    fn test_stdio_config_default_fields() {
        let config = McpStdioServerConfig::new("python");
        assert_eq!(config.config_type, Some("stdio".to_string()));
        assert_eq!(config.command, "python");
        assert!(config.args.is_none());
        assert!(config.env.is_none());
    }
}
