//! Task manager — manages task lifecycle during request execution.
//!
//! Mirrors Python SDK's `TaskManager` from `a2a.server.tasks.task_manager`.
//!
//! Responsible for retrieving, saving, and updating the [`Task`] object based on
//! events received from the agent. Handles the mapping between streaming events
//! (status updates, artifact updates) and the persisted task state.
//!
//! Also includes the `append_artifact_to_task` utility (from Python SDK's
//! `a2a.utils.helpers.append_artifact_to_task`).

use tracing::{debug, info, warn};

use crate::error::{A2AError, A2AResult};
use crate::types::{
    Artifact, Message, StreamResponse, Task, TaskArtifactUpdateEvent, TaskState, TaskStatus,
    TaskStatusUpdateEvent,
};

use super::task_store::TaskStore;

/// Manages a task's lifecycle during execution of a request.
///
/// Responsible for retrieving, saving, and updating the `Task` object based on
/// events received from the agent.
///
/// Mirrors Python SDK's `TaskManager` from `a2a.server.tasks.task_manager`.
pub struct TaskManager {
    /// The task ID, if known from the request.
    task_id: Option<String>,

    /// The context ID, if known from the request.
    context_id: Option<String>,

    /// The task store for persistence.
    task_store: Box<dyn TaskStore>,

    /// The initial message that created the task (used when creating a new task).
    initial_message: Option<Message>,

    /// The current in-memory task state.
    current_task: Option<Task>,
}

impl TaskManager {
    /// Create a new TaskManager.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The task ID, if known from the request.
    /// * `context_id` - The context ID, if known from the request.
    /// * `task_store` - The task store for persistence.
    /// * `initial_message` - The message that initiated the task, if any.
    pub fn new(
        task_id: Option<String>,
        context_id: Option<String>,
        task_store: Box<dyn TaskStore>,
        initial_message: Option<Message>,
    ) -> Result<Self, A2AError> {
        if let Some(ref id) = task_id {
            if id.is_empty() {
                return Err(A2AError::InvalidParams {
                    message: "Task ID must be a non-empty string".to_string(),
                    data: None,
                });
            }
        }

        debug!(
            task_id = ?task_id,
            context_id = ?context_id,
            "TaskManager initialized"
        );

        Ok(Self {
            task_id,
            context_id,
            task_store,
            initial_message,
            current_task: None,
        })
    }

    /// Retrieves the current task object, either from memory or the store.
    ///
    /// If `task_id` is set, it first checks the in-memory `current_task`,
    /// then attempts to load it from the task store.
    pub async fn get_task(&mut self) -> A2AResult<Option<Task>> {
        let Some(ref task_id) = self.task_id else {
            debug!("task_id is not set, cannot get task");
            return Ok(None);
        };

        if self.current_task.is_some() {
            return Ok(self.current_task.clone());
        }

        debug!(task_id = %task_id, "Attempting to get task from store");
        let task = self.task_store.get(task_id).await?;
        if task.is_some() {
            debug!(task_id = %task_id, "Task retrieved successfully");
        } else {
            debug!(task_id = %task_id, "Task not found");
        }
        self.current_task = task.clone();
        Ok(task)
    }

