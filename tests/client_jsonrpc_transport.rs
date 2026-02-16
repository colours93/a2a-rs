//! Port of Python SDK tests/client/transports/test_jsonrpc_client.py
//!
//! Tests for the JsonRpcTransport — the JSON-RPC over HTTP transport layer.
//!
//! Python tests use httpx mocking (respx, MagicMock). In Rust, we test
//! construction and URL handling. Network-level tests (send_message_success,
//! streaming, error propagation) are skipped as they require a mock HTTP server.
//!
//! Skipped tests (require HTTP mocking):
//! - test_send_message_success (mock POST response)
//! - test_send_message_error_response (JSON-RPC error response)
//! - test_send_message_streaming_success (SSE mock)
//! - test_send_message_streaming_comment_success (SSE with comments)
//! - test_send_request_http_status_error (HTTP error)
//! - test_send_request_json_decode_error (JSON parse error)
//! - test_send_request_httpx_request_error (network error)
//! - test_send_message_client_timeout (timeout)
//! - test_get_task_success (mock response)
//! - test_cancel_task_success (mock response)
//! - test_set_task_callback_success (mock response)
//! - test_get_task_callback_success (mock response)
//! - test_send_message_streaming_sse_error (SSE error)
//! - test_send_message_streaming_json_error (malformed SSE)
//! - test_send_message_streaming_request_error (network during SSE)
//! - test_get_card_no_card_provided (card fetch via GET)
//! - test_get_card_with_extended_card_support (extended card RPC)
//! - test_close (httpx close)
//! - Extension header tests (X-A2A-Extensions header)
//! - test_send_message_streaming_server_error_propagates (403 during SSE)

use a2a_rs::client::{JsonRpcTransport, Transport, TransportConfig};
use a2a_rs::types::*;
use std::time::Duration;

// ============================================================================
// Construction tests (mirrors Python TestJsonRpcTransport.test_init_*)
// ============================================================================

#[test]
fn test_transport_construction_with_url() {
    let transport = JsonRpcTransport::new("http://agent.example.com/api");
    assert_eq!(transport.url(), "http://agent.example.com/api");
}

#[test]
fn test_transport_construction_with_string() {
    let url = String::from("http://agent.example.com/api");
    let transport = JsonRpcTransport::new(url);
    assert_eq!(transport.url(), "http://agent.example.com/api");
}

#[test]
fn test_transport_with_custom_config() {
    let config = TransportConfig {
        timeout: Duration::from_secs(30),
        headers: [("X-Custom".to_string(), "value".to_string())].into(),
    };
    let transport = JsonRpcTransport::with_config("http://example.com", config);
    assert_eq!(transport.url(), "http://example.com");
}

#[test]
fn test_transport_with_reqwest_client() {
    let client = reqwest::Client::new();
    let transport = JsonRpcTransport::with_client("http://example.com", client);
    assert_eq!(transport.url(), "http://example.com");
}

#[test]
fn test_transport_with_timeout_builder() {
    let transport =
        JsonRpcTransport::new("http://example.com").with_timeout(Duration::from_secs(120));
    assert_eq!(transport.url(), "http://example.com");
}

#[test]
fn test_transport_with_header_builder() {
    let transport =
        JsonRpcTransport::new("http://example.com").with_header("Authorization", "Bearer token123");
    assert_eq!(transport.url(), "http://example.com");
}

#[test]
fn test_transport_debug() {
    let transport = JsonRpcTransport::new("http://example.com/rpc");
    let debug = format!("{:?}", transport);
    assert!(debug.contains("http://example.com/rpc"));
}

#[test]
fn test_transport_clone() {
    let transport = JsonRpcTransport::new("http://example.com/rpc");
    let cloned = transport.clone();
    assert_eq!(cloned.url(), "http://example.com/rpc");
}

// ============================================================================
// Transport config tests
// ============================================================================

#[test]
fn test_default_transport_config() {
    let config = TransportConfig::default();
    assert_eq!(config.timeout, Duration::from_secs(60));
    assert!(config.headers.is_empty());
}

