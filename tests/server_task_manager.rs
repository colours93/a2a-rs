//! Tests for TaskManager — ported from Python SDK's
//! tests/server/tasks/test_task_manager.py

use a2a_rs::server::task_manager::{append_artifact_to_task, TaskEvent, TaskManager};
use a2a_rs::server::task_store::InMemoryTaskStore;
use a2a_rs::server::TaskStore;
use a2a_rs::types::*;

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

fn make_task_with_state(id: &str, ctx: &str, state: TaskState) -> Task {
    let mut t = make_task(id, ctx);
    t.status.state = state;
    t
}

fn make_status_event(task_id: &str, ctx_id: &str, state: TaskState) -> TaskStatusUpdateEvent {
    TaskStatusUpdateEvent {
        task_id: task_id.to_string(),
        context_id: ctx_id.to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus {
            state,
            message: None,
            timestamp: None,
        },
        r#final: matches!(
            state,
            TaskState::Completed | TaskState::Failed | TaskState::Canceled | TaskState::Rejected
        ),
        metadata: None,
    }
}

fn make_artifact_event(task_id: &str, ctx_id: &str, artifact_id: &str) -> TaskArtifactUpdateEvent {
    TaskArtifactUpdateEvent {
        task_id: task_id.to_string(),
        context_id: ctx_id.to_string(),
        kind: "artifact-update".to_string(),
        artifact: Artifact {
            artifact_id: artifact_id.to_string(),
            name: Some("test".to_string()),
            description: None,
            parts: vec![Part::text("content")],
            metadata: None,
            extensions: None,
        },
        append: None,
        last_chunk: None,
        metadata: None,
    }
}

// ---- Constructor tests ----

#[test]
fn test_task_manager_invalid_empty_task_id() {
    let store = Box::new(InMemoryTaskStore::new());
    let result = TaskManager::new(Some("".to_string()), Some("ctx1".to_string()), store, None);
    assert!(result.is_err());
}

#[test]
fn test_task_manager_valid_construction() {
    let store = Box::new(InMemoryTaskStore::new());
    let mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    );
    assert!(mgr.is_ok());
    let mgr = mgr.unwrap();
    assert_eq!(mgr.task_id(), Some("t1"));
    assert_eq!(mgr.context_id(), Some("ctx1"));
}

#[test]
fn test_task_manager_none_task_id() {
    let store = Box::new(InMemoryTaskStore::new());
    let mgr = TaskManager::new(None, None, store, None).unwrap();
    assert_eq!(mgr.task_id(), None);
    assert_eq!(mgr.context_id(), None);
}

// ---- get_task tests ----

#[tokio::test]
async fn test_get_task_existing() {
    let store = Box::new(InMemoryTaskStore::new());
    let task = make_task("t1", "ctx1");
    store.save(task.clone()).await.unwrap();

    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    let result = mgr.get_task().await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().id, "t1");
}

#[tokio::test]
async fn test_get_task_nonexistent() {
    let store = Box::new(InMemoryTaskStore::new());
    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    let result = mgr.get_task().await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_task_no_task_id() {
    let store = Box::new(InMemoryTaskStore::new());
    let mut mgr = TaskManager::new(None, Some("ctx1".to_string()), store, None).unwrap();
    let result = mgr.get_task().await.unwrap();
    assert!(result.is_none());
}

// ---- save_task_event tests ----

#[tokio::test]
async fn test_save_task_event_new_task() {
    let store = Box::new(InMemoryTaskStore::new());
    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    let task = make_task("t1", "ctx1");
    let result = mgr
        .save_task_event(TaskEvent::Task(task.clone()))
        .await
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().id, "t1");
}

#[tokio::test]
async fn test_save_task_event_status_update() {
    let store = Box::new(InMemoryTaskStore::new());
    let task = make_task("t1", "ctx1");
    store.save(task).await.unwrap();

    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    let event = make_status_event("t1", "ctx1", TaskState::Working);
    let result = mgr
        .save_task_event(TaskEvent::StatusUpdate(event))
        .await
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().status.state, TaskState::Working);
}

