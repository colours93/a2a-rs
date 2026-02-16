//! Request handler — coordinates agent execution, task storage, and event delivery.
//!
//! Mirrors Python SDK's `RequestHandler(ABC)` and `DefaultRequestHandler` from
//! `a2a.server.request_handlers`.
//!
//! The [`RequestHandler`] trait defines the interface that the axum integration
//! layer calls for each JSON-RPC method. [`DefaultRequestHandler`] provides
//! the standard implementation that wires together an [`AgentExecutor`],
//! [`TaskStore`], and [`EventQueue`].

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::error::{A2AError, A2AResult};
use crate::types::{
    Message, Part, SendMessageResponse, StreamResponse, Task, TaskState, TaskStatus,
    TaskStatusUpdateEvent,
};

use super::agent_executor::{AgentExecutor, RequestContext};
use super::event_queue::EventQueue;
use super::task_store::{TaskListParams, TaskListResponse, TaskStore};

/// Parameters for `message/send` and `message/stream`.
#[derive(Debug, Clone)]
pub struct SendMessageParams {
    /// The message to send to the agent.
    pub message: Message,

    /// Optional configuration for the send operation.
    pub configuration: Option<SendMessageConfiguration>,

    /// Optional metadata attached to the request.
    pub metadata: Option<serde_json::Value>,

    /// Optional tenant identifier.
    pub tenant: Option<String>,
}

/// Configuration options for message sending.
#[derive(Debug, Clone)]
pub struct SendMessageConfiguration {
    /// Accepted output MIME types / modes.
    pub accepted_output_modes: Option<Vec<String>>,

    /// If `true`, the server should block until the task completes.
    /// If `false` or `None`, the server may return immediately with a
    /// `submitted` or `working` task.
    pub blocking: Option<bool>,

    /// Maximum number of history messages to include in the response.
    pub history_length: Option<usize>,

    /// Push notification configuration.
    pub push_notification_config: Option<serde_json::Value>,
}

/// Parameters for `tasks/get`.
#[derive(Debug, Clone)]
pub struct GetTaskParams {
    /// The task ID to retrieve.
    pub id: String,

    /// Maximum number of history messages to include.
    pub history_length: Option<usize>,

    /// Optional metadata.
    pub metadata: Option<serde_json::Value>,

    /// Optional tenant identifier.
    pub tenant: Option<String>,
}

/// Parameters for `tasks/cancel`.
#[derive(Debug, Clone)]
pub struct CancelTaskParams {
    /// The task ID to cancel.
    pub id: String,

    /// Optional metadata.
    pub metadata: Option<serde_json::Value>,

    /// Optional tenant identifier.
    pub tenant: Option<String>,
}

/// Parameters for `tasks/subscribe`.
#[derive(Debug, Clone)]
pub struct SubscribeToTaskParams {
    /// The task ID to subscribe to.
    pub id: String,

    /// Optional metadata.
    pub metadata: Option<serde_json::Value>,

    /// Optional tenant identifier.
    pub tenant: Option<String>,
}

// Re-export from types.rs — uses proto oneof serialization pattern.
// SendMessageResponse is imported from crate::types above.

/// Trait for handling A2A JSON-RPC requests.
///
/// Each method corresponds to an A2A JSON-RPC method. The axum integration
/// layer dispatches incoming requests to these methods.
///
/// Mirrors Python SDK's `RequestHandler(ABC)` from
/// `a2a.server.request_handlers.request_handler`.
#[async_trait]
pub trait RequestHandler: Send + Sync {
    /// Handle `message/send` — execute agent logic and return the completed task or message.
    async fn on_message_send(&self, params: SendMessageParams) -> A2AResult<SendMessageResponse>;

    /// Handle `message/stream` — execute agent logic and return an event stream.
    async fn on_message_send_stream(
        &self,
        params: SendMessageParams,
    ) -> A2AResult<broadcast::Receiver<StreamResponse>>;

