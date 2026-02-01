//! SDK Control Protocol types.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// SDK Control interrupt request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SDKControlInterruptRequest {
    pub subtype: String,
}

impl Default for SDKControlInterruptRequest {
    fn default() -> Self {
        Self {
            subtype: "interrupt".to_string(),
        }
    }
}

/// SDK Control permission request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SDKControlPermissionRequest {
    pub subtype: String,
    pub tool_name: String,
    pub input: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_suggestions: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_path: Option<String>,
}

/// SDK Control initialize request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SDKControlInitializeRequest {
    pub subtype: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<HashMap<String, Value>>,
}

impl Default for SDKControlInitializeRequest {
    fn default() -> Self {
        Self {
            subtype: "initialize".to_string(),
            hooks: None,
        }
    }
}

/// SDK Control set permission mode request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SDKControlSetPermissionModeRequest {
    pub subtype: String,
    pub mode: String,
}

impl SDKControlSetPermissionModeRequest {
    pub fn new(mode: impl Into<String>) -> Self {
        Self {
            subtype: "set_permission_mode".to_string(),
            mode: mode.into(),
        }
    }
}

/// SDK Control set model request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SDKControlSetModelRequest {
    pub subtype: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl SDKControlSetModelRequest {
    pub fn new(model: Option<String>) -> Self {
        Self {
            subtype: "set_model".to_string(),
            model,
        }
    }
}

/// SDK Hook callback request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SDKHookCallbackRequest {
    pub subtype: String,
    pub callback_id: String,
    pub input: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
}

/// SDK Control MCP message request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SDKControlMcpMessageRequest {
    pub subtype: String,
    pub server_name: String,
    pub message: Value,
}

/// SDK Control rewind files request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SDKControlRewindFilesRequest {
    pub subtype: String,
    pub user_message_id: String,
}

impl SDKControlRewindFilesRequest {
    pub fn new(user_message_id: impl Into<String>) -> Self {
        Self {
            subtype: "rewind_files".to_string(),
            user_message_id: user_message_id.into(),
        }
    }
}

/// SDK Control MCP status request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SDKControlMcpStatusRequest {
    pub subtype: String,
}

impl Default for SDKControlMcpStatusRequest {
    fn default() -> Self {
        Self {
            subtype: "mcp_status".to_string(),
        }
    }
}

/// SDK Control request variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum SDKControlRequestVariant {
    Interrupt,
    CanUseTool {
        tool_name: String,
        input: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        permission_suggestions: Option<Vec<Value>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        blocked_path: Option<String>,
    },
    Initialize {
        #[serde(skip_serializing_if = "Option::is_none")]
        hooks: Option<HashMap<String, Value>>,
    },
    SetPermissionMode {
        mode: String,
    },
    SetModel {
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    HookCallback {
        callback_id: String,
        input: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_use_id: Option<String>,
    },
    McpMessage {
        server_name: String,
        message: Value,
    },
    RewindFiles {
        user_message_id: String,
    },
    McpStatus,
}

/// SDK Control request wrapper.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SDKControlRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub request_id: String,
    pub request: SDKControlRequestVariant,
}

impl SDKControlRequest {
    pub fn new(request_id: impl Into<String>, request: SDKControlRequestVariant) -> Self {
        Self {
            request_type: "control_request".to_string(),
            request_id: request_id.into(),
            request,
        }
    }

    /// Create an interrupt request.
    pub fn interrupt(request_id: impl Into<String>) -> Self {
        Self::new(request_id, SDKControlRequestVariant::Interrupt)
    }

    /// Create an initialize request.
    pub fn initialize(
        request_id: impl Into<String>,
        hooks: Option<HashMap<String, Value>>,
    ) -> Self {
        Self::new(request_id, SDKControlRequestVariant::Initialize { hooks })
    }

    /// Create a set permission mode request.
    pub fn set_permission_mode(request_id: impl Into<String>, mode: impl Into<String>) -> Self {
        Self::new(
            request_id,
            SDKControlRequestVariant::SetPermissionMode { mode: mode.into() },
        )
    }

    /// Create a set model request.
    pub fn set_model(request_id: impl Into<String>, model: Option<String>) -> Self {
        Self::new(request_id, SDKControlRequestVariant::SetModel { model })
    }

    /// Create an MCP status request.
    pub fn mcp_status(request_id: impl Into<String>) -> Self {
        Self::new(request_id, SDKControlRequestVariant::McpStatus)
    }

    /// Create a rewind files request.
    pub fn rewind_files(request_id: impl Into<String>, user_message_id: impl Into<String>) -> Self {
        Self::new(
            request_id,
            SDKControlRequestVariant::RewindFiles {
                user_message_id: user_message_id.into(),
            },
        )
    }
}

/// Success control response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlResponseSuccess {
    pub subtype: String,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Value>,
}

/// Error control response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlResponseError {
    pub subtype: String,
    pub request_id: String,
    pub error: String,
}

/// Control response variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "subtype", rename_all = "lowercase")]
pub enum ControlResponseVariant {
    Success {
        request_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        response: Option<Value>,
    },
    Error {
        request_id: String,
        error: String,
    },
}