    /// Processes a task-related event and saves the updated task state.
    ///
    /// Ensures task and context IDs match or are set from the event.
    /// Handles `Task`, `TaskStatusUpdateEvent`, and `TaskArtifactUpdateEvent`.
    ///
    /// Mirrors Python SDK's `TaskManager.save_task_event`.
    pub async fn save_task_event(&mut self, event: TaskEvent) -> A2AResult<Option<Task>> {
        let (task_id_from_event, context_id_from_event) = match &event {
            TaskEvent::Task(t) => (t.id.clone(), t.context_id.clone()),
            TaskEvent::StatusUpdate(e) => (e.task_id.clone(), e.context_id.clone()),
            TaskEvent::ArtifactUpdate(e) => (e.task_id.clone(), e.context_id.clone()),
        };

        // Validate task ID consistency
        if let Some(ref our_id) = self.task_id {
            if *our_id != task_id_from_event {
                return Err(A2AError::InvalidParams {
                    message: format!(
                        "Task in event doesn't match TaskManager {} : {}",
                        our_id, task_id_from_event
                    ),
                    data: None,
                });
            }
        }
        if self.task_id.is_none() {
            self.task_id = Some(task_id_from_event.clone());
        }

        // Validate context ID consistency
        if let Some(ref our_ctx) = self.context_id {
            if *our_ctx != context_id_from_event {
                return Err(A2AError::InvalidParams {
                    message: format!(
                        "Context in event doesn't match TaskManager {} : {}",
                        our_ctx, context_id_from_event
                    ),
                    data: None,
                });
            }
        }
        if self.context_id.is_none() {
            self.context_id = Some(context_id_from_event);
        }

        debug!(
            event_type = %event.type_name(),
            task_id = %task_id_from_event,
            "Processing save of task event"
        );

        match event {
            TaskEvent::Task(task) => {
                self.save_task(task.clone()).await?;
                Ok(Some(task))
            }
            TaskEvent::StatusUpdate(status_event) => {
                let mut task = self.ensure_task_from_status(&status_event).await?;

                debug!(
                    task_id = %task.id,
                    new_state = %status_event.status.state,
                    "Updating task status"
                );

                // Move current status message to history before replacing
                if let Some(ref msg) = task.status.message {
                    let history = task.history.get_or_insert_with(Vec::new);
                    history.push(msg.clone());
                }

                // Merge event metadata into task metadata
                if let Some(event_meta) = status_event.metadata {
                    let task_meta = task
                        .metadata
                        .get_or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
                    if let (Some(task_obj), Some(event_obj)) =
                        (task_meta.as_object_mut(), event_meta.as_object())
                    {
                        for (k, v) in event_obj {
                            task_obj.insert(k.clone(), v.clone());
                        }
                    }
                }

                task.status = status_event.status;
                self.save_task(task.clone()).await?;
                Ok(Some(task))
            }
            TaskEvent::ArtifactUpdate(artifact_event) => {
                let mut task = self.ensure_task_from_artifact(&artifact_event).await?;

                debug!(task_id = %task.id, "Appending artifact to task");
                append_artifact_to_task(&mut task, &artifact_event);

                self.save_task(task.clone()).await?;
                Ok(Some(task))
            }
        }
    }

    /// Ensures a Task object exists in memory for a status update event,
    /// loading from store or creating new if needed.
    async fn ensure_task_from_status(&mut self, event: &TaskStatusUpdateEvent) -> A2AResult<Task> {
        if let Some(ref task) = self.current_task {
            return Ok(task.clone());
        }

        if let Some(ref task_id) = self.task_id {
            debug!(task_id = %task_id, "Attempting to retrieve existing task");
            if let Some(task) = self.task_store.get(task_id).await? {
                self.current_task = Some(task.clone());
                return Ok(task);
            }
        }

        info!(
            task_id = %event.task_id,
            context_id = %event.context_id,
            "Task not found. Creating new task for event."
        );
        let task = self.init_task_obj(event.task_id.clone(), event.context_id.clone());
        self.save_task(task.clone()).await?;
        Ok(task)
    }

    /// Ensures a Task object exists in memory for an artifact update event,
    /// loading from store or creating new if needed.
    async fn ensure_task_from_artifact(
        &mut self,
        event: &TaskArtifactUpdateEvent,
    ) -> A2AResult<Task> {
        if let Some(ref task) = self.current_task {
            return Ok(task.clone());
        }

        if let Some(ref task_id) = self.task_id {
            debug!(task_id = %task_id, "Attempting to retrieve existing task");
            if let Some(task) = self.task_store.get(task_id).await? {
                self.current_task = Some(task.clone());
                return Ok(task);
            }
        }

        info!(
            task_id = %event.task_id,
            context_id = %event.context_id,
            "Task not found. Creating new task for artifact event."
        );
        let task = self.init_task_obj(event.task_id.clone(), event.context_id.clone());
        self.save_task(task.clone()).await?;
        Ok(task)
    }

    /// Process a `StreamResponse` event, updating task state if applicable.
    ///
    /// If the event is task-related, the internal task state is updated and persisted.
    /// Non-task events (e.g., direct Messages) are passed through.
    ///
    /// Mirrors Python SDK's `TaskManager.process`.
    pub async fn process(&mut self, event: StreamResponse) -> A2AResult<StreamResponse> {
        match &event {
            StreamResponse::Task(task) => {
                self.save_task_event(TaskEvent::Task(task.clone())).await?;
            }
            StreamResponse::StatusUpdate(status) => {
                self.save_task_event(TaskEvent::StatusUpdate(status.clone()))
                    .await?;
            }
            StreamResponse::ArtifactUpdate(artifact) => {
                self.save_task_event(TaskEvent::ArtifactUpdate(artifact.clone()))
                    .await?;
            }
            StreamResponse::Message(_) => {
                // Messages are not persisted in the task store
            }
        }
        Ok(event)
    }

    /// Updates a task object by adding a new message to its history.
    ///
    /// If the task has a message in its current status, that message is moved
    /// to the history first.
    ///
    /// Mirrors Python SDK's `TaskManager.update_with_message`.
    pub fn update_with_message(&mut self, message: Message, task: &mut Task) {
        if let Some(ref status_msg) = task.status.message {
            let history = task.history.get_or_insert_with(Vec::new);
            history.push(status_msg.clone());
            task.status.message = None;
        }
        let history = task.history.get_or_insert_with(Vec::new);
        history.push(message);
        self.current_task = Some(task.clone());
    }

