//! Quick start example for Claude Agent SDK.
//!
//! This example demonstrates basic usage of the query() function
//! for one-shot interactions with Claude Code.

use claude_agent_sdk::{query, ClaudeAgentOptions, Message};
use futures::StreamExt;
use tokio::pin;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure options for the query
    let options = ClaudeAgentOptions::builder()
        .system_prompt("You are a helpful assistant. Be concise.")
        .max_turns(1)
        .build();

    // Send a simple query
    println!("Sending query to Claude Code...");
    let stream = query(
        "What is 2 + 2? Just respond with the number.",
        Some(options),
    )
    .await?;

    // Pin the stream before iterating (required for async_stream)
    pin!(stream);

    // Process the stream of messages
    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => match &message {
                Message::Assistant(assistant) => {
                    println!("Assistant: {}", assistant.text());
                }
                Message::Result(result) => {
                    println!("\n--- Query Complete ---");
                    println!("Duration: {}ms", result.duration_ms);
                    if let Some(cost) = result.total_cost_usd {
                        println!("Cost: ${:.4}", cost);
                    }
                    break;
                }
                Message::System(system) => {
                    println!("System: {:?}", system.subtype);
                }
                _ => {}
            },
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