    /// Handle `tasks/get` — retrieve a task by ID.
    async fn on_get_task(&self, params: GetTaskParams) -> A2AResult<Task>;

    /// Handle `tasks/list` — list tasks matching filter criteria.
    async fn on_list_tasks(&self, params: TaskListParams) -> A2AResult<TaskListResponse>;

    /// Handle `tasks/cancel` — cancel a running task.
    async fn on_cancel_task(&self, params: CancelTaskParams) -> A2AResult<Task>;

    /// Handle `tasks/resubscribe` — re-subscribe to events for a running task.
    ///
    /// Allows a client to re-attach to a running streaming task's event stream.
    /// Default implementation returns `UnsupportedOperation`.
    async fn on_resubscribe_to_task(
        &self,
        params: SubscribeToTaskParams,
    ) -> A2AResult<broadcast::Receiver<StreamResponse>> {
        let _ = params;
        Err(A2AError::UnsupportedOperation {
            message: "tasks/resubscribe is not supported".to_string(),
            data: None,
        })
    }

    /// Handle `tasks/subscribe` — subscribe to events for an existing task.
    async fn on_subscribe_to_task(
        &self,
        params: SubscribeToTaskParams,
    ) -> A2AResult<broadcast::Receiver<StreamResponse>>;

    /// Handle `tasks/pushNotificationConfig/set`.
    ///
    /// Default implementation returns `UnsupportedOperation`.
    async fn on_set_task_push_notification_config(
        &self,
        _params: serde_json::Value,
    ) -> A2AResult<serde_json::Value> {
        Err(A2AError::UnsupportedOperation {
            message: "Push notification config is not supported".to_string(),
            data: None,
        })
    }

    /// Handle `tasks/pushNotificationConfig/get`.
    ///
    /// Default implementation returns `UnsupportedOperation`.
    async fn on_get_task_push_notification_config(
        &self,
        _params: serde_json::Value,
    ) -> A2AResult<serde_json::Value> {
        Err(A2AError::UnsupportedOperation {
            message: "Push notification config is not supported".to_string(),
            data: None,
        })
    }

    /// Handle `tasks/pushNotificationConfig/list`.
    ///
    /// Default implementation returns `UnsupportedOperation`.
    async fn on_list_task_push_notification_config(
        &self,
        _params: serde_json::Value,
    ) -> A2AResult<serde_json::Value> {
        Err(A2AError::UnsupportedOperation {
            message: "Push notification config is not supported".to_string(),
            data: None,
        })
    }

    /// Handle `tasks/pushNotificationConfig/delete`.
    ///
    /// Default implementation returns `UnsupportedOperation`.
    async fn on_delete_task_push_notification_config(
        &self,
        _params: serde_json::Value,
    ) -> A2AResult<()> {
        Err(A2AError::UnsupportedOperation {
            message: "Push notification config is not supported".to_string(),
            data: None,
        })
    }
}

/// Tracks a running agent execution.
struct RunningAgent {
    /// Handle to the spawned tokio task running the agent.
    handle: JoinHandle<()>,
    /// The event queue for this execution.
    event_queue: EventQueue,
}

/// Default request handler — standard implementation wiring executor, store, and events.
///
/// This is the primary implementation of [`RequestHandler`] that coordinates:
/// - An [`AgentExecutor`] for running agent logic
/// - A [`TaskStore`] for persisting task state
/// - An [`EventQueue`] per task for streaming events
///
/// # Lifecycle
///
/// 1. `on_message_send` or `on_message_send_stream` creates a new task (or looks up
///    an existing one by context ID), persists it, and spawns the agent executor.
/// 2. The executor publishes events to the task's `EventQueue`.
/// 3. For `message/send`: events are consumed until a terminal state, then the
///    final task is returned.
/// 4. For `message/stream`: the event receiver is returned directly for SSE delivery.
/// 5. `on_cancel_task` calls the executor's cancel method and waits for the
///    cancellation event.
pub struct DefaultRequestHandler {
    executor: Arc<dyn AgentExecutor>,
    task_store: Arc<dyn TaskStore>,
    /// Per-task event queues and running agent handles.
    running_agents: Mutex<HashMap<String, RunningAgent>>,
}

