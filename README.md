# Claude Agent SDK for Rust

Rust SDK for Claude Agent. This SDK provides a native Rust interface for interacting with Claude Code.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
claude-agent-sdk = "0.1"
tokio = { version = "1", features = ["full"] }
futures = "0.3"
```

**Prerequisites:**

- Rust 1.70+
- Claude Code CLI installed: `curl -fsSL https://claude.ai/install.sh | bash`

## Quick Start

```rust
use claude_agent_sdk::{query, ClaudeAgentOptions, Message};
use futures::StreamExt;
use tokio::pin;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = ClaudeAgentOptions::builder()
        .system_prompt("You are a helpful assistant")
        .max_turns(1)
        .build();

    let stream = query("What is 2 + 2?", Some(options)).await?;
    pin!(stream);

    while let Some(result) = stream.next().await {
        match result {
            Ok(message) => match &message {
                Message::Assistant(assistant) => {
                    println!("{}", assistant.text());
                }
                Message::Result(result) => {
                    if let Some(cost) = result.total_cost_usd {
                        println!("Cost: ${:.4}", cost);
                    }
                    break;
                }
                _ => {}
            },
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}
```

## Basic Usage: query()

`query()` is a function for one-shot queries to Claude Code. It returns a stream of messages.

```rust
use claude_agent_sdk::{query, ClaudeAgentOptions};
use futures::StreamExt;
use tokio::pin;

// Simple query
let stream = query("Hello Claude", None).await?;

// With options
let options = ClaudeAgentOptions::builder()
    .system_prompt("You are a helpful assistant")
    .max_turns(1)
    .build();

let stream = query("Tell me a joke", Some(options)).await?;
```

### Using Tools

```rust
let options = ClaudeAgentOptions::builder()
    .tools(vec!["Read".into(), "Write".into(), "Bash".into()])
    .permission_mode(PermissionMode::AcceptEdits)
    .build();

let stream = query("Create a hello.rs file", Some(options)).await?;
```

### Working Directory

```rust
let options = ClaudeAgentOptions::builder()
    .cwd("/path/to/project")
    .build();

let stream = query("What files are here?", Some(options)).await?;
```

## Client

`ClaudeSDKClient` supports bidirectional, interactive conversations with Claude Code.

Unlike `query()`, `ClaudeSDKClient` additionally enables **permission callbacks** and **hooks**, both defined as Rust functions.

```rust
use claude_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
use futures::StreamExt;
use tokio::pin;

let options = ClaudeAgentOptions::builder()
    .cwd("/path/to/project")
    .model("claude-sonnet-4-5")
    .build();

let mut client = ClaudeSDKClient::new(options);
client.connect().await?;

client.send_message("Help me understand this codebase").await?;

{
    let messages = client.receive_messages();
    pin!(messages);

    while let Some(result) = messages.next().await {
        let msg = result?;
        if msg.is_result() {
            break;
        }
        // Handle messages...
    }
}

client.disconnect().await?;
```

### Permission Callbacks

For fine-grained tool permission control:

```rust
use std::sync::Arc;
use claude_agent_sdk::{
    ClaudeAgentOptions, CanUseToolFn, PermissionResult,
    PermissionResultAllow, PermissionResultDeny,
};

let can_use_tool: CanUseToolFn = Arc::new(|tool_name, input, _ctx| {
    Box::pin(async move {
        if tool_name == "Bash" {
            if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
                if cmd.contains("rm -rf") {
                    return PermissionResult::Deny(PermissionResultDeny {
                        message: "Destructive commands are not allowed".into(),
                    });
                }
            }
        }
        PermissionResult::Allow(PermissionResultAllow {})
    })
});

let options = ClaudeAgentOptions::builder()
    .can_use_tool(can_use_tool)
    .build();
```

### Hooks

Hooks are Rust functions that Claude Code invokes at specific points of the agent loop:

