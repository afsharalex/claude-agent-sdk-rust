//! # Claude Agent SDK for Rust
//!
//! A Rust SDK for interacting with the Claude Code CLI, providing both
//! one-shot queries and interactive, bidirectional conversations.
//!
//! ## Quick Start
//!
//! ### One-shot Query
//!
//! Use the `query()` function for simple, stateless queries:
//!
//! ```no_run
//! use claude_agent_sdk::{query, ClaudeAgentOptions};
//! use futures::StreamExt;
//! use tokio::pin;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let options = ClaudeAgentOptions::builder()
//!         .system_prompt("You are a helpful assistant")
//!         .build();
//!
//!     let stream = query("What is 2 + 2?", Some(options)).await?;
//!     pin!(stream);
//!
//!     while let Some(result) = stream.next().await {
//!         if let Ok(msg) = result {
//!             if let Some(assistant) = msg.as_assistant() {
//!                 println!("{}", assistant.text());
//!             }
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Interactive Client
//!
//! Use `ClaudeSDKClient` for multi-turn conversations:
//!
//! ```no_run
//! use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
//! use futures::StreamExt;
//! use tokio::pin;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let options = ClaudeAgentOptions::builder()
//!         .system_prompt("You are helpful")
//!         .build();
//!
//!     let mut client = ClaudeSDKClient::new(options);
//!     client.connect().await?;
//!
//!     client.send_message("Hello!").await?;
//!
//!     // Receive messages in a scope to release borrow before disconnect
//!     {
//!         let messages = client.receive_messages();
//!         pin!(messages);
//!         while let Some(result) = messages.next().await {
//!             let msg = result?;
//!             if msg.is_result() {
//!                 break;
//!             }
//!             println!("{:?}", msg);
//!         }
//!     }
//!
//!     client.disconnect().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - **One-shot queries**: Simple `query()` function for fire-and-forget interactions
//! - **Interactive client**: `ClaudeSDKClient` for multi-turn conversations
//! - **Streaming**: Full support for streaming responses
//! - **Control protocol**: Interrupt, change permissions, and more during conversations
//! - **Type safety**: Strongly typed messages and configurations
//! - **Async/await**: Built on tokio for efficient async operations
//!
//! ## Module Organization
//!
//! - [`error`]: Error types and result aliases
//! - [`types`]: All type definitions (messages, configurations, etc.)
//! - [`transport`]: Transport layer for CLI communication
//! - [`query`]: One-shot query function
//! - [`client`]: Interactive client for conversations

#![allow(missing_docs)]
#![warn(clippy::all)]

pub mod client;
pub mod error;
pub(crate) mod internal;
pub mod query;
pub mod transport;
pub mod types;

// Re-export main types at crate root for convenience
pub use client::ClaudeSDKClient;
pub use error::{ClaudeSDKError, Result};
pub use query::query;
pub use types::{
    // Config
    AgentDefinition,
    // Messages
    AssistantMessage,
    AssistantMessageError,
    ClaudeAgentOptions,
    ClaudeAgentOptionsBuilder,
    // Content
    ContentBlock,
    // Control
    ControlResponseVariant,
    // Hooks
    HookContext,
    HookEvent,
    HookInput,
    HookJSONOutput,
    HookMatcher,
    HookPermissionDecision,
    HookSpecificOutput,
    // MCP
    McpHttpServerConfig,
    McpSSEServerConfig,
    McpSdkServerConfig,
    McpServerConfig,
    McpServers,
    McpStdioServerConfig,
    Message,
    // Permissions
    PermissionBehavior,
    PermissionMode,
    PermissionResult,
    PermissionResultAllow,
    PermissionResultDeny,
    PermissionRuleValue,
    PermissionUpdate,
    PermissionUpdateDestination,
    PermissionUpdateType,
    ResultMessage,
    SDKControlRequest,
    SDKControlRequestVariant,
    SDKControlResponse,
    // Sandbox
    SandboxIgnoreViolations,
    SandboxNetworkConfig,
    SandboxSettings,
    SdkBeta,
    SdkPluginConfig,
    SettingSource,
    StreamEvent,
    SystemMessage,
    SystemPrompt,
    SystemPromptPreset,
    TextBlock,
    ThinkingBlock,
    ToolPermissionContext,
    ToolResultBlock,
    ToolUseBlock,
    Tools,
    ToolsPreset,
    UserMessage,
    UserMessageContent,
};

// Re-export transport trait
pub use transport::Transport;

/// SDK version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_builder_pattern() {
        let options = ClaudeAgentOptions::builder()
            .system_prompt("Be helpful")
            .model("claude-3-5-sonnet")
            .max_turns(10)
            .permission_mode(PermissionMode::AcceptEdits)
            .build();

        assert!(options.system_prompt.is_some());
        assert_eq!(options.model, Some("claude-3-5-sonnet".to_string()));
        assert_eq!(options.max_turns, Some(10));
        assert_eq!(options.permission_mode, Some(PermissionMode::AcceptEdits));
    }

    #[test]
    fn test_message_types() {
        let text = ContentBlock::text("Hello");
        assert!(text.is_text());
        assert_eq!(text.as_text(), Some("Hello"));
    }
}
