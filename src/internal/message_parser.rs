//! Message parser for Claude Code SDK responses.

use serde_json::Value;

use crate::error::{ClaudeSDKError, Result};
use crate::types::{
    AssistantMessage, AssistantMessageError, ContentBlock, Message, ResultMessage, StreamEvent,
    SystemMessage, UserMessage, UserMessageContent,
};

/// Parse a message from CLI output into typed Message objects.
///
/// # Arguments
/// * `data` - Raw message dictionary from CLI output
///
/// # Returns
/// Parsed Message object
///
/// # Errors
/// Returns MessageParseError if parsing fails or message type is unrecognized.
pub fn parse_message(data: Value) -> Result<Message> {
    let obj = match data {
        Value::Object(ref o) => o,
        _ => {
            return Err(ClaudeSDKError::message_parse(
                format!(
                    "Invalid message data type (expected object, got {})",
                    value_type_name(&data)
                ),
                Some(data),
            ));
        }
    };

    let message_type = match obj.get("type").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => {
            return Err(ClaudeSDKError::message_parse(
                "Message missing 'type' field",
                Some(data),
            ));
        }
    };

    match message_type {
        "user" => parse_user_message(obj),
        "assistant" => parse_assistant_message(obj),
        "system" => parse_system_message(obj),
        "result" => parse_result_message(obj),
        "stream_event" => parse_stream_event(obj),
        _ => Err(ClaudeSDKError::message_parse(
            format!("Unknown message type: {}", message_type),
            Some(data),
        )),
    }
}

fn parse_user_message(obj: &serde_json::Map<String, Value>) -> Result<Message> {
    let message = obj.get("message").ok_or_else(|| {
        ClaudeSDKError::message_parse("Missing 'message' field in user message", None)
    })?;

    let content_value = message.get("content").ok_or_else(|| {
        ClaudeSDKError::message_parse("Missing 'content' field in user message", None)
    })?;

    let content = if let Some(text) = content_value.as_str() {
        UserMessageContent::Text(text.to_string())
    } else if let Some(blocks) = content_value.as_array() {
        let parsed_blocks: Vec<ContentBlock> = blocks
            .iter()
            .filter_map(|block| parse_content_block(block).ok())
            .collect();
        UserMessageContent::Blocks(parsed_blocks)
    } else {
        UserMessageContent::Text(content_value.to_string())
    };

    let uuid = obj.get("uuid").and_then(|v| v.as_str()).map(String::from);
    let parent_tool_use_id = obj
        .get("parent_tool_use_id")
        .and_then(|v| v.as_str())
        .map(String::from);
    let tool_use_result = obj.get("tool_use_result").cloned();

    Ok(Message::User(UserMessage {
        content,
        uuid,
        parent_tool_use_id,
        tool_use_result,
    }))
}

fn parse_assistant_message(obj: &serde_json::Map<String, Value>) -> Result<Message> {
    let message = obj.get("message").ok_or_else(|| {
        ClaudeSDKError::message_parse("Missing 'message' field in assistant message", None)
    })?;

    let content_value = message.get("content").ok_or_else(|| {
        ClaudeSDKError::message_parse("Missing 'content' field in assistant message", None)
    })?;

    let content_blocks: Vec<ContentBlock> = if let Some(blocks) = content_value.as_array() {
        blocks
            .iter()
            .filter_map(|block| parse_content_block(block).ok())
            .collect()
    } else {
        Vec::new()
    };

    let model = message
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let parent_tool_use_id = obj
        .get("parent_tool_use_id")
        .and_then(|v| v.as_str())
        .map(String::from);

    let error = message
        .get("error")
        .and_then(|v| v.as_str())
        .and_then(parse_assistant_error);

    Ok(Message::Assistant(AssistantMessage {
        content: content_blocks,
        model,
        parent_tool_use_id,
        error,
    }))
}

fn parse_system_message(obj: &serde_json::Map<String, Value>) -> Result<Message> {
    let subtype = obj
        .get("subtype")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ClaudeSDKError::message_parse("Missing 'subtype' field in system message", None)
        })?
        .to_string();

    let mut data_map = std::collections::HashMap::new();
    for (k, v) in obj {
        data_map.insert(k.clone(), v.clone());
    }

    Ok(Message::System(SystemMessage {
        subtype,
        data: data_map,
    }))
}

fn parse_result_message(obj: &serde_json::Map<String, Value>) -> Result<Message> {
    let subtype = obj
        .get("subtype")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let duration_ms = obj
        .get("duration_ms")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| {
            ClaudeSDKError::message_parse("Missing 'duration_ms' field in result message", None)
        })?;

    let duration_api_ms = obj
        .get("duration_api_ms")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let is_error = obj
        .get("is_error")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let num_turns = obj.get("num_turns").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

    let session_id = obj
        .get("session_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ClaudeSDKError::message_parse("Missing 'session_id' field in result message", None)
        })?
        .to_string();

    let total_cost_usd = obj.get("total_cost_usd").and_then(|v| v.as_f64());
    let usage = obj.get("usage").cloned();
    let result = obj.get("result").and_then(|v| v.as_str()).map(String::from);
    let structured_output = obj.get("structured_output").cloned();

    Ok(Message::Result(ResultMessage {
        subtype,
        duration_ms,
        duration_api_ms,
        is_error,
        num_turns,
        session_id,
        total_cost_usd,
        usage,
        result,
        structured_output,
    }))
}

