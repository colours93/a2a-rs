//! # a2a-rs — Rust SDK for the Agent-to-Agent (A2A) Protocol v0.3
//!
//! This crate provides a complete Rust implementation of the
//! [A2A protocol](https://a2a-protocol.org/latest/specification/), enabling
//! AI agents to communicate with each other over JSON-RPC 2.0 with real-time
//! streaming via Server-Sent Events (SSE).
//!
//! ## Overview
//!
//! The A2A protocol allows agents to:
//! - Send messages and receive task-based responses
//! - Stream real-time status and artifact updates via SSE
//! - Maintain multi-turn conversations with context tracking
//! - Delegate tasks to other agents
//! - Cancel running tasks
//!
//! This SDK provides:
//! - **Complete type definitions** matching the A2A v0.3 protobuf specification
//! - **Client** for calling remote A2A agents ([`client::A2AClient`])
//! - **Server** framework for building A2A-compatible agents ([`server::AgentExecutor`])
//! - **Ergonomic builders** for constructing complex types ([`AgentCardBuilder`], [`ClientBuilder`], [`ServerBuilder`])
//!
//! ## Feature flags
//!
//! | Feature  | Default | Description |
//! |----------|---------|-------------|
//! | `client` | yes     | HTTP client for calling A2A agents (reqwest + SSE) |
//! | `server` | yes     | Server traits + axum integration for building agents |
//! | `full`   | no      | Enable all features |
//!
//! ## Quick Start: Client
//!
//! ```no_run
//! use a2a_rs::client::{A2AClient, SendMessageResponse};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to an A2A agent (auto-discovers endpoint)
//!     let client = A2AClient::from_url("http://localhost:7420").await?;
//!
//!     // Send a simple text message
//!     let response = client.send_text("Write a haiku about Rust").await?;
//!     match response {
//!         SendMessageResponse::Task(task) => {
//!             println!("Task: {} (status: {})", task.id, task.status.state);
//!         }
//!         SendMessageResponse::Message(msg) => {
//!             println!("Direct reply: {:?}", msg);
//!         }
//!     }
//!
//!     // Or stream responses in real-time
//!     let mut stream = client.send_text_stream("Tell me a story").await?;
//!     while let Some(event) = stream.next().await {
//!         match event? {
//!             a2a_rs::types::StreamResponse::StatusUpdate(update) => {
//!                 println!("Status: {:?}", update.status.state);
//!             }
//!             a2a_rs::types::StreamResponse::ArtifactUpdate(artifact) => {
//!                 println!("Artifact: {:?}", artifact.artifact.name);
//!             }
//!             _ => {}
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Quick Start: Server
//!
//! Implement [`server::AgentExecutor`] to define your agent's behavior:
//!
//! ```rust,ignore
//! use a2a_rs::server::{AgentExecutor, RequestContext, EventQueue, TaskUpdater};
//! use a2a_rs::types::Part;
//! use a2a_rs::error::A2AResult;
//! use async_trait::async_trait;
//!
//! struct EchoAgent;
//!
//! #[async_trait]
//! impl AgentExecutor for EchoAgent {
//!     async fn execute(
//!         &self,
//!         context: RequestContext,
//!         event_queue: EventQueue,
//!     ) -> A2AResult<()> {
//!         let updater = TaskUpdater::new(
//!             event_queue,
//!             context.task_id.clone(),
//!             context.context_id.clone(),
//!         );
//!
//!         // Extract text from the incoming message
//!         let text = context.message.parts.iter()
//!             .find_map(|p| match p {
//!                 Part::Text { text, .. } => Some(text.clone()),
//!                 _ => None,
//!             })
//!             .unwrap_or_else(|| "No text received".to_string());
//!
//!         // Echo it back
//!         let response = format!("Echo: {}", text);
//!         updater.complete(Some(&response)).await?;
//!
//!         Ok(())
//!     }
//!
//!     async fn cancel(
//!         &self,
//!         context: RequestContext,
//!         event_queue: EventQueue,
//!     ) -> A2AResult<()> {
//!         let updater = TaskUpdater::new(
//!             event_queue,
//!             context.task_id,
//!             context.context_id,
//!         );
//!         updater.cancel(None).await?;
//!         Ok(())
//!     }
//! }
//! ```
//!
//! Then set up the HTTP server:
//!
//! ```rust,ignore
//! use a2a_rs::server::{a2a_router, DefaultRequestHandler, InMemoryTaskStore};
//! use a2a_rs::AgentCardBuilder;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Build an agent card
//!     let agent_card = AgentCardBuilder::new("Echo Agent")
//!         .description("A simple agent that echoes back your messages")
//!         .version("1.0.0")
//!         .url("http://localhost:3000/a2a")
//!         .build()?;
//!
//!     let executor = Arc::new(EchoAgent);
//!     let store = Arc::new(InMemoryTaskStore::new());
//!     let handler = Arc::new(DefaultRequestHandler::new(executor, store));
//!
//!     // Build the router with A2A routes
//!     let app = a2a_router(handler, agent_card);
//!
//!     // Start the server
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
//!     axum::serve(listener, app).await?;
//!     Ok(())
//! }
//! ```
//!
//! The server automatically provides:
//! - `POST /a2a` — JSON-RPC 2.0 endpoint for all A2A methods
//! - `GET /.well-known/agent.json` — Agent card discovery
//!
//! ## Protocol Compliance
//!
//! This crate implements **A2A protocol v0.3** as defined in the
//! [official specification](https://a2a-protocol.org/latest/specification/).
//!
//! All types match the protobuf definitions at
//! [`a2a.proto`](https://github.com/a2aproject/A2A/blob/main/specification/a2a.proto).
//!
//! Supported JSON-RPC methods:
//! - `message/send` — Send a message and get a task
//! - `message/stream` — Send a message with SSE streaming
//! - `tasks/get` — Retrieve a task by ID
//! - `tasks/list` — List tasks with filtering
//! - `tasks/cancel` — Cancel a running task
//! - `tasks/subscribe` — Subscribe to task updates (SSE)
//!
//! ## Architecture
//!
//! ### Client
//!
//! - [`client::A2AClient`] — High-level client with typed methods for all A2A operations
//! - [`client::CardResolver`] — Discovers agent cards from `/.well-known/agent.json`
//! - [`client::JsonRpcTransport`] — HTTP transport with JSON-RPC 2.0 encoding
//! - [`client::SseStream`] — Server-Sent Events stream for real-time updates
//!
//! ### Server
//!
//! - [`server::AgentExecutor`] — Trait for implementing your agent's logic
//! - [`server::RequestHandler`] — Trait for handling JSON-RPC requests
//! - [`server::DefaultRequestHandler`] — Reference implementation of `RequestHandler`
//! - [`server::TaskStore`] — Trait for task persistence
//! - [`server::InMemoryTaskStore`] — In-memory task store implementation
//! - [`server::EventQueue`] — Broadcast channel for SSE events
//! - [`server::TaskUpdater`] — Helper for publishing task status/artifact updates
//! - [`server::a2a_router`] — Creates an axum `Router` with A2A routes
//!
//! ### Core Types
//!
//! - [`types::Task`] — A2A task with status, history, and artifacts
//! - [`types::Message`] — A message with text/file/data parts
//! - [`types::Part`] — Content part (text, file, or structured data)
//! - [`types::TaskState`] — Task lifecycle state machine
//! - [`types::StreamResponse`] — SSE event types (status updates, artifact updates)
//! - [`types::AgentCard`] — Agent metadata and capabilities
//! - [`error::A2AError`] — Error types with JSON-RPC error codes
//!
//! ## Examples
//!
//! See the `examples/` directory for complete, runnable examples:
//! - `echo_agent.rs` — Minimal agent that echoes messages back
//! - `hello_client.rs` — Simple client that sends a message
//! - `streaming_client.rs` — Client with SSE streaming
//! - `multi_turn.rs` — Multi-turn conversation with context tracking

