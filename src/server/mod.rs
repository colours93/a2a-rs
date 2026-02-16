//! A2A server framework — traits and implementations for building A2A agents.
//!
//! Mirrors the Python SDK's server module architecture:
//!
//! - [`AgentExecutor`] trait — implement your agent logic
//! - [`RequestContext`] — execution context with task IDs, message, metadata
//! - [`ServerCallContext`] — per-request context with extensions and state
//! - [`RequestContextBuilder`] trait + [`SimpleRequestContextBuilder`] — build contexts
//! - [`TaskStore`] trait + [`InMemoryTaskStore`] — task persistence
//! - [`TaskUpdater`] — thread-safe task state transition helper
//! - [`EventQueue`] — broadcast channel for streaming events
//! - [`QueueManager`] trait + [`InMemoryQueueManager`] — per-task queue management
//! - [`EventConsumer`] — consumes events from a queue (one-shot or streaming)
//! - [`RequestHandler`] trait + [`DefaultRequestHandler`] — JSON-RPC dispatch
//! - [`a2a_router`] — ready-made axum routes for A2A servers
//!
//! # Quick start
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use a2a_rs::server::*;
//! use a2a_rs::types::AgentCard;
//!
//! // 1. Implement your agent logic.
//! struct MyAgent;
//!
//! #[async_trait::async_trait]
//! impl AgentExecutor for MyAgent {
//!     async fn execute(&self, ctx: RequestContext, queue: EventQueue) -> a2a_rs::A2AResult<()> {
//!         let updater = TaskUpdater::new(queue, ctx.task_id, ctx.context_id);
//!         updater.start_work(Some("Processing...")).await?;
//!         // ... do work ...
//!         updater.complete(Some("Done!")).await?;
//!         Ok(())
//!     }
//!
//!     async fn cancel(&self, ctx: RequestContext, queue: EventQueue) -> a2a_rs::A2AResult<()> {
//!         let updater = TaskUpdater::new(queue, ctx.task_id, ctx.context_id);
//!         updater.cancel(None).await?;
//!         Ok(())
//!     }
//! }
//!
//! // 2. Wire up the server.
//! let executor: Arc<dyn AgentExecutor> = Arc::new(MyAgent);
//! let store: Arc<dyn TaskStore> = Arc::new(InMemoryTaskStore::new());
//! let handler: Arc<dyn RequestHandler> = Arc::new(
//!     DefaultRequestHandler::new(executor, store)
//! );
//!
//! // 3. Create the router and serve.
//! let app = a2a_router(handler, agent_card);
//! ```

pub mod agent_executor;
pub mod axum_integration;
pub mod event_queue;
pub mod request_handler;
pub mod task_manager;
pub mod task_store;
pub mod task_updater;

// Re-export key types at the server module level for convenience.
pub use crate::types::SendMessageResponse;
pub use agent_executor::{
    AgentExecutor, RequestContext, RequestContextBuilder, ServerCallContext,
    SimpleRequestContextBuilder,
};
pub use axum_integration::a2a_router;
pub use event_queue::{
    EventConsumer, EventQueue, InMemoryQueueManager, NoTaskQueue, QueueManager, TaskQueueExists,
};
pub use request_handler::{
    CancelTaskParams, DefaultRequestHandler, GetTaskParams, RequestHandler,
    SendMessageConfiguration, SendMessageParams, SubscribeToTaskParams,
};
pub use task_manager::{append_artifact_to_task, TaskEvent, TaskManager};
pub use task_store::{InMemoryTaskStore, TaskListParams, TaskListResponse, TaskStore};
pub use task_updater::TaskUpdater;