fn parse_stream_event(obj: &serde_json::Map<String, Value>) -> Result<Message> {
    let uuid = obj
        .get("uuid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ClaudeSDKError::message_parse("Missing 'uuid' field in stream_event", None))?
        .to_string();

    let session_id = obj
        .get("session_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ClaudeSDKError::message_parse("Missing 'session_id' field in stream_event", None)
        })?
        .to_string();

    let event = obj.get("event").cloned().unwrap_or(Value::Null);

    let parent_tool_use_id = obj
        .get("parent_tool_use_id")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(Message::StreamEvent(StreamEvent {
        uuid,
        session_id,
        event,
        parent_tool_use_id,
    }))
}

fn parse_content_block(block: &Value) -> Result<ContentBlock> {
    let obj = block.as_object().ok_or_else(|| {
        ClaudeSDKError::message_parse("Content block is not an object", Some(block.clone()))
    })?;

    let block_type = obj.get("type").and_then(|v| v.as_str()).ok_or_else(|| {
        ClaudeSDKError::message_parse("Content block missing 'type' field", Some(block.clone()))
    })?;

    match block_type {
        "text" => {
            let text = obj
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(ContentBlock::Text { text })
        }
        "thinking" => {
            let thinking = obj
                .get("thinking")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let signature = obj
                .get("signature")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(ContentBlock::Thinking {
                thinking,
                signature,
            })
        }
        "tool_use" => {
            let id = obj
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name = obj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let input = obj.get("input").cloned().unwrap_or(Value::Null);
            Ok(ContentBlock::ToolUse { id, name, input })
        }
        "tool_result" => {
            let tool_use_id = obj
                .get("tool_use_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let content = obj.get("content").cloned();
            let is_error = obj.get("is_error").and_then(|v| v.as_bool());
            Ok(ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            })
        }
        _ => Err(ClaudeSDKError::message_parse(
            format!("Unknown content block type: {}", block_type),
            Some(block.clone()),
        )),
    }
}

