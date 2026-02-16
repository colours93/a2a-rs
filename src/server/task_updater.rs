//! Task updater — thread-safe helper for publishing task state transitions.
//!
//! Mirrors Python SDK's `TaskUpdater` from `a2a.server.tasks.task_updater`.
//!
//! The updater enforces the A2A state machine: once a task reaches a terminal
//! state (completed, failed, canceled, rejected), no further status updates
//! are accepted. It provides convenience methods for common transitions and
//! handles artifact ID generation.

use chrono::Utc;
use tokio::sync::Mutex;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::error::{A2AError, A2AResult};
use crate::types::{
    Artifact, Message, Part, StreamResponse, TaskArtifactUpdateEvent, TaskState, TaskStatus,
    TaskStatusUpdateEvent,
};

use super::event_queue::EventQueue;

/// Thread-safe task state transition helper.
///
/// Wraps an [`EventQueue`] and provides ergonomic methods for publishing
/// status updates and artifacts. Tracks whether the task has reached a
/// terminal state and rejects further updates after that point.
///
/// # Terminal states
///
/// The following states are terminal — once reached, no further updates
/// are accepted:
/// - `completed`
/// - `failed`
/// - `canceled`
/// - `rejected`
///
/// # Thread safety
///
/// All mutation is protected by a `tokio::sync::Mutex`, making it safe
/// to share across tasks via `Arc<TaskUpdater>`.
pub struct TaskUpdater {
    event_queue: EventQueue,
    task_id: String,
    context_id: String,
    state: Mutex<UpdaterState>,
}

/// Internal mutable state protected by the mutex.
struct UpdaterState {
    terminal_reached: bool,
    artifact_counter: u64,
}

impl TaskUpdater {
    /// Create a new task updater for the given task and context IDs.
    pub fn new(event_queue: EventQueue, task_id: String, context_id: String) -> Self {
        Self {
            event_queue,
            task_id,
            context_id,
            state: Mutex::new(UpdaterState {
                terminal_reached: false,
                artifact_counter: 0,
            }),
        }
    }

    /// Returns `true` if the task has reached a terminal state.
    pub async fn is_terminal(&self) -> bool {
        let state = self.state.lock().await;
        state.terminal_reached
    }

    /// Publish a status update event with a full `Message` object.
    ///
    /// Mirrors Python SDK's `TaskUpdater.update_status(state, message, final, timestamp, metadata)`.
    ///
    /// If the state is terminal (completed, failed, canceled, rejected), `final` is
    /// automatically set to `true` regardless of the provided value.
    ///
    /// An optional `timestamp` can be provided as an ISO 8601 string. If `None`,
    /// the current UTC time is used (matching Python SDK behavior).
    ///
    /// # Errors
    ///
    /// Returns an error if the task has already reached a terminal state.
    pub async fn update_status(
        &self,
        task_state: TaskState,
        message: Option<Message>,
        r#final: bool,
        metadata: Option<serde_json::Value>,
    ) -> A2AResult<()> {
        self.update_status_with_timestamp(task_state, message, r#final, None, metadata)
            .await
    }

    /// Publish a status update event with a full `Message` object and optional timestamp.
    ///
    /// This is the full-parameter version matching Python SDK's
    /// `TaskUpdater.update_status(state, message, final, timestamp, metadata)`.
    ///
    /// # Errors
    ///
    /// Returns an error if the task has already reached a terminal state.
    pub async fn update_status_with_timestamp(
        &self,
        task_state: TaskState,
        message: Option<Message>,
        r#final: bool,
        timestamp: Option<String>,
        metadata: Option<serde_json::Value>,
    ) -> A2AResult<()> {
        let is_terminal = Self::is_terminal_state(&task_state);
        let is_final = if is_terminal { true } else { r#final };

        // Check and update terminal state under the lock, then drop it before
        // the async enqueue_event call to avoid holding the mutex across an
        // await point.
        {
            let mut state = self.state.lock().await;

            if state.terminal_reached {
                warn!(
                    task_id = %self.task_id,
                    requested_state = ?task_state,
                    "Attempted status update after terminal state"
                );
                return Err(A2AError::Other(format!(
                    "Task {} has already reached a terminal state — cannot transition to {:?}",
                    self.task_id, task_state
                )));
            }

            if is_terminal {
                state.terminal_reached = true;
            }
        }

        let current_timestamp = timestamp.unwrap_or_else(|| Utc::now().to_rfc3339());

        let event = StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
            task_id: self.task_id.clone(),
            context_id: self.context_id.clone(),
            kind: "status-update".to_string(),
            status: TaskStatus {
                state: task_state,
                message,
                timestamp: Some(current_timestamp),
            },
            r#final: is_final,
            metadata,
        });

        self.event_queue.enqueue_event(event).await?;

        debug!(
            task_id = %self.task_id,
            state = ?task_state,
            terminal = is_terminal,
            "Status update published"
        );

        Ok(())
    }

    /// Publish a status update with an optional text message.
    ///
    /// This is a convenience wrapper around [`update_status`](Self::update_status)
    /// that creates a `Message` with a single text `Part` and role `agent`.
    pub async fn update_status_text(
        &self,
        task_state: TaskState,
        message: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) -> A2AResult<()> {
        let status_message =
            message.map(|text| self.new_agent_message(vec![Part::text(text)], None));

        self.update_status(task_state, status_message, false, metadata)
            .await
    }

