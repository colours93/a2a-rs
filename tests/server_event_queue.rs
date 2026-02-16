//! Tests for EventQueue, EventConsumer, and InMemoryQueueManager — ported from
//! Python SDK's tests/server/events/ directory.

use a2a_rs::error::A2AError;
use a2a_rs::server::{EventConsumer, EventQueue, InMemoryQueueManager, QueueManager};
use a2a_rs::types::*;

// ============================================================
// EventQueue tests
// ============================================================

#[test]
fn test_event_queue_new() {
    let queue = EventQueue::new(128);
    assert!(!queue.is_closed());
    assert_eq!(queue.subscriber_count(), 0);
}

#[test]
fn test_event_queue_default_capacity() {
    let queue = EventQueue::with_default_capacity();
    assert!(!queue.is_closed());
}

#[test]
fn test_event_queue_default_trait() {
    let queue = EventQueue::default();
    assert!(!queue.is_closed());
}

#[test]
#[should_panic(expected = "capacity must be greater than 0")]
fn test_event_queue_zero_capacity_panics() {
    let _ = EventQueue::new(0);
}

#[test]
fn test_event_queue_subscribe_increments_count() {
    let queue = EventQueue::new(64);
    assert_eq!(queue.subscriber_count(), 0);
    let _rx1 = queue.subscribe();
    assert_eq!(queue.subscriber_count(), 1);
    let _rx2 = queue.subscribe();
    assert_eq!(queue.subscriber_count(), 2);
}

#[tokio::test]
async fn test_event_queue_enqueue_and_receive() {
    let queue = EventQueue::new(64);
    let mut rx = queue.subscribe();

    let event = StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
        task_id: "t1".to_string(),
        context_id: "c1".to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: None,
        },
        r#final: false,
        metadata: None,
    });

    queue.enqueue_event(event).await.unwrap();

    let received = rx.try_recv().unwrap();
    match received {
        StreamResponse::StatusUpdate(u) => {
            assert_eq!(u.task_id, "t1");
            assert_eq!(u.status.state, TaskState::Working);
        }
        _ => panic!("Expected StatusUpdate"),
    }
}

#[tokio::test]
async fn test_event_queue_publish_sync() {
    let queue = EventQueue::new(64);
    let mut rx = queue.subscribe();

    let event = StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
        task_id: "t1".to_string(),
        context_id: "c1".to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus {
            state: TaskState::Completed,
            message: None,
            timestamp: None,
        },
        r#final: true,
        metadata: None,
    });

    queue.publish(event).unwrap();
    let received = rx.try_recv().unwrap();
    match received {
        StreamResponse::StatusUpdate(u) => assert!(u.r#final),
        _ => panic!("Expected StatusUpdate"),
    }
}

#[tokio::test]
async fn test_event_queue_close() {
    let queue = EventQueue::new(64);
    assert!(!queue.is_closed());
    queue.close().await;
    assert!(queue.is_closed());
}

#[tokio::test]
async fn test_closed_queue_drops_enqueue() {
    let queue = EventQueue::new(64);
    let mut rx = queue.subscribe();

    queue.close().await;

    let event = StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
        task_id: "t1".to_string(),
        context_id: "c1".to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: None,
        },
        r#final: false,
        metadata: None,
    });

    // Should not error, just silently drop
    queue.enqueue_event(event).await.unwrap();
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn test_closed_queue_drops_publish() {
    let queue = EventQueue::new(64);
    let mut rx = queue.subscribe();
    queue.close().await;

    let event = StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
        task_id: "t1".to_string(),
        context_id: "c1".to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: None,
        },
        r#final: false,
        metadata: None,
    });

    queue.publish(event).unwrap();
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn test_event_queue_multiple_subscribers() {
    let queue = EventQueue::new(64);
    let mut rx1 = queue.subscribe();
    let mut rx2 = queue.subscribe();

    let event = StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
        task_id: "t1".to_string(),
        context_id: "c1".to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: None,
        },
        r#final: false,
        metadata: None,
    });

    queue.enqueue_event(event).await.unwrap();

    assert!(rx1.try_recv().is_ok());
    assert!(rx2.try_recv().is_ok());
}

