//! Content block types for Claude SDK messages.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Text content block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextBlock {
    pub text: String,
}

impl TextBlock {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

/// Thinking content block (Claude's internal reasoning).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingBlock {
    pub thinking: String,
    pub signature: String,
}

impl ThinkingBlock {
    pub fn new(thinking: impl Into<String>, signature: impl Into<String>) -> Self {
        Self {
            thinking: thinking.into(),
            signature: signature.into(),
        }
    }
}

/// Tool use content block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolUseBlock {
    pub id: String,
    pub name: String,
    pub input: Value,
}

impl ToolUseBlock {
    pub fn new(id: impl Into<String>, name: impl Into<String>, input: Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            input,
        }
    }
}

/// Tool result content block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResultBlock {
    pub tool_use_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolResultBlock {
    pub fn new(tool_use_id: impl Into<String>) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            content: None,
            is_error: None,
        }
    }

    pub fn with_content(mut self, content: Value) -> Self {
        self.content = Some(content);
        self
    }

    pub fn with_error(mut self, is_error: bool) -> Self {
        self.is_error = Some(is_error);
        self
    }
}

/// Content block enum representing all possible content types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
        signature: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

impl ContentBlock {
    /// Create a new text content block.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create a new thinking content block.
    pub fn thinking(thinking: impl Into<String>, signature: impl Into<String>) -> Self {
        Self::Thinking {
            thinking: thinking.into(),
            signature: signature.into(),
        }
    }

    /// Create a new tool use content block.
    pub fn tool_use(id: impl Into<String>, name: impl Into<String>, input: Value) -> Self {
        Self::ToolUse {
            id: id.into(),
            name: name.into(),
            input,
        }
    }

    /// Create a new tool result content block.
    pub fn tool_result(
        tool_use_id: impl Into<String>,
        content: Option<Value>,
        is_error: Option<bool>,
    ) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content,
            is_error,
        }
    }

    /// Returns true if this is a text block.
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text { .. })
    }

    /// Returns true if this is a thinking block.
    pub fn is_thinking(&self) -> bool {
        matches!(self, Self::Thinking { .. })
    }

    /// Returns true if this is a tool use block.
    pub fn is_tool_use(&self) -> bool {
        matches!(self, Self::ToolUse { .. })
    }

    /// Returns true if this is a tool result block.
    pub fn is_tool_result(&self) -> bool {
        matches!(self, Self::ToolResult { .. })
    }

    /// Get the text content if this is a text block.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }
}

impl From<TextBlock> for ContentBlock {
    fn from(block: TextBlock) -> Self {
        Self::Text { text: block.text }
    }
}

impl From<ThinkingBlock> for ContentBlock {
    fn from(block: ThinkingBlock) -> Self {
        Self::Thinking {
            thinking: block.thinking,
            signature: block.signature,
        }
    }
}

impl From<ToolUseBlock> for ContentBlock {
    fn from(block: ToolUseBlock) -> Self {
        Self::ToolUse {
            id: block.id,
            name: block.name,
            input: block.input,
        }
    }
}

