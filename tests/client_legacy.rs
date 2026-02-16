//! Port of Python SDK tests/client/test_legacy_client.py
//!
//! Tests for the legacy client compatibility layer.
//!
//! Python has A2AClient (legacy) and A2AGrpcClient. In Rust, there's only
//! A2AClient with the Transport trait, so we test that the client works
//! with JSON-RPC transport (the only transport available).
//!
//! Skipped tests:
//! - test_a2a_grpc_client_get_task (no gRPC support in Rust SDK)
//! - Mock-based send_message test (requires HTTP mocking)

use a2a_rs::client::{create_text_message, A2AClient, Transport};
use a2a_rs::error::{A2AError, A2AResult};
use a2a_rs::types::*;
use async_trait::async_trait;

// Reuse mock transport pattern
struct MockTransport {
    response: JsonRpcResponse,
}

impl MockTransport {
    fn with_task(id: &str, state: TaskState) -> Self {
        let result = serde_json::json!({
            "kind": "task",
            "id": id,
            "contextId": "ctx-auto",
            "status": {"state": serde_json::to_value(state).unwrap()}
        });
        Self {
            response: JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: Some(JsonRpcId::String("1".to_string())),
                result: Some(result),
                error: None,
            },
        }
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn send(&self, _request: &JsonRpcRequest) -> A2AResult<JsonRpcResponse> {
        Ok(self.response.clone())
    }
    async fn send_stream(&self, _request: &JsonRpcRequest) -> A2AResult<a2a_rs::client::SseStream> {
        Err(A2AError::Transport("not implemented".to_string()))
    }
}

// ============================================================================
// Equivalent of test_a2a_client_send_message
// ============================================================================

#[tokio::test]
async fn test_client_send_message_returns_task() {
    let transport = MockTransport::with_task("task-123", TaskState::Completed);
    let client = A2AClient::with_transport(Box::new(transport));

    let message = create_text_message(Role::User, "Hello");
    let params = SendMessageParams {
        message,
        configuration: None,
        metadata: None,
        tenant: None,
    };

    let response = client.send_message(params).await.unwrap();
    match response {
        SendMessageResponse::Task(task) => {
            assert_eq!(task.id, "task-123");
        }
        _ => panic!("expected Task response"),
    }
}

// ============================================================================
// Client methods dispatch correct JSON-RPC methods
// ============================================================================

struct MethodRecorder {
    inner: MockTransport,
    method: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl MethodRecorder {
    fn new(
        task_id: &str,
        state: TaskState,
    ) -> (Self, std::sync::Arc<std::sync::Mutex<Option<String>>>) {
        let method = std::sync::Arc::new(std::sync::Mutex::new(None));
        let recorder = Self {
            inner: MockTransport::with_task(task_id, state),
            method: method.clone(),
        };
        (recorder, method)
    }
}

#[async_trait]
impl Transport for MethodRecorder {
    async fn send(&self, request: &JsonRpcRequest) -> A2AResult<JsonRpcResponse> {
        *self.method.lock().unwrap() = Some(request.method.clone());
        self.inner.send(request).await
    }
    async fn send_stream(&self, request: &JsonRpcRequest) -> A2AResult<a2a_rs::client::SseStream> {
        self.inner.send_stream(request).await
    }
}

#[tokio::test]
async fn test_send_text_dispatches_message_send() {
    let (transport, method) = MethodRecorder::new("t1", TaskState::Completed);
    let client = A2AClient::with_transport(Box::new(transport));
    let _ = client.send_text("test").await;
    assert_eq!(method.lock().unwrap().as_deref(), Some("message/send"));
}

#[tokio::test]
async fn test_get_task_dispatches_tasks_get() {
    let (transport, method) = MethodRecorder::new("t1", TaskState::Working);
    let client = A2AClient::with_transport(Box::new(transport));
    let _ = client.get_task_by_id("t1", None).await;
    assert_eq!(method.lock().unwrap().as_deref(), Some("tasks/get"));
}

#[tokio::test]
async fn test_cancel_task_dispatches_tasks_cancel() {
    let (transport, method) = MethodRecorder::new("t1", TaskState::Canceled);
    let client = A2AClient::with_transport(Box::new(transport));
    let _ = client.cancel_task_by_id("t1").await;
    assert_eq!(method.lock().unwrap().as_deref(), Some("tasks/cancel"));
}
