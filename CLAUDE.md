# Claude Agent SDK for Rust

A Rust SDK for interacting with the Claude Code CLI, providing both one-shot queries and interactive, bidirectional conversations.

## Quick Reference

```bash
just ci          # Run all CI checks (fmt, clippy, test)
just test        # Run tests
just fmt         # Format code
just clippy      # Run linter
just docs        # Generate and open documentation
```

## Project Structure

```
src/
├── lib.rs                    # Public API exports
├── client.rs                 # ClaudeSDKClient for interactive conversations
├── query.rs                  # One-shot query() function
├── error.rs                  # Error types (ClaudeSDKError, Result)
├── internal/
│   ├── mod.rs
│   ├── message_parser.rs     # JSON message parsing
│   └── query_handler.rs      # Bidirectional control protocol
├── transport/
│   ├── mod.rs
│   └── subprocess.rs         # CLI subprocess transport
└── types/
    ├── mod.rs
    ├── config.rs             # ClaudeAgentOptions, callbacks
    ├── message.rs            # Message enum and variants
    ├── content.rs            # ContentBlock types
    ├── control.rs            # Control protocol types
    ├── permission.rs         # Permission types
    ├── hook.rs               # Hook types and callbacks
    ├── mcp.rs                # MCP server configuration
    └── sandbox.rs            # Sandbox settings
```

## Architecture

### Two Usage Patterns

1. **One-shot queries** (`query()` function): Simple, stateless, fire-and-forget
2. **Interactive client** (`ClaudeSDKClient`): Multi-turn, bidirectional, with control protocol

### Key Components

- **Transport**: Subprocess transport spawns Claude CLI and communicates via stdin/stdout
- **QueryHandler**: Manages bidirectional control protocol, handles incoming control requests
- **Message types**: Strongly typed messages (User, Assistant, System, Result, StreamEvent)

### Control Protocol

The SDK implements a bidirectional control protocol:
- **Outgoing**: `control_request` messages for interrupts, permission changes, MCP status
- **Incoming**: `control_request` messages from CLI for `can_use_tool` and hook callbacks
- Response queue pattern: collect responses during message streaming, flush after

## Key Types

```rust
// Configuration
ClaudeAgentOptions::builder()
    .system_prompt("...")
    .model("claude-3-5-sonnet")
    .max_turns(10)
    .permission_mode(PermissionMode::AcceptEdits)
    .can_use_tool(callback)  // Tool permission callback
    .hooks(HashMap::new())    // Hook callbacks
    .build();

// Messages
Message::User(UserMessage)
Message::Assistant(AssistantMessage)
Message::System(SystemMessage)
Message::Result(ResultMessage)
Message::StreamEvent(StreamEvent)

// Content blocks
ContentBlock::Text(TextBlock)
ContentBlock::ToolUse(ToolUseBlock)
ContentBlock::ToolResult(ToolResultBlock)
ContentBlock::Thinking(ThinkingBlock)
```

## Callback Types

Async callbacks use this pattern:
```rust
pub type CanUseToolFn = Arc<
    dyn Fn(String, Value, ToolPermissionContext)
        -> Pin<Box<dyn Future<Output = PermissionResult> + Send>>
        + Send + Sync,
>;
```

## Testing

### Running Tests

```bash
just test              # All tests
just test-one <name>   # Specific test
just test-verbose      # With output
```

### Test Categories

- **Unit tests**: In-module `#[cfg(test)]` blocks
- **Integration tests**: Marked `#[ignore]` (require Claude CLI installed)
- **Mock transport**: `MockTransport` for testing without CLI

### Adding Tests

Tests use `MockTransport` to simulate CLI responses:
```rust
let messages = vec![
    json!({
        "type": "assistant",
        "message": { "id": "msg_1", "role": "assistant", ... }
    }),
    json!({
        "type": "result",
        "result": "success",
        "session_id": "test-session",
        ...
    }),
];
let transport = MockTransport::new(messages);
```

## Common Patterns

### Error Handling

```rust
use crate::error::{ClaudeSDKError, Result};

// All public functions return Result<T>
pub async fn some_operation() -> Result<T> {
    // Use ? operator freely
    let value = fallible_operation()?;
    Ok(value)
}
```

### Streaming with async_stream

```rust
use async_stream::try_stream;

fn receive_messages(&mut self) -> impl Stream<Item = Result<Message>> + '_ {
    try_stream! {
        while let Some(result) = stream.next().await {
            let msg = result?;
            yield msg;
        }
    }
}
```

### Builder Pattern

All option structs use builders:
```rust
ClaudeAgentOptions::builder()
    .field1(value1)
    .field2(value2)
    .build()
```

## Code Style

- Run `cargo fmt` before committing
- All clippy warnings are errors (`-D warnings`)
- Use `Result<T>` from `crate::error`, not `std::result::Result`
- Prefer `impl Into<String>` for string parameters
- Document public items with `///` doc comments

## Dependencies

Key dependencies:
- `tokio`: Async runtime with process, sync, time features
- `serde`/`serde_json`: Serialization
- `futures`: Stream traits and utilities
- `async-stream`: `try_stream!` macro for generators
- `async-trait`: Async trait methods
- `thiserror`: Error derive macros

## Debugging

### Enable Tracing

The SDK uses `tracing` for logging. Enable in your app:
```rust
tracing_subscriber::fmt::init();
```

### Common Issues

1. **"Not connected" errors**: Call `client.connect().await` before operations
2. **Stream borrow errors**: Use scoped blocks or `pin!()` macro
3. **Missing fields in tests**: Result messages need `session_id`, `duration_ms`

## Reference Implementation

This SDK mirrors the Python Claude Agent SDK. Key files for reference:
- Python `ClaudeAPIClient` → Rust `ClaudeSDKClient`
- Python `query()` → Rust `query()`
- Python `ClaudeAgentOptions` → Rust `ClaudeAgentOptions`
