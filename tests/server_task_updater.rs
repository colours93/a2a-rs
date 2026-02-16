//! Tests for TaskUpdater — ported from Python SDK's
//! tests/server/tasks/test_task_updater.py

use a2a_rs::server::{EventQueue, TaskUpdater};
use a2a_rs::types::*;

fn make_updater() -> (TaskUpdater, EventQueue) {
    let queue = EventQueue::new(256);
    let updater = TaskUpdater::new(queue.clone(), "t1".to_string(), "ctx1".to_string());
    (updater, queue)
}

// ---- Basic construction ----

#[tokio::test]
async fn test_task_updater_construction() {
    let (updater, _queue) = make_updater();
    assert_eq!(updater.task_id(), "t1");
    assert_eq!(updater.context_id(), "ctx1");
    assert!(!updater.is_terminal().await);
}

// ---- update_status tests ----

#[tokio::test]
async fn test_update_status_working() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    updater
        .update_status(TaskState::Working, None, false, None)
        .await
        .unwrap();

    let event = rx.try_recv().unwrap();
    match event {
        StreamResponse::StatusUpdate(update) => {
            assert_eq!(update.task_id, "t1");
            assert_eq!(update.context_id, "ctx1");
            assert_eq!(update.status.state, TaskState::Working);
            assert!(!update.r#final);
            assert!(update.status.timestamp.is_some());
        }
        _ => panic!("Expected StatusUpdate"),
    }
}

#[tokio::test]
async fn test_update_status_with_message() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    let msg = Message::agent("m1", "Processing...");
    updater
        .update_status(TaskState::Working, Some(msg), false, None)
        .await
        .unwrap();

    let event = rx.try_recv().unwrap();
    match event {
        StreamResponse::StatusUpdate(update) => {
            let msg = update.status.message.unwrap();
            assert_eq!(msg.role, Role::Agent);
        }
        _ => panic!("Expected StatusUpdate"),
    }
}

#[tokio::test]
async fn test_update_status_with_metadata() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    let metadata = serde_json::json!({"key": "value"});
    updater
        .update_status(TaskState::Working, None, false, Some(metadata.clone()))
        .await
        .unwrap();

    let event = rx.try_recv().unwrap();
    match event {
        StreamResponse::StatusUpdate(update) => {
            assert_eq!(update.metadata.unwrap()["key"], "value");
        }
        _ => panic!("Expected StatusUpdate"),
    }
}

// ---- Terminal state tests ----

#[tokio::test]
async fn test_complete_sets_terminal() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    updater.complete(None).await.unwrap();

    assert!(updater.is_terminal().await);

    let event = rx.try_recv().unwrap();
    match event {
        StreamResponse::StatusUpdate(update) => {
            assert_eq!(update.status.state, TaskState::Completed);
            assert!(update.r#final);
        }
        _ => panic!("Expected StatusUpdate"),
    }
}

#[tokio::test]
async fn test_failed_sets_terminal() {
    let (updater, _queue) = make_updater();
    updater.failed(None).await.unwrap();
    assert!(updater.is_terminal().await);
}

#[tokio::test]
async fn test_cancel_sets_terminal() {
    let (updater, _queue) = make_updater();
    updater.cancel(None).await.unwrap();
    assert!(updater.is_terminal().await);
}

#[tokio::test]
async fn test_reject_sets_terminal() {
    let (updater, _queue) = make_updater();
    updater.reject(None).await.unwrap();
    assert!(updater.is_terminal().await);
}