#[tokio::test]
async fn test_save_task_event_artifact_update() {
    let store = Box::new(InMemoryTaskStore::new());
    let task = make_task("t1", "ctx1");
    store.save(task).await.unwrap();

    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    let event = make_artifact_event("t1", "ctx1", "a1");
    let result = mgr
        .save_task_event(TaskEvent::ArtifactUpdate(event))
        .await
        .unwrap();
    assert!(result.is_some());
    let task = result.unwrap();
    assert!(task.artifacts.is_some());
    assert_eq!(task.artifacts.unwrap().len(), 1);
}

#[tokio::test]
async fn test_save_task_event_metadata_update() {
    let store = Box::new(InMemoryTaskStore::new());
    let task = make_task("t1", "ctx1");
    store.save(task).await.unwrap();

    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    let mut event = make_status_event("t1", "ctx1", TaskState::Working);
    event.metadata = Some(serde_json::json!({"meta_key": "meta_value"}));

    let result = mgr
        .save_task_event(TaskEvent::StatusUpdate(event))
        .await
        .unwrap();
    let task = result.unwrap();
    assert_eq!(task.metadata.unwrap()["meta_key"], "meta_value");
}

#[tokio::test]
async fn test_save_task_event_mismatched_task_id() {
    let store = Box::new(InMemoryTaskStore::new());
    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    let task = make_task("wrong-id", "ctx1");
    let result = mgr.save_task_event(TaskEvent::Task(task)).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_save_task_event_mismatched_context_id() {
    let store = Box::new(InMemoryTaskStore::new());
    let task = make_task("t1", "ctx1");
    store.save(task).await.unwrap();

    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    let event = make_status_event("t1", "wrong-ctx", TaskState::Working);
    let result = mgr.save_task_event(TaskEvent::StatusUpdate(event)).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_save_task_event_no_task_id_in_manager() {
    let store = Box::new(InMemoryTaskStore::new());
    let mut mgr = TaskManager::new(None, None, store, None).unwrap();

    let task = make_task("new-task", "new-ctx");
    let result = mgr.save_task_event(TaskEvent::Task(task)).await.unwrap();
    assert!(result.is_some());
    assert_eq!(mgr.task_id(), Some("new-task"));
    assert_eq!(mgr.context_id(), Some("new-ctx"));
}

#[tokio::test]
async fn test_save_task_event_creates_task_for_status_event() {
    let store = Box::new(InMemoryTaskStore::new());
    let mut mgr = TaskManager::new(None, None, store, None).unwrap();

    let event = make_status_event("event-task", "event-ctx", TaskState::Completed);
    let result = mgr
        .save_task_event(TaskEvent::StatusUpdate(event))
        .await
        .unwrap();
    assert!(result.is_some());
    let task = result.unwrap();
    assert_eq!(task.id, "event-task");
    assert_eq!(task.status.state, TaskState::Completed);
    assert_eq!(mgr.task_id(), Some("event-task"));
}

// ---- Status message history tests ----

#[tokio::test]
async fn test_status_message_moved_to_history() {
    let store = Box::new(InMemoryTaskStore::new());
    let mut task = make_task("t1", "ctx1");
    task.status.message = Some(Message::agent("m1", "Initial status"));
    store.save(task).await.unwrap();

    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    let mut event = make_status_event("t1", "ctx1", TaskState::Working);
    event.status.message = Some(Message::agent("m2", "Working now"));
    let result = mgr
        .save_task_event(TaskEvent::StatusUpdate(event))
        .await
        .unwrap();
    let task = result.unwrap();

    // The old status message should be in history
    let history = task.history.unwrap();
    assert!(history.iter().any(|m| m.message_id == "m1"));
    // The new message should be the current status
    assert_eq!(task.status.message.unwrap().message_id, "m2");
}

// ---- update_with_message tests ----

#[test]
fn test_update_with_message_moves_status_to_history() {
    let store = Box::new(InMemoryTaskStore::new());
    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    let mut task = make_task("t1", "ctx1");
    task.status.message = Some(Message::agent("m1", "Status msg"));
    let new_msg = Message::user("m2", "User input");
    mgr.update_with_message(new_msg, &mut task);

    assert!(task.status.message.is_none());
    let history = task.history.as_ref().unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].message_id, "m1");
    assert_eq!(history[1].message_id, "m2");
}