    /// Returns the current task ID.
    pub fn task_id(&self) -> Option<&str> {
        self.task_id.as_deref()
    }

    /// Returns the current context ID.
    pub fn context_id(&self) -> Option<&str> {
        self.context_id.as_deref()
    }

    // -- Private helpers --

    /// Initializes a new task object.
    fn init_task_obj(&self, task_id: String, context_id: String) -> Task {
        debug!(
            task_id = %task_id,
            context_id = %context_id,
            "Initializing new Task object"
        );

        let history = self.initial_message.as_ref().map(|msg| vec![msg.clone()]);

        Task {
            id: task_id,
            context_id,
            kind: "task".to_string(),
            status: TaskStatus {
                state: TaskState::Submitted,
                message: None,
                timestamp: None,
            },
            artifacts: None,
            history,
            metadata: None,
        }
    }

    /// Saves the given task to the store and updates the in-memory cache.
    async fn save_task(&mut self, task: Task) -> A2AResult<()> {
        debug!(task_id = %task.id, "Saving task");
        self.task_store.save(task.clone()).await?;

        if self.task_id.is_none() {
            info!(task_id = %task.id, "New task created");
            self.task_id = Some(task.id.clone());
            self.context_id = Some(task.context_id.clone());
        }

        self.current_task = Some(task);
        Ok(())
    }
}

/// A task-related event that the TaskManager can process.
///
/// Mirrors the Python SDK's `Task | TaskStatusUpdateEvent | TaskArtifactUpdateEvent`
/// union used in `TaskManager.save_task_event`.
pub enum TaskEvent {
    /// A complete task snapshot.
    Task(Task),
    /// A status update event.
    StatusUpdate(TaskStatusUpdateEvent),
    /// An artifact update event.
    ArtifactUpdate(TaskArtifactUpdateEvent),
}

impl TaskEvent {
    /// Returns a human-readable type name for logging.
    fn type_name(&self) -> &'static str {
        match self {
            TaskEvent::Task(_) => "Task",
            TaskEvent::StatusUpdate(_) => "TaskStatusUpdateEvent",
            TaskEvent::ArtifactUpdate(_) => "TaskArtifactUpdateEvent",
        }
    }
}

