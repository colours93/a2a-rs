//! A2A client — call remote A2A agents.
//!
//! Mirrors the Python SDK's client architecture:
//!
//! - [`A2AClient`] — high-level client with typed methods for every A2A
//!   JSON-RPC operation (send messages, get/cancel tasks, subscribe to streams)
//! - [`CardResolver`] — discover agent cards via the well-known URL convention
//! - [`Transport`] / [`JsonRpcTransport`] — pluggable transport layer
//! - [`SseStream`] — parsed SSE event stream for streaming responses
//!
//! # Quick Start
//!
//! ```no_run
//! use a2a_rs::client::{A2AClient, SendMessageResponse};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to an agent (resolves agent card automatically):
//! let client = A2AClient::from_url("http://localhost:7420").await?;
//!
//! // Send a text message:
//! let response = client.send_text("Hello, agent!").await?;
//! match response {
//!     SendMessageResponse::Task(task) => {
//!         println!("Task {} — status: {}", task.id, task.status.state);
//!     }
//!     SendMessageResponse::Message(msg) => {
//!         println!("Direct reply: {:?}", msg);
//!     }
//! }
//!
//! // Stream responses:
//! let mut stream = client.send_text_stream("Write a haiku").await?;
//! while let Some(event) = stream.next().await {
//!     println!("{:?}", event?);
//! }
//! # Ok(())
//! # }
//! ```

mod a2a_client;
mod card_resolver;
mod sse;
mod transport;

pub use a2a_client::{create_text_message, A2AClient};
// Re-export from types for backward compat — previously this was a duplicate enum.
pub use crate::types::SendMessageResponse;
pub use card_resolver::CardResolver;
pub use sse::{SseStream, SseStreamAdapter};
pub use transport::{JsonRpcTransport, Transport, TransportConfig};