#[tokio::test]
async fn test_terminal_state_prevents_further_updates() {
    let (updater, _queue) = make_updater();
    updater.complete(None).await.unwrap();

    // Further updates should fail
    let result = updater
        .update_status(TaskState::Working, None, false, None)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_complete_then_fail_rejected() {
    let (updater, _queue) = make_updater();
    updater.complete(None).await.unwrap();
    let result = updater.failed(None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_complete_then_cancel_rejected() {
    let (updater, _queue) = make_updater();
    updater.complete(None).await.unwrap();
    let result = updater.cancel(None).await;
    assert!(result.is_err());
}

// ---- Non-terminal states ----

#[tokio::test]
async fn test_submit_is_not_terminal() {
    let (updater, _queue) = make_updater();
    updater.submit(None).await.unwrap();
    assert!(!updater.is_terminal().await);
}

#[tokio::test]
async fn test_working_is_not_terminal() {
    let (updater, _queue) = make_updater();
    updater.start_work(None).await.unwrap();
    assert!(!updater.is_terminal().await);
}

#[tokio::test]
async fn test_input_required_is_not_terminal() {
    let (updater, _queue) = make_updater();
    updater.requires_input(None, false).await.unwrap();
    assert!(!updater.is_terminal().await);
}

#[tokio::test]
async fn test_auth_required_is_not_terminal() {
    let (updater, _queue) = make_updater();
    updater.requires_auth(None, false).await.unwrap();
    assert!(!updater.is_terminal().await);
}

// ---- Multiple transitions ----

#[tokio::test]
async fn test_working_then_complete() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    updater.start_work(None).await.unwrap();
    updater.complete(None).await.unwrap();

    let event1 = rx.try_recv().unwrap();
    let event2 = rx.try_recv().unwrap();

    match event1 {
        StreamResponse::StatusUpdate(u) => assert_eq!(u.status.state, TaskState::Working),
        _ => panic!("Expected StatusUpdate"),
    }
    match event2 {
        StreamResponse::StatusUpdate(u) => {
            assert_eq!(u.status.state, TaskState::Completed);
            assert!(u.r#final);
        }
        _ => panic!("Expected StatusUpdate"),
    }
}

#[tokio::test]
async fn test_multiple_working_updates() {
    let (updater, _queue) = make_updater();
    updater.start_work(None).await.unwrap();
    updater.start_work(None).await.unwrap();
    updater.start_work(None).await.unwrap();
    // Should still not be terminal
    assert!(!updater.is_terminal().await);
}

// ---- Convenience text methods ----

#[tokio::test]
async fn test_complete_with_text() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    updater.complete_with_text("Done!").await.unwrap();

    let event = rx.try_recv().unwrap();
    match event {
        StreamResponse::StatusUpdate(update) => {
            assert_eq!(update.status.state, TaskState::Completed);
            assert!(update.r#final);
            let msg = update.status.message.unwrap();
            assert_eq!(msg.role, Role::Agent);
            match &msg.parts[0] {
                Part::Text { text, .. } => assert_eq!(text, "Done!"),
                _ => panic!("Expected text part"),
            }
        }
        _ => panic!("Expected StatusUpdate"),
    }
}

#[tokio::test]
async fn test_failed_with_text() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    updater.failed_with_text("Error occurred").await.unwrap();

    let event = rx.try_recv().unwrap();
    match event {
        StreamResponse::StatusUpdate(update) => {
            assert_eq!(update.status.state, TaskState::Failed);
            assert!(update.r#final);
        }
        _ => panic!("Expected StatusUpdate"),
    }
}

#[tokio::test]
async fn test_start_work_with_text() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    updater.start_work_with_text("Processing...").await.unwrap();

    let event = rx.try_recv().unwrap();
    match event {
        StreamResponse::StatusUpdate(update) => {
            assert_eq!(update.status.state, TaskState::Working);
            assert!(!update.r#final);
        }
        _ => panic!("Expected StatusUpdate"),
    }
}

// ---- Artifact tests ----

#[tokio::test]
async fn test_add_artifact() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    updater
        .add_artifact(
            vec![Part::text("artifact content")],
            Some("custom-id".to_string()),
            Some("my artifact".to_string()),
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let event = rx.try_recv().unwrap();
    match event {
        StreamResponse::ArtifactUpdate(update) => {
            assert_eq!(update.task_id, "t1");
            assert_eq!(update.context_id, "ctx1");
            assert_eq!(update.artifact.artifact_id, "custom-id");
            assert_eq!(update.artifact.name, Some("my artifact".to_string()));
        }
        _ => panic!("Expected ArtifactUpdate"),
    }
}

#[tokio::test]
async fn test_add_artifact_auto_id() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    updater
        .add_artifact(
            vec![Part::text("content")],
            None, // Auto-generate ID
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    let event = rx.try_recv().unwrap();
    match event {
        StreamResponse::ArtifactUpdate(update) => {
            // ID should be a UUID
            assert!(!update.artifact.artifact_id.is_empty());
            assert!(update.artifact.artifact_id.contains('-')); // UUID format
        }
        _ => panic!("Expected ArtifactUpdate"),
    }
}

#[tokio::test]
async fn test_add_artifact_with_append() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    updater
        .add_artifact(
            vec![Part::text("chunk 1")],
            Some("a1".to_string()),
            None,
            None,
            Some(false),
            None,
            None,
        )
        .await
        .unwrap();

    updater
        .add_artifact(
            vec![Part::text("chunk 2")],
            Some("a1".to_string()),
            None,
            None,
            Some(true),
            Some(true),
            None,
        )
        .await
        .unwrap();

    let event1 = rx.try_recv().unwrap();
    let event2 = rx.try_recv().unwrap();

    match event1 {
        StreamResponse::ArtifactUpdate(u) => {
            assert_eq!(u.append, Some(false));
        }
        _ => panic!("Expected ArtifactUpdate"),
    }
    match event2 {
        StreamResponse::ArtifactUpdate(u) => {
            assert_eq!(u.append, Some(true));
            assert_eq!(u.last_chunk, Some(true));
        }
        _ => panic!("Expected ArtifactUpdate"),
    }
}

#[tokio::test]
async fn test_add_artifact_after_terminal_state_succeeds() {
    // Per Python SDK, add_artifact does NOT check terminal state
    let (updater, queue) = make_updater();
    let _rx = queue.subscribe();

    updater.complete(None).await.unwrap();

    // Should succeed even though task is terminal
    let result = updater
        .add_artifact(
            vec![Part::text("late artifact")],
            Some("a1".to_string()),
            None,
            None,
            None,
            None,
            None,
        )
        .await;
    assert!(result.is_ok());
}

// ---- new_agent_message tests ----

#[test]
fn test_new_agent_message() {
    let queue = EventQueue::new(256);
    let updater = TaskUpdater::new(queue, "t1".to_string(), "ctx1".to_string());

    let msg = updater.new_agent_message(vec![Part::text("hello")], None);

    assert_eq!(msg.role, Role::Agent);
    assert!(!msg.message_id.is_empty());
    assert_eq!(msg.context_id, Some("ctx1".to_string()));
    assert_eq!(msg.task_id, Some("t1".to_string()));
    assert_eq!(msg.parts.len(), 1);
}

#[test]
fn test_new_agent_message_with_metadata() {
    let queue = EventQueue::new(256);
    let updater = TaskUpdater::new(queue, "t1".to_string(), "ctx1".to_string());

    let metadata = serde_json::json!({"key": "value"});
    let msg = updater.new_agent_message(vec![Part::text("hello")], Some(metadata.clone()));

    assert_eq!(msg.metadata, Some(metadata));
}

// ---- Final flag behavior ----

#[tokio::test]
async fn test_terminal_state_forces_final_true() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    // Even if we pass final=false, terminal states should force final=true
    updater
        .update_status(TaskState::Completed, None, false, None)
        .await
        .unwrap();

    let event = rx.try_recv().unwrap();
    match event {
        StreamResponse::StatusUpdate(update) => {
            assert!(update.r#final);
        }
        _ => panic!("Expected StatusUpdate"),
    }
}

#[tokio::test]
async fn test_non_terminal_preserves_final_false() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    updater
        .update_status(TaskState::Working, None, false, None)
        .await
        .unwrap();

    let event = rx.try_recv().unwrap();
    match event {
        StreamResponse::StatusUpdate(update) => {
            assert!(!update.r#final);
        }
        _ => panic!("Expected StatusUpdate"),
    }
}

#[tokio::test]
async fn test_input_required_with_final_true() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    updater.requires_input(None, true).await.unwrap();

    let event = rx.try_recv().unwrap();
    match event {
        StreamResponse::StatusUpdate(update) => {
            assert_eq!(update.status.state, TaskState::InputRequired);
            assert!(update.r#final);
        }
        _ => panic!("Expected StatusUpdate"),
    }
}

// ---- Timestamp tests ----

#[tokio::test]
async fn test_status_update_has_timestamp() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    updater.start_work(None).await.unwrap();

    let event = rx.try_recv().unwrap();
    match event {
        StreamResponse::StatusUpdate(update) => {
            assert!(update.status.timestamp.is_some());
            let ts = update.status.timestamp.unwrap();
            assert!(ts.contains("T")); // ISO 8601 format
        }
        _ => panic!("Expected StatusUpdate"),
    }
}

#[tokio::test]
async fn test_custom_timestamp() {
    let (updater, queue) = make_updater();
    let mut rx = queue.subscribe();

    let custom_ts = "2024-01-01T00:00:00Z".to_string();
    updater
        .update_status_with_timestamp(
            TaskState::Working,
            None,
            false,
            Some(custom_ts.clone()),
            None,
        )
        .await
        .unwrap();

    let event = rx.try_recv().unwrap();
    match event {
        StreamResponse::StatusUpdate(update) => {
            assert_eq!(update.status.timestamp, Some(custom_ts));
        }
        _ => panic!("Expected StatusUpdate"),
    }
}

// ---- Concurrent access tests ----

#[tokio::test]
async fn test_concurrent_updates_terminal_check() {
    let queue = EventQueue::new(256);
    let updater = std::sync::Arc::new(TaskUpdater::new(
        queue,
        "t1".to_string(),
        "ctx1".to_string(),
    ));

    // Start multiple tasks trying to complete
    let mut handles = vec![];
    for _ in 0..5 {
        let updater = updater.clone();
        handles.push(tokio::spawn(async move { updater.complete(None).await }));
    }

    let results: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Exactly one should succeed, the rest should fail
    let successes = results.iter().filter(|r| r.is_ok()).count();
    let failures = results.iter().filter(|r| r.is_err()).count();
    assert_eq!(successes, 1);
    assert_eq!(failures, 4);
}
