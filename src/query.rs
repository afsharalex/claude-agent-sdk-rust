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
}
