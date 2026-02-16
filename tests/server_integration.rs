//! Integration tests for the A2A server with real HTTP roundtrips.
//!
//! These tests spin up an actual axum server and verify the full
//! JSON-RPC request/response cycle via reqwest.

mod common;

use common::{message_send_request, start_test_server, EchoAgent, FailingAgent, SlowEchoAgent};
use std::sync::Arc;

/// Test that message/send returns a valid JSON-RPC response with a completed task.
#[tokio::test]
async fn message_send_returns_completed_task() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = message_send_request(1, "Hello, Agent!");
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let json: serde_json::Value = resp.json().await.unwrap();

    // Verify JSON-RPC envelope
    assert_eq!(json["jsonrpc"], "2.0");
    assert_eq!(json["id"], 1);
    assert!(json.get("error").is_none());

    // Python SDK: SendMessageResponse serializes flat (no wrapper key).
    // The result IS the task object directly with "kind": "task".
    let result = &json["result"];
    assert_eq!(result["kind"], "task");
    assert!(result["id"].is_string());
    assert!(result["contextId"].is_string());

    // Task should be completed
    assert_eq!(result["status"]["state"], "completed");
}

/// Test that the echo agent echoes text back in the status message.
#[tokio::test]
async fn echo_agent_echoes_text() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = message_send_request(1, "Rust is great!");
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    let json: serde_json::Value = resp.json().await.unwrap();
    let result = &json["result"];

    // Python SDK: result is the task directly (flat)
    // The completed status should contain the echoed message
    let status_msg = &result["status"]["message"];
    assert_eq!(status_msg["role"], "agent");

    // Check that the text contains our echo
    let text = status_msg["parts"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("Echo: Rust is great!"),
        "Expected echo text, got: {}",
        text
    );
}

/// Test that tasks/get retrieves a task by ID after creation.
#[tokio::test]
async fn tasks_get_retrieves_created_task() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    // First, create a task via message/send
    let send_body = message_send_request(1, "Hello");
    let send_resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&send_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Python SDK: result is flat (no wrapper key)
    let task_id = send_resp["result"]["id"].as_str().unwrap();

    // Now retrieve it via tasks/get
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

    assert_eq!(get_resp["jsonrpc"], "2.0");
    assert_eq!(get_resp["id"], 2);
    assert!(get_resp.get("error").is_none());

    let task = &get_resp["result"];
    assert_eq!(task["id"], task_id);
    assert_eq!(task["status"]["state"], "completed");
}

/// Test that the slow echo agent produces artifacts.
#[tokio::test]
async fn slow_echo_agent_produces_artifacts() {
    let (base_url, _handle) = start_test_server(Arc::new(SlowEchoAgent)).await;
    let client = reqwest::Client::new();

    let body = message_send_request(1, "Process this");
    let resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Python SDK: result is flat (no wrapper key)
    let result = &resp["result"];
    assert_eq!(result["status"]["state"], "completed");

    // Should have artifacts
    let artifacts = result["artifacts"].as_array().unwrap();
    assert!(!artifacts.is_empty(), "Expected at least one artifact");
    assert_eq!(artifacts[0]["name"], "output");
    assert!(artifacts[0]["parts"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Process this"));
}

/// Test that a failing agent produces a failed task.
#[tokio::test]
async fn failing_agent_returns_failed_task() {
    let (base_url, _handle) = start_test_server(Arc::new(FailingAgent)).await;
    let client = reqwest::Client::new();

    let body = message_send_request(1, "This will fail");
    let resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Python SDK: result is flat (no wrapper key)
    let result = &resp["result"];
    assert_eq!(result["status"]["state"], "failed");
}

/// Test that the response preserves the JSON-RPC request ID.
#[tokio::test]
async fn preserves_jsonrpc_request_id() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    // Test with numeric ID
    let body = message_send_request(42, "Hello");
    let resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resp["id"], 42);

    // Test with string ID
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "my-req-id",
        "method": "message/send",
        "params": {
            "message": {
                "messageId": "m1",
                "role": "user",
                "parts": [{"kind": "text", "text": "Hello"}]
            }
        }
    });
    let resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resp["id"], "my-req-id");
}

/// Test that multiple requests can be sent to the same server.
#[tokio::test]
async fn multiple_requests_same_server() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    for i in 1..=5 {
        let body = message_send_request(i, &format!("Message #{}", i));
        let resp: serde_json::Value = client
            .post(format!("{}/a2a", base_url))
            .json(&body)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        assert_eq!(resp["id"], i);
        // Python SDK: result is flat (no wrapper key)
        assert_eq!(resp["result"]["status"]["state"], "completed");
    }
}
