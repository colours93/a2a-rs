//! Tests for InMemoryTaskStore — ported from Python SDK's
//! tests/server/tasks/test_inmemory_task_store.py

use a2a_rs::server::task_store::{TaskListParams, TaskListResponse};
use a2a_rs::server::{InMemoryTaskStore, TaskStore};
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
    Task {
        id: id.to_string(),
        context_id: ctx.to_string(),
        kind: "task".to_string(),
        status: TaskStatus {
            state,
            message: None,
            timestamp: None,
        },
        artifacts: None,
        history: None,
        metadata: None,
    }
}

// ---- Basic CRUD tests ----

#[tokio::test]
async fn test_save_and_get_task() {
    let store = InMemoryTaskStore::new();
    let task = make_task("t1", "ctx1");
    store.save(task.clone()).await.unwrap();

    let retrieved = store.get("t1").await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, "t1");
    assert_eq!(retrieved.context_id, "ctx1");
    assert_eq!(retrieved.status.state, TaskState::Submitted);
}

#[tokio::test]
async fn test_get_nonexistent_task() {
    let store = InMemoryTaskStore::new();
    let result = store.get("nonexistent").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_save_overwrites_existing() {
    let store = InMemoryTaskStore::new();
    let task1 = make_task("t1", "ctx1");
    store.save(task1).await.unwrap();

    let task2 = make_task_with_state("t1", "ctx1", TaskState::Working);
    store.save(task2).await.unwrap();

    let retrieved = store.get("t1").await.unwrap().unwrap();
    assert_eq!(retrieved.status.state, TaskState::Working);
}

#[tokio::test]
async fn test_delete_task() {
    let store = InMemoryTaskStore::new();
    let task = make_task("t1", "ctx1");
    store.save(task).await.unwrap();

    store.delete("t1").await.unwrap();
    let result = store.get("t1").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_delete_nonexistent_task() {
    let store = InMemoryTaskStore::new();
    // Should not error
    store.delete("nonexistent").await.unwrap();
}

// ---- List tests ----

#[tokio::test]
async fn test_list_all_tasks() {
    let store = InMemoryTaskStore::new();
    store.save(make_task("t1", "ctx1")).await.unwrap();
    store.save(make_task("t2", "ctx1")).await.unwrap();
    store.save(make_task("t3", "ctx2")).await.unwrap();

    let result = store.list(&TaskListParams::default()).await.unwrap();
    assert_eq!(result.tasks.len(), 3);
}

#[tokio::test]
async fn test_list_empty_store() {
    let store = InMemoryTaskStore::new();
    let result = store.list(&TaskListParams::default()).await.unwrap();
    assert_eq!(result.tasks.len(), 0);
    assert!(result.next_page_token.is_none());
}

#[tokio::test]
async fn test_list_filter_by_context_id() {
    let store = InMemoryTaskStore::new();
    store.save(make_task("t1", "ctx1")).await.unwrap();
    store.save(make_task("t2", "ctx1")).await.unwrap();
    store.save(make_task("t3", "ctx2")).await.unwrap();

    let params = TaskListParams {
        context_id: Some("ctx1".to_string()),
        ..Default::default()
    };
    let result = store.list(&params).await.unwrap();
    assert_eq!(result.tasks.len(), 2);
    assert!(result.tasks.iter().all(|t| t.context_id == "ctx1"));
}

#[tokio::test]
async fn test_list_filter_by_status() {
    let store = InMemoryTaskStore::new();
    store
        .save(make_task_with_state("t1", "ctx1", TaskState::Submitted))
        .await
        .unwrap();
    store
        .save(make_task_with_state("t2", "ctx1", TaskState::Working))
        .await
        .unwrap();
    store
        .save(make_task_with_state("t3", "ctx1", TaskState::Completed))
        .await
        .unwrap();

    let params = TaskListParams {
        status: Some(vec![TaskState::Working, TaskState::Completed]),
        ..Default::default()
    };
    let result = store.list(&params).await.unwrap();
    assert_eq!(result.tasks.len(), 2);
}

#[tokio::test]
async fn test_list_pagination() {
    let store = InMemoryTaskStore::new();
    for i in 0..5 {
        store
            .save(make_task(&format!("t{}", i), "ctx1"))
            .await
            .unwrap();
    }

    // First page
    let params = TaskListParams {
        page_size: Some(2),
        ..Default::default()
    };
    let result = store.list(&params).await.unwrap();
    assert_eq!(result.tasks.len(), 2);
    assert!(result.next_page_token.is_some());

    // Second page
    let params = TaskListParams {
        page_size: Some(2),
        page_token: result.next_page_token,
        ..Default::default()
    };
    let result = store.list(&params).await.unwrap();
    assert_eq!(result.tasks.len(), 2);
    assert!(result.next_page_token.is_some());

    // Third page (last)
    let params = TaskListParams {
        page_size: Some(2),
        page_token: result.next_page_token,
        ..Default::default()
    };
    let result = store.list(&params).await.unwrap();
    assert_eq!(result.tasks.len(), 1);
    assert!(result.next_page_token.is_none());
}

#[tokio::test]
async fn test_list_combined_filters() {
    let store = InMemoryTaskStore::new();
    store
        .save(make_task_with_state("t1", "ctx1", TaskState::Submitted))
        .await
        .unwrap();
    store
        .save(make_task_with_state("t2", "ctx1", TaskState::Working))
        .await
        .unwrap();
    store
        .save(make_task_with_state("t3", "ctx2", TaskState::Working))
        .await
        .unwrap();
    store
        .save(make_task_with_state("t4", "ctx1", TaskState::Completed))
        .await
        .unwrap();

    let params = TaskListParams {
        context_id: Some("ctx1".to_string()),
        status: Some(vec![TaskState::Working]),
        ..Default::default()
    };
    let result = store.list(&params).await.unwrap();
    assert_eq!(result.tasks.len(), 1);
    assert_eq!(result.tasks[0].id, "t2");
}

#[tokio::test]
async fn test_list_invalid_page_token() {
    let store = InMemoryTaskStore::new();
    store.save(make_task("t1", "ctx1")).await.unwrap();
    store.save(make_task("t2", "ctx1")).await.unwrap();

    // Invalid token should start from beginning
    let params = TaskListParams {
        page_token: Some("invalid-token".to_string()),
        ..Default::default()
    };
    let result = store.list(&params).await.unwrap();
    assert_eq!(result.tasks.len(), 2);
}

// ---- Task with details ----

#[tokio::test]
async fn test_save_task_with_artifacts() {
    let store = InMemoryTaskStore::new();
    let mut task = make_task("t1", "ctx1");
    task.artifacts = Some(vec![Artifact {
        artifact_id: "a1".to_string(),
        name: Some("test artifact".to_string()),
        description: None,
        parts: vec![Part::text("artifact content")],
        metadata: None,
        extensions: None,
    }]);
    store.save(task).await.unwrap();

    let retrieved = store.get("t1").await.unwrap().unwrap();
    let artifacts = retrieved.artifacts.unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].artifact_id, "a1");
}