#[test]
fn test_update_with_message_no_existing_status_message() {
    let store = Box::new(InMemoryTaskStore::new());
    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    let mut task = make_task("t1", "ctx1");
    let new_msg = Message::user("m1", "First message");
    mgr.update_with_message(new_msg, &mut task);

    let history = task.history.as_ref().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].message_id, "m1");
}

// ---- process (StreamResponse) tests ----

#[tokio::test]
async fn test_process_status_update() {
    let store = Box::new(InMemoryTaskStore::new());
    let task = make_task("t1", "ctx1");
    store.save(task).await.unwrap();

    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    let event = StreamResponse::StatusUpdate(make_status_event("t1", "ctx1", TaskState::Working));
    let result = mgr.process(event).await.unwrap();
    match result {
        StreamResponse::StatusUpdate(update) => {
            assert_eq!(update.status.state, TaskState::Working);
        }
        _ => panic!("Expected StatusUpdate"),
    }
}

#[tokio::test]
async fn test_process_artifact_update() {
    let store = Box::new(InMemoryTaskStore::new());
    let task = make_task("t1", "ctx1");
    store.save(task).await.unwrap();

    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    let event = StreamResponse::ArtifactUpdate(make_artifact_event("t1", "ctx1", "a1"));
    let result = mgr.process(event).await.unwrap();
    match result {
        StreamResponse::ArtifactUpdate(update) => {
            assert_eq!(update.artifact.artifact_id, "a1");
        }
        _ => panic!("Expected ArtifactUpdate"),
    }
}

#[tokio::test]
async fn test_process_message_passthrough() {
    let store = Box::new(InMemoryTaskStore::new());
    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    let msg = Message::agent("m1", "Direct message");
    let event = StreamResponse::Message(msg.clone());
    let result = mgr.process(event).await.unwrap();
    match result {
        StreamResponse::Message(m) => {
            assert_eq!(m.message_id, "m1");
        }
        _ => panic!("Expected Message"),
    }
}

// ---- append_artifact_to_task tests (already partially covered in task_manager module) ----

#[test]
fn test_append_artifact_new_artifact() {
    let mut task = make_task("t1", "ctx1");
    let event = TaskArtifactUpdateEvent {
        task_id: "t1".to_string(),
        context_id: "ctx1".to_string(),
        kind: "artifact-update".to_string(),
        artifact: Artifact {
            artifact_id: "a1".to_string(),
            name: None,
            description: None,
            parts: vec![Part::text("hello")],
            metadata: None,
            extensions: None,
        },
        append: None,
        last_chunk: None,
        metadata: None,
    };

    append_artifact_to_task(&mut task, &event);
    assert_eq!(task.artifacts.as_ref().unwrap().len(), 1);
}

#[test]
fn test_append_artifact_replace_existing() {
    let mut task = make_task("t1", "ctx1");
    task.artifacts = Some(vec![Artifact {
        artifact_id: "a1".to_string(),
        name: None,
        description: None,
        parts: vec![Part::text("old")],
        metadata: None,
        extensions: None,
    }]);

    let event = TaskArtifactUpdateEvent {
        task_id: "t1".to_string(),
        context_id: "ctx1".to_string(),
        kind: "artifact-update".to_string(),
        artifact: Artifact {
            artifact_id: "a1".to_string(),
            name: None,
            description: None,
            parts: vec![Part::text("new")],
            metadata: None,
            extensions: None,
        },
        append: Some(false),
        last_chunk: None,
        metadata: None,
    };

    append_artifact_to_task(&mut task, &event);
    assert_eq!(task.artifacts.as_ref().unwrap().len(), 1);
    match &task.artifacts.as_ref().unwrap()[0].parts[0] {
        Part::Text { text, .. } => assert_eq!(text, "new"),
        _ => panic!("Expected text part"),
    }
}