#[tokio::test]
async fn test_event_queue_no_subscribers_ok() {
    // Publishing with no subscribers should not error
    let queue = EventQueue::new(64);
    let event = StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
        task_id: "t1".to_string(),
        context_id: "c1".to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: None,
        },
        r#final: false,
        metadata: None,
    });

    queue.enqueue_event(event).await.unwrap();
    queue
        .publish(StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "c1".to_string(),
            kind: "status-update".to_string(),
            status: TaskStatus {
                state: TaskState::Working,
                message: None,
                timestamp: None,
            },
            r#final: false,
            metadata: None,
        }))
        .unwrap();
}

// ---- Tap / child queue tests ----

#[tokio::test]
async fn test_event_queue_tap_creates_child() {
    let parent = EventQueue::new(64);
    let child = parent.tap().await;
    let mut child_rx = child.subscribe();

    let event = StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
        task_id: "t1".to_string(),
        context_id: "c1".to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: None,
        },
        r#final: false,
        metadata: None,
    });

    parent.enqueue_event(event).await.unwrap();

    // Child should receive the event
    let received = child_rx.try_recv().unwrap();
    match received {
        StreamResponse::StatusUpdate(u) => assert_eq!(u.task_id, "t1"),
        _ => panic!("Expected StatusUpdate"),
    }
}

#[tokio::test]
async fn test_close_parent_closes_children() {
    let parent = EventQueue::new(64);
    let child = parent.tap().await;

    assert!(!child.is_closed());
    parent.close().await;
    assert!(parent.is_closed());
    assert!(child.is_closed());
}

#[tokio::test]
async fn test_tap_multiple_children() {
    let parent = EventQueue::new(64);
    let child1 = parent.tap().await;
    let child2 = parent.tap().await;
    let mut rx1 = child1.subscribe();
    let mut rx2 = child2.subscribe();

    let event = StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
        task_id: "t1".to_string(),
        context_id: "c1".to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: None,
        },
        r#final: false,
        metadata: None,
    });

    parent.enqueue_event(event).await.unwrap();

    assert!(rx1.try_recv().is_ok());
    assert!(rx2.try_recv().is_ok());
}

// ============================================================
// EventConsumer tests
// ============================================================

fn make_status_event(task_id: &str, state: TaskState, is_final: bool) -> StreamResponse {
    StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
        task_id: task_id.to_string(),
        context_id: "c1".to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus {
            state,
            message: None,
            timestamp: None,
        },
        r#final: is_final,
        metadata: None,
    })
}

#[tokio::test]
async fn test_event_consumer_consume_one() {
    let queue = EventQueue::new(64);
    let mut consumer = EventConsumer::new(queue.clone());

    // Enqueue AFTER consumer is created (broadcast only delivers to existing subscribers)
    queue
        .enqueue_event(make_status_event("t1", TaskState::Working, false))
        .await
        .unwrap();

    let event = consumer.consume_one().await.unwrap();
    match event {
        StreamResponse::StatusUpdate(u) => assert_eq!(u.task_id, "t1"),
        _ => panic!("Expected StatusUpdate"),
    }
}