#[tokio::test]
async fn test_save_task_with_history() {
    let store = InMemoryTaskStore::new();
    let mut task = make_task("t1", "ctx1");
    task.history = Some(vec![
        Message::user("m1", "Hello"),
        Message::agent("m2", "Hi there"),
    ]);
    store.save(task).await.unwrap();

    let retrieved = store.get("t1").await.unwrap().unwrap();
    let history = retrieved.history.unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].role, Role::User);
    assert_eq!(history[1].role, Role::Agent);
}

#[tokio::test]
async fn test_save_task_with_metadata() {
    let store = InMemoryTaskStore::new();
    let mut task = make_task("t1", "ctx1");
    task.metadata = Some(serde_json::json!({"key": "value", "nested": {"a": 1}}));
    store.save(task).await.unwrap();

    let retrieved = store.get("t1").await.unwrap().unwrap();
    let metadata = retrieved.metadata.unwrap();
    assert_eq!(metadata["key"], "value");
    assert_eq!(metadata["nested"]["a"], 1);
}

#[tokio::test]
async fn test_task_update_preserves_order() {
    let store = InMemoryTaskStore::new();
    store.save(make_task("t1", "ctx1")).await.unwrap();
    store.save(make_task("t2", "ctx1")).await.unwrap();
    store.save(make_task("t3", "ctx1")).await.unwrap();

    // Update t2
    store
        .save(make_task_with_state("t2", "ctx1", TaskState::Completed))
        .await
        .unwrap();

    let result = store.list(&TaskListParams::default()).await.unwrap();
    assert_eq!(result.tasks.len(), 3);
    // Order should be preserved (insertion order, not update order)
    assert_eq!(result.tasks[0].id, "t1");
    assert_eq!(result.tasks[1].id, "t2");
    assert_eq!(result.tasks[2].id, "t3");
    // But t2 should have the updated state
    assert_eq!(result.tasks[1].status.state, TaskState::Completed);
}

// ---- Concurrency tests ----

#[tokio::test]
async fn test_concurrent_saves() {
    let store = std::sync::Arc::new(InMemoryTaskStore::new());
    let mut handles = vec![];

    for i in 0..10 {
        let store = store.clone();
        handles.push(tokio::spawn(async move {
            store
                .save(make_task(&format!("t{}", i), "ctx1"))
                .await
                .unwrap();
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let result = store.list(&TaskListParams::default()).await.unwrap();
    assert_eq!(result.tasks.len(), 10);
}

#[tokio::test]
async fn test_concurrent_gets() {
    let store = std::sync::Arc::new(InMemoryTaskStore::new());
    store.save(make_task("t1", "ctx1")).await.unwrap();

    let mut handles = vec![];
    for _ in 0..10 {
        let store = store.clone();
        handles.push(tokio::spawn(async move { store.get("t1").await.unwrap() }));
    }

    for h in handles {
        let result = h.await.unwrap();
        assert!(result.is_some());
    }
}
