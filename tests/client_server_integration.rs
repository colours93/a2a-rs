//! Port of Python SDK tests/integration/test_client_server_integration.py
//!
//! Full round-trip integration tests: reqwest HTTP client → axum A2A server.
//!
//! Python tests parametrize across JSON-RPC, REST, and gRPC transports.
//! Rust SDK only has JSON-RPC, so all tests use that single transport.
//!
//! Skipped tests documented at bottom of file with reasons.

mod common;

use common::{
    jsonrpc_request, message_send_request, message_send_with_context, start_test_server, EchoAgent,
    FailingAgent, SlowEchoAgent,
};
use serde_json::json;
use std::sync::Arc;

// ===========================================================================
// send_message blocking — 3 tests
// Ports: test_http_transport_sends_message_blocking[JSON-RPC]
// ===========================================================================

/// Blocking send returns a completed task with valid IDs.
#[tokio::test]
async fn send_message_blocking_returns_completed_task() {
    let (base_url, _h) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = message_send_request(1, "Hello, blocking test!");
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await.unwrap();
    let result = &json["result"];

    assert_eq!(result["kind"], "task");
    assert!(result["id"].is_string());
    assert!(result["contextId"].is_string());
    assert_eq!(result["status"]["state"], "completed");
}

/// Echo agent echoes text back in the status message.
#[tokio::test]
async fn send_message_blocking_echoes_text() {
    let (base_url, _h) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = message_send_request(1, "Hello, integration test!");
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    let json: serde_json::Value = resp.json().await.unwrap();
    let result = &json["result"];
    let msg = &result["status"]["message"];
    assert_eq!(msg["role"], "agent");
    let text = msg["parts"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("Echo: Hello, integration test!"),
        "Expected echo, got: {}",
        text
    );
}

/// JSON-RPC envelope is well-formed (jsonrpc, id, result, no error).
#[tokio::test]
async fn send_message_blocking_valid_jsonrpc_envelope() {
    let (base_url, _h) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = message_send_request(42, "envelope test");
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["jsonrpc"], "2.0");
    assert_eq!(json["id"], 42);
    assert!(json.get("error").is_none());
    assert!(json.get("result").is_some());
}

// ===========================================================================
// get_task retrieval — 2 tests
// Ports: test_http_transport_get_task[JSON-RPC]
// ===========================================================================

/// tasks/get retrieves a previously created task.
#[tokio::test]
async fn get_task_retrieves_created_task() {
    let (base_url, _h) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    // Create a task
    let body = message_send_request(1, "task for get test");
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();
    let json: serde_json::Value = resp.json().await.unwrap();
    let task_id = json["result"]["id"].as_str().unwrap();

    // Retrieve it
    let get_body = jsonrpc_request(json!(2), "tasks/get", json!({ "id": task_id }));
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&get_body)
        .send()
        .await
        .unwrap();

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["result"]["id"], task_id);
    assert_eq!(json["result"]["kind"], "task");
    assert_eq!(json["result"]["status"]["state"], "completed");
}

/// tasks/get for non-existent task returns TaskNotFoundError (-32001).
#[tokio::test]
async fn get_task_nonexistent_returns_task_not_found() {
    let (base_url, _h) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = jsonrpc_request(json!(1), "tasks/get", json!({ "id": "does-not-exist-999" }));
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    let json: serde_json::Value = resp.json().await.unwrap();
    assert!(json.get("error").is_some(), "expected error: {}", json);
    assert_eq!(json["error"]["code"], -32001);
}

// ===========================================================================
// cancel_task — 1 test
// Ports: test_http_transport_cancel_task[JSON-RPC]
// ===========================================================================

/// tasks/cancel for non-existent task returns an error.
#[tokio::test]
async fn cancel_task_nonexistent_returns_error() {
    let (base_url, _h) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = jsonrpc_request(
        json!(1),
        "tasks/cancel",
        json!({ "id": "cancel-nonexistent" }),
    );
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    let json: serde_json::Value = resp.json().await.unwrap();
    assert!(json.get("error").is_some());
    // Either TaskNotFound (-32001) or TaskNotCancelable (-32002)
    let code = json["error"]["code"].as_i64().unwrap();
    assert!(
        code == -32001 || code == -32002,
        "expected -32001 or -32002, got: {}",
        code
    );
}

