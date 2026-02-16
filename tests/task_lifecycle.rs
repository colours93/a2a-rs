//! Integration tests for the full task lifecycle.
//!
//! Tests cover: create -> get -> list -> cancel -> verify state transitions.

mod common;

use a2a_rs::server::InMemoryTaskStore;
use common::{
    message_send_request, message_send_with_context, start_test_server,
    start_test_server_with_store, EchoAgent,
};
use std::sync::Arc;

/// Test that tasks/list returns all created tasks.
#[tokio::test]
async fn tasks_list_returns_created_tasks() {
    let store = Arc::new(InMemoryTaskStore::new());
    let (base_url, _handle) =
        start_test_server_with_store(Arc::new(EchoAgent), store.clone()).await;
    let client = reqwest::Client::new();

    // Create 3 tasks
    for i in 1..=3 {
        let body = message_send_request(i, &format!("Task #{}", i));
        let _resp: serde_json::Value = client
            .post(format!("{}/a2a", base_url))
            .json(&body)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
    }

    // List all tasks
    let list_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tasks/list",
        "params": {}
    });

    let list_resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&list_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(list_resp["jsonrpc"], "2.0");
    assert_eq!(list_resp["id"], 10);
    assert!(list_resp.get("error").is_none());

    let tasks = list_resp["result"]["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 3, "Expected 3 tasks, got {}", tasks.len());

    // All should be completed (echo agent completes immediately)
    for task in tasks {
        assert_eq!(task["status"]["state"], "completed");
    }
}

/// Test that tasks/list can filter by context_id.
#[tokio::test]
async fn tasks_list_filter_by_context_id() {
    let store = Arc::new(InMemoryTaskStore::new());
    let (base_url, _handle) =
        start_test_server_with_store(Arc::new(EchoAgent), store.clone()).await;
    let client = reqwest::Client::new();

    // Create tasks with different context IDs
    let ctx_a = "context-aaa";
    let ctx_b = "context-bbb";

    let body_a = message_send_with_context(1, "Msg A1", ctx_a, None);
    let _: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&body_a)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let body_b1 = message_send_with_context(2, "Msg B1", ctx_b, None);
    let _: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&body_b1)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let body_b2 = message_send_with_context(3, "Msg B2", ctx_b, None);
    let _: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&body_b2)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // List tasks filtered by context B
    let list_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tasks/list",
        "params": { "contextId": ctx_b }
    });

    let list_resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&list_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let tasks = list_resp["result"]["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 2, "Expected 2 tasks in context B");
    for task in tasks {
        assert_eq!(task["contextId"], ctx_b);
    }
}

/// Test that tasks/get returns task with history.
#[tokio::test]
async fn tasks_get_includes_history() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    // Create a task
    let body = message_send_request(1, "Hello history");
    let resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Python SDK flat format: result is the task directly
    let task_id = resp["result"]["id"].as_str().unwrap();

    // Get the task
    let get_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tasks/get",
        "params": { "id": task_id }
    });

    let get_resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&get_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let task = &get_resp["result"];
    // Should have history with at least the original user message
    let history = task["history"].as_array().unwrap();
    assert!(!history.is_empty(), "Expected non-empty history");

    // First message should be the user's message
    assert_eq!(history[0]["role"], "user");
    assert_eq!(history[0]["parts"][0]["text"], "Hello history");
}

/// Test that tasks/get with historyLength trims history.
#[tokio::test]
async fn tasks_get_trims_history_by_length() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    // Create a task
    let body = message_send_request(1, "Hello");
    let resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Python SDK flat format
    let task_id = resp["result"]["id"].as_str().unwrap();

    // Get with historyLength=1
    let get_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tasks/get",
        "params": { "id": task_id, "historyLength": 1 }
    });

    let get_resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&get_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let history = get_resp["result"]["history"].as_array().unwrap();
    assert!(
        history.len() <= 1,
        "Expected at most 1 history entry, got {}",
        history.len()
    );
}

/// Test that tasks/get for non-existent task returns TaskNotFound error.
#[tokio::test]
async fn tasks_get_not_found() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let get_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tasks/get",
        "params": { "id": "nonexistent-task-id" }
    });

    let resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&get_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(resp.get("error").is_some());
    assert_eq!(resp["error"]["code"], -32001); // TaskNotFound
}

/// Test that completed tasks cannot be canceled.
#[tokio::test]
async fn cancel_completed_task_returns_error() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    // Create and complete a task
    let body = message_send_request(1, "Complete me");
    let resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Python SDK flat format
    let task_id = resp["result"]["id"].as_str().unwrap();
    assert_eq!(resp["result"]["status"]["state"], "completed");

    // Try to cancel the completed task
    let cancel_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tasks/cancel",
        "params": { "id": task_id }
    });

    let cancel_resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&cancel_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(cancel_resp.get("error").is_some());
    assert_eq!(cancel_resp["error"]["code"], -32002); // TaskNotCancelable
}

/// Test tasks/cancel for a non-existent task returns TaskNotFound.
#[tokio::test]
async fn cancel_nonexistent_task_returns_error() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let cancel_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tasks/cancel",
        "params": { "id": "does-not-exist" }
    });

    let cancel_resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&cancel_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(cancel_resp.get("error").is_some());
    assert_eq!(cancel_resp["error"]["code"], -32001); // TaskNotFound
}

/// Test that the result uses Python SDK flat kind-based format.
#[tokio::test]
async fn result_uses_flat_kind_format() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = message_send_request(1, "Check format");
    let resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Python SDK: SendMessageResponse serializes flat with "kind" discriminator
    assert_eq!(
        resp["result"]["kind"], "task",
        "Expected flat 'kind': 'task' in result"
    );
    assert!(
        resp["result"]["id"].is_string(),
        "Task id should be at top level of result"
    );
}
