//! Utility functions for creating A2A Task objects.

use crate::error::{A2AError, A2AResult};
use crate::types::{Artifact, Message, Part, Task, TaskState, TaskStatus};
use uuid::Uuid;

/// Creates a new Task object from an initial user message.
///
/// Generates task and context IDs if not provided in the message.
///
/// # Arguments
///
/// * `request` - The initial `Message` object from the user.
///
/// # Returns
///
/// A new `Task` object initialized with 'submitted' status and the input message in history.
///
/// # Errors
///
/// Returns an error if:
/// - The message parts are empty
/// - Any TextPart has empty content
///
/// # Example
///
/// ```
/// use a2a_rs::types::{Message, Part, Role};
/// use a2a_rs::utils::new_task;
///
/// let message = Message {
///     message_id: "msg-1".to_string(),
///     role: Role::User,
///     kind: "message".to_string(),
///     parts: vec![Part::Text { text: "Hello".to_string(), metadata: None }],
///     context_id: None,
///     task_id: None,
///     metadata: None,
///     extensions: None,
///     reference_task_ids: None,
/// };
/// let task = new_task(message).unwrap();
/// assert_eq!(task.status.state, a2a_rs::types::TaskState::Submitted);
/// ```
pub fn new_task(request: Message) -> A2AResult<Task> {
    // Validate message parts
    if request.parts.is_empty() {
        return Err(A2AError::invalid_params("Message parts cannot be empty"));
    }

    // Check for empty text parts
    for part in &request.parts {
        if let Part::Text { text, .. } = part {
            if text.is_empty() {
                return Err(A2AError::invalid_params("TextPart content cannot be empty"));
            }
        }
    }

    let task_id = request
        .task_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let context_id = request
        .context_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    Ok(Task {
        id: task_id,
        context_id,
        kind: "task".to_string(),
        status: TaskStatus {
            state: TaskState::Submitted,
            message: None,
            timestamp: None,
        },
        artifacts: None,
        history: Some(vec![request]),
        metadata: None,
    })
}

/// Creates a Task object in the 'completed' state.
///
/// Useful for constructing a final Task representation when the agent
/// finishes and produces artifacts.
///
/// # Arguments
///
/// * `task_id` - The ID of the task.
/// * `context_id` - The context ID of the task.
/// * `artifacts` - A list of `Artifact` objects produced by the task.
/// * `history` - An optional list of `Message` objects representing the task history.
///
/// # Returns
///
/// A `Task` object with status set to 'completed'.
///
/// # Errors
///
/// Returns an error if artifacts is empty or contains non-Artifact types.
///
/// # Example
///
/// ```
/// use a2a_rs::types::Artifact;
/// use a2a_rs::utils::{completed_task, new_text_artifact};
///
/// let artifact = new_text_artifact("Result", "Task complete", None::<String>);
/// let task = completed_task(
///     "task-123",
///     "ctx-456",
///     vec![artifact],
///     None,
/// ).unwrap();
/// assert_eq!(task.status.state, a2a_rs::types::TaskState::Completed);
/// ```
pub fn completed_task(
    task_id: impl Into<String>,
    context_id: impl Into<String>,
    artifacts: Vec<Artifact>,
    history: Option<Vec<Message>>,
) -> A2AResult<Task> {
    if artifacts.is_empty() {
        return Err(A2AError::invalid_params(
            "artifacts must be a non-empty list of Artifact objects",
        ));
    }

    Ok(Task {
        id: task_id.into(),
        context_id: context_id.into(),
        kind: "task".to_string(),
        status: TaskStatus {
            state: TaskState::Completed,
            message: None,
            timestamp: None,
        },
        artifacts: Some(artifacts),
        history,
        metadata: None,
    })
}

/// Applies history_length parameter on task and returns a new task object.
///
/// # Arguments
///
/// * `task` - The original task object with complete history
/// * `history_length` - History length configuration value
///
/// # Returns
///
/// A new task object with limited history
///
/// # Example
///
/// ```
/// use a2a_rs::types::{Message, Part, Role, Task, TaskState, TaskStatus};
/// use a2a_rs::utils::apply_history_length;
///
/// let messages: Vec<Message> = (0..10).map(|i| Message {
///     message_id: format!("msg-{}", i),
///     role: Role::User,
///     kind: "message".to_string(),
///     parts: vec![Part::Text { text: format!("Message {}", i), metadata: None }],
///     context_id: None,
///     task_id: None,
///     metadata: None,
///     extensions: None,
///     reference_task_ids: None,
/// }).collect();
///
/// let task = Task {
///     id: "task-1".to_string(),
///     context_id: "ctx-1".to_string(),
///     kind: "task".to_string(),
///     status: TaskStatus { state: TaskState::Working, message: None, timestamp: None },
///     artifacts: None,
///     history: Some(messages),
///     metadata: None,
/// };
///
/// let limited_task = apply_history_length(task, Some(5));
/// assert_eq!(limited_task.history.unwrap().len(), 5);
/// ```
pub fn apply_history_length(mut task: Task, history_length: Option<usize>) -> Task {
    if let Some(length) = history_length {
        if length > 0 {
            if let Some(ref mut history) = task.history {
                let total = history.len();
                if total > length {
                    *history = history.split_off(total - length);
                }
            }
        }
    }
    task
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Part, Role};

    #[test]
    fn test_new_task_status() {
        let message = Message {
            role: Role::User,
            kind: "message".to_string(),
            parts: vec![Part::Text {
                text: "test message".to_string(),
                metadata: None,
            }],
            message_id: Uuid::new_v4().to_string(),
            context_id: None,
            task_id: None,
            metadata: None,
            extensions: None,
            reference_task_ids: None,
        };
        let task = new_task(message).unwrap();
        assert_eq!(task.status.state, TaskState::Submitted);
    }

    #[test]
    fn test_completed_task_status() {
        let artifact = crate::utils::new_text_artifact("test", "content", None::<String>);
        let task = completed_task("task-1", "ctx-1", vec![artifact], None).unwrap();
        assert_eq!(task.status.state, TaskState::Completed);
    }

    #[test]
    fn test_completed_task_empty_artifacts_fails() {
        let result = completed_task("task-1", "ctx-1", vec![], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_history_length() {
        let messages: Vec<Message> = (0..10)
            .map(|i| Message {
                message_id: format!("msg-{}", i),
                role: Role::User,
                kind: "message".to_string(),
                parts: vec![Part::Text {
                    text: format!("Message {}", i),
                    metadata: None,
                }],
                context_id: None,
                task_id: None,
                metadata: None,
                extensions: None,
                reference_task_ids: None,
            })
            .collect();

        let task = Task {
            id: "task-1".to_string(),
            context_id: "ctx-1".to_string(),
            kind: "task".to_string(),
            status: TaskStatus {
                state: TaskState::Working,
                message: None,
                timestamp: None,
            },
            artifacts: None,
            history: Some(messages),
            metadata: None,
        };

        let limited_task = apply_history_length(task, Some(5));
        let history = limited_task.history.unwrap();
        assert_eq!(history.len(), 5);
        // Verify it's the LAST 5 messages
        assert_eq!(history[0].message_id, "msg-5");
        assert_eq!(history[4].message_id, "msg-9");
    }
}
