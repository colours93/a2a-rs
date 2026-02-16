//! Agent executor trait — the core integration point for agent logic.
//!
//! Mirrors Python SDK's `AgentExecutor(ABC)` from
//! `a2a.server.agent_execution.agent_executor`.
//!
//! Also provides:
//! - [`RequestContext`] — mirrors Python SDK's `RequestContext` from
//!   `a2a.server.agent_execution.context`
//! - [`ServerCallContext`] — mirrors Python SDK's `ServerCallContext` from
//!   `a2a.server.context`
//! - [`RequestContextBuilder`] trait + [`SimpleRequestContextBuilder`] — mirrors
//!   Python SDK's `RequestContextBuilder` and `SimpleRequestContextBuilder`
//!
//! Implementors provide the actual agent logic: reading from a [`RequestContext`]
//! and publishing events (status updates, artifacts, messages) to an [`EventQueue`].

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::error::A2AResult;
use crate::types::{Message, SendMessageConfiguration, SendMessageParams, Task};

use super::event_queue::EventQueue;
use super::task_store::TaskStore;

// ---------------------------------------------------------------------------
// ServerCallContext — per-request server-side context
// ---------------------------------------------------------------------------

/// Server call context — per-request context with user info and extensions.
///
/// Mirrors Python SDK's `ServerCallContext` from `a2a.server.context`.
///
/// This holds arbitrary per-request state, authentication info, and
/// extension negotiation data.
#[derive(Debug, Clone, Default)]
pub struct ServerCallContext {
    /// Arbitrary per-request state.
    pub state: std::collections::HashMap<String, Value>,

    /// Extensions that the client requested to activate.
    pub requested_extensions: HashSet<String>,

    /// Extensions that have been activated for this request.
    pub activated_extensions: HashSet<String>,
}

// ---------------------------------------------------------------------------
// RequestContext — agent execution context
// ---------------------------------------------------------------------------

/// Context for an agent execution request.
///
/// Contains all the information an agent needs to process a request:
/// the task identifiers, the incoming message, the existing task state
/// (if any), and optional metadata.
///
/// Mirrors Python SDK's `RequestContext` from `a2a.server.agent_execution.context`.
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Unique identifier for this task.
    pub task_id: String,

    /// Conversation context identifier — groups related tasks.
    pub context_id: String,

    /// The incoming user message that triggered this execution.
    /// `None` for cancel requests (mirrors Python SDK where `request` can be `None`).
    pub message: Option<Message>,

    /// The existing task, if this is a continuation of a previous request.
    /// `None` for new tasks.
    pub task: Option<Task>,

    /// Optional configuration from the client request (output modes, blocking, etc.).
    pub configuration: Option<SendMessageConfiguration>,

    /// Related tasks (e.g., tasks referenced via `reference_task_ids` in the message).
    pub related_tasks: Vec<Task>,

    /// Optional metadata from the client request.
    pub metadata: Option<Value>,

    /// Server call context with per-request state and extensions.
    ///
    /// Mirrors Python SDK's `RequestContext._call_context`.
    pub call_context: Option<ServerCallContext>,
}