    /// Add an artifact to the task.
    ///
    /// Publishes a `TaskArtifactUpdateEvent` with the given parts.
    /// An artifact ID is auto-generated if not provided.
    ///
    /// Mirrors Python SDK's `TaskUpdater.add_artifact(parts, artifact_id, name, metadata, append, last_chunk, extensions)`.
    ///
    /// NOTE: Unlike `update_status`, this method does NOT check for terminal state.
    /// The Python SDK's `add_artifact` has no terminal state guard — artifacts can
    /// be added independently of the task's status. This matches the Python SDK behavior.
    #[allow(clippy::too_many_arguments)]
    pub async fn add_artifact(
        &self,
        parts: Vec<Part>,
        artifact_id: Option<String>,
        name: Option<String>,
        metadata: Option<serde_json::Value>,
        append: Option<bool>,
        last_chunk: Option<bool>,
        extensions: Option<Vec<String>>,
    ) -> A2AResult<()> {
        let artifact_id = if let Some(id) = artifact_id {
            id
        } else {
            let mut state = self.state.lock().await;
            state.artifact_counter += 1;
            Uuid::new_v4().to_string()
        };

        let event = StreamResponse::ArtifactUpdate(TaskArtifactUpdateEvent {
            task_id: self.task_id.clone(),
            context_id: self.context_id.clone(),
            kind: "artifact-update".to_string(),
            artifact: Artifact {
                artifact_id: artifact_id.clone(),
                parts,
                name,
                description: None,
                metadata: metadata.clone(),
                extensions,
            },
            append,
            last_chunk,
            metadata,
        });

        self.event_queue.enqueue_event(event).await?;

        debug!(
            task_id = %self.task_id,
            artifact_id = %artifact_id,
            "Artifact update published"
        );

        Ok(())
    }

    // ---- Convenience methods for common state transitions ----
    // These mirror the Python SDK's convenience methods exactly.

    /// Transition to `completed` state.
    ///
    /// This is a terminal state — no further updates will be accepted.
    pub async fn complete(&self, message: Option<Message>) -> A2AResult<()> {
        self.update_status(TaskState::Completed, message, true, None)
            .await
    }

    /// Transition to `failed` state.
    ///
    /// This is a terminal state — no further updates will be accepted.
    pub async fn failed(&self, message: Option<Message>) -> A2AResult<()> {
        self.update_status(TaskState::Failed, message, true, None)
            .await
    }

    /// Transition to `canceled` state.
    ///
    /// This is a terminal state — no further updates will be accepted.
    pub async fn cancel(&self, message: Option<Message>) -> A2AResult<()> {
        self.update_status(TaskState::Canceled, message, true, None)
            .await
    }

    /// Transition to `rejected` state.
    ///
    /// This is a terminal state — no further updates will be accepted.
    pub async fn reject(&self, message: Option<Message>) -> A2AResult<()> {
        self.update_status(TaskState::Rejected, message, true, None)
            .await
    }

    /// Transition to `submitted` state.
    pub async fn submit(&self, message: Option<Message>) -> A2AResult<()> {
        self.update_status(TaskState::Submitted, message, false, None)
            .await
    }

    /// Transition to `working` state.
    ///
    /// Signals that the agent has begun processing.
    pub async fn start_work(&self, message: Option<Message>) -> A2AResult<()> {
        self.update_status(TaskState::Working, message, false, None)
            .await
    }

    /// Transition to `input-required` state.
    ///
    /// The agent is waiting for additional user input before proceeding.
    /// Python SDK allows specifying `final` for input-required.
    pub async fn requires_input(&self, message: Option<Message>, r#final: bool) -> A2AResult<()> {
        self.update_status(TaskState::InputRequired, message, r#final, None)
            .await
    }

    /// Transition to `auth-required` state.
    ///
    /// The agent requires authentication before proceeding.
    /// Python SDK allows specifying `final` for auth-required.
    pub async fn requires_auth(&self, message: Option<Message>, r#final: bool) -> A2AResult<()> {
        self.update_status(TaskState::AuthRequired, message, r#final, None)
            .await
    }

    // ---- Text convenience methods (Rust-specific ergonomics) ----

    /// Transition to `completed` with a text message.
    pub async fn complete_with_text(&self, text: &str) -> A2AResult<()> {
        self.update_status_text(TaskState::Completed, Some(text), None)
            .await
    }

    /// Transition to `failed` with a text message.
    pub async fn failed_with_text(&self, text: &str) -> A2AResult<()> {
        self.update_status_text(TaskState::Failed, Some(text), None)
            .await
    }

    /// Transition to `working` with a text message.
    pub async fn start_work_with_text(&self, text: &str) -> A2AResult<()> {
        self.update_status_text(TaskState::Working, Some(text), None)
            .await
    }

    /// Create a new agent message (without publishing it).
    ///
    /// Useful when you need to build a `Message` with custom parts
    /// for use in a status update or artifact.
    ///
    /// Mirrors Python SDK's `TaskUpdater.new_agent_message(parts, metadata=None)`.
    pub fn new_agent_message(
        &self,
        parts: Vec<Part>,
        metadata: Option<serde_json::Value>,
    ) -> Message {
        Message {
            message_id: Uuid::new_v4().to_string(),
            role: crate::types::Role::Agent,
            kind: "message".to_string(),
            parts,
            context_id: Some(self.context_id.clone()),
            task_id: Some(self.task_id.clone()),
            metadata,
            extensions: None,
            reference_task_ids: None,
        }
    }

    /// Check whether a given state is terminal.
    fn is_terminal_state(state: &TaskState) -> bool {
        matches!(
            state,
            TaskState::Completed | TaskState::Failed | TaskState::Canceled | TaskState::Rejected
        )
    }

    /// Get the task ID this updater is tracking.
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Get the context ID this updater is tracking.
    pub fn context_id(&self) -> &str {
        &self.context_id
    }
}