#[tokio::test]
async fn test_event_consumer_consume_one_empty() {
    let queue = EventQueue::new(64);
    let mut consumer = EventConsumer::new(queue);
    let result = consumer.consume_one().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_event_consumer_consume_all_stops_on_final() {
    let queue = EventQueue::new(64);
    let mut consumer = EventConsumer::new(queue.clone());

    // Spawn a task to enqueue events after consumer exists
    let q = queue.clone();
    tokio::spawn(async move {
        q.enqueue_event(make_status_event("t1", TaskState::Working, false))
            .await
            .unwrap();
        q.enqueue_event(make_status_event("t1", TaskState::Completed, true))
            .await
            .unwrap();
    });

    let events = consumer.consume_all().await;

    assert_eq!(events.len(), 2);
    assert!(queue.is_closed());
}

#[tokio::test]
async fn test_event_consumer_consume_all_message_is_final() {
    let queue = EventQueue::new(64);
    let mut consumer = EventConsumer::new(queue.clone());

    let q = queue.clone();
    tokio::spawn(async move {
        let msg_event = StreamResponse::Message(Message::agent("m1", "done"));
        q.enqueue_event(msg_event).await.unwrap();
    });

    let events = consumer.consume_all().await;

    assert_eq!(events.len(), 1);
    assert!(queue.is_closed());
}

#[tokio::test]
async fn test_event_consumer_next_event() {
    let queue = EventQueue::new(64);
    let mut consumer = EventConsumer::new(queue.clone());

    let q = queue.clone();
    tokio::spawn(async move {
        q.enqueue_event(make_status_event("t1", TaskState::Working, false))
            .await
            .unwrap();
        q.enqueue_event(make_status_event("t1", TaskState::Completed, true))
            .await
            .unwrap();
    });

    let e1 = consumer.next_event().await;
    assert!(e1.is_some());

    let e2 = consumer.next_event().await;
    assert!(e2.is_some());
}

#[tokio::test]
async fn test_event_consumer_set_exception_stops_consume_all() {
    let queue = EventQueue::new(64);
    let consumer = EventConsumer::new(queue.clone());

    // Set exception before consuming
    consumer
        .set_exception(A2AError::InternalError {
            message: "agent crashed".to_string(),
            data: None,
        })
        .await;

    // Need a new consumer since we consumed the rx
    let mut consumer2 = EventConsumer::new(queue);
    // Set exception on consumer2
    consumer2
        .set_exception(A2AError::InternalError {
            message: "agent crashed".to_string(),
            data: None,
        })
        .await;

    let events = consumer2.consume_all().await;
    assert!(events.is_empty());
}

#[tokio::test]
async fn test_event_consumer_set_exception_stops_next_event() {
    let queue = EventQueue::new(64);
    let mut consumer = EventConsumer::new(queue);

    consumer
        .set_exception(A2AError::InternalError {
            message: "fail".to_string(),
            data: None,
        })
        .await;

    let event = consumer.next_event().await;
    assert!(event.is_none());
}

#[tokio::test]
async fn test_event_consumer_exception_handle() {
    let queue = EventQueue::new(64);
    let consumer = EventConsumer::new(queue);

    let handle = consumer.exception_handle();
    {
        let mut exc = handle.lock().await;
        *exc = Some(A2AError::InternalError {
            message: "external error".to_string(),
            data: None,
        });
    }

    // The consumer should see the exception now
    let handle = consumer.exception_handle();
    let exc = handle.lock().await;
    assert!(exc.is_some());
}

// ---- is_final_event tests ----

#[tokio::test]
async fn test_artifact_update_is_not_final() {
    let queue = EventQueue::new(64);
    let mut consumer = EventConsumer::new(queue.clone());

    let q = queue.clone();
    tokio::spawn(async move {
        let event = StreamResponse::ArtifactUpdate(TaskArtifactUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "c1".to_string(),
            kind: "artifact-update".to_string(),
            artifact: Artifact {
                artifact_id: "a1".to_string(),
                parts: vec![Part::text("content")],
                name: None,
                description: None,
                metadata: None,
                extensions: None,
            },
            append: None,
            last_chunk: None,
            metadata: None,
        });
        q.enqueue_event(event).await.unwrap();
        q.enqueue_event(make_status_event("t1", TaskState::Completed, true))
            .await
            .unwrap();
    });

    let events = consumer.consume_all().await;
    // Should have both events — artifact is not final, completed is
    assert_eq!(events.len(), 2);
}

