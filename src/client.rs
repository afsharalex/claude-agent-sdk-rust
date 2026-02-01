//! Claude SDK Client for interacting with Claude Code.

use futures::{Stream, StreamExt};
use serde_json::{json, Value};

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

        // Create query handler with callbacks from options
        let can_use_tool = self.options.can_use_tool.clone();
        let hooks = self.options.hooks.clone();
        let handler = QueryHandler::new(
            transport,
            true, // streaming mode
            can_use_tool,
            hooks,
            60, // initialize timeout
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
    ///
    /// This method handles bidirectional control protocol, processing
    /// control requests from the CLI and queuing responses. Call
    /// `flush_responses()` after consuming messages to send any
    /// queued responses back to the CLI.
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

    /// Flush any pending control responses to the CLI.
    ///
    /// Call this after consuming messages from `receive_messages()` to
    /// send any queued control responses back to the CLI.
    pub async fn flush_responses(&mut self) -> Result<()> {
        let handler = self.query_handler.as_mut().ok_or_else(|| {
            ClaudeSDKError::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        handler.flush_responses().await
    }

    /// Receive messages until a ResultMessage is received.
    ///
    /// This method consumes the message stream until it finds a ResultMessage,
    /// matching the Python SDK behavior. Returns the ResultMessage if found.
    ///
    /// After this method returns, call `flush_responses()` to send any
    /// queued control responses back to the CLI.
    ///
    /// To process individual messages as they arrive, use `receive_messages()`
    /// instead and check for `is_result()` manually.
    pub async fn receive_response(&mut self) -> Option<Result<Message>> {
        let handler = self.query_handler.as_mut()?;
        let stream = handler.receive_messages();
        tokio::pin!(stream);

        while let Some(result) = stream.next().await {
            match result {
                Ok(msg) => {
                    if msg.is_result() {
                        return Some(Ok(msg));
                    }
                    // Continue processing until we get a result
                }
                Err(e) => {
                    return Some(Err(e));
                }
            }
        }

        // Stream ended without a result message
        None
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
    use crate::types::PermissionMode;

    #[test]
    fn test_client_creation() {
        let options = ClaudeAgentOptions::builder().system_prompt("Test").build();
        let client = ClaudeSDKClient::new(options);
        assert!(!client.is_connected());
    }

    #[test]
    fn test_client_creation_with_model() {
        let options = ClaudeAgentOptions::builder()
            .model("claude-3-5-sonnet")
            .build();
        let client = ClaudeSDKClient::new(options);
        assert!(!client.is_connected());
    }

    #[test]
    fn test_client_creation_with_max_turns() {
        let options = ClaudeAgentOptions::builder().max_turns(10).build();
        let client = ClaudeSDKClient::new(options);
        assert!(!client.is_connected());
    }

    #[test]
    fn test_client_creation_with_permission_mode() {
        let options = ClaudeAgentOptions::builder()
            .permission_mode(PermissionMode::AcceptEdits)
            .build();
        let client = ClaudeSDKClient::new(options);
        assert!(!client.is_connected());
    }

    #[test]
    fn test_default_client() {
        let client = ClaudeSDKClient::default_client();
        assert!(!client.is_connected());
    }

    #[test]
    fn test_client_get_server_info_before_connect() {
        let client = ClaudeSDKClient::default_client();
        assert!(client.get_server_info().is_none());
    }

    #[test]
    fn test_client_is_connected_initially_false() {
        let client = ClaudeSDKClient::default_client();
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn test_send_message_without_connect_fails() {
        let mut client = ClaudeSDKClient::default_client();
        let result = client.send_message("Hello").await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("Not connected"));
        }
    }

    #[tokio::test]
    async fn test_send_raw_without_connect_fails() {
        let mut client = ClaudeSDKClient::default_client();
        let result = client.send_raw(json!({"type": "test"})).await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.to_string().contains("Not connected"));
        }
    }

    #[tokio::test]
    async fn test_interrupt_without_connect_fails() {
        let mut client = ClaudeSDKClient::default_client();
        let result = client.interrupt().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_permission_mode_without_connect_fails() {
        let mut client = ClaudeSDKClient::default_client();
        let result = client.set_permission_mode("acceptEdits").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_model_without_connect_fails() {
        let mut client = ClaudeSDKClient::default_client();
        let result = client
            .set_model(Some("claude-3-5-sonnet".to_string()))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rewind_files_without_connect_fails() {
        let mut client = ClaudeSDKClient::default_client();
        let result = client.rewind_files("msg-123").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_mcp_status_without_connect_fails() {
        let mut client = ClaudeSDKClient::default_client();
        let result = client.get_mcp_status().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_flush_responses_without_connect_fails() {
        let mut client = ClaudeSDKClient::default_client();
        let result = client.flush_responses().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_disconnect_without_connect_succeeds() {
        let mut client = ClaudeSDKClient::default_client();
        // Disconnect without connecting should succeed (no-op)
        let result = client.disconnect().await;
        assert!(result.is_ok());
        assert!(!client.is_connected());
    }

    #[tokio::test]
    async fn test_receive_response_without_connect_returns_none() {
        let mut client = ClaudeSDKClient::default_client();
        let result = client.receive_response().await;
        assert!(result.is_none());
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

    #[tokio::test]
    #[ignore]
    async fn test_client_send_and_receive() {
        let options = ClaudeAgentOptions::builder()
            .max_turns(1)
            .system_prompt("Reply with only 'OK'")
            .build();

        let mut client = ClaudeSDKClient::new(options);
        client.connect().await.unwrap();

        client.send_message("Say OK").await.unwrap();

        let response = client.receive_response().await;
        assert!(response.is_some());

        client.disconnect().await.unwrap();
    }
}