/// Appends an artifact to a task based on an artifact update event.
///
/// Handles creating the artifacts list if it doesn't exist, adding new artifacts,
/// and appending parts to existing artifacts based on the `append` flag.
///
/// Mirrors Python SDK's `append_artifact_to_task` from `a2a.utils.helpers`.
pub fn append_artifact_to_task(task: &mut Task, event: &TaskArtifactUpdateEvent) {
    let artifacts = task.artifacts.get_or_insert_with(Vec::new);

    let new_artifact: &Artifact = &event.artifact;
    let artifact_id = &new_artifact.artifact_id;
    let append_parts = event.append.unwrap_or(false);

    // Find existing artifact by ID
    let existing_idx = artifacts.iter().position(|a| a.artifact_id == *artifact_id);

    if !append_parts {
        // First chunk for this artifact
        if let Some(idx) = existing_idx {
            // Replace the existing artifact entirely
            debug!(
                artifact_id = %artifact_id,
                task_id = %task.id,
                "Replacing artifact"
            );
            artifacts[idx] = new_artifact.clone();
        } else {
            // Add as new artifact
            debug!(
                artifact_id = %artifact_id,
                task_id = %task.id,
                "Adding new artifact"
            );
            artifacts.push(new_artifact.clone());
        }
    } else if let Some(idx) = existing_idx {
        // Append new parts to existing artifact
        debug!(
            artifact_id = %artifact_id,
            task_id = %task.id,
            "Appending parts to artifact"
        );
        artifacts[idx].parts.extend(new_artifact.parts.clone());
    } else {
        // Received append=true for nonexistent artifact — ignore
        warn!(
            artifact_id = %artifact_id,
            task_id = %task.id,
            "Received append=true for nonexistent artifact. Ignoring chunk."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::task_store::InMemoryTaskStore;
    use crate::types::{Part, TaskState, TaskStatus};

    fn make_task(id: &str, ctx: &str) -> Task {
        Task {
            id: id.to_string(),
            context_id: ctx.to_string(),
            kind: "task".to_string(),
            status: TaskStatus {
                state: TaskState::Submitted,
                message: None,
                timestamp: None,
            },
            artifacts: None,
            history: None,
            metadata: None,
        }
    }

    fn make_artifact(id: &str, text: &str) -> Artifact {
        Artifact {
            artifact_id: id.to_string(),
            name: None,
            description: None,
            parts: vec![Part::text(text)],
            metadata: None,
            extensions: None,
        }
    }

    #[test]
    fn append_artifact_new() {
        let mut task = make_task("t1", "ctx1");
        let event = TaskArtifactUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: "artifact-update".to_string(),
            artifact: make_artifact("a1", "hello"),
            append: None,
            last_chunk: None,
            metadata: None,
        };

        append_artifact_to_task(&mut task, &event);

        assert_eq!(task.artifacts.as_ref().unwrap().len(), 1);
        assert_eq!(task.artifacts.as_ref().unwrap()[0].artifact_id, "a1");
    }

    #[test]
    fn append_artifact_replace() {
        let mut task = make_task("t1", "ctx1");
        task.artifacts = Some(vec![make_artifact("a1", "old")]);

        let event = TaskArtifactUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: "artifact-update".to_string(),
            artifact: make_artifact("a1", "new"),
            append: Some(false),
            last_chunk: None,
            metadata: None,
        };

        append_artifact_to_task(&mut task, &event);

        assert_eq!(task.artifacts.as_ref().unwrap().len(), 1);
        match &task.artifacts.as_ref().unwrap()[0].parts[0] {
            Part::Text { text, .. } => assert_eq!(text, "new"),
            _ => panic!("expected text part"),
        }
    }

    #[test]
    fn append_artifact_append_parts() {
        let mut task = make_task("t1", "ctx1");
        task.artifacts = Some(vec![make_artifact("a1", "part1")]);

        let event = TaskArtifactUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: "artifact-update".to_string(),
            artifact: make_artifact("a1", "part2"),
            append: Some(true),
            last_chunk: None,
            metadata: None,
        };

        append_artifact_to_task(&mut task, &event);

        assert_eq!(task.artifacts.as_ref().unwrap().len(), 1);
        assert_eq!(task.artifacts.as_ref().unwrap()[0].parts.len(), 2);
    }

    #[test]
    fn append_artifact_nonexistent_ignored() {
        let mut task = make_task("t1", "ctx1");
        task.artifacts = Some(vec![]);

        let event = TaskArtifactUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: "artifact-update".to_string(),
            artifact: make_artifact("a_missing", "data"),
            append: Some(true),
            last_chunk: None,
            metadata: None,
        };

        append_artifact_to_task(&mut task, &event);

        // Should not have added the artifact
        assert!(task.artifacts.as_ref().unwrap().is_empty());
    }

    #[tokio::test]
    async fn task_manager_creates_task_on_status_event() {
        let store = Box::new(InMemoryTaskStore::new());
        let mut mgr = TaskManager::new(None, None, store, None).unwrap();

        let event = TaskEvent::StatusUpdate(TaskStatusUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: "status-update".to_string(),
            status: TaskStatus {
                state: TaskState::Working,
                message: None,
                timestamp: None,
            },
            r#final: false,
            metadata: None,
        });

        let result = mgr.save_task_event(event).await.unwrap();
        assert!(result.is_some());

        let task = result.unwrap();
        assert_eq!(task.id, "t1");
        assert_eq!(task.status.state, TaskState::Working);
    }

    #[tokio::test]
    async fn task_manager_rejects_mismatched_task_id() {
        let store = Box::new(InMemoryTaskStore::new());
        let mut mgr = TaskManager::new(Some("t1".to_string()), None, store, None).unwrap();

        let event = TaskEvent::StatusUpdate(TaskStatusUpdateEvent {
            task_id: "t2".to_string(),
            context_id: "ctx1".to_string(),
            kind: "status-update".to_string(),
            status: TaskStatus {
                state: TaskState::Working,
                message: None,
                timestamp: None,
            },
            r#final: false,
            metadata: None,
        });

        let result = mgr.save_task_event(event).await;
        assert!(result.is_err());
    }

    #[test]
    fn update_with_message_moves_status_to_history() {
        let store = Box::new(InMemoryTaskStore::new());
        let mut mgr = TaskManager::new(
            Some("t1".to_string()),
            Some("ctx1".to_string()),
            store,
            None,
        )
        .unwrap();

        let status_msg = Message::agent("m1", "Status message");
        let mut task = make_task("t1", "ctx1");
        task.status.message = Some(status_msg);

        let new_msg = Message::user("m2", "New user message");
        mgr.update_with_message(new_msg, &mut task);

        // Status message should have been moved to history
        assert!(task.status.message.is_none());
        let history = task.history.as_ref().unwrap();
        assert_eq!(history.len(), 2); // status msg + new msg
        assert_eq!(history[0].message_id, "m1");
        assert_eq!(history[1].message_id, "m2");
    }
}
