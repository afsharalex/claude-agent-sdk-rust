//! Message types for Claude SDK.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::content::ContentBlock;

/// Assistant message error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistantMessageError {
    AuthenticationFailed,
    BillingError,
    RateLimit,
    InvalidRequest,
    ServerError,
    Unknown,
}

/// User message content - can be a string or list of content blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserMessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

impl From<String> for UserMessageContent {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<&str> for UserMessageContent {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<Vec<ContentBlock>> for UserMessageContent {
    fn from(blocks: Vec<ContentBlock>) -> Self {
        Self::Blocks(blocks)
    }
}

/// User message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserMessage {
    pub content: UserMessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_result: Option<Value>,
}

impl UserMessage {
    pub fn new(content: impl Into<UserMessageContent>) -> Self {
        Self {
            content: content.into(),
            uuid: None,
            parent_tool_use_id: None,
            tool_use_result: None,
        }
    }

    pub fn with_uuid(mut self, uuid: impl Into<String>) -> Self {
        self.uuid = Some(uuid.into());
        self
    }

    pub fn with_parent_tool_use_id(mut self, id: impl Into<String>) -> Self {
        self.parent_tool_use_id = Some(id.into());
        self
    }

    pub fn with_tool_use_result(mut self, result: Value) -> Self {
        self.tool_use_result = Some(result);
        self
    }
}

/// Assistant message with content blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub content: Vec<ContentBlock>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AssistantMessageError>,
}

impl AssistantMessage {
    pub fn new(content: Vec<ContentBlock>, model: impl Into<String>) -> Self {
        Self {
            content,
            model: model.into(),
            parent_tool_use_id: None,
            error: None,
        }
    }

    pub fn with_parent_tool_use_id(mut self, id: impl Into<String>) -> Self {
        self.parent_tool_use_id = Some(id.into());
        self
    }

    pub fn with_error(mut self, error: AssistantMessageError) -> Self {
        self.error = Some(error);
        self
    }

    /// Get all text content from this message.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| block.as_text())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Get all tool use blocks from this message.
    pub fn tool_uses(&self) -> Vec<&ContentBlock> {
        self.content
            .iter()
            .filter(|block| block.is_tool_use())
            .collect()
    }
}

/// System message with metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemMessage {
    pub subtype: String,
    pub data: HashMap<String, Value>,
}

impl SystemMessage {
    pub fn new(subtype: impl Into<String>, data: HashMap<String, Value>) -> Self {
        Self {
            subtype: subtype.into(),
            data,
        }
    }
}

/// Result message with cost and usage information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultMessage {
    pub subtype: String,
    pub duration_ms: i64,
    pub duration_api_ms: i64,
    pub is_error: bool,
    pub num_turns: i32,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<Value>,
}

impl ResultMessage {
    pub fn new(
        subtype: impl Into<String>,
        duration_ms: i64,
        duration_api_ms: i64,
        is_error: bool,
        num_turns: i32,
        session_id: impl Into<String>,
    ) -> Self {
        Self {
            subtype: subtype.into(),
            duration_ms,
            duration_api_ms,
            is_error,
            num_turns,
            session_id: session_id.into(),
            total_cost_usd: None,
            usage: None,
            result: None,
            structured_output: None,
        }
    }

    pub fn with_cost(mut self, cost: f64) -> Self {
        self.total_cost_usd = Some(cost);
        self
    }

    pub fn with_usage(mut self, usage: Value) -> Self {
        self.usage = Some(usage);
        self
    }

    pub fn with_result(mut self, result: impl Into<String>) -> Self {
        self.result = Some(result.into());
        self
    }

    pub fn with_structured_output(mut self, output: Value) -> Self {
        self.structured_output = Some(output);
        self
    }
}

/// Stream event for partial message updates during streaming.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamEvent {
    pub uuid: String,
    pub session_id: String,
    pub event: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
}

impl StreamEvent {
    pub fn new(uuid: impl Into<String>, session_id: impl Into<String>, event: Value) -> Self {
        Self {
            uuid: uuid.into(),
            session_id: session_id.into(),
            event,
            parent_tool_use_id: None,
        }
    }

    pub fn with_parent_tool_use_id(mut self, id: impl Into<String>) -> Self {
        self.parent_tool_use_id = Some(id.into());
        self
    }
}

/// Message enum representing all possible message types.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
    System(SystemMessage),
    Result(ResultMessage),
    StreamEvent(StreamEvent),
}