impl RequestContext {
    /// Extracts text content from the user's message parts.
    ///
    /// Mirrors Python SDK's `RequestContext.get_user_input()`.
    ///
    /// Returns a single string containing all text content from the user message,
    /// joined by the specified delimiter. Returns an empty string if no message
    /// is present or if it contains no text parts.
    pub fn get_user_input(&self, delimiter: &str) -> String {
        let Some(ref message) = self.message else {
            return String::new();
        };

        message
            .parts
            .iter()
            .filter_map(|part| {
                if let crate::types::Part::Text { text, .. } = part {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(delimiter)
    }

    /// Attach a related task to this context.
    ///
    /// Useful for scenarios like tool execution where a new task might be spawned.
    ///
    /// Mirrors Python SDK's `RequestContext.attach_related_task(task)`.
    pub fn attach_related_task(&mut self, task: Task) {
        self.related_tasks.push(task);
    }

    /// Add an extension to the set of activated extensions for this request.
    ///
    /// This causes the extension to be indicated back to the client in the response.
    ///
    /// Mirrors Python SDK's `RequestContext.add_activated_extension(uri)`.
    pub fn add_activated_extension(&mut self, uri: String) {
        if let Some(ref mut ctx) = self.call_context {
            ctx.activated_extensions.insert(uri);
        }
    }

    /// Extensions that the client requested to activate.
    ///
    /// Mirrors Python SDK's `RequestContext.requested_extensions` property.
    pub fn requested_extensions(&self) -> HashSet<String> {
        self.call_context
            .as_ref()
            .map(|ctx| ctx.requested_extensions.clone())
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// RequestContextBuilder — builds RequestContext from request params
// ---------------------------------------------------------------------------

/// Builder for constructing [`RequestContext`] from request parameters.
///
/// Mirrors Python SDK's `RequestContextBuilder(ABC)` from
/// `a2a.server.agent_execution.request_context_builder`.
#[async_trait]
pub trait RequestContextBuilder: Send + Sync {
    /// Build a [`RequestContext`] from the given parameters.
    async fn build(
        &self,
        params: Option<&SendMessageParams>,
        task_id: Option<&str>,
        context_id: Option<&str>,
        task: Option<&Task>,
        call_context: Option<ServerCallContext>,
    ) -> A2AResult<RequestContext>;
}

/// Simple implementation of [`RequestContextBuilder`] that optionally
/// populates referred tasks from a [`TaskStore`].
///
/// Mirrors Python SDK's `SimpleRequestContextBuilder` from
/// `a2a.server.agent_execution.simple_request_context_builder`.
pub struct SimpleRequestContextBuilder {
    task_store: Option<Arc<dyn TaskStore>>,
    should_populate_referred_tasks: bool,
}

impl SimpleRequestContextBuilder {
    /// Create a new builder.
    ///
    /// If `should_populate_referred_tasks` is `true`, the builder will fetch
    /// tasks referenced in `params.message.reference_task_ids` from the
    /// `task_store` and populate `related_tasks` in the `RequestContext`.
    pub fn new(
        task_store: Option<Arc<dyn TaskStore>>,
        should_populate_referred_tasks: bool,
    ) -> Self {
        Self {
            task_store,
            should_populate_referred_tasks,
        }
    }
}

impl Default for SimpleRequestContextBuilder {
    fn default() -> Self {
        Self::new(None, false)
    }
}

#[async_trait]
impl RequestContextBuilder for SimpleRequestContextBuilder {
    async fn build(
        &self,
        params: Option<&SendMessageParams>,
        task_id: Option<&str>,
        context_id: Option<&str>,
        task: Option<&Task>,
        call_context: Option<ServerCallContext>,
    ) -> A2AResult<RequestContext> {
        let mut related_tasks = Vec::new();

        // Populate referred tasks if configured and reference IDs are present.
        if self.should_populate_referred_tasks {
            if let (Some(store), Some(params)) = (&self.task_store, params) {
                if let Some(ref ref_ids) = params.message.reference_task_ids {
                    for ref_id in ref_ids {
                        if let Ok(Some(t)) = store.get(ref_id).await {
                            related_tasks.push(t);
                        }
                    }
                }
            }
        }

        // Determine task_id — prefer explicit, then from message, then from task.
        let resolved_task_id = task_id
            .map(|s| s.to_string())
            .or_else(|| params.and_then(|p| p.message.task_id.clone()))
            .or_else(|| task.map(|t| t.id.clone()))
            .unwrap_or_default();

        // Determine context_id — prefer explicit, then from message, then from task.
        let resolved_context_id = context_id
            .map(|s| s.to_string())
            .or_else(|| params.and_then(|p| p.message.context_id.clone()))
            .or_else(|| task.map(|t| t.context_id.clone()))
            .unwrap_or_default();

        Ok(RequestContext {
            task_id: resolved_task_id,
            context_id: resolved_context_id,
            message: params.map(|p| p.message.clone()),
            task: task.cloned(),
            configuration: params.and_then(|p| p.configuration.clone()),
            related_tasks,
            metadata: params.and_then(|p| p.metadata.clone()),
            call_context,
        })
    }
}

// ---------------------------------------------------------------------------
// AgentExecutor trait
// ---------------------------------------------------------------------------

/// Core trait for agent execution logic.
///
/// Implement this trait to define your agent's behavior. The server framework
/// calls [`execute`](AgentExecutor::execute) when a new message arrives and
/// [`cancel`](AgentExecutor::cancel) when a cancellation is requested.
///
/// Mirrors Python SDK's `AgentExecutor(ABC)` from
/// `a2a.server.agent_execution.agent_executor`.
///
/// # Examples
///
/// ```rust,ignore
/// use a2a_rs::server::{AgentExecutor, RequestContext, EventQueue, TaskUpdater};
/// use a2a_rs::error::A2AResult;
/// use async_trait::async_trait;
///
/// struct MyAgent;
///
/// #[async_trait]
/// impl AgentExecutor for MyAgent {
///     async fn execute(&self, context: RequestContext, event_queue: EventQueue) -> A2AResult<()> {
///         let updater = TaskUpdater::new(
///             event_queue,
///             context.task_id,
///             context.context_id,
///         );
///         updater.complete(Some("Done!")).await?;
///         Ok(())
///     }
///
///     async fn cancel(&self, context: RequestContext, event_queue: EventQueue) -> A2AResult<()> {
///         let updater = TaskUpdater::new(
///             event_queue,
///             context.task_id,
///             context.context_id,
///         );
///         updater.cancel(None).await?;
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait AgentExecutor: Send + Sync {
    /// Execute the agent's logic for a given request.
    ///
    /// The agent should read necessary information from the `context` and
    /// publish events (`TaskStatusUpdateEvent`, `TaskArtifactUpdateEvent`,
    /// or complete `Task`/`Message` objects) to the `event_queue`.
    ///
    /// This method should return once the agent's execution is complete
    /// or yields control (e.g., enters an `input-required` state).
    async fn execute(&self, context: RequestContext, event_queue: EventQueue) -> A2AResult<()>;

    /// Request the agent to cancel an ongoing task.
    ///
    /// The agent should attempt to stop the task identified by `context.task_id`
    /// and publish a `TaskStatusUpdateEvent` with state `canceled` to the
    /// `event_queue`.
    async fn cancel(&self, context: RequestContext, event_queue: EventQueue) -> A2AResult<()>;
}