impl DefaultRequestHandler {
    /// Create a new default request handler.
    pub fn new(executor: Arc<dyn AgentExecutor>, task_store: Arc<dyn TaskStore>) -> Self {
        Self {
            executor,
            task_store,
            running_agents: Mutex::new(HashMap::new()),
        }
    }

    /// Create or retrieve a task for the given message.
    ///
    /// Mirrors Python SDK's `_setup_message_execution` task resolution logic:
    /// 1. If `task_id` is set, look up the existing task and validate state.
    /// 2. If the task exists but is terminal, return `InvalidParams`.
    /// 3. If `task_id` is set but doesn't exist, return `TaskNotFound`.
    /// 4. Otherwise create a new task in `submitted` state.
    async fn get_or_create_task(&self, params: &SendMessageParams) -> A2AResult<Task> {
        // Check if the message references an existing task.
        if let Some(ref task_id) = params.message.task_id {
            if let Some(task) = self.task_store.get(task_id).await? {
                // Verify it's not in a terminal state (mirrors Python SDK check).
                if Self::is_terminal(&task.status.state) {
                    return Err(A2AError::InvalidParams {
                        message: format!(
                            "Task {} is in terminal state: {}",
                            task_id, task.status.state
                        ),
                        data: None,
                    });
                }
                // Add the new message to history (mirrors Python's update_with_message).
                // Python SDK moves status.message to history first, then clears it.
                let mut updated_task = task;
                if let Some(ref status_msg) = updated_task.status.message {
                    let history = updated_task.history.get_or_insert_with(Vec::new);
                    history.push(status_msg.clone());
                    updated_task.status.message = None;
                }
                let history = updated_task.history.get_or_insert_with(Vec::new);
                history.push(params.message.clone());
                self.task_store.save(updated_task.clone()).await?;
                return Ok(updated_task);
            } else {
                // task_id was specified but doesn't exist (mirrors Python SDK).
                return Err(A2AError::TaskNotFound {
                    message: format!("Task {} was specified but does not exist", task_id),
                    data: None,
                });
            }
        }

        // Create a new task.
        let task_id = Uuid::new_v4().to_string();
        let context_id = params
            .message
            .context_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let task = Task {
            id: task_id,
            context_id,
            kind: "task".to_string(),
            status: TaskStatus {
                state: TaskState::Submitted,
                message: None,
                timestamp: Some(chrono::Utc::now().to_rfc3339()),
            },
            artifacts: None,
            history: Some(vec![params.message.clone()]),
            metadata: params.metadata.clone(),
        };

        self.task_store.save(task.clone()).await?;
        debug!(task_id = %task.id, "Created new task");

        Ok(task)
    }