impl Message {
    /// Returns true if this is a user message.
    pub fn is_user(&self) -> bool {
        matches!(self, Self::User(_))
    }

    /// Returns true if this is an assistant message.
    pub fn is_assistant(&self) -> bool {
        matches!(self, Self::Assistant(_))
    }

    /// Returns true if this is a system message.
    pub fn is_system(&self) -> bool {
        matches!(self, Self::System(_))
    }

    /// Returns true if this is a result message.
    pub fn is_result(&self) -> bool {
        matches!(self, Self::Result(_))
    }

    /// Returns true if this is a stream event.
    pub fn is_stream_event(&self) -> bool {
        matches!(self, Self::StreamEvent(_))
    }

    /// Get as user message if applicable.
    pub fn as_user(&self) -> Option<&UserMessage> {
        match self {
            Self::User(msg) => Some(msg),
            _ => None,
        }
    }

    /// Get as assistant message if applicable.
    pub fn as_assistant(&self) -> Option<&AssistantMessage> {
        match self {
            Self::Assistant(msg) => Some(msg),
            _ => None,
        }
    }

    /// Get as system message if applicable.
    pub fn as_system(&self) -> Option<&SystemMessage> {
        match self {
            Self::System(msg) => Some(msg),
            _ => None,
        }
    }

    /// Get as result message if applicable.
    pub fn as_result(&self) -> Option<&ResultMessage> {
        match self {
            Self::Result(msg) => Some(msg),
            _ => None,
        }
    }

    /// Get as stream event if applicable.
    pub fn as_stream_event(&self) -> Option<&StreamEvent> {
        match self {
            Self::StreamEvent(event) => Some(event),
            _ => None,
        }
    }
}

impl From<UserMessage> for Message {
    fn from(msg: UserMessage) -> Self {
        Self::User(msg)
    }
}

impl From<AssistantMessage> for Message {
    fn from(msg: AssistantMessage) -> Self {
        Self::Assistant(msg)
    }
}

impl From<SystemMessage> for Message {
    fn from(msg: SystemMessage) -> Self {
        Self::System(msg)
    }
}

impl From<ResultMessage> for Message {
    fn from(msg: ResultMessage) -> Self {
        Self::Result(msg)
    }
}

impl From<StreamEvent> for Message {
    fn from(event: StreamEvent) -> Self {
        Self::StreamEvent(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_user_message_text() {
        let msg = UserMessage::new("Hello, Claude!");
        match &msg.content {
            UserMessageContent::Text(text) => assert_eq!(text, "Hello, Claude!"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_user_message_blocks() {
        let blocks = vec![ContentBlock::text("Hello")];
        let msg = UserMessage::new(blocks);
        match &msg.content {
            UserMessageContent::Blocks(b) => assert_eq!(b.len(), 1),
            _ => panic!("Expected block content"),
        }
    }

    #[test]
    fn test_assistant_message_text_extraction() {
        let msg = AssistantMessage::new(
            vec![ContentBlock::text("Hello "), ContentBlock::text("World!")],
            "claude-3-5-sonnet",
        );
        assert_eq!(msg.text(), "Hello World!");
    }

    #[test]
    fn test_result_message() {
        let msg = ResultMessage::new("success", 1000, 800, false, 3, "session-123")
            .with_cost(0.05)
            .with_result("Task completed");
        assert_eq!(msg.total_cost_usd, Some(0.05));
        assert_eq!(msg.result, Some("Task completed".to_string()));
    }

    #[test]
    fn test_stream_event() {
        let event = StreamEvent::new(
            "uuid-1",
            "session-1",
            json!({"type": "content_block_delta"}),
        );
        assert_eq!(event.uuid, "uuid-1");
        assert_eq!(event.session_id, "session-1");
    }

    #[test]
    fn test_message_type_checks() {
        let user_msg: Message = UserMessage::new("test").into();
        assert!(user_msg.is_user());
        assert!(!user_msg.is_assistant());

        let assistant_msg: Message = AssistantMessage::new(vec![], "model").into();
        assert!(assistant_msg.is_assistant());
        assert!(!assistant_msg.is_user());
    }

    #[test]
    fn test_message_as_conversions() {
        let msg: Message = UserMessage::new("test").into();
        assert!(msg.as_user().is_some());
        assert!(msg.as_assistant().is_none());
    }
}