impl From<ToolResultBlock> for ContentBlock {
    fn from(block: ToolResultBlock) -> Self {
        Self::ToolResult {
            tool_use_id: block.tool_use_id,
            content: block.content,
            is_error: block.is_error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_text_block_serde() {
        let block = ContentBlock::text("Hello, world!");
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello, world!\""));

        let parsed: ContentBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, block);
    }

    #[test]
    fn test_thinking_block_serde() {
        let block = ContentBlock::thinking("Let me think...", "sig123");
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"thinking\""));

        let parsed: ContentBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, block);
    }

    #[test]
    fn test_tool_use_block_serde() {
        let block = ContentBlock::tool_use("id1", "Bash", json!({"command": "ls"}));
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"tool_use\""));

        let parsed: ContentBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, block);
    }

    #[test]
    fn test_tool_result_block_serde() {
        let block = ContentBlock::tool_result("id1", Some(json!("output")), Some(false));
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"tool_result\""));

        let parsed: ContentBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, block);
    }

    #[test]
    fn test_content_block_helpers() {
        let text = ContentBlock::text("test");
        assert!(text.is_text());
        assert!(!text.is_thinking());
        assert_eq!(text.as_text(), Some("test"));

        let thinking = ContentBlock::thinking("thought", "sig");
        assert!(thinking.is_thinking());
        assert_eq!(thinking.as_text(), None);

        let tool_use = ContentBlock::tool_use("id", "name", json!({}));
        assert!(tool_use.is_tool_use());

        let tool_result = ContentBlock::tool_result("id", None, None);
        assert!(tool_result.is_tool_result());
    }

    #[test]
    fn test_from_conversions() {
        let text_block = TextBlock::new("hello");
        let content: ContentBlock = text_block.into();
        assert!(content.is_text());

        let thinking_block = ThinkingBlock::new("thought", "sig");
        let content: ContentBlock = thinking_block.into();
        assert!(content.is_thinking());
    }

    #[test]
    fn test_tool_use_block_from_conversion() {
        let tool_use_block = ToolUseBlock::new("tool-123", "Bash", json!({"command": "ls"}));
        let content: ContentBlock = tool_use_block.into();
        assert!(content.is_tool_use());
    }

    #[test]
    fn test_tool_result_block_from_conversion() {
        let tool_result_block = ToolResultBlock::new("tool-123")
            .with_content(json!("output"))
            .with_error(false);
        let content: ContentBlock = tool_result_block.into();
        assert!(content.is_tool_result());
    }

    #[test]
    fn test_text_block_new() {
        let block = TextBlock::new("Hello");
        assert_eq!(block.text, "Hello");
    }

    #[test]
    fn test_thinking_block_new() {
        let block = ThinkingBlock::new("I'm thinking", "signature123");
        assert_eq!(block.thinking, "I'm thinking");
        assert_eq!(block.signature, "signature123");
    }

    #[test]
    fn test_tool_use_block_new() {
        let block = ToolUseBlock::new("id-1", "Bash", json!({"cmd": "ls"}));
        assert_eq!(block.id, "id-1");
        assert_eq!(block.name, "Bash");
        assert_eq!(block.input["cmd"], "ls");
    }

    #[test]
    fn test_tool_result_block_builder() {
        let block = ToolResultBlock::new("tool-123");
        assert_eq!(block.tool_use_id, "tool-123");
        assert!(block.content.is_none());
        assert!(block.is_error.is_none());

        let block_with_content = ToolResultBlock::new("tool-456").with_content(json!("result"));
        assert!(block_with_content.content.is_some());

        let block_with_error = ToolResultBlock::new("tool-789").with_error(true);
        assert_eq!(block_with_error.is_error, Some(true));
    }

    #[test]
    fn test_content_block_all_types_not_text() {
        let thinking = ContentBlock::thinking("t", "s");
        assert!(!thinking.is_text());
        assert!(!thinking.is_tool_use());
        assert!(!thinking.is_tool_result());
        assert!(thinking.as_text().is_none());

        let tool_use = ContentBlock::tool_use("id", "name", json!({}));
        assert!(!tool_use.is_text());
        assert!(!tool_use.is_thinking());
        assert!(!tool_use.is_tool_result());

        let tool_result = ContentBlock::tool_result("id", None, None);
        assert!(!tool_result.is_text());
        assert!(!tool_result.is_thinking());
        assert!(!tool_result.is_tool_use());
    }

    #[test]
    fn test_content_block_serialization_roundtrip() {
        let blocks = vec![
            ContentBlock::text("Hello"),
            ContentBlock::thinking("Hmm", "sig"),
            ContentBlock::tool_use("id1", "Bash", json!({})),
            ContentBlock::tool_result("id1", Some(json!("done")), Some(false)),
        ];

        for block in blocks {
            let json = serde_json::to_string(&block).unwrap();
            let parsed: ContentBlock = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, block);
        }
    }

    #[test]
    fn test_tool_result_with_none_values() {
        let block = ContentBlock::tool_result("id1", None, None);
        let json = serde_json::to_string(&block).unwrap();
        // content and is_error should not be in the JSON when None
        assert!(!json.contains("\"content\""));
        assert!(!json.contains("\"is_error\""));
    }
}