#[test]
fn test_custom_transport_config() {
    let config = TransportConfig {
        timeout: Duration::from_secs(5),
        headers: [
            ("X-API-Key".to_string(), "secret".to_string()),
            ("X-Custom".to_string(), "value".to_string()),
        ]
        .into(),
    };
    assert_eq!(config.timeout, Duration::from_secs(5));
    assert_eq!(config.headers.len(), 2);
    assert_eq!(config.headers["X-API-Key"], "secret");
}

// ============================================================================
// Request building tests (JSON-RPC format verification)
// ============================================================================

#[test]
fn test_json_rpc_request_serialization() {
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::String("req-1".to_string())),
        method: "message/send".to_string(),
        params: Some(
            serde_json::json!({"message": {"messageId": "m1", "role": "user", "parts": []}}),
        ),
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["jsonrpc"], "2.0");
    assert_eq!(json["id"], "req-1");
    assert_eq!(json["method"], "message/send");
    assert!(json["params"]["message"].is_object());
}

#[test]
fn test_json_rpc_request_with_numeric_id() {
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::Number(42)),
        method: "tasks/get".to_string(),
        params: Some(serde_json::json!({"id": "task-abc"})),
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["id"], 42);
    assert_eq!(json["method"], "tasks/get");
}

// ============================================================================
// Response parsing tests
// ============================================================================

#[test]
fn test_json_rpc_response_with_result() {
    let json = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "123",
        "result": {
            "kind": "task",
            "id": "task-abc",
            "contextId": "session-xyz",
            "status": {"state": "working"}
        }
    });

    let response: JsonRpcResponse = serde_json::from_value(json).unwrap();
    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    let result = response.result.unwrap();
    assert_eq!(result["id"], "task-abc");
}

#[test]
fn test_json_rpc_response_with_error() {
    let json = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "123",
        "error": {
            "code": -32602,
            "message": "Invalid params"
        }
    });

    let response: JsonRpcResponse = serde_json::from_value(json).unwrap();
    assert!(response.result.is_none());
    assert!(response.error.is_some());

    let error = response.error.unwrap();
    assert_eq!(error.code, -32602);
    assert_eq!(error.message, "Invalid params");
}

#[test]
fn test_json_rpc_response_with_error_data() {
    let json = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "123",
        "error": {
            "code": -32001,
            "message": "Task not found",
            "data": {"taskId": "task-xyz"}
        }
    });

    let response: JsonRpcResponse = serde_json::from_value(json).unwrap();
    let error = response.error.unwrap();
    assert_eq!(error.code, -32001);
    assert!(error.data.is_some());
    assert_eq!(error.data.unwrap()["taskId"], "task-xyz");
}

// ============================================================================
// SendMessageParams serialization (used by transport)
// ============================================================================

#[test]
fn test_send_message_params_serialization() {
    let params = SendMessageParams {
        message: Message {
            message_id: "msg-1".to_string(),
            role: Role::User,
            kind: "message".to_string(),
            parts: vec![Part::text("Hello")],
            context_id: None,
            task_id: None,
            reference_task_ids: None,
            metadata: None,
            extensions: None,
        },
        configuration: None,
        metadata: None,
        tenant: None,
    };

    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["message"]["messageId"], "msg-1");
    assert_eq!(json["message"]["role"], "user");
}

#[test]
fn test_send_message_params_with_configuration() {
    let params = SendMessageParams {
        message: Message {
            message_id: "msg-2".to_string(),
            role: Role::User,
            kind: "message".to_string(),
            parts: vec![Part::text("test")],
            context_id: None,
            task_id: None,
            reference_task_ids: None,
            metadata: None,
            extensions: None,
        },
        configuration: Some(SendMessageConfiguration {
            history_length: Some(5),
            blocking: Some(true),
            accepted_output_modes: Some(vec!["text/plain".to_string()]),
            push_notification_config: None,
        }),
        metadata: None,
        tenant: None,
    };

    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["configuration"]["historyLength"], 5);
    assert_eq!(json["configuration"]["blocking"], true);
}
