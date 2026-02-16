//! Port of Python SDK tests/client/test_client_task_manager.py
//!
//! Tests for client-side task state management — tracking tasks, applying
//! status updates, and managing artifacts.
//!
//! Python's ClientTaskManager is a client-side state accumulator. The Rust SDK
//! doesn't have an exact equivalent class, but we test the same behaviors
//! using Task type manipulation and SendMessageResponse parsing.
//!
//! Some Python tests (mock-based save_task_event, process) are skipped as they
//! test Python-specific class internals.

use a2a_rs::types::*;

// ============================================================================
// Task construction and field access
// ============================================================================

#[test]
fn test_task_construction() {
    let task = Task {
        id: "task123".to_string(),
        context_id: "context456".to_string(),
        kind: "task".to_string(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: None,
        },
        history: None,
        artifacts: None,
        metadata: None,
    };

    assert_eq!(task.id, "task123");
    assert_eq!(task.context_id, "context456");
    assert_eq!(task.status.state, TaskState::Working);
}

// ============================================================================
// TaskStatus with message
// ============================================================================

#[test]
fn test_task_status_with_message() {
    let message = Message {
        message_id: "msg1".to_string(),
        role: Role::Agent,
        kind: "message".to_string(),
        parts: vec![Part::text("Status update")],
        context_id: None,
        task_id: None,
        reference_task_ids: None,
        metadata: None,
        extensions: None,
    };

    let status = TaskStatus {
        state: TaskState::Completed,
        message: Some(message),
        timestamp: None,
    };

    assert_eq!(status.state, TaskState::Completed);
    assert!(status.message.is_some());
    let msg = status.message.unwrap();
    assert_eq!(msg.role, Role::Agent);
}

// ============================================================================
// TaskStatusUpdateEvent deserialization
// ============================================================================

#[test]
fn test_status_update_event_deserialization() {
    let json = serde_json::json!({
        "kind": "status-update",
        "taskId": "task123",
        "contextId": "context456",
        "status": {
            "state": "completed",
            "message": {
                "kind": "message",
                "messageId": "msg1",
                "role": "agent",
                "parts": [{"kind": "text", "text": "Done!"}]
            }
        },
        "final": true
    });

    let event: TaskStatusUpdateEvent = serde_json::from_value(json).unwrap();
    assert_eq!(event.task_id, "task123");
    assert_eq!(event.context_id, "context456");
    assert_eq!(event.status.state, TaskState::Completed);
    assert!(event.status.message.is_some());
    assert_eq!(event.r#final, true);
}

#[test]
fn test_status_update_creates_task_if_not_exists() {
    // Equivalent: test_save_task_event_creates_task_if_not_exists
    // Build a task from a status update event
    let event = TaskStatusUpdateEvent {
        task_id: "new_task".to_string(),
        context_id: "new_context".to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: None,
        },
        r#final: false,
        metadata: None,
    };

    // Create a task from the update
    let task = Task {
        id: event.task_id.clone(),
        context_id: event.context_id.clone(),
        kind: "task".to_string(),
        status: event.status.clone(),
        history: None,
        artifacts: None,
        metadata: None,
    };

    assert_eq!(task.id, "new_task");
    assert_eq!(task.context_id, "new_context");
    assert_eq!(task.status.state, TaskState::Working);
}

// ============================================================================
// TaskArtifactUpdateEvent deserialization
// ============================================================================

#[test]
fn test_artifact_update_event_deserialization() {
    let json = serde_json::json!({
        "kind": "artifact-update",
        "taskId": "task123",
        "contextId": "context456",
        "artifact": {
            "artifactId": "art1",
            "parts": [{"kind": "text", "text": "artifact content"}]
        }
    });

    let event: TaskArtifactUpdateEvent = serde_json::from_value(json).unwrap();
    assert_eq!(event.task_id, "task123");
    assert_eq!(event.artifact.artifact_id, "art1");
    assert_eq!(event.artifact.parts.len(), 1);
}

// ============================================================================
// Task with history
// ============================================================================