#[test]
fn test_append_artifact_append_parts() {
    let mut task = make_task("t1", "ctx1");
    task.artifacts = Some(vec![Artifact {
        artifact_id: "a1".to_string(),
        name: None,
        description: None,
        parts: vec![Part::text("part1")],
        metadata: None,
        extensions: None,
    }]);

    let event = TaskArtifactUpdateEvent {
        task_id: "t1".to_string(),
        context_id: "ctx1".to_string(),
        kind: "artifact-update".to_string(),
        artifact: Artifact {
            artifact_id: "a1".to_string(),
            name: None,
            description: None,
            parts: vec![Part::text("part2")],
            metadata: None,
            extensions: None,
        },
        append: Some(true),
        last_chunk: None,
        metadata: None,
    };

    append_artifact_to_task(&mut task, &event);
    assert_eq!(task.artifacts.as_ref().unwrap().len(), 1);
    assert_eq!(task.artifacts.as_ref().unwrap()[0].parts.len(), 2);
}

#[test]
fn test_append_artifact_nonexistent_ignored() {
    let mut task = make_task("t1", "ctx1");
    task.artifacts = Some(vec![]);

    let event = TaskArtifactUpdateEvent {
        task_id: "t1".to_string(),
        context_id: "ctx1".to_string(),
        kind: "artifact-update".to_string(),
        artifact: Artifact {
            artifact_id: "missing".to_string(),
            name: None,
            description: None,
            parts: vec![Part::text("data")],
            metadata: None,
            extensions: None,
        },
        append: Some(true),
        last_chunk: None,
        metadata: None,
    };

    append_artifact_to_task(&mut task, &event);
    assert!(task.artifacts.as_ref().unwrap().is_empty());
}

// ---- Initial message in history ----

#[tokio::test]
async fn test_task_manager_with_initial_message() {
    let store = Box::new(InMemoryTaskStore::new());
    let initial_msg = Message::user("m1", "Hello agent");
    let mut mgr = TaskManager::new(None, None, store, Some(initial_msg)).unwrap();

    // Create task via status event — should include initial message in history
    let event = make_status_event("t1", "ctx1", TaskState::Working);
    let result = mgr
        .save_task_event(TaskEvent::StatusUpdate(event))
        .await
        .unwrap();
    let task = result.unwrap();

    // Initial message should be in history
    let history = task.history.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].message_id, "m1");
}

// ---- Multiple status updates ----

#[tokio::test]
async fn test_multiple_status_transitions() {
    let store = Box::new(InMemoryTaskStore::new());
    let task = make_task("t1", "ctx1");
    store.save(task).await.unwrap();

    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    // Submitted -> Working
    let event = make_status_event("t1", "ctx1", TaskState::Working);
    mgr.save_task_event(TaskEvent::StatusUpdate(event))
        .await
        .unwrap();

    // Working -> Completed
    let event = make_status_event("t1", "ctx1", TaskState::Completed);
    let result = mgr
        .save_task_event(TaskEvent::StatusUpdate(event))
        .await
        .unwrap();
    let task = result.unwrap();
    assert_eq!(task.status.state, TaskState::Completed);
}

// ---- Metadata merging ----

#[tokio::test]
async fn test_metadata_merge_across_events() {
    let store = Box::new(InMemoryTaskStore::new());
    let mut task = make_task("t1", "ctx1");
    task.metadata = Some(serde_json::json!({"existing": "value"}));
    store.save(task).await.unwrap();

    let mut mgr = TaskManager::new(
        Some("t1".to_string()),
        Some("ctx1".to_string()),
        store,
        None,
    )
    .unwrap();

    let mut event = make_status_event("t1", "ctx1", TaskState::Working);
    event.metadata = Some(serde_json::json!({"new_key": "new_value"}));
    let result = mgr
        .save_task_event(TaskEvent::StatusUpdate(event))
        .await
        .unwrap();
    let task = result.unwrap();
    let metadata = task.metadata.unwrap();
    assert_eq!(metadata["existing"], "value");
    assert_eq!(metadata["new_key"], "new_value");
}