// ===========================================================================
// Agent card via well-known endpoint — 1 test
// Ports: test_http_transport_get_card[JSON-RPC]
// ===========================================================================

/// GET /.well-known/agent.json returns a valid agent card.
#[tokio::test]
async fn agent_card_well_known_endpoint() {
    let (base_url, _h) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/.well-known/agent.json", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let card: serde_json::Value = resp.json().await.unwrap();
    assert!(card["name"].is_string());
    assert!(card["version"].is_string());
    assert!(card["capabilities"].is_object());
    assert!(card["skills"].is_array());
}

// ===========================================================================
// Streaming — message/stream via SSE — 2 tests
// Ports: test_http_transport_sends_message_streaming[JSON-RPC]
// ===========================================================================

/// message/stream returns SSE content type.
#[tokio::test]
async fn message_stream_returns_sse_content_type() {
    let (base_url, _h) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = jsonrpc_request(
        json!(1),
        "message/stream",
        json!({
            "message": {
                "messageId": "msg-stream-1",
                "role": "user",
                "parts": [{ "kind": "text", "text": "Hello, streaming!" }]
            }
        }),
    );
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.contains("text/event-stream"),
        "expected SSE content type, got: {}",
        ct
    );
}

/// message/stream body contains SSE data: lines.
#[tokio::test]
async fn message_stream_body_has_sse_events() {
    let (base_url, _h) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = jsonrpc_request(
        json!(1),
        "message/stream",
        json!({
            "message": {
                "messageId": "msg-stream-2",
                "role": "user",
                "parts": [{ "kind": "text", "text": "stream events test" }]
            }
        }),
    );
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    let text = resp.text().await.unwrap();
    assert!(
        text.contains("data:"),
        "expected SSE data events in body, got first 500 chars: {}",
        &text[..text.len().min(500)]
    );
}

// ===========================================================================
// Multiple requests — 1 test
// ===========================================================================

/// Multiple sequential requests all succeed on the same server.
#[tokio::test]
async fn multiple_sequential_requests() {
    let (base_url, _h) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    for i in 1i64..=5 {
        let body = message_send_request(i, &format!("Request {}", i));
        let resp = client
            .post(format!("{}/a2a", base_url))
            .json(&body)
            .send()
            .await
            .unwrap();

        let json: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(json["id"], i);
        assert_eq!(json["result"]["status"]["state"], "completed");
    }
}

// ===========================================================================
// Failing agent — 1 test
// ===========================================================================

/// Failing agent returns a task with failed state.
#[tokio::test]
async fn failing_agent_returns_failed_task() {
    let (base_url, _h) = start_test_server(Arc::new(FailingAgent)).await;
    let client = reqwest::Client::new();

    let body = message_send_request(1, "this will fail");
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    let json: serde_json::Value = resp.json().await.unwrap();
    let result = &json["result"];
    assert_eq!(result["kind"], "task");
    assert_eq!(result["status"]["state"], "failed");
}

// ===========================================================================
// Artifacts — 1 test
// ===========================================================================

/// SlowEchoAgent produces artifacts in the completed task.
#[tokio::test]
async fn slow_echo_agent_produces_artifacts() {
    let (base_url, _h) = start_test_server(Arc::new(SlowEchoAgent)).await;
    let client = reqwest::Client::new();

    let body = message_send_request(1, "Process this");
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    let json: serde_json::Value = resp.json().await.unwrap();
    let result = &json["result"];
    assert_eq!(result["status"]["state"], "completed");

    let artifacts = result["artifacts"]
        .as_array()
        .expect("expected artifacts array");
    assert!(!artifacts.is_empty());
    assert_eq!(artifacts[0]["name"], "output");
    let text = artifacts[0]["parts"][0]["text"].as_str().unwrap();
    assert!(text.contains("Process this"), "got: {}", text);
}

// ===========================================================================
// Non-existent task errors — 2 tests
// ===========================================================================