#[test]
fn test_task_with_history() {
    let msg = Message {
        message_id: "msg1".to_string(),
        role: Role::User,
        kind: "message".to_string(),
        parts: vec![Part::text("Hello")],
        context_id: None,
        task_id: None,
        reference_task_ids: None,
        metadata: None,
        extensions: None,
    };

    let task = Task {
        id: "task123".to_string(),
        context_id: "context456".to_string(),
        kind: "task".to_string(),
        status: TaskStatus {
            state: TaskState::Completed,
            message: None,
            timestamp: None,
        },
        history: Some(vec![msg]),
        artifacts: None,
        metadata: None,
    };

    assert_eq!(task.history.as_ref().unwrap().len(), 1);
    assert_eq!(task.history.as_ref().unwrap()[0].message_id, "msg1");
}

// ============================================================================
// Task with artifacts
// ============================================================================

#[test]
fn test_task_with_artifacts() {
    let artifact = Artifact {
        artifact_id: "art1".to_string(),
        name: Some("output".to_string()),
        description: None,
        parts: vec![Part::text("artifact content")],
        metadata: None,
        extensions: None,
    };

    let task = Task {
        id: "task123".to_string(),
        context_id: "context456".to_string(),
        kind: "task".to_string(),
        status: TaskStatus {
            state: TaskState::Completed,
            message: None,
            timestamp: None,
        },
        history: None,
        artifacts: Some(vec![artifact]),
        metadata: None,
    };

    let arts = task.artifacts.as_ref().unwrap();
    assert_eq!(arts.len(), 1);
    assert_eq!(arts[0].artifact_id, "art1");
    assert_eq!(arts[0].name.as_deref(), Some("output"));
}

// ============================================================================
// Status message moved to history (matches test_update_with_message_moves_status_message)
// ============================================================================

#[test]
fn test_status_message_can_be_moved_to_history() {
    let status_message = Message {
        message_id: "status_msg".to_string(),
        role: Role::Agent,
        kind: "message".to_string(),
        parts: vec![Part::text("Status")],
        context_id: None,
        task_id: None,
        reference_task_ids: None,
        metadata: None,
        extensions: None,
    };

    let user_message = Message {
        message_id: "user_msg".to_string(),
        role: Role::User,
        kind: "message".to_string(),
        parts: vec![Part::text("Hello")],
        context_id: None,
        task_id: None,
        reference_task_ids: None,
        metadata: None,
        extensions: None,
    };

    // Simulate: move status message to history, add user message
    let mut history = vec![status_message];
    history.push(user_message);

    let task = Task {
        id: "task123".to_string(),
        context_id: "context456".to_string(),
        kind: "task".to_string(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None, // cleared after moving to history
            timestamp: None,
        },
        history: Some(history),
        artifacts: None,
        metadata: None,
    };

    assert!(task.status.message.is_none());
    assert_eq!(task.history.as_ref().unwrap().len(), 2);
    assert_eq!(task.history.as_ref().unwrap()[0].message_id, "status_msg");
    assert_eq!(task.history.as_ref().unwrap()[1].message_id, "user_msg");
}

// ============================================================================
// SendMessageResponse parsing
// ============================================================================

#[test]
fn test_send_message_response_task() {
    let json = serde_json::json!({
        "kind": "task",
        "id": "task-123",
        "contextId": "ctx-456",
        "status": {"state": "completed"}
    });

    let response: SendMessageResponse = serde_json::from_value(json).unwrap();
    match response {
        SendMessageResponse::Task(task) => {
            assert_eq!(task.id, "task-123");
        }
        _ => panic!("expected Task"),
    }
}

#[test]
fn test_send_message_response_message() {
    let json = serde_json::json!({
        "kind": "message",
        "messageId": "m1",
        "role": "agent",
        "parts": [{"kind": "text", "text": "Hi"}]
    });

    let response: SendMessageResponse = serde_json::from_value(json).unwrap();
    match response {
        SendMessageResponse::Message(msg) => {
            assert_eq!(msg.message_id, "m1");
            assert_eq!(msg.role, Role::Agent);
        }
        _ => panic!("expected Message"),
    }
}