impl ControlResponseVariant {
    /// Get the request ID.
    pub fn request_id(&self) -> &str {
        match self {
            Self::Success { request_id, .. } => request_id,
            Self::Error { request_id, .. } => request_id,
        }
    }

    /// Check if this is a success response.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Check if this is an error response.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    /// Get the response data if this is a success.
    pub fn response(&self) -> Option<&Value> {
        match self {
            Self::Success { response, .. } => response.as_ref(),
            Self::Error { .. } => None,
        }
    }

    /// Get the error message if this is an error.
    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Success { .. } => None,
            Self::Error { error, .. } => Some(error),
        }
    }
}

/// SDK Control response wrapper.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SDKControlResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub response: ControlResponseVariant,
}

impl SDKControlResponse {
    /// Create a success response.
    pub fn success(request_id: impl Into<String>, response: Option<Value>) -> Self {
        Self {
            response_type: "control_response".to_string(),
            response: ControlResponseVariant::Success {
                request_id: request_id.into(),
                response,
            },
        }
    }

    /// Create an error response.
    pub fn error(request_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            response_type: "control_response".to_string(),
            response: ControlResponseVariant::Error {
                request_id: request_id.into(),
                error: error.into(),
            },
        }
    }

    /// Get the request ID.
    pub fn request_id(&self) -> &str {
        self.response.request_id()
    }

    /// Check if this is a success response.
    pub fn is_success(&self) -> bool {
        self.response.is_success()
    }

    /// Check if this is an error response.
    pub fn is_error(&self) -> bool {
        self.response.is_error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_control_request_interrupt() {
        let request = SDKControlRequest::interrupt("req-1");
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"type\":\"control_request\""));
        assert!(json.contains("\"request_id\":\"req-1\""));
        assert!(json.contains("\"subtype\":\"interrupt\""));
    }

    #[test]
    fn test_control_request_initialize() {
        let request = SDKControlRequest::initialize("req-2", None);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"subtype\":\"initialize\""));
    }

    #[test]
    fn test_control_request_set_permission_mode() {
        let request = SDKControlRequest::set_permission_mode("req-3", "acceptEdits");
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"subtype\":\"set_permission_mode\""));
        assert!(json.contains("\"mode\":\"acceptEdits\""));
    }

    #[test]
    fn test_control_response_success() {
        let response = SDKControlResponse::success("req-1", Some(json!({"status": "ok"})));
        assert!(response.is_success());
        assert!(!response.is_error());
        assert_eq!(response.request_id(), "req-1");
    }

    #[test]
    fn test_control_response_error() {
        let response = SDKControlResponse::error("req-1", "Something went wrong");
        assert!(response.is_error());
        assert!(!response.is_success());
        assert_eq!(response.response.error(), Some("Something went wrong"));
    }

    #[test]
    fn test_control_response_serde() {
        let response = SDKControlResponse::success("req-1", Some(json!({"data": 123})));
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"type\":\"control_response\""));
        assert!(json.contains("\"subtype\":\"success\""));

        let parsed: SDKControlResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, response);
    }

    #[test]
    fn test_sdk_control_request_new() {
        let request = SDKControlRequest::new("req-1", SDKControlRequestVariant::Interrupt);
        assert_eq!(request.request_type, "control_request");
        assert_eq!(request.request_id, "req-1");
    }

    #[test]
    fn test_sdk_control_request_set_model() {
        let request = SDKControlRequest::set_model("req-1", Some("claude-3-5-sonnet".to_string()));
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"subtype\":\"set_model\""));
        assert!(json.contains("\"model\":\"claude-3-5-sonnet\""));
    }

    #[test]
    fn test_sdk_control_request_set_model_none() {
        let request = SDKControlRequest::set_model("req-1", None);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"subtype\":\"set_model\""));
        // model should not be serialized when None
    }

    #[test]
    fn test_sdk_control_request_mcp_status() {
        let request = SDKControlRequest::mcp_status("req-1");
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"subtype\":\"mcp_status\""));
    }

    #[test]
    fn test_sdk_control_request_rewind_files() {
        let request = SDKControlRequest::rewind_files("req-1", "user-msg-123");
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"subtype\":\"rewind_files\""));
        assert!(json.contains("\"user_message_id\":\"user-msg-123\""));
    }

    #[test]
    fn test_control_response_variant_request_id() {
        let success = ControlResponseVariant::Success {
            request_id: "req-1".to_string(),
            response: None,
        };
        assert_eq!(success.request_id(), "req-1");

        let error = ControlResponseVariant::Error {
            request_id: "req-2".to_string(),
            error: "something went wrong".to_string(),
        };
        assert_eq!(error.request_id(), "req-2");
    }

    #[test]
    fn test_control_response_variant_is_success() {
        let success = ControlResponseVariant::Success {
            request_id: "req-1".to_string(),
            response: Some(json!({})),
        };
        assert!(success.is_success());
        assert!(!success.is_error());
    }

    #[test]
    fn test_control_response_variant_is_error() {
        let error = ControlResponseVariant::Error {
            request_id: "req-1".to_string(),
            error: "error message".to_string(),
        };
        assert!(error.is_error());
        assert!(!error.is_success());
    }

    #[test]
    fn test_control_response_variant_response() {
        let success = ControlResponseVariant::Success {
            request_id: "req-1".to_string(),
            response: Some(json!({"key": "value"})),
        };
        assert!(success.response().is_some());
        assert_eq!(success.response().unwrap()["key"], "value");

        let error = ControlResponseVariant::Error {
            request_id: "req-1".to_string(),
            error: "error".to_string(),
        };
        assert!(error.response().is_none());
    }

    #[test]
    fn test_control_response_variant_error() {
        let success = ControlResponseVariant::Success {
            request_id: "req-1".to_string(),
            response: None,
        };
        assert!(success.error().is_none());

        let error = ControlResponseVariant::Error {
            request_id: "req-1".to_string(),
            error: "error message".to_string(),
        };
        assert_eq!(error.error(), Some("error message"));
    }

    #[test]
    fn test_sdk_control_interrupt_request_default() {
        let request = SDKControlInterruptRequest::default();
        assert_eq!(request.subtype, "interrupt");
    }

    #[test]
    fn test_sdk_control_initialize_request_default() {
        let request = SDKControlInitializeRequest::default();
        assert_eq!(request.subtype, "initialize");
        assert!(request.hooks.is_none());
    }

    #[test]
    fn test_sdk_control_set_permission_mode_request_new() {
        let request = SDKControlSetPermissionModeRequest::new("acceptEdits");
        assert_eq!(request.subtype, "set_permission_mode");
        assert_eq!(request.mode, "acceptEdits");
    }

    #[test]
    fn test_sdk_control_set_model_request_new() {
        let request = SDKControlSetModelRequest::new(Some("claude-3-5-sonnet".to_string()));
        assert_eq!(request.subtype, "set_model");
        assert_eq!(request.model, Some("claude-3-5-sonnet".to_string()));
    }

    #[test]
    fn test_sdk_control_rewind_files_request_new() {
        let request = SDKControlRewindFilesRequest::new("user-msg-123");
        assert_eq!(request.subtype, "rewind_files");
        assert_eq!(request.user_message_id, "user-msg-123");
    }

    #[test]
    fn test_sdk_control_mcp_status_request_default() {
        let request = SDKControlMcpStatusRequest::default();
        assert_eq!(request.subtype, "mcp_status");
    }

    #[test]
    fn test_control_request_variant_can_use_tool() {
        let variant = SDKControlRequestVariant::CanUseTool {
            tool_name: "Bash".to_string(),
            input: json!({"command": "ls"}),
            permission_suggestions: None,
            blocked_path: None,
        };
        let json = serde_json::to_string(&variant).unwrap();
        assert!(json.contains("\"subtype\":\"can_use_tool\""));
        assert!(json.contains("\"tool_name\":\"Bash\""));
    }

    #[test]
    fn test_control_request_variant_hook_callback() {
        let variant = SDKControlRequestVariant::HookCallback {
            callback_id: "hook_0".to_string(),
            input: json!({"event": "test"}),
            tool_use_id: Some("tool-123".to_string()),
        };
        let json = serde_json::to_string(&variant).unwrap();
        assert!(json.contains("\"subtype\":\"hook_callback\""));
        assert!(json.contains("\"callback_id\":\"hook_0\""));
    }

    #[test]
    fn test_control_request_variant_mcp_message() {
        let variant = SDKControlRequestVariant::McpMessage {
            server_name: "test-server".to_string(),
            message: json!({"jsonrpc": "2.0", "method": "tools/list", "id": 1}),
        };
        let json = serde_json::to_string(&variant).unwrap();
        assert!(json.contains("\"subtype\":\"mcp_message\""));
        assert!(json.contains("\"server_name\":\"test-server\""));
    }

    #[test]
    fn test_control_request_deserialization() {
        let json_str = r#"{
            "type": "control_request",
            "request_id": "test-req",
            "request": {
                "subtype": "interrupt"
            }
        }"#;
        let request: SDKControlRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(request.request_id, "test-req");
        assert!(matches!(
            request.request,
            SDKControlRequestVariant::Interrupt
        ));
    }

    #[test]
    fn test_control_response_deserialization() {
        let json_str = r#"{
            "type": "control_response",
            "response": {
                "subtype": "success",
                "request_id": "test-req",
                "response": {"status": "ok"}
            }
        }"#;
        let response: SDKControlResponse = serde_json::from_str(json_str).unwrap();
        assert!(response.is_success());
        assert_eq!(response.request_id(), "test-req");
    }

    #[test]
    fn test_control_request_initialize_with_hooks() {
        let mut hooks = HashMap::new();
        hooks.insert("PreToolUse".to_string(), json!([{"matcher": "Bash"}]));
        let request = SDKControlRequest::initialize("req-1", Some(hooks));
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"subtype\":\"initialize\""));
        assert!(json.contains("\"hooks\""));
    }
}