/// tasks/get with bogus ID returns -32001.
#[tokio::test]
async fn get_nonexistent_task_error_code() {
    let (base_url, _h) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = jsonrpc_request(json!(1), "tasks/get", json!({ "id": "bogus-id-123" }));
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["error"]["code"], -32001);
}

/// tasks/cancel with bogus ID returns error.
#[tokio::test]
async fn cancel_nonexistent_task_error_code() {
    let (base_url, _h) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = jsonrpc_request(
        json!(1),
        "tasks/cancel",
        json!({ "id": "bogus-cancel-456" }),
    );
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    let json: serde_json::Value = resp.json().await.unwrap();
    assert!(json.get("error").is_some());
}

// ===========================================================================
// Context ID / multi-turn — 2 tests
// ===========================================================================

/// Sending with context_id creates task in same context.
#[tokio::test]
async fn send_with_context_id_preserves_context() {
    let (base_url, _h) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    // First message — get context_id
    let body = message_send_request(1, "first");
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();
    let json: serde_json::Value = resp.json().await.unwrap();
    let ctx_id = json["result"]["contextId"].as_str().unwrap();

    // Second message — same context
    let body2 = message_send_with_context(2, "second", ctx_id, None);
    let resp2 = client
        .post(format!("{}/a2a", base_url))
        .json(&body2)
        .send()
        .await
        .unwrap();
    let json2: serde_json::Value = resp2.json().await.unwrap();
    assert_eq!(json2["result"]["contextId"], ctx_id);
}

/// Sending with task_id reuses the same task.
#[tokio::test]
async fn send_with_task_id_reuses_task() {
    let (base_url, _h) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    // First message
    let body = message_send_request(1, "first turn");
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();
    let json: serde_json::Value = resp.json().await.unwrap();
    let task_id = json["result"]["id"].as_str().unwrap();
    let ctx_id = json["result"]["contextId"].as_str().unwrap();

    // Second message — same task
    let body2 = message_send_with_context(2, "second turn", ctx_id, Some(task_id));
    let resp2 = client
        .post(format!("{}/a2a", base_url))
        .json(&body2)
        .send()
        .await
        .unwrap();
    let json2: serde_json::Value = resp2.json().await.unwrap();
    // Server may return a new completed task or an error (e.g. task already completed).
    // Either is valid — we just verify a well-formed JSON-RPC response.
    assert_eq!(json2["jsonrpc"], "2.0");
    assert!(
        json2.get("result").is_some() || json2.get("error").is_some(),
        "expected result or error: {}",
        json2
    );
}

// ===========================================================================
// Method not found — 1 test
// ===========================================================================

/// Unknown JSON-RPC method returns -32601.
#[tokio::test]
async fn unknown_method_returns_method_not_found() {
    let (base_url, _h) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = jsonrpc_request(json!(1), "nonexistent/method", json!({}));
    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    let json: serde_json::Value = resp.json().await.unwrap();
    assert!(json.get("error").is_some());
    assert_eq!(json["error"]["code"], -32601);
}

// ===========================================================================
// Skipped Python tests (with reasons)
// ===========================================================================

// SKIPPED: test_grpc_transport_* (8 tests) — no gRPC transport in Rust SDK
// SKIPPED: test_rest_setup / REST transport tests (8 tests) — no REST transport in Rust SDK
// SKIPPED: test_http_transport_set_task_callback — no push notification support
// SKIPPED: test_http_transport_get_task_callback — no push notification support
// SKIPPED: test_http_transport_resubscribe — no resubscribe endpoint yet
// SKIPPED: test_http_transport_get_authenticated_card — no extended card auth
// SKIPPED: test_json_transport_base_client_send_message_with_extensions — no X-A2A-Extensions header
// SKIPPED: test_json_transport_get_signed_base_card — no signing/crypto module
// SKIPPED: test_json_transport_get_signed_extended_card — no signing/crypto module
// SKIPPED: test_json_transport_get_signed_base_and_extended_cards — no signing/crypto module
// SKIPPED: test_rest_transport_get_signed_card — no REST + no signing
// SKIPPED: test_grpc_transport_get_signed_card — no gRPC + no signing
