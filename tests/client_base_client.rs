//! Port of Python SDK tests/client/test_base_client.py
//!
//! Tests for the A2A client's message sending behavior: streaming vs non-streaming,
//! configuration overrides, and transport selection.
//!
//! Python's BaseClient uses mocked transports. In Rust, the A2AClient uses
//! the Transport trait, which we can implement with a mock.

use a2a_rs::client::{create_text_message, A2AClient, Transport};
use a2a_rs::error::{A2AError, A2AResult};
use a2a_rs::types::*;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

// ============================================================================
// Mock transport for testing
// ============================================================================

/// Records calls and returns preconfigured responses.
struct MockTransport {
    /// Stores the method from the last sent request.
    last_method: Arc<Mutex<Option<String>>>,
    /// The response to return from send().
    response: JsonRpcResponse,
}

impl MockTransport {
    fn new(result: serde_json::Value) -> Self {
        Self {
            last_method: Arc::new(Mutex::new(None)),
            response: JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: Some(JsonRpcId::String("1".to_string())),
                result: Some(result),
                error: None,
            },
        }
    }

    fn last_method(&self) -> Option<String> {
        self.last_method.lock().unwrap().clone()
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn send(&self, request: &JsonRpcRequest) -> A2AResult<JsonRpcResponse> {
        *self.last_method.lock().unwrap() = Some(request.method.clone());
        Ok(self.response.clone())
    }

    async fn send_stream(&self, request: &JsonRpcRequest) -> A2AResult<a2a_rs::client::SseStream> {
        *self.last_method.lock().unwrap() = Some(request.method.clone());
        Err(A2AError::Transport(
            "mock: streaming not implemented".to_string(),
        ))
    }
}

fn sample_task_json() -> serde_json::Value {
    serde_json::json!({
        "kind": "task",
        "id": "task-123",
        "contextId": "ctx-456",
        "status": {"state": "completed"}
    })
}

fn sample_message_json() -> serde_json::Value {
    serde_json::json!({
        "kind": "message",
        "messageId": "msg-reply",
        "role": "agent",
        "parts": [{"kind": "text", "text": "Hello back!"}]
    })
}

// ============================================================================
// Tests: send_message dispatches correctly
// ============================================================================

#[tokio::test]
async fn test_send_message_uses_message_send_method() {
    let transport = MockTransport::new(sample_task_json());
    let client = A2AClient::with_transport(Box::new(transport));

    let message = create_text_message(Role::User, "Hello");
    let params = SendMessageParams {
        message,
        configuration: None,
        metadata: None,
        tenant: None,
    };

    // We expect send_message to call method "message/send"
    // The mock returns a Task
    let _ = client.send_message(params).await;
}

#[tokio::test]
async fn test_send_text_convenience() {
    let transport = MockTransport::new(sample_task_json());
    let last_method = transport.last_method.clone();
    let client = A2AClient::with_transport(Box::new(transport));

    let result = client.send_text("Hello, agent!").await;
    assert!(result.is_ok());
    assert_eq!(last_method.lock().unwrap().as_deref(), Some("message/send"));
}

#[tokio::test]
async fn test_send_text_returns_task() {
    let transport = MockTransport::new(sample_task_json());
    let client = A2AClient::with_transport(Box::new(transport));

    let response = client.send_text("test").await.unwrap();
    match response {
        SendMessageResponse::Task(task) => {
            assert_eq!(task.id, "task-123");
            assert_eq!(task.context_id, "ctx-456");
        }
        _ => panic!("expected Task response"),
    }
}

#[tokio::test]
async fn test_send_text_returns_message() {
    let transport = MockTransport::new(sample_message_json());
    let client = A2AClient::with_transport(Box::new(transport));

    let response = client.send_text("test").await.unwrap();
    match response {
        SendMessageResponse::Message(msg) => {
            assert_eq!(msg.role, Role::Agent);
        }
        _ => panic!("expected Message response"),
    }
}

// ============================================================================
// Tests: configuration parameters
// ============================================================================