    /// Spawn the agent executor for a task.
    ///
    /// Returns the event queue for subscribing to events.
    ///
    /// Mirrors Python SDK's `_run_event_stream` — executes the agent and closes
    /// the queue afterwards. Does NOT auto-publish a `Working` status; that is
    /// the responsibility of the `AgentExecutor` implementation (matching the
    /// Python SDK where `_run_event_stream` just calls `execute` + `close`).
    async fn spawn_executor(
        &self,
        task: &Task,
        message: &Message,
        configuration: Option<&SendMessageConfiguration>,
    ) -> A2AResult<EventQueue> {
        let event_queue = EventQueue::with_default_capacity();

        // Convert the request_handler's SendMessageConfiguration to the
        // types.rs SendMessageConfiguration used by RequestContext.
        let types_config = configuration.map(|c| crate::types::SendMessageConfiguration {
            accepted_output_modes: c.accepted_output_modes.clone(),
            push_notification_config: c
                .push_notification_config
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            history_length: c.history_length.map(|h| h as i32),
            blocking: c.blocking,
        });

        let context = RequestContext {
            task_id: task.id.clone(),
            context_id: task.context_id.clone(),
            message: Some(message.clone()),
            task: Some(task.clone()),
            configuration: types_config,
            related_tasks: Vec::new(),
            metadata: task.metadata.clone(),
            call_context: None,
        };

        let executor = Arc::clone(&self.executor);
        let queue_clone = event_queue.clone();
        let task_id = task.id.clone();
        let context_id = task.context_id.clone();

        let handle = tokio::spawn(async move {
            // Execute the agent — state transitions (working, etc.) are the
            // agent's responsibility, matching the Python SDK pattern.
            if let Err(e) = executor.execute(context, queue_clone.clone()).await {
                error!(task_id = %task_id, error = %e, "Agent execution failed");

                // Publish a failed status (matches Python SDK behavior where
                // execution errors result in a failed task).
                let failed_event = StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
                    task_id: task_id.clone(),
                    context_id: context_id.clone(),
                    kind: "status-update".to_string(),
                    status: TaskStatus {
                        state: TaskState::Failed,
                        message: Some(Message {
                            message_id: Uuid::new_v4().to_string(),
                            role: crate::types::Role::Agent,
                            kind: "message".to_string(),
                            parts: vec![Part::text(format!("Agent execution failed: {}", e))],
                            context_id: None,
                            task_id: Some(task_id.clone()),
                            metadata: None,
                            extensions: None,
                            reference_task_ids: None,
                        }),
                        timestamp: Some(chrono::Utc::now().to_rfc3339()),
                    },
                    r#final: true,
                    metadata: None,
                });
                let _ = queue_clone.publish(failed_event);
            }
            // Note: Python SDK calls queue.close() here. Our broadcast channel
            // auto-closes when all senders are dropped, achieving the same effect.
        });

        // Track the running agent (mirrors Python's _register_producer).
        let mut running = self.running_agents.lock().await;
        running.insert(
            task.id.clone(),
            RunningAgent {
                handle,
                event_queue: event_queue.clone(),
            },
        );

        Ok(event_queue)
    }

    /// Consume events from the queue until a terminal state is reached.
    ///
    /// Updates the task in the store as events arrive. Returns the final task.
    async fn consume_until_terminal(
        &self,
        task_id: &str,
        mut rx: broadcast::Receiver<StreamResponse>,
    ) -> A2AResult<Task> {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    self.apply_event(task_id, &event).await?;

                    if let StreamResponse::StatusUpdate(ref update) = event {
                        if Self::is_terminal(&update.status.state) || update.r#final {
                            break;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    // Channel closed — agent is done.
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(task_id = %task_id, missed = n, "Event consumer lagged");
                    // Continue receiving.
                }
            }
        }

        // Clean up the running agent entry.
        {
            let mut running = self.running_agents.lock().await;
            running.remove(task_id);
        }

        // Return the final task state.
        self.task_store
            .get(task_id)
            .await?
            .ok_or_else(|| A2AError::TaskNotFound {
                message: task_id.to_string(),
                data: None,
            })
    }

    /// Apply a stream event to the persisted task state.
    ///
    /// Mirrors Python SDK's `TaskManager.save_task_event` behavior:
    /// - For `StatusUpdate`: moves current status.message to history first, merges
    ///   event metadata into task metadata, then updates status.
    /// - For `ArtifactUpdate`: uses `append_artifact_to_task` logic — when
    ///   `append=true` and the artifact doesn't exist, the chunk is ignored.
    /// - For `Task`: replaces the entire task.
    /// - For `Message`: appends to history.
    async fn apply_event(&self, task_id: &str, event: &StreamResponse) -> A2AResult<()> {
        let mut task =
            self.task_store
                .get(task_id)
                .await?
                .ok_or_else(|| A2AError::TaskNotFound {
                    message: task_id.to_string(),
                    data: None,
                })?;

        match event {
            StreamResponse::StatusUpdate(update) => {
                // Python SDK moves the CURRENT status.message to history
                // BEFORE replacing with the new status.
                if let Some(ref current_msg) = task.status.message {
                    let history = task.history.get_or_insert_with(Vec::new);
                    history.push(current_msg.clone());
                }

                // Merge event metadata into task metadata (mirrors Python SDK).
                if let Some(ref event_meta) = update.metadata {
                    if let Some(ref mut task_meta) = task.metadata {
                        if let (Some(task_obj), Some(event_obj)) =
                            (task_meta.as_object_mut(), event_meta.as_object())
                        {
                            for (k, v) in event_obj {
                                task_obj.insert(k.clone(), v.clone());
                            }
                        }
                    } else {
                        task.metadata = Some(event_meta.clone());
                    }
                }

                task.status = update.status.clone();
            }
            StreamResponse::ArtifactUpdate(update) => {
                let artifacts = task.artifacts.get_or_insert_with(Vec::new);
                let append_parts = update.append.unwrap_or(false);
                let artifact_id = &update.artifact.artifact_id;

                // Find existing artifact by ID.
                let existing_idx = artifacts.iter().position(|a| &a.artifact_id == artifact_id);

                if !append_parts {
                    // First chunk — replace existing or add new.
                    if let Some(idx) = existing_idx {
                        artifacts[idx] = update.artifact.clone();
                    } else {
                        artifacts.push(update.artifact.clone());
                    }
                } else if let Some(idx) = existing_idx {
                    // Append parts to existing artifact.
                    artifacts[idx].parts.extend(update.artifact.parts.clone());
                } else {
                    // append=true but no existing artifact — ignore per Python SDK.
                    warn!(
                        task_id = %task_id,
                        artifact_id = %artifact_id,
                        "Received append=True for nonexistent artifact — ignoring chunk"
                    );
                }
            }
            StreamResponse::Task(updated_task) => {
                task = updated_task.clone();
            }
            StreamResponse::Message(msg) => {
                let history = task.history.get_or_insert_with(Vec::new);
                history.push(msg.clone());
            }
        }

        self.task_store.save(task).await
    }

    /// Check if a state is terminal.
    fn is_terminal(state: &TaskState) -> bool {
        matches!(
            state,
            TaskState::Completed | TaskState::Failed | TaskState::Canceled | TaskState::Rejected
        )
    }

    /// Trim task history to the requested length.
    ///
    /// Mirrors Python SDK's `apply_history_length`:
    /// - Only trims if `max_length` is `Some` AND > 0 AND history exists.
    /// - Keeps the most recent N messages (tail).
    fn trim_history(task: &mut Task, max_length: Option<usize>) {
        if let Some(max) = max_length {
            if max > 0 {
                if let Some(ref mut history) = task.history {
                    if history.len() > max {
                        let start = history.len() - max;
                        *history = history.split_off(start);
                    }
                }
            }
        }
    }
}

