//! Integration tests for JSON-RPC and A2A error handling.
//!
//! Tests verify that error responses use the correct error codes
//! from the A2A specification.

mod common;

use common::{start_test_server, EchoAgent};
use std::sync::Arc;

/// Test that an unknown method returns -32601 (Method Not Found).
#[tokio::test]
async fn unknown_method_returns_method_not_found() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "nonexistent/method",
        "params": {}
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

    assert!(resp.get("error").is_some());
    assert_eq!(resp["error"]["code"], -32601);
    assert!(resp["error"]["message"]
        .as_str()
        .unwrap()
        .contains("nonexistent/method"));
}

/// Test that an invalid JSON-RPC version returns -32600 (Invalid Request).
#[tokio::test]
async fn invalid_jsonrpc_version_returns_invalid_request() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "1.0",
        "id": 1,
        "method": "message/send",
        "params": {}
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

    assert!(resp.get("error").is_some());
    assert_eq!(resp["error"]["code"], -32600);
}

/// Test that message/send with missing 'message' param returns -32602 (Invalid Params).
#[tokio::test]
async fn message_send_missing_message_returns_invalid_params() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "message/send",
        "params": {
            "notAMessage": "hello"
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

    assert!(resp.get("error").is_some());
    assert_eq!(resp["error"]["code"], -32602);
}

/// Test that tasks/get with missing 'id' returns -32602 (Invalid Params).
#[tokio::test]
async fn tasks_get_missing_id_returns_invalid_params() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tasks/get",
        "params": {
            "notAnId": "something"
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

    assert!(resp.get("error").is_some());
    assert_eq!(resp["error"]["code"], -32602);
}

/// Test that tasks/cancel with missing 'id' returns -32602 (Invalid Params).
#[tokio::test]
async fn tasks_cancel_missing_id_returns_invalid_params() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tasks/cancel",
        "params": {}
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

    assert!(resp.get("error").is_some());
    assert_eq!(resp["error"]["code"], -32602);
}

/// Test that tasks/get for non-existent task returns -32001 (TaskNotFound).
#[tokio::test]
async fn task_not_found_error_code() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tasks/get",
        "params": { "id": "no-such-task" }
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

    assert!(resp.get("error").is_some());
    assert_eq!(resp["error"]["code"], -32001);
}

/// Test that tasks/cancel on a completed task returns -32002 (TaskNotCancelable).
#[tokio::test]
async fn task_not_cancelable_error_code() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    // Create and complete a task
    let send_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "message/send",
        "params": {
            "message": {
                "messageId": "m1",
                "role": "user",
                "parts": [{"kind": "text", "text": "Complete first"}]
            }
        }
    });

    let send_resp: serde_json::Value = client
        .post(format!("{}/a2a", base_url))
        .json(&send_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Python SDK: SendMessageResponse serializes flat (no wrapper key),
    // so the result IS the task object directly with "kind": "task"
    let task_id = send_resp["result"]["id"].as_str().unwrap();

    // Now try to cancel
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
    assert_eq!(cancel_resp["error"]["code"], -32002);
}

/// Test that error responses always have the expected JSON-RPC envelope.
#[tokio::test]
async fn error_responses_have_correct_envelope() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "tasks/get",
        "params": { "id": "nonexistent" }
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

    // Verify JSON-RPC envelope
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 42);
    assert!(resp.get("result").is_none());

    // Verify error object structure
    let error = &resp["error"];
    assert!(error["code"].is_number());
    assert!(error["message"].is_string());
}

/// Test that message/stream with invalid params returns JSON error (not SSE).
#[tokio::test]
async fn message_stream_invalid_params_returns_json_error() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "message/stream",
        "params": {
            "notAMessage": "hello"
        }
    });

    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    // Should return JSON, not SSE
    let content_type = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        content_type.contains("application/json"),
        "Expected JSON error response, got content-type: {}",
        content_type
    );

    let json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(json["error"]["code"], -32602);
}

/// Verify all A2A error code constants match the specification.
#[test]
fn error_code_constants_match_spec() {
    use a2a_rs::error;

    // JSON-RPC standard errors
    assert_eq!(error::PARSE_ERROR, -32700);
    assert_eq!(error::INVALID_REQUEST, -32600);
    assert_eq!(error::METHOD_NOT_FOUND, -32601);
    assert_eq!(error::INVALID_PARAMS, -32602);
    assert_eq!(error::INTERNAL_ERROR, -32603);

    // A2A-specific errors
    assert_eq!(error::TASK_NOT_FOUND, -32001);
    assert_eq!(error::TASK_NOT_CANCELABLE, -32002);
    assert_eq!(error::PUSH_NOTIFICATION_NOT_SUPPORTED, -32003);
    assert_eq!(error::UNSUPPORTED_OPERATION, -32004);
    assert_eq!(error::CONTENT_TYPE_NOT_SUPPORTED, -32005);
    assert_eq!(error::INVALID_AGENT_RESPONSE, -32006);
    assert_eq!(error::AUTHENTICATED_EXTENDED_CARD_NOT_CONFIGURED, -32007);
}

/// Verify A2AError correctly maps to JSON-RPC error codes.
#[test]
fn a2a_error_code_mapping() {
    use a2a_rs::error::A2AError;

    let cases: Vec<(A2AError, i64)> = vec![
        (A2AError::parse_error("test"), -32700),
        (A2AError::invalid_request("test"), -32600),
        (A2AError::method_not_found("test"), -32601),
        (A2AError::invalid_params("test"), -32602),
        (A2AError::internal_error("test"), -32603),
        (A2AError::task_not_found("test"), -32001),
        (A2AError::task_not_cancelable("test"), -32002),
        (A2AError::push_notification_not_supported("test"), -32003),
        (A2AError::unsupported_operation("test"), -32004),
        (A2AError::content_type_not_supported("test"), -32005),
        (A2AError::invalid_agent_response("test"), -32006),
        (
            A2AError::authenticated_extended_card_not_configured("test"),
            -32007,
        ),
    ];

    for (error, expected_code) in cases {
        assert_eq!(
            error.code(),
            expected_code,
            "Error {:?} should have code {}",
            error,
            expected_code
        );
    }
}