// ============================================================
// InMemoryQueueManager tests
// ============================================================

#[tokio::test]
async fn test_queue_manager_add_and_get() {
    let mgr = InMemoryQueueManager::new();
    let queue = EventQueue::new(64);

    mgr.add("t1", queue).await.unwrap();
    let retrieved = mgr.get("t1").await;
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_queue_manager_get_nonexistent() {
    let mgr = InMemoryQueueManager::new();
    assert!(mgr.get("nope").await.is_none());
}

#[tokio::test]
async fn test_queue_manager_add_duplicate_errors() {
    let mgr = InMemoryQueueManager::new();
    let q1 = EventQueue::new(64);
    let q2 = EventQueue::new(64);

    mgr.add("t1", q1).await.unwrap();
    let result = mgr.add("t1", q2).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_queue_manager_tap() {
    let mgr = InMemoryQueueManager::new();
    let queue = EventQueue::new(64);
    mgr.add("t1", queue).await.unwrap();

    let child = mgr.tap("t1").await;
    assert!(child.is_some());
}

#[tokio::test]
async fn test_queue_manager_tap_nonexistent() {
    let mgr = InMemoryQueueManager::new();
    assert!(mgr.tap("nope").await.is_none());
}

#[tokio::test]
async fn test_queue_manager_close() {
    let mgr = InMemoryQueueManager::new();
    let queue = EventQueue::new(64);
    mgr.add("t1", queue).await.unwrap();

    mgr.close("t1").await.unwrap();
    // Should be removed
    assert!(mgr.get("t1").await.is_none());
}

#[tokio::test]
async fn test_queue_manager_close_nonexistent() {
    let mgr = InMemoryQueueManager::new();
    let result = mgr.close("nope").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_queue_manager_create_or_tap_creates_new() {
    let mgr = InMemoryQueueManager::new();

    let queue = mgr.create_or_tap("t1").await;
    assert!(!queue.is_closed());

    // Should now exist
    assert!(mgr.get("t1").await.is_some());
}

#[tokio::test]
async fn test_queue_manager_create_or_tap_taps_existing() {
    let mgr = InMemoryQueueManager::new();
    let original = EventQueue::new(64);
    mgr.add("t1", original).await.unwrap();

    // create_or_tap should tap the existing queue
    let child = mgr.create_or_tap("t1").await;
    let mut child_rx = child.subscribe();

    // Publish on the original — child should receive via tap
    let parent = mgr.get("t1").await.unwrap();
    parent
        .enqueue_event(make_status_event("t1", TaskState::Working, false))
        .await
        .unwrap();

    assert!(child_rx.try_recv().is_ok());
}

#[tokio::test]
async fn test_queue_manager_multiple_tasks() {
    let mgr = InMemoryQueueManager::new();
    mgr.add("t1", EventQueue::new(64)).await.unwrap();
    mgr.add("t2", EventQueue::new(64)).await.unwrap();
    mgr.add("t3", EventQueue::new(64)).await.unwrap();

    assert!(mgr.get("t1").await.is_some());
    assert!(mgr.get("t2").await.is_some());
    assert!(mgr.get("t3").await.is_some());

    mgr.close("t2").await.unwrap();
    assert!(mgr.get("t1").await.is_some());
    assert!(mgr.get("t2").await.is_none());
    assert!(mgr.get("t3").await.is_some());
}

#[tokio::test]
async fn test_queue_manager_close_actually_closes_queue() {
    let mgr = InMemoryQueueManager::new();
    let queue = EventQueue::new(64);
    let queue_clone = queue.clone();
    mgr.add("t1", queue).await.unwrap();

    mgr.close("t1").await.unwrap();
    // The original queue should be closed
    assert!(queue_clone.is_closed());
}
