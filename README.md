# a2a-rs

[![Crates.io](https://img.shields.io/crates/v/a2a-rs.svg)](https://crates.io/crates/a2a-rs)
[![Documentation](https://docs.rs/a2a-rs/badge.svg)](https://docs.rs/a2a-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Rust SDK for the [Agent-to-Agent (A2A) protocol](https://a2a-protocol.org/) v0.3**

A2A is an open protocol for AI agents to communicate with each other over JSON-RPC 2.0, enabling agent-to-agent collaboration, task delegation, and real-time streaming of status updates and artifacts via Server-Sent Events (SSE).

This crate provides:
- **Complete type definitions** for the A2A v0.3 specification
- **Client** for calling remote A2A agents with streaming support
- **Server** framework for building your own A2A agents with axum integration
- **Ergonomic builders** and helpers for common operations

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `client` | ✅ | HTTP client for calling A2A agents (reqwest + SSE) |
| `server` | ✅ | Server traits + axum integration for building agents |
| `full` | ❌ | Enable all features |

## Installation

```toml
[dependencies]
a2a-rs = "0.1"
```

Or with specific features:

```toml
[dependencies]
# Client only
a2a-rs = { version = "0.1", default-features = false, features = ["client"] }

# Server only
a2a-rs = { version = "0.1", default-features = false, features = ["server"] }
```

## Quick Start: Client

```rust
use a2a_rs::client::A2AClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to an A2A agent (auto-discovers endpoint from agent card)
    let client = A2AClient::from_url("http://localhost:7420").await?;

    // Send a simple text message
    let task = client.send_text("Write a haiku about Rust").await?;
    println!("Task created: {} (status: {})", task.id, task.status.state);

    // Or stream responses in real-time
    let mut stream = client.send_text_stream("Tell me a story").await?;
    while let Some(event) = stream.next().await {
        match event? {
            a2a_rs::types::StreamResponse::StatusUpdate(update) => {
                println!("Status: {:?}", update.status.state);
            }
            a2a_rs::types::StreamResponse::ArtifactUpdate(artifact) => {
                println!("Artifact received: {:?}", artifact.artifact.name);
            }
            _ => {}
        }
    }

    Ok(())
}
```

### Multi-turn Conversations

Use `context_id` to maintain conversation history across multiple messages:

```rust
use uuid::Uuid;

let context_id = Uuid::new_v4().to_string();

// First message
let task1 = client.send_text_in_context(
    "My favorite color is blue",
    &context_id
).await?;

// Follow-up message in the same context
let task2 = client.send_text_in_context(
    "What's my favorite color?",
    &context_id
).await?;
```

### Working with Tasks

```rust
// Get task status
let task = client.get_task("task-123", None).await?;

// List all tasks in a context
use a2a_rs::types::{ListTasksParams, TaskState};
let response = client.list_tasks(ListTasksParams {
    context_id: Some("context-456".to_string()),
    status: Some(vec![TaskState::Completed]),
    page_size: Some(10),
    page_token: None,
}).await?;

// Cancel a running task
let cancelled_task = client.cancel_task("task-789").await?;

// Subscribe to task updates (SSE stream)
let mut updates = client.subscribe("task-123").await?;
while let Some(event) = updates.next().await {
    println!("Update: {:?}", event?);
}
```

## Quick Start: Server

Implement the `AgentExecutor` trait to define your agent's behavior:

```rust
use a2a_rs::server::{AgentExecutor, RequestContext, EventQueue, TaskUpdater};
use a2a_rs::types::Part;
use a2a_rs::error::A2AResult;
use async_trait::async_trait;

struct EchoAgent;

#[async_trait]
impl AgentExecutor for EchoAgent {
    async fn execute(
        &self,
        context: RequestContext,
        event_queue: EventQueue,
    ) -> A2AResult<()> {
        let updater = TaskUpdater::new(
            event_queue,
            context.task_id.clone(),
            context.context_id.clone(),
        );

        // Extract text from the incoming message
        let text = context.message.parts.iter()
            .find_map(|p| match p {
                Part::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "No text received".to_string());

        // Echo it back
        let response = format!("Echo: {}", text);
        updater.complete(Some(&response)).await?;

        Ok(())
    }

    async fn cancel(
        &self,
        context: RequestContext,
        event_queue: EventQueue,
    ) -> A2AResult<()> {
        let updater = TaskUpdater::new(
            event_queue,
            context.task_id,
            context.context_id,
        );
        updater.cancel(None).await?;
        Ok(())
    }
}
```

Then set up the HTTP server with axum:

```rust
use a2a_rs::server::{a2a_router, DefaultRequestHandler, InMemoryTaskStore};
use a2a_rs::types::{AgentCard, AgentInterface, AgentSkill};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the agent card
    let agent_card = AgentCard {
        name: "Echo Agent".to_string(),
        description: "A simple agent that echoes back your messages".to_string(),
        version: "1.0.0".to_string(),
        supported_interfaces: vec![AgentInterface {
            url: "http://localhost:3000/a2a".to_string(),
            protocol_binding: "JSONRPC".to_string(),
            protocol_version: "0.3".to_string(),
            tenant: None,
        }],
        capabilities: vec!["text".to_string()],
        default_input_modes: vec!["text".to_string()],
        default_output_modes: vec!["text".to_string()],
        skills: vec![AgentSkill {
            id: "echo".to_string(),
            name: "Echo".to_string(),
            description: "Echoes back your message".to_string(),
            tags: vec!["demo".to_string()],
            examples: None,
            input_modes: None,
            output_modes: None,
            security_requirements: None,
        }],
        provider: None,
        documentation_url: None,
        security_schemes: None,
        security_requirements: None,
        signatures: None,
        icon_url: None,
    };

    // Create the executor and task store
    let executor = Arc::new(EchoAgent);
    let store = Arc::new(InMemoryTaskStore::new());

    // Create the request handler
    let handler = Arc::new(DefaultRequestHandler::new(executor, store));

    // Build the router
    let app = a2a_router(handler, agent_card);

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("A2A agent listening on http://localhost:3000");
    println!("Agent card: http://localhost:3000/.well-known/agent.json");
    axum::serve(listener, app).await?;

    Ok(())
}
```

The server automatically provides:
- `POST /a2a` — JSON-RPC 2.0 endpoint for all A2A methods
- `GET /.well-known/agent.json` — Agent card discovery

## Architecture

### Client Architecture

```
A2AClient
  ├─ CardResolver      → Discovers agent cards from /.well-known/agent.json
  ├─ JsonRpcTransport  → HTTP transport with JSON-RPC 2.0 encoding
  └─ SseStream         → Server-Sent Events stream for real-time updates
```

The client handles:
- Agent card resolution and endpoint discovery
- JSON-RPC 2.0 request/response serialization
- SSE stream parsing for `message/stream` and `tasks/subscribe`
- Error mapping from JSON-RPC error codes to `A2AError`

### Server Architecture

```
axum Router
  ├─ handle_jsonrpc         → JSON-RPC 2.0 dispatch
  │   ├─ message/send       → Synchronous message processing
  │   ├─ message/stream     → Streaming message processing (SSE)
  │   ├─ tasks/get          → Retrieve task by ID
  │   ├─ tasks/list         → List tasks with filtering
  │   ├─ tasks/cancel       → Cancel a task
  │   └─ tasks/subscribe    → Subscribe to task updates (SSE)
  │
  └─ handle_agent_card      → Serve agent card at /.well-known/agent.json

RequestHandler (trait)
  └─ DefaultRequestHandler
      ├─ AgentExecutor    → Your agent logic (trait)
      ├─ TaskStore        → Task persistence (InMemoryTaskStore or custom)
      └─ EventQueue       → Broadcast channel for SSE events
```

You implement `AgentExecutor` to define your agent's behavior. The framework handles:
- JSON-RPC request parsing and validation
- Task lifecycle management (submitted → working → completed/failed)
- SSE event broadcasting to connected clients
- Multi-turn conversation context tracking

## Comparison to Python/JS SDKs

This SDK mirrors the official Python SDK (`a2a-python`) and JS SDK (`@a2a-js/sdk`) architecture:

| Component | Python | JavaScript | Rust (this crate) |
|-----------|--------|------------|-------------------|
| Client | `Client` / `BaseClient` | `A2AClient` | `A2AClient` |
| Server executor | `AgentExecutor(ABC)` | `AgentExecutor` interface | `AgentExecutor` trait |
| Task store | `TaskStore(ABC)` | `TaskStore` | `TaskStore` trait |
| Request handler | `RequestHandler` | `RequestHandler` | `RequestHandler` trait |
| Axum integration | Flask/FastAPI | Express/Hono | axum |

Key differences:
- **Type safety**: Full compile-time type checking via serde
- **Async/await**: Built on tokio for high-performance concurrent I/O
- **Zero-copy parsing**: Efficient JSON parsing with serde_json
- **Trait-based**: Extensible via traits instead of inheritance

## Examples

See the `examples/` directory for complete, runnable examples:
- `echo_agent.rs` — Minimal agent that echoes messages back
- `hello_client.rs` — Simple client that sends a message and prints the result
- `streaming_client.rs` — Client with SSE streaming
- `multi_turn.rs` — Multi-turn conversation with context tracking

Run an example:

```bash
cargo run --example echo_agent
```

## Protocol Compliance

This crate implements **A2A protocol v0.3** as defined in the [official specification](https://a2a-protocol.org/latest/specification/).

All types match the protobuf definitions at [`a2a.proto`](https://github.com/a2aproject/A2A/blob/main/specification/a2a.proto).

Supported JSON-RPC methods:
- `message/send` — Send a message and get a task
- `message/stream` — Send a message with SSE streaming
- `tasks/get` — Retrieve a task by ID
- `tasks/list` — List tasks with filtering
- `tasks/cancel` — Cancel a running task
- `tasks/subscribe` — Subscribe to task updates (SSE)

Error codes match the A2A specification:
- `-32001` — TaskNotFoundError
- `-32002` — TaskNotCancelableError
- `-32003` — PushNotificationNotSupportedError
- `-32004` — UnsupportedOperationError
- `-32005` — ContentTypeNotSupportedError
- `-32600` to `-32700` — Standard JSON-RPC errors

## License

MIT

## Contributing

Contributions welcome! Please ensure all code matches the official A2A v0.3 specification.

When adding new types, verify against:
- [Official protobuf spec](https://github.com/a2aproject/A2A/blob/main/specification/a2a.proto)
- [Python SDK](https://github.com/a2aproject/a2a-python) for reference behavior
- [JavaScript SDK](https://github.com/a2aproject/a2a-js) for JSON-RPC serialization patterns

## Resources

- [A2A Protocol Website](https://a2a-protocol.org/)
- [A2A Specification](https://a2a-protocol.org/latest/specification/)
- [Official GitHub](https://github.com/a2aproject/A2A)
- [Python SDK](https://github.com/a2aproject/a2a-python)
- [JavaScript SDK](https://github.com/a2aproject/a2a-js)