#[async_trait]
impl RequestHandler for DefaultRequestHandler {
    async fn on_message_send(&self, params: SendMessageParams) -> A2AResult<SendMessageResponse> {
        let task = self.get_or_create_task(&params).await?;
        let event_queue = self
            .spawn_executor(&task, &params.message, params.configuration.as_ref())
            .await?;
        let rx = event_queue.subscribe();

        // Consume events until terminal.
        let mut final_task = self.consume_until_terminal(&task.id, rx).await?;

        // Apply history_length trimming.
        let history_length = params.configuration.as_ref().and_then(|c| c.history_length);
        Self::trim_history(&mut final_task, history_length);

        Ok(SendMessageResponse::Task(final_task))
    }

    async fn on_message_send_stream(
        &self,
        params: SendMessageParams,
    ) -> A2AResult<broadcast::Receiver<StreamResponse>> {
        let task = self.get_or_create_task(&params).await?;
        let event_queue = self
            .spawn_executor(&task, &params.message, params.configuration.as_ref())
            .await?;
        let rx = event_queue.subscribe();

        // Spawn a background task to persist events as they arrive.
        let task_id = task.id.clone();
        let task_store = Arc::clone(&self.task_store);

        // We need a separate subscription for persistence.
        let mut persist_rx = event_queue.subscribe();

        tokio::spawn(async move {
            loop {
                match persist_rx.recv().await {
                    Ok(event) => {
                        // Apply event to task store — we need to inline the logic here
                        // since we can't call self methods from a spawned task.
                        if let Ok(Some(mut task)) = {
                            let store = &task_store;
                            store.get(&task_id).await
                        } {
                            match &event {
                                StreamResponse::StatusUpdate(update) => {
                                    // Move current status.message to history first
                                    // (mirrors Python SDK's save_task_event).
                                    if let Some(ref current_msg) = task.status.message {
                                        let history = task.history.get_or_insert_with(Vec::new);
                                        history.push(current_msg.clone());
                                    }
                                    // Merge event metadata into task metadata.
                                    if let Some(ref event_meta) = update.metadata {
                                        if let Some(ref mut task_meta) = task.metadata {
                                            if let (Some(task_obj), Some(event_obj)) =
                                                (task_meta.as_object_mut(), event_meta.as_object())
                                            {
                                                for (k, v) in event_obj {
                                                    task_obj.insert(k.clone(), v.clone());
                                                }
                                            }
                                        } else {
                                            task.metadata = Some(event_meta.clone());
                                        }
                                    }
                                    task.status = update.status.clone();
                                }
                                StreamResponse::ArtifactUpdate(update) => {
                                    let artifacts = task.artifacts.get_or_insert_with(Vec::new);
                                    let append_parts = update.append.unwrap_or(false);
                                    let artifact_id = &update.artifact.artifact_id;
                                    let existing_idx = artifacts
                                        .iter()
                                        .position(|a| &a.artifact_id == artifact_id);
                                    if !append_parts {
                                        if let Some(idx) = existing_idx {
                                            artifacts[idx] = update.artifact.clone();
                                        } else {
                                            artifacts.push(update.artifact.clone());
                                        }
                                    } else if let Some(idx) = existing_idx {
                                        artifacts[idx].parts.extend(update.artifact.parts.clone());
                                    }
                                    // append=true with no existing artifact: silently ignore
                                }
                                StreamResponse::Task(updated_task) => {
                                    task = updated_task.clone();
                                }
                                StreamResponse::Message(msg) => {
                                    let history = task.history.get_or_insert_with(Vec::new);
                                    history.push(msg.clone());
                                }
                            }
                            let _ = task_store.save(task).await;
                        }

                        // Check for terminal state.
                        if let StreamResponse::StatusUpdate(ref update) = event {
                            if DefaultRequestHandler::is_terminal(&update.status.state)
                                || update.r#final
                            {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(task_id = %task_id, missed = n, "Persist consumer lagged");
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn on_get_task(&self, params: GetTaskParams) -> A2AResult<Task> {
        let mut task =
            self.task_store
                .get(&params.id)
                .await?
                .ok_or_else(|| A2AError::TaskNotFound {
                    message: params.id.clone(),
                    data: None,
                })?;

        Self::trim_history(&mut task, params.history_length);
        Ok(task)
    }

    async fn on_list_tasks(&self, params: TaskListParams) -> A2AResult<TaskListResponse> {
        self.task_store.list(&params).await
    }

    async fn on_cancel_task(&self, params: CancelTaskParams) -> A2AResult<Task> {
        // Look up the task.
        let task =
            self.task_store
                .get(&params.id)
                .await?
                .ok_or_else(|| A2AError::TaskNotFound {
                    message: params.id.clone(),
                    data: None,
                })?;

        // Can't cancel a terminal task (mirrors Python SDK check).
        if Self::is_terminal(&task.status.state) {
            return Err(A2AError::TaskNotCancelable {
                message: format!(
                    "Task cannot be canceled - current state: {}",
                    task.status.state
                ),
                data: None,
            });
        }

        // Get or create event queue for this task.
        let event_queue = {
            let running = self.running_agents.lock().await;
            if let Some(agent) = running.get(&params.id) {
                agent.event_queue.clone()
            } else {
                // No running agent — create a temporary queue.
                EventQueue::with_default_capacity()
            }
        };

        let rx = event_queue.subscribe();

        // Call the executor's cancel method.
        // Python SDK passes `None` for the request in cancel context.
        let context = RequestContext {
            task_id: task.id.clone(),
            context_id: task.context_id.clone(),
            message: None,
            task: Some(task.clone()),
            configuration: None,
            related_tasks: Vec::new(),
            metadata: params.metadata,
            call_context: None,
        };

        self.executor.cancel(context, event_queue.clone()).await?;

        // Cancel the ongoing producer task, if one exists
        // (mirrors Python SDK's `producer_task.cancel()`).
        {
            let running = self.running_agents.lock().await;
            if let Some(agent) = running.get(&params.id) {
                agent.handle.abort();
            }
        }

        // Consume events until terminal.
        let final_task = self.consume_until_terminal(&task.id, rx).await?;

        // Validate the cancel result (mirrors Python SDK).
        // Python SDK raises TaskNotCancelableError if the result state is not canceled.
        if final_task.status.state != TaskState::Canceled {
            return Err(A2AError::TaskNotCancelable {
                message: format!(
                    "Task cannot be canceled - current state: {}",
                    final_task.status.state
                ),
                data: None,
            });
        }

        Ok(final_task)
    }

    async fn on_subscribe_to_task(
        &self,
        params: SubscribeToTaskParams,
    ) -> A2AResult<broadcast::Receiver<StreamResponse>> {
        // Verify the task exists.
        let task =
            self.task_store
                .get(&params.id)
                .await?
                .ok_or_else(|| A2AError::TaskNotFound {
                    message: params.id.clone(),
                    data: None,
                })?;

        // If the task is already terminal, return an error.
        if Self::is_terminal(&task.status.state) {
            return Err(A2AError::InvalidParams {
                message: format!(
                    "Task {} is in terminal state {:?} — cannot subscribe",
                    params.id, task.status.state
                ),
                data: None,
            });
        }

        // Get the event queue for this running task.
        let running = self.running_agents.lock().await;
        if let Some(agent) = running.get(&params.id) {
            Ok(agent.event_queue.subscribe())
        } else {
            Err(A2AError::TaskNotFound {
                message: format!(
                    "Task {} has no active agent execution — cannot subscribe",
                    params.id
                ),
                data: None,
            })
        }
    }

    async fn on_resubscribe_to_task(
        &self,
        params: SubscribeToTaskParams,
    ) -> A2AResult<broadcast::Receiver<StreamResponse>> {
        // Verify the task exists (mirrors Python SDK).
        let task =
            self.task_store
                .get(&params.id)
                .await?
                .ok_or_else(|| A2AError::TaskNotFound {
                    message: params.id.clone(),
                    data: None,
                })?;

        // If the task is already terminal, return an error (mirrors Python SDK).
        if Self::is_terminal(&task.status.state) {
            return Err(A2AError::InvalidParams {
                message: format!(
                    "Task {} is in terminal state: {}",
                    params.id, task.status.state
                ),
                data: None,
            });
        }

        // Get the event queue for this running task (mirrors Python SDK's queue_manager.tap).
        let running = self.running_agents.lock().await;
        if let Some(agent) = running.get(&params.id) {
            Ok(agent.event_queue.subscribe())
        } else {
            Err(A2AError::TaskNotFound {
                message: format!("Task {} has no active agent execution", params.id),
                data: None,
            })
        }
    }
}