fn parse_assistant_error(error: &str) -> Option<AssistantMessageError> {
    match error {
        "authentication_failed" => Some(AssistantMessageError::AuthenticationFailed),
        "billing_error" => Some(AssistantMessageError::BillingError),
        "rate_limit" => Some(AssistantMessageError::RateLimit),
        "invalid_request" => Some(AssistantMessageError::InvalidRequest),
        "server_error" => Some(AssistantMessageError::ServerError),
        _ => Some(AssistantMessageError::Unknown),
    }
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_user_message_text() {
        let data = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": "Hello, Claude!"
            }
        });

        let msg = parse_message(data).unwrap();
        assert!(msg.is_user());

        if let Message::User(user_msg) = msg {
            match user_msg.content {
                UserMessageContent::Text(text) => assert_eq!(text, "Hello, Claude!"),
                _ => panic!("Expected text content"),
            }
        }
    }

    #[test]
    fn test_parse_user_message_with_blocks() {
        let data = json!({
            "type": "user",
            "uuid": "uuid-123",
            "message": {
                "role": "user",
                "content": [
                    {"type": "text", "text": "Hello"},
                    {"type": "tool_result", "tool_use_id": "tool-1", "content": "result"}
                ]
            }
        });

        let msg = parse_message(data).unwrap();
        if let Message::User(user_msg) = msg {
            assert_eq!(user_msg.uuid, Some("uuid-123".to_string()));
            match user_msg.content {
                UserMessageContent::Blocks(blocks) => assert_eq!(blocks.len(), 2),
                _ => panic!("Expected block content"),
            }
        }
    }

    #[test]
    fn test_parse_assistant_message() {
        let data = json!({
            "type": "assistant",
            "message": {
                "model": "claude-3-5-sonnet",
                "content": [
                    {"type": "text", "text": "Hello!"},
                    {"type": "thinking", "thinking": "Let me think...", "signature": "sig123"}
                ]
            }
        });

        let msg = parse_message(data).unwrap();
        assert!(msg.is_assistant());

        if let Message::Assistant(asst_msg) = msg {
            assert_eq!(asst_msg.model, "claude-3-5-sonnet");
            assert_eq!(asst_msg.content.len(), 2);
        }
    }

    #[test]
    fn test_parse_assistant_message_with_tool_use() {
        let data = json!({
            "type": "assistant",
            "message": {
                "model": "claude-3-5-sonnet",
                "content": [
                    {
                        "type": "tool_use",
                        "id": "tool-123",
                        "name": "Bash",
                        "input": {"command": "ls"}
                    }
                ]
            }
        });

        let msg = parse_message(data).unwrap();
        if let Message::Assistant(asst_msg) = msg {
            assert_eq!(asst_msg.content.len(), 1);
            assert!(asst_msg.content[0].is_tool_use());
        }
    }

    #[test]
    fn test_parse_system_message() {
        let data = json!({
            "type": "system",
            "subtype": "init",
            "data": {"key": "value"}
        });

        let msg = parse_message(data).unwrap();
        assert!(msg.is_system());

        if let Message::System(sys_msg) = msg {
            assert_eq!(sys_msg.subtype, "init");
        }
    }

    #[test]
    fn test_parse_result_message() {
        let data = json!({
            "type": "result",
            "subtype": "success",
            "duration_ms": 1000,
            "duration_api_ms": 800,
            "is_error": false,
            "num_turns": 3,
            "session_id": "session-123",
            "total_cost_usd": 0.05
        });

        let msg = parse_message(data).unwrap();
        assert!(msg.is_result());

        if let Message::Result(result_msg) = msg {
            assert_eq!(result_msg.session_id, "session-123");
            assert_eq!(result_msg.duration_ms, 1000);
            assert_eq!(result_msg.total_cost_usd, Some(0.05));
        }
    }

    #[test]
    fn test_parse_stream_event() {
        let data = json!({
            "type": "stream_event",
            "uuid": "uuid-1",
            "session_id": "session-1",
            "event": {"type": "content_block_delta"}
        });

        let msg = parse_message(data).unwrap();
        assert!(msg.is_stream_event());

        if let Message::StreamEvent(event) = msg {
            assert_eq!(event.uuid, "uuid-1");
            assert_eq!(event.session_id, "session-1");
        }
    }

    #[test]
    fn test_parse_unknown_type() {
        let data = json!({
            "type": "unknown_type"
        });

        let result = parse_message(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_type() {
        let data = json!({
            "content": "hello"
        });

        let result = parse_message(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_data_type() {
        let data = json!("not an object");

        let result = parse_message(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_user_message_with_parent_tool_use_id() {
        let data = json!({
            "type": "user",
            "uuid": "uuid-1",
            "parent_tool_use_id": "tool-use-123",
            "message": {
                "role": "user",
                "content": "Tool result"
            }
        });

        let msg = parse_message(data).unwrap();
        if let Message::User(user_msg) = msg {
            assert_eq!(
                user_msg.parent_tool_use_id,
                Some("tool-use-123".to_string())
            );
        } else {
            panic!("Expected user message");
        }
    }

    #[test]
    fn test_parse_assistant_message_with_error() {
        let data = json!({
            "type": "assistant",
            "message": {
                "model": "claude-3-5-sonnet",
                "content": [],
                "error": "rate_limit"
            }
        });

        let msg = parse_message(data).unwrap();
        if let Message::Assistant(asst_msg) = msg {
            assert!(asst_msg.error.is_some());
        } else {
            panic!("Expected assistant message");
        }
    }

    #[test]
    fn test_parse_result_message_with_all_fields() {
        let data = json!({
            "type": "result",
            "subtype": "success",
            "duration_ms": 5000,
            "duration_api_ms": 4500,
            "is_error": false,
            "num_turns": 5,
            "session_id": "session-abc",
            "total_cost_usd": 0.123,
            "result": "Task completed successfully",
            "usage": {
                "input_tokens": 100,
                "output_tokens": 200
            }
        });

        let msg = parse_message(data).unwrap();
        if let Message::Result(result_msg) = msg {
            assert_eq!(result_msg.session_id, "session-abc");
            assert_eq!(result_msg.num_turns, 5);
            assert_eq!(result_msg.total_cost_usd, Some(0.123));
            assert_eq!(
                result_msg.result,
                Some("Task completed successfully".to_string())
            );
            assert!(result_msg.usage.is_some());
        } else {
            panic!("Expected result message");
        }
    }

    #[test]
    fn test_parse_array_data_type() {
        let data = json!([1, 2, 3]);
        let result = parse_message(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_null_data_type() {
        let data = json!(null);
        let result = parse_message(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_content_block_text() {
        let data = json!({
            "type": "assistant",
            "message": {
                "model": "claude-3-5-sonnet",
                "content": [
                    {"type": "text", "text": "Hello, world!"}
                ]
            }
        });

        let msg = parse_message(data).unwrap();
        if let Message::Assistant(asst_msg) = msg {
            assert_eq!(asst_msg.content.len(), 1);
            assert!(asst_msg.content[0].is_text());
            assert_eq!(asst_msg.content[0].as_text(), Some("Hello, world!"));
        } else {
            panic!("Expected assistant message");
        }
    }

    #[test]
    fn test_parse_content_block_tool_result() {
        let data = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "tool-123",
                        "content": {"result": "success"},
                        "is_error": false
                    }
                ]
            }
        });

        let msg = parse_message(data).unwrap();
        if let Message::User(user_msg) = msg {
            match user_msg.content {
                crate::types::UserMessageContent::Blocks(blocks) => {
                    assert_eq!(blocks.len(), 1);
                    assert!(blocks[0].is_tool_result());
                }
                _ => panic!("Expected blocks content"),
            }
        } else {
            panic!("Expected user message");
        }
    }
}
