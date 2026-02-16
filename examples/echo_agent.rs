//! Echo Agent — a minimal A2A server that echoes messages back.
//!
//! Run with:
//! ```sh
//! cargo run --example echo_agent
//! ```
//!
//! Then test with curl:
//! ```sh
//! # Check agent card
//! curl http://localhost:3000/.well-known/agent.json | jq
//!
//! # Send a message
//! curl -X POST http://localhost:3000/a2a \
//!   -H "Content-Type: application/json" \
//!   -d '{
//!     "jsonrpc": "2.0",
//!     "id": 1,
//!     "method": "message/send",
//!     "params": {
//!       "message": {
//!         "messageId": "m1",
//!         "role": "user",
//!         "parts": [{"kind": "text", "text": "Hello, agent!"}]
//!       }
//!     }
//!   }'
//! ```

use std::sync::Arc;

use a2a_rs::builders::AgentCardBuilder;
use a2a_rs::error::A2AResult;
use a2a_rs::server::{
    a2a_router, AgentExecutor, DefaultRequestHandler, EventQueue, InMemoryTaskStore,
    RequestContext, TaskUpdater,
};
use a2a_rs::types::Part;
use async_trait::async_trait;

/// A simple agent that echoes back whatever you send it.
struct EchoAgent;

#[async_trait]
impl AgentExecutor for EchoAgent {
    async fn execute(&self, context: RequestContext, event_queue: EventQueue) -> A2AResult<()> {
        let updater = TaskUpdater::new(
            event_queue,
            context.task_id.clone(),
            context.context_id.clone(),
        );

        // Extract text from the incoming message using the helper method.
        let text = {
            let input = context.get_user_input("\n");
            if input.is_empty() {
                "No text received".to_string()
            } else {
                input
            }
        };

        // Add an artifact with the echoed text.
        updater
            .add_artifact(
                vec![Part::text(format!("Echo: {}", text))],
                None,
                Some("echo-response".to_string()),
                None,
                None,
                Some(true),
                None,
            )
            .await?;

        // Mark the task as completed.
        updater
            .complete_with_text(&format!("Echoed: {}", text))
            .await?;

        Ok(())
    }

    async fn cancel(&self, context: RequestContext, event_queue: EventQueue) -> A2AResult<()> {
        let updater = TaskUpdater::new(event_queue, context.task_id, context.context_id);
        updater.cancel(None).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for log output.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Build the agent card describing this agent's capabilities.
    let agent_card = AgentCardBuilder::new(
        "Echo Agent",
        "A simple agent that echoes messages back",
        "0.1.0",
    )
    .with_jsonrpc_interface("http://localhost:3000/a2a")
    .with_streaming(true)
    .with_skill(
        "echo",
        "Echo",
        "Echoes back any text message you send",
        vec!["echo".to_string(), "test".to_string()],
    )
    .build();

    // Create the server components.
    let executor: Arc<dyn AgentExecutor> = Arc::new(EchoAgent);
    let store: Arc<dyn a2a_rs::server::TaskStore> = Arc::new(InMemoryTaskStore::new());
    let handler = Arc::new(DefaultRequestHandler::new(executor, store));

    // Build the axum router with A2A routes.
    let app = a2a_router(handler, agent_card);

    // Start the server.
    let addr = "0.0.0.0:3000";
    println!("Echo Agent listening on http://{}", addr);
    println!("  Agent card: http://{}/.well-known/agent.json", addr);
    println!("  A2A endpoint: http://{}/a2a", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
