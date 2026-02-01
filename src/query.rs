//! Query function for one-shot interactions with Claude Code.

use futures::{Stream, StreamExt};

use crate::error::Result;
use crate::internal::parse_message;
use crate::transport::{SubprocessCLITransport, Transport};
use crate::types::{ClaudeAgentOptions, Message};

/// Query Claude Code for one-shot or unidirectional streaming interactions.
///
/// This function is ideal for simple, stateless queries where you don't need
/// bidirectional communication or conversation management. For interactive,
/// stateful conversations, use `ClaudeSDKClient` instead.
///
/// # Key differences from ClaudeSDKClient
///
/// - **Unidirectional**: Send all messages upfront, receive all responses
/// - **Stateless**: Each query is independent, no conversation state
/// - **Simple**: Fire-and-forget style, no connection management
/// - **No interrupts**: Cannot interrupt or send follow-up messages
///
/// # When to use query()
///
/// - Simple one-off questions ("What is 2+2?")
/// - Batch processing of independent prompts
/// - Code generation or analysis tasks
/// - Automated scripts and CI/CD pipelines
/// - When you know all inputs upfront
///
/// # Arguments
///
/// * `prompt` - The prompt to send to Claude
/// * `options` - Optional configuration (defaults to `ClaudeAgentOptions::default()` if None)
///
/// # Returns
///
/// A stream of messages from the conversation.
///
/// # Example
///
/// ```no_run
/// use claude_agent_sdk::{query, ClaudeAgentOptions};
/// use futures::StreamExt;
/// use tokio::pin;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let options = ClaudeAgentOptions::builder()
///         .system_prompt("You are a helpful assistant")
///         .build();
///
///     let stream = query("What is the capital of France?", Some(options)).await?;
///     pin!(stream);
///
///     while let Some(result) = stream.next().await {
///         match result {
///             Ok(message) => println!("{:?}", message),
///             Err(e) => eprintln!("Error: {}", e),
///         }
///     }
///
///     Ok(())
/// }
/// ```
pub async fn query(
    prompt: impl Into<String>,
    options: Option<ClaudeAgentOptions>,
) -> Result<impl Stream<Item = Result<Message>>> {
    let options = options.unwrap_or_default();
    let prompt = prompt.into();

    // Create transport
    let mut transport = SubprocessCLITransport::new(prompt, options)?;
    transport.connect().await?;

    // Create message stream
    let stream = async_stream::try_stream! {
        let msg_stream = transport.read_messages();
        tokio::pin!(msg_stream);

        while let Some(result) = msg_stream.next().await {
            let data = result?;
            let message = parse_message(data)?;
            yield message;
        }
    };

    Ok(stream)
}