pub mod builders;
pub mod error;
pub mod types;
pub mod utils;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "server")]
pub mod server;

/// Prelude module that re-exports commonly used types and traits.
///
/// Import this module with `use a2a_rs::prelude::*;` to get access to the most
/// frequently used types without having to import them individually.
///
/// # Example
///
/// ```
/// use a2a_rs::prelude::*;
///
/// // Now you have access to common types like:
/// // - Message, Part, Task, TaskState, Role
/// // - AgentCard, AgentSkill, AgentCapabilities
/// // - A2AError, A2AResult
/// // - builders like AgentCardBuilder
/// ```
pub mod prelude {
    // Core types
    pub use crate::types::{
        AgentCapabilities, AgentCard, AgentInterface, AgentSkill, Artifact, FileContent,
        FileWithBytes, FileWithUri, Message, Part, Role, SendMessageConfiguration,
        SendMessageParams, StreamResponse, Task, TaskArtifactUpdateEvent, TaskState, TaskStatus,
        TaskStatusUpdateEvent,
    };

    // Error types
    pub use crate::error::{A2AError, A2AResult};

    // Builders
    pub use crate::builders::AgentCardBuilder;

    #[cfg(feature = "client")]
    pub use crate::builders::ClientBuilder;

    #[cfg(feature = "client")]
    pub use crate::client::A2AClient;

    #[cfg(feature = "server")]
    pub use crate::builders::ServerBuilder;

    #[cfg(feature = "server")]
    pub use crate::server::{
        a2a_router, AgentExecutor, EventConsumer, EventQueue, InMemoryQueueManager,
        InMemoryTaskStore, QueueManager, RequestContext, RequestContextBuilder, ServerCallContext,
        SimpleRequestContextBuilder, TaskManager, TaskStore, TaskUpdater,
    };
}

// Re-export core types at crate root for convenience.
pub use builders::AgentCardBuilder;
pub use error::{A2AError, A2AResult};
pub use types::*;

#[cfg(feature = "client")]
pub use builders::ClientBuilder;

#[cfg(feature = "server")]
pub use builders::ServerBuilder;
