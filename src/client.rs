//! Claude SDK Client for interacting with Claude Code.

use futures::{Stream, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::error::{ClaudeSDKError, Result};
use crate::internal::QueryHandler;
use crate::transport::{SubprocessCLITransport, Transport};
use crate::types::{ClaudeAgentOptions, Message};

/// Client for bidirectional, interactive conversations with Claude Code.
///
/// This client provides full control over the conversation flow with support
/// for streaming, interrupts, and dynamic message sending. For simple one-shot
/// queries, consider using the `query()` function instead.
///
/// # Key features
///
/// - **Bidirectional**: Send and receive messages at any time
/// - **Stateful**: Maintains conversation context across messages
/// - **Interactive**: Send follow-ups based on responses
/// - **Control flow**: Support for interrupts and session management
///
/// # When to use ClaudeSDKClient
///
/// - Building chat interfaces or conversational UIs
/// - Interactive debugging or exploration sessions
/// - Multi-turn conversations with context
/// - When you need to react to Claude's responses
/// - Real-time applications with user input
/// - When you need interrupt capabilities
///
/// # When to use query() instead
///
/// - Simple one-off questions
/// - Batch processing of prompts
/// - Fire-and-forget automation scripts
/// - When all inputs are known upfront
/// - Stateless operations
///
/// # Example
///
/// ```no_run
/// use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions, Message};
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let options = ClaudeAgentOptions::builder()
///         .system_prompt("You are helpful")
///         .build();
///
///     let mut client = ClaudeSDKClient::new(options);
///     client.connect().await?;
///
///     client.send_message("Hello!").await?;
///
///     while let Some(result) = client.receive_response().await {
///         match result {
///             Ok(msg) => {
///                 if msg.is_result() {
///                     break;
///                 }
///                 println!("{:?}", msg);
///             }
///             Err(e) => eprintln!("Error: {}", e),
///         }
///     }
///
///     client.disconnect().await?;
///     Ok(())
/// }
/// ```
pub struct ClaudeSDKClient {
    options: ClaudeAgentOptions,
    query_handler: Option<QueryHandler>,
    connected: bool,
}

impl ClaudeSDKClient {
    /// Create a new Claude SDK client with the given options.
    pub fn new(options: ClaudeAgentOptions) -> Self {
        Self {
            options,
            query_handler: None,
            connected: false,
        }
    }

    /// Create a new client with default options.
    pub fn default_client() -> Self {
        Self::new(ClaudeAgentOptions::default())
    }

    /// Connect to Claude with optional initial prompt.
    ///
    /// If no prompt is provided, the client connects in streaming mode
    /// ready to send messages.
    pub async fn connect(&mut self) -> Result<()> {
        self.connect_with_prompt(None).await
    }

    /// Connect to Claude with an initial prompt.
    pub async fn connect_with_prompt(&mut self, prompt: Option<String>) -> Result<()> {
        if self.connected {
            return Ok(());
        }

        // Create transport
        let mut transport: Box<dyn Transport> = if let Some(prompt) = prompt {
            Box::new(SubprocessCLITransport::new(prompt, self.options.clone())?)
        } else {
            Box::new(SubprocessCLITransport::streaming(self.options.clone())?)
        };

        transport.connect().await?;

        // Create query handler
        let hooks = HashMap::new(); // TODO: Convert hooks from options
        let handler = QueryHandler::new(
            transport, true, // streaming mode
            None, // TODO: can_use_tool callback
            hooks, 60, // initialize timeout
        );

        self.query_handler = Some(handler);
        self.connected = true;

        // Initialize if needed
        if let Some(ref mut handler) = self.query_handler {
            handler.initialize().await?;
        }

        Ok(())
    }

    /// Send a text message to Claude.
    pub async fn send_message(&mut self, message: impl Into<String>) -> Result<()> {
        let handler = self.query_handler.as_mut().ok_or_else(|| {
            ClaudeSDKError::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        let msg = json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": message.into()
            },
            "parent_tool_use_id": null,
            "session_id": "default"
        });

        let json_str = serde_json::to_string(&msg)?;
        handler.write(&format!("{}\n", json_str)).await
    }

    /// Send a raw JSON message to Claude.
    pub async fn send_raw(&mut self, message: Value) -> Result<()> {
        let handler = self.query_handler.as_mut().ok_or_else(|| {
            ClaudeSDKError::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        let json_str = serde_json::to_string(&message)?;
        handler.write(&format!("{}\n", json_str)).await
    }

    /// Receive all messages from Claude.
    ///
    /// Returns a stream of messages. Use with `StreamExt::next()` or
    /// `collect()` to consume messages.
    pub fn receive_messages(&mut self) -> impl Stream<Item = Result<Message>> + '_ {
        async_stream::try_stream! {
            let handler = self.query_handler.as_mut().ok_or_else(|| {
                ClaudeSDKError::CLIConnection("Not connected. Call connect() first.".to_string())
            })?;

            let stream = handler.receive_messages();
            tokio::pin!(stream);

            while let Some(result) = stream.next().await {
                let msg: Message = result?;
                yield msg;
            }
        }
    }

    /// Receive messages until a ResultMessage is received.
    ///
    /// This is a convenience method that automatically stops after
    /// receiving the final result message.
    pub async fn receive_response(&mut self) -> Option<Result<Message>> {
        let handler = self.query_handler.as_mut()?;
        let stream = handler.receive_messages();
        tokio::pin!(stream);

        stream.next().await
    }

    /// Send interrupt signal.
    ///
    /// This attempts to stop the current operation.
    pub async fn interrupt(&mut self) -> Result<()> {
        let handler = self.query_handler.as_mut().ok_or_else(|| {
            ClaudeSDKError::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        handler.interrupt().await
    }

    /// Change permission mode during conversation.
    ///
    /// # Arguments
    ///
    /// * `mode` - The permission mode to set:
    ///   - `"default"`: CLI prompts for dangerous tools
    ///   - `"acceptEdits"`: Auto-accept file edits
    ///   - `"bypassPermissions"`: Allow all tools (use with caution)
    pub async fn set_permission_mode(&mut self, mode: &str) -> Result<()> {
        let handler = self.query_handler.as_mut().ok_or_else(|| {
            ClaudeSDKError::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        handler.set_permission_mode(mode).await
    }

    /// Change the AI model during conversation.
    ///
    /// # Arguments
    ///
    /// * `model` - The model to use, or None to use default
    pub async fn set_model(&mut self, model: Option<String>) -> Result<()> {
        let handler = self.query_handler.as_mut().ok_or_else(|| {
            ClaudeSDKError::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        handler.set_model(model).await
    }

    /// Rewind tracked files to their state at a specific user message.
    ///
    /// Requires `enable_file_checkpointing` to be set in options.
    ///
    /// # Arguments
    ///
    /// * `user_message_id` - UUID of the user message to rewind to
    pub async fn rewind_files(&mut self, user_message_id: &str) -> Result<()> {
        let handler = self.query_handler.as_mut().ok_or_else(|| {
            ClaudeSDKError::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        handler.rewind_files(user_message_id).await
    }

    /// Get current MCP server connection status.
    ///
    /// Returns a dictionary with MCP server status information.
    pub async fn get_mcp_status(&mut self) -> Result<Value> {
        let handler = self.query_handler.as_mut().ok_or_else(|| {
            ClaudeSDKError::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        handler.get_mcp_status().await
    }

    /// Get server initialization info.
    ///
    /// Returns initialization information from the Claude Code server
    /// including available commands and output styles.
    pub fn get_server_info(&self) -> Option<&Value> {
        self.query_handler
            .as_ref()
            .and_then(|h| h.initialization_result())
    }

    /// Disconnect from Claude.
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut handler) = self.query_handler.take() {
            handler.close().await?;
        }
        self.connected = false;
        Ok(())
    }

    /// Check if the client is connected.
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

impl Drop for ClaudeSDKClient {
    fn drop(&mut self) {
        // Note: We can't do async cleanup in Drop
        // Users should call disconnect() explicitly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let options = ClaudeAgentOptions::builder().system_prompt("Test").build();
        let client = ClaudeSDKClient::new(options);
        assert!(!client.is_connected());
    }

    #[test]
    fn test_default_client() {
        let client = ClaudeSDKClient::default_client();
        assert!(!client.is_connected());
    }

    // Integration tests require Claude CLI
    #[tokio::test]
    #[ignore]
    async fn test_client_connect_disconnect() {
        let options = ClaudeAgentOptions::builder().max_turns(1).build();
        let mut client = ClaudeSDKClient::new(options);

        client.connect().await.unwrap();
        assert!(client.is_connected());

        client.disconnect().await.unwrap();
        assert!(!client.is_connected());
    }
}