```rust
use std::collections::HashMap;
use claude_agent_sdk::{HookEvent, HookMatcher, HookCallbackFn};

let pre_tool_hook: HookCallbackFn = Arc::new(|input, tool_use_id, ctx| {
    Box::pin(async move {
        println!("Tool being used: {}", tool_use_id);
        // Return hook output...
        Ok(serde_json::json!({}))
    })
});

let mut hooks = HashMap::new();
hooks.insert(HookEvent::PreToolUse, vec![
    HookMatcher::builder()
        .matcher("Bash")
        .hook(pre_tool_hook)
        .timeout(30)
        .build(),
]);

let options = ClaudeAgentOptions::builder()
    .hooks(hooks)
    .build();
```

## Types

Key types defined in this crate:

- `ClaudeAgentOptions` - Configuration options (use builder pattern)
- `AssistantMessage`, `UserMessage`, `SystemMessage`, `ResultMessage` - Message types
- `TextBlock`, `ToolUseBlock`, `ToolResultBlock`, `ThinkingBlock` - Content blocks
- `HookEvent`, `HookMatcher`, `HookCallbackFn` - Hook types
- `PermissionResult`, `PermissionResultAllow`, `PermissionResultDeny` - Permission types

## Error Handling

```rust
use claude_agent_sdk::{query, ClaudeSDKError};

let stream = query("Hello", None).await;

match stream {
    Ok(s) => { /* process stream */ }
    Err(e) => match e {
        ClaudeSDKError::CLINotFound { .. } => {
            println!("Please install Claude Code");
        }
        ClaudeSDKError::CLIConnection(msg) => {
            println!("Connection failed: {}", msg);
        }
        ClaudeSDKError::Process { exit_code, .. } => {
            println!("Process failed with exit code: {:?}", exit_code);
        }
        ClaudeSDKError::MessageParse { message, .. } => {
            println!("Failed to parse response: {}", message);
        }
        _ => println!("Error: {}", e),
    }
}
```

## Available Options

| Option | Description |
|--------|-------------|
| `.cwd(path)` | Set working directory |
| `.model(model)` | Set AI model |
| `.system_prompt(prompt)` | Set system prompt |
| `.max_turns(n)` | Limit conversation turns |
| `.max_budget_usd(amount)` | Set cost budget |
| `.permission_mode(mode)` | Set permission mode |
| `.tools(tools)` | Specify allowed tools |
| `.disallowed_tools(tools)` | Specify disallowed tools |
| `.mcp_servers(servers)` | Configure MCP servers |
| `.hooks(hooks)` | Register hooks |
| `.can_use_tool(callback)` | Set permission callback |
| `.sandbox(settings)` | Configure sandbox |
| `.agents(agents)` | Define subagents |
| `.env(env)` | Set environment variables |

See `src/types/config.rs` for all available options.

## Examples

See the `examples/` directory for complete working examples:

- [examples/quick_start.rs](examples/quick_start.rs) - Basic one-shot query

Run an example:

```bash
cargo run --example quick_start
```

## Feature Parity with Python SDK

This Rust SDK implements feature parity with the [Python Claude Agent SDK](https://github.com/anthropics/claude-agent-sdk-python), with Rust-idiomatic adaptations:

| Python | Rust |
|--------|------|
| `async with client` | `client.connect()` / `client.disconnect()` |
| `async for message` | `while let Some(msg) = stream.next().await` |
| `snake_case` | `snake_case` (same) |
| `@dataclass` | Rust structs with `#[derive(Serialize, Deserialize)]` |
| `Optional[T]` | `Option<T>` |
| `Callable[..., Awaitable[T]]` | `Arc<dyn Fn(...) -> Pin<Box<dyn Future<Output = T>>>>` |

## Attribution

This project is a Rust implementation inspired by Anthropic's [Claude Agent SDK for Python](https://github.com/anthropics/claude-agent-sdk-python). The Python SDK served as the reference implementation for the API design and feature set.

## License

MIT License - see [LICENSE](LICENSE) for details.

This project is a community-developed Rust port. The original [Python SDK](https://github.com/anthropics/claude-agent-sdk-python) by Anthropic is also MIT licensed.

**Note:** Use of Claude and Claude Code is subject to Anthropic's [Terms of Service](https://www.anthropic.com/legal/consumer-terms).
