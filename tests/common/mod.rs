//! Shared test utilities for integration tests.

use std::sync::Arc;

use a2a_rs::builders::AgentCardBuilder;
use a2a_rs::error::{A2AError, A2AResult};
use a2a_rs::server::{
    a2a_router, AgentExecutor, DefaultRequestHandler, EventQueue, InMemoryTaskStore,
    RequestContext, TaskStore, TaskUpdater,
};
use a2a_rs::types::Part;
use async_trait::async_trait;

/// A simple echo agent that echoes back the text from the user's message.
pub struct EchoAgent;

#[async_trait]
impl AgentExecutor for EchoAgent {
    async fn execute(&self, context: RequestContext, event_queue: EventQueue) -> A2AResult<()> {
        let updater = TaskUpdater::new(
            event_queue,
            context.task_id.clone(),
            context.context_id.clone(),
        );

        // Extract text from incoming message
        let text = {
            let input = context.get_user_input("\n");
            if input.is_empty() {
                "No text received".to_string()
            } else {
                input
            }
        };

        let response = format!("Echo: {}", text);
        updater.complete_with_text(&response).await?;
        Ok(())
    }

    async fn cancel(&self, context: RequestContext, event_queue: EventQueue) -> A2AResult<()> {
        let updater = TaskUpdater::new(event_queue, context.task_id, context.context_id);
        updater.cancel(None).await?;
        Ok(())
    }
}

/// A slow echo agent that publishes intermediate status updates before completing.
pub struct SlowEchoAgent;

#[async_trait]
impl AgentExecutor for SlowEchoAgent {
    async fn execute(&self, context: RequestContext, event_queue: EventQueue) -> A2AResult<()> {
        let updater = TaskUpdater::new(
            event_queue,
            context.task_id.clone(),
            context.context_id.clone(),
        );

        // Extract text
        let text = {
            let input = context.get_user_input("\n");
            if input.is_empty() {
                "No text".to_string()
            } else {
                input
            }
        };

        // Add an artifact before completing
        updater
            .add_artifact(
                vec![Part::text(format!("Processed: {}", text))],
                None,
                Some("output".to_string()),
                None,
                None,
                Some(true),
                None,
            )
            .await?;

        updater
            .complete_with_text(&format!("Done: {}", text))
            .await?;
        Ok(())
    }

    async fn cancel(&self, context: RequestContext, event_queue: EventQueue) -> A2AResult<()> {
        let updater = TaskUpdater::new(event_queue, context.task_id, context.context_id);
        updater.cancel(None).await?;
        Ok(())
    }
}

/// An agent that always fails.
pub struct FailingAgent;

#[async_trait]
impl AgentExecutor for FailingAgent {
    async fn execute(&self, _context: RequestContext, _event_queue: EventQueue) -> A2AResult<()> {
        Err(A2AError::internal_error("Agent intentionally failed"))
    }

    async fn cancel(&self, context: RequestContext, event_queue: EventQueue) -> A2AResult<()> {
        let updater = TaskUpdater::new(event_queue, context.task_id, context.context_id);
        updater.cancel(None).await?;
        Ok(())
    }
}

/// Build a default agent card for testing.
pub fn test_agent_card(url: &str) -> a2a_rs::types::AgentCard {
    AgentCardBuilder::new("Test Echo Agent", "An echo agent for testing", "0.1.0")
        .with_jsonrpc_interface(url)
        .with_streaming(true)
        .with_skill(
            "echo",
            "Echo",
            "Echoes back messages",
            vec!["test".to_string()],
        )
        .build()
}

/// Start a test server on a random port. Returns the base URL and a handle to shut it down.
pub async fn start_test_server(
    executor: Arc<dyn AgentExecutor>,
) -> (String, tokio::task::JoinHandle<()>) {
    start_test_server_with_store(executor, Arc::new(InMemoryTaskStore::new())).await
}

/// Start a test server on a random port with a specific task store.
pub async fn start_test_server_with_store(
    executor: Arc<dyn AgentExecutor>,
    store: Arc<dyn TaskStore>,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    let handler = Arc::new(DefaultRequestHandler::new(executor, store));
    let agent_card = test_agent_card(&format!("{}/a2a", base_url));
    let app = a2a_router(handler, agent_card);

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Brief wait for the server to start accepting connections.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    (base_url, handle)
}

/// Helper to build a JSON-RPC request body.
pub fn jsonrpc_request(
    id: serde_json::Value,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    })
}

/// Helper to build a message/send request body.
pub fn message_send_request(id: i64, text: &str) -> serde_json::Value {
    jsonrpc_request(
        serde_json::json!(id),
        "message/send",
        serde_json::json!({
            "message": {
                "messageId": format!("test-msg-{}", id),
                "role": "user",
                "parts": [{"kind": "text", "text": text}]
            }
        }),
    )
}

/// Helper to build a message/send request with context_id and task_id.
pub fn message_send_with_context(
    id: i64,
    text: &str,
    context_id: &str,
    task_id: Option<&str>,
) -> serde_json::Value {
    let mut message = serde_json::json!({
        "messageId": format!("test-msg-{}", id),
        "role": "user",
        "parts": [{"kind": "text", "text": text}],
        "contextId": context_id
    });
    if let Some(tid) = task_id {
        message["taskId"] = serde_json::json!(tid);
    }
    jsonrpc_request(
        serde_json::json!(id),
        "message/send",
        serde_json::json!({ "message": message }),
    )
}