/// Query Claude Code with a custom transport.
///
/// This variant allows you to provide your own transport implementation,
/// useful for testing or custom communication channels.
///
/// # Arguments
///
/// * `prompt` - The prompt to send to Claude
/// * `transport` - A custom transport implementation
/// * `options` - Optional configuration
///
/// # Returns
///
/// A stream of messages from the conversation.
pub async fn query_with_transport<T: Transport + 'static>(
    _prompt: impl Into<String>,
    mut transport: T,
    _options: Option<ClaudeAgentOptions>,
) -> Result<impl Stream<Item = Result<Message>>> {
    transport.connect().await?;

    let stream = async_stream::try_stream! {
        let msg_stream = transport.read_messages();
        tokio::pin!(msg_stream);

        while let Some(result) = msg_stream.next().await {
            let data = result?;
            let message = parse_message(data)?;
            yield message;
        }
    };

    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ClaudeSDKError;
    use async_trait::async_trait;
    use serde_json::json;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    /// Mock transport for testing query functions.
    struct MockTransport {
        messages: Arc<Mutex<Vec<serde_json::Value>>>,
        connected: Arc<AtomicBool>,
        should_fail_connect: bool,
    }

    impl MockTransport {
        fn new(messages: Vec<serde_json::Value>) -> Self {
            Self {
                messages: Arc::new(Mutex::new(messages)),
                connected: Arc::new(AtomicBool::new(false)),
                should_fail_connect: false,
            }
        }

        fn failing_connect() -> Self {
            Self {
                messages: Arc::new(Mutex::new(vec![])),
                connected: Arc::new(AtomicBool::new(false)),
                should_fail_connect: true,
            }
        }
    }

    #[async_trait]
    impl Transport for MockTransport {
        async fn connect(&mut self) -> Result<()> {
            if self.should_fail_connect {
                return Err(ClaudeSDKError::CLIConnection(
                    "Mock connection failed".to_string(),
                ));
            }
            self.connected.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn write(&mut self, _data: &str) -> Result<()> {
            Ok(())
        }

        fn read_messages(
            &mut self,
        ) -> Pin<Box<dyn Stream<Item = Result<serde_json::Value>> + Send + '_>> {
            let messages = self.messages.clone();
            Box::pin(async_stream::try_stream! {
                let mut guard = messages.lock().await;
                for msg in std::mem::take(&mut *guard) {
                    yield msg;
                }
            })
        }

        async fn read_next_message(&mut self) -> Result<Option<serde_json::Value>> {
            let mut guard = self.messages.lock().await;
            if guard.is_empty() {
                Ok(None)
            } else {
                Ok(Some(guard.remove(0)))
            }
        }

        async fn close(&mut self) -> Result<()> {
            self.connected.store(false, Ordering::SeqCst);
            Ok(())
        }

        fn is_ready(&self) -> bool {
            self.connected.load(Ordering::SeqCst)
        }

        async fn end_input(&mut self) -> Result<()> {
            Ok(())
        }
    }

    // Note: These tests require the Claude CLI to be installed
    // They are marked as ignored by default

    #[tokio::test]
    #[ignore]
    async fn test_query_basic() {
        let options = ClaudeAgentOptions::builder().max_turns(1).build();

        let stream = query("Say 'hello' and nothing else", Some(options))
            .await
            .unwrap();

        let messages: Vec<_> = stream.collect().await;
        assert!(!messages.is_empty());
    }

    #[tokio::test]
    async fn test_query_with_transport_streams_messages() {
        let messages = vec![
            json!({
                "type": "assistant",
                "message": {
                    "id": "msg_1",
                    "role": "assistant",
                    "content": [{"type": "text", "text": "Hello!"}],
                    "model": "claude-3-5-sonnet",
                    "stop_reason": "end_turn"
                }
            }),
            json!({
                "type": "result",
                "result": "success",
                "session_id": "test-session-123",
                "cost_usd": 0.01,
                "tokens_in": 10,
                "tokens_out": 5,
                "duration_ms": 1000
            }),
        ];

        let transport = MockTransport::new(messages);
        let stream = query_with_transport("Hello", transport, None)
            .await
            .unwrap();
        tokio::pin!(stream);

        let mut received = Vec::new();
        while let Some(result) = stream.next().await {
            received.push(result.unwrap());
        }

        assert_eq!(received.len(), 2);
        assert!(received[0].is_assistant());
        assert!(received[1].is_result());
    }

    #[tokio::test]
    async fn test_query_with_transport_handles_empty_stream() {
        let transport = MockTransport::new(vec![]);
        let stream = query_with_transport("Hello", transport, None)
            .await
            .unwrap();
        tokio::pin!(stream);

        let messages: Vec<_> = stream.collect().await;
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_query_with_transport_connect_failure() {
        let transport = MockTransport::failing_connect();
        let result = query_with_transport("Hello", transport, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_with_transport_multiple_messages() {
        let messages = vec![
            json!({
                "type": "system",
                "subtype": "init",
                "cwd": "/test",
                "session_id": "test-session"
            }),
            json!({
                "type": "assistant",
                "message": {
                    "id": "msg_1",
                    "role": "assistant",
                    "content": [{"type": "text", "text": "Message 1"}],
                    "model": "claude-3-5-sonnet",
                    "stop_reason": null
                }
            }),
            json!({
                "type": "assistant",
                "message": {
                    "id": "msg_2",
                    "role": "assistant",
                    "content": [{"type": "text", "text": "Message 2"}],
                    "model": "claude-3-5-sonnet",
                    "stop_reason": "end_turn"
                }
            }),
            json!({
                "type": "result",
                "result": "success",
                "session_id": "test-session-123",
                "cost_usd": 0.02,
                "tokens_in": 20,
                "tokens_out": 10,
                "duration_ms": 2000
            }),
        ];

        let transport = MockTransport::new(messages);
        let stream = query_with_transport("Test", transport, None).await.unwrap();
        tokio::pin!(stream);

        let received: Vec<_> = stream.collect().await;
        assert_eq!(received.len(), 4);

        // All should be Ok
        for msg in &received {
            assert!(msg.is_ok());
        }
    }

    #[tokio::test]
    async fn test_query_with_transport_user_message() {
        let messages = vec![json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": "Test input"
            }
        })];

        let transport = MockTransport::new(messages);
        let stream = query_with_transport("Test", transport, None).await.unwrap();
        tokio::pin!(stream);

        let received: Vec<_> = stream.collect().await;
        assert_eq!(received.len(), 1);
        assert!(received[0].as_ref().unwrap().is_user());
    }

    #[tokio::test]
    async fn test_query_default_options() {
        // Test that None options uses default
        let transport = MockTransport::new(vec![json!({
            "type": "result",
            "result": "success",
            "session_id": "test-session-123",
            "cost_usd": 0.0,
            "tokens_in": 0,
            "tokens_out": 0,
            "duration_ms": 0
        })]);

        let stream = query_with_transport("Test", transport, None).await.unwrap();
        tokio::pin!(stream);

        let received: Vec<_> = stream.collect().await;
        assert_eq!(received.len(), 1);
    }

    #[tokio::test]
    async fn test_query_with_options() {
        let options = ClaudeAgentOptions::builder()
            .system_prompt("Be concise")
            .max_turns(5)
            .build();

        let transport = MockTransport::new(vec![json!({
            "type": "result",
            "result": "success",
            "session_id": "test-session-123",
            "cost_usd": 0.0,
            "tokens_in": 0,
            "tokens_out": 0,
            "duration_ms": 0
        })]);

        let stream = query_with_transport("Test", transport, Some(options))
            .await
            .unwrap();
        tokio::pin!(stream);

        let received: Vec<_> = stream.collect().await;
        assert_eq!(received.len(), 1);
    }

    #[tokio::test]
    async fn test_query_with_tool_use_message() {
        let messages = vec![json!({
            "type": "assistant",
            "message": {
                "id": "msg_1",
                "role": "assistant",
                "content": [{
                    "type": "tool_use",
                    "id": "tool_1",
                    "name": "Bash",
                    "input": {"command": "ls -la"}
                }],
                "model": "claude-3-5-sonnet",
                "stop_reason": "tool_use"
            }
        })];

        let transport = MockTransport::new(messages);
        let stream = query_with_transport("Run ls", transport, None)
            .await
            .unwrap();
        tokio::pin!(stream);

        let received: Vec<_> = stream.collect().await;
        assert_eq!(received.len(), 1);
        assert!(received[0].as_ref().unwrap().is_assistant());
    }

    #[tokio::test]
    async fn test_query_stream_ends_on_result() {
        // Verify that we can detect result messages
        let messages = vec![
            json!({
                "type": "assistant",
                "message": {
                    "id": "msg_1",
                    "role": "assistant",
                    "content": [{"type": "text", "text": "Working..."}],
                    "model": "claude-3-5-sonnet",
                    "stop_reason": null
                }
            }),
            json!({
                "type": "result",
                "result": "success",
                "session_id": "test-session-123",
                "cost_usd": 0.01,
                "tokens_in": 10,
                "tokens_out": 5,
                "duration_ms": 500
            }),
        ];

        let transport = MockTransport::new(messages);
        let stream = query_with_transport("Test", transport, None).await.unwrap();
        tokio::pin!(stream);

        let mut found_result = false;
        while let Some(result) = stream.next().await {
            if let Ok(msg) = result {
                if msg.is_result() {
                    found_result = true;
                    break;
                }
            }
        }

        assert!(found_result);
    }
}
