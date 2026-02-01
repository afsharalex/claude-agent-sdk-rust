//! Error types for Claude Agent SDK.

use serde_json::Value;
use thiserror::Error;

/// Base error type for all Claude SDK errors.
#[derive(Error, Debug)]
pub enum ClaudeSDKError {
    /// Raised when unable to connect to Claude Code.
    #[error("CLI connection error: {0}")]
    CLIConnection(String),

    /// Raised when Claude Code is not found or not installed.
    #[error("CLI not found: {message}")]
    CLINotFound {
        message: String,
        cli_path: Option<String>,
    },

    /// Raised when the CLI process fails.
    #[error("Process error: {message}")]
    Process {
        message: String,
        exit_code: Option<i32>,
        stderr: Option<String>,
    },

    /// Raised when unable to decode JSON from CLI output.
    #[error("JSON decode error: {0}")]
    Json(#[from] serde_json::Error),

    /// Raised when unable to parse a message from CLI output.
    #[error("Message parse error: {message}")]
    MessageParse {
        message: String,
        data: Option<Value>,
    },

    /// Raised when an operation times out.
    #[error("Timeout error: {0}")]
    Timeout(String),

    /// Raised when an IO operation fails.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Raised when a control protocol error occurs.
    #[error("Control protocol error: {0}")]
    ControlProtocol(String),

    /// Raised when an invalid configuration is provided.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

impl ClaudeSDKError {
    /// Create a new CLI not found error with a helpful message.
    pub fn cli_not_found(cli_path: Option<String>) -> Self {
        let message = if let Some(ref path) = cli_path {
            format!("Claude Code not found at: {}", path)
        } else {
            "Claude Code not found. Install with:\n  \
             npm install -g @anthropic-ai/claude-code\n\n\
             If already installed locally, try:\n  \
             export PATH=\"$HOME/node_modules/.bin:$PATH\"\n\n\
             Or provide the path via ClaudeAgentOptions:\n  \
             ClaudeAgentOptions::builder().cli_path(\"/path/to/claude\").build()"
                .to_string()
        };
        Self::CLINotFound { message, cli_path }
    }

    /// Create a new process error.
    pub fn process_error(
        message: impl Into<String>,
        exit_code: Option<i32>,
        stderr: Option<String>,
    ) -> Self {
        let mut msg = message.into();
        if let Some(code) = exit_code {
            msg = format!("{} (exit code: {})", msg, code);
        }
        if let Some(ref err) = stderr {
            msg = format!("{}\nError output: {}", msg, err);
        }
        Self::Process {
            message: msg,
            exit_code,
            stderr,
        }
    }

    /// Create a new message parse error.
    pub fn message_parse(message: impl Into<String>, data: Option<Value>) -> Self {
        Self::MessageParse {
            message: message.into(),
            data,
        }
    }
}

/// Result type alias for ClaudeSDKError.
pub type Result<T> = std::result::Result<T, ClaudeSDKError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_not_found_with_path() {
        let err = ClaudeSDKError::cli_not_found(Some("/usr/bin/claude".to_string()));
        assert!(err
            .to_string()
            .contains("Claude Code not found at: /usr/bin/claude"));
    }

    #[test]
    fn test_cli_not_found_without_path() {
        let err = ClaudeSDKError::cli_not_found(None);
        assert!(err.to_string().contains("npm install -g"));
    }

    #[test]
    fn test_process_error_with_all_fields() {
        let err = ClaudeSDKError::process_error(
            "Command failed",
            Some(1),
            Some("Permission denied".to_string()),
        );
        let msg = err.to_string();
        assert!(msg.contains("Command failed"));
        assert!(msg.contains("exit code: 1"));
        assert!(msg.contains("Permission denied"));
    }

    #[test]
    fn test_message_parse_error() {
        let err = ClaudeSDKError::message_parse("Missing type field", None);
        assert!(err.to_string().contains("Missing type field"));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_err = serde_json::from_str::<Value>("invalid").unwrap_err();
        let err: ClaudeSDKError = json_err.into();
        assert!(matches!(err, ClaudeSDKError::Json(_)));
    }

    #[test]
    fn test_cli_connection_error() {
        let err = ClaudeSDKError::CLIConnection("Failed to connect".to_string());
        assert_eq!(err.to_string(), "CLI connection error: Failed to connect");
    }

    #[test]
    fn test_timeout_error() {
        let err = ClaudeSDKError::Timeout("Request timed out after 60s".to_string());
        assert!(err.to_string().contains("60s"));
    }

    #[test]
    fn test_control_protocol_error() {
        let err = ClaudeSDKError::ControlProtocol("Invalid control message".to_string());
        assert!(err.to_string().contains("Invalid control message"));
    }

    #[test]
    fn test_invalid_config_error() {
        let err = ClaudeSDKError::InvalidConfig("Missing required field".to_string());
        assert!(err.to_string().contains("Missing required field"));
    }

    #[test]
    fn test_process_error_without_optional_fields() {
        let err = ClaudeSDKError::process_error("Command failed", None, None);
        let msg = err.to_string();
        assert!(msg.contains("Command failed"));
        assert!(!msg.contains("exit code"));
        assert!(!msg.contains("Error output"));
    }

    #[test]
    fn test_message_parse_error_with_data() {
        let data = serde_json::json!({"invalid": "message"});
        let err = ClaudeSDKError::message_parse("Invalid message format", Some(data.clone()));
        assert!(err.to_string().contains("Invalid message format"));
        if let ClaudeSDKError::MessageParse { data: Some(d), .. } = err {
            assert_eq!(d, data);
        } else {
            panic!("Expected MessageParse error with data");
        }
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let err: ClaudeSDKError = io_err.into();
        assert!(matches!(err, ClaudeSDKError::Io(_)));
        assert!(err.to_string().contains("File not found"));
    }
}
