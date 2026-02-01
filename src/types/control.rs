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
}