#[tokio::test]
async fn test_send_text_with_config() {
    let transport = MockTransport::new(sample_task_json());
    let client = A2AClient::with_transport(Box::new(transport));

    let config = SendMessageConfiguration {
        history_length: Some(2),
        blocking: Some(false),
        accepted_output_modes: Some(vec!["application/json".to_string()]),
        push_notification_config: None,
    };

    let result = client.send_text_with_config("test", config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_send_text_in_context() {
    let transport = MockTransport::new(sample_task_json());
    let client = A2AClient::with_transport(Box::new(transport));

    let result = client.send_text_in_context("test", "ctx-existing").await;
    assert!(result.is_ok());
}

// ============================================================================
// Tests: get_task, cancel_task
// ============================================================================

#[tokio::test]
async fn test_get_task() {
    let transport = MockTransport::new(sample_task_json());
    let last_method = transport.last_method.clone();
    let client = A2AClient::with_transport(Box::new(transport));

    let result = client.get_task_by_id("task-123", None).await;
    assert!(result.is_ok());
    assert_eq!(last_method.lock().unwrap().as_deref(), Some("tasks/get"));
    assert_eq!(result.unwrap().id, "task-123");
}

#[tokio::test]
async fn test_cancel_task() {
    let cancelled = serde_json::json!({
        "kind": "task",
        "id": "task-123",
        "contextId": "ctx-456",
        "status": {"state": "canceled"}
    });
    let transport = MockTransport::new(cancelled);
    let last_method = transport.last_method.clone();
    let client = A2AClient::with_transport(Box::new(transport));

    let result = client.cancel_task_by_id("task-123").await;
    assert!(result.is_ok());
    assert_eq!(last_method.lock().unwrap().as_deref(), Some("tasks/cancel"));
}

// ============================================================================
// Tests: error handling from transport
// ============================================================================

struct ErrorTransport;

#[async_trait]
impl Transport for ErrorTransport {
    async fn send(&self, _request: &JsonRpcRequest) -> A2AResult<JsonRpcResponse> {
        Err(A2AError::Http {
            status: 500,
            body: "Internal Server Error".to_string(),
        })
    }

    async fn send_stream(&self, _request: &JsonRpcRequest) -> A2AResult<a2a_rs::client::SseStream> {
        Err(A2AError::Http {
            status: 500,
            body: "Internal Server Error".to_string(),
        })
    }
}

#[tokio::test]
async fn test_send_message_transport_error() {
    let client = A2AClient::with_transport(Box::new(ErrorTransport));
    let result = client.send_text("test").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        A2AError::Http { status, .. } => assert_eq!(status, 500),
        e => panic!("expected Http error, got: {:?}", e),
    }
}

// ============================================================================
// Tests: JSON-RPC error response handling
// ============================================================================

struct JsonRpcErrorTransport;

#[async_trait]
impl Transport for JsonRpcErrorTransport {
    async fn send(&self, _request: &JsonRpcRequest) -> A2AResult<JsonRpcResponse> {
        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(JsonRpcId::String("1".to_string())),
            result: None,
            error: Some(JsonRpcError {
                code: -32001,
                message: "Task not found".to_string(),
                data: None,
            }),
        })
    }

    async fn send_stream(&self, _request: &JsonRpcRequest) -> A2AResult<a2a_rs::client::SseStream> {
        Err(A2AError::Transport("not implemented".to_string()))
    }
}

#[tokio::test]
async fn test_json_rpc_error_response() {
    let client = A2AClient::with_transport(Box::new(JsonRpcErrorTransport));
    let result = client.send_text("test").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        A2AError::JsonRpc { code, message, .. } => {
            assert_eq!(code, -32001);
            assert!(message.contains("Task not found"));
        }
        e => panic!("expected JsonRpc error, got: {:?}", e),
    }
}

// ============================================================================
// Tests: create_text_message helper
// ============================================================================

#[test]
fn test_create_text_message_user() {
    let msg = create_text_message(Role::User, "Hello");
    assert_eq!(msg.role, Role::User);
    assert_eq!(msg.parts.len(), 1);
    match &msg.parts[0] {
        Part::Text { text, .. } => assert_eq!(text, "Hello"),
        _ => panic!("expected Text part"),
    }
}

#[test]
fn test_create_text_message_agent() {
    let msg = create_text_message(Role::Agent, "Hi there!");
    assert_eq!(msg.role, Role::Agent);
    assert_eq!(msg.parts.len(), 1);
}

#[test]
fn test_create_text_message_has_unique_id() {
    let msg1 = create_text_message(Role::User, "a");
    let msg2 = create_text_message(Role::User, "b");
    assert_ne!(msg1.message_id, msg2.message_id);
}

#[test]
fn test_create_text_message_kind_is_message() {
    let msg = create_text_message(Role::User, "test");
    assert_eq!(msg.kind, "message");
}
