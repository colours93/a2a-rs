//! Echo Agent with File Task Store — demonstrates FileTaskStore for task visualization.
//!
//! This variant of the echo agent uses FileTaskStore instead of InMemoryTaskStore,
//! writing tasks to JSON files that can be visualized with the TUI monitor.
//!
//! Run with:
//! ```sh
//! # Terminal 1: Start the agent with file-based storage
//! cargo run --example echo_agent_file
//!
//! # Terminal 2: Watch tasks in real-time
//! cargo run --example tui_monitor -- ./tasks
//!
//! # Terminal 3: Send messages to create tasks
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
use std::path::PathBuf;

use a2a_rs::builders::AgentCardBuilder;
use a2a_rs::error::A2AResult;
use a2a_rs::server::{
    a2a_router, AgentExecutor, DefaultRequestHandler, EventQueue, FileTaskStore, RequestContext,
    TaskUpdater,
};
use a2a_rs::types::Part;

/// Echo agent implementation — just echoes back the text it receives.
struct EchoAgent;

#[async_trait::async_trait]
impl AgentExecutor for EchoAgent {
    async fn execute(&self, ctx: RequestContext, queue: EventQueue) -> A2AResult<()> {
        let updater = TaskUpdater::new(queue, ctx.task_id.clone(), ctx.context_id.clone());

        updater.start_work_with_text("Processing your message...").await?;

        // Extract text from the incoming message using the helper method.
        let text = {
            let input = ctx.get_user_input("\n");
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

        updater.complete_with_text("Done!").await?;
        Ok(())
    }

    async fn cancel(&self, ctx: RequestContext, queue: EventQueue) -> A2AResult<()> {
        let updater = TaskUpdater::new(queue, ctx.task_id, ctx.context_id);
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
        "Echo Agent (File Store)",
        "A simple agent that echoes messages back (with file-based task storage)",
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

    // Create the server components with FileTaskStore
    let executor: Arc<dyn AgentExecutor> = Arc::new(EchoAgent);
    
    // Use FileTaskStore that writes to ./tasks directory
    let tasks_dir = PathBuf::from("./tasks");
    println!("Using task directory: {}", tasks_dir.display());
    let store = Arc::new(FileTaskStore::new(tasks_dir).await?);

    let handler = Arc::new(DefaultRequestHandler::new(executor, store));

    // Build the axum router with A2A routes.
    let app = a2a_router(handler, agent_card);

    // Start the server.
    let addr = "0.0.0.0:3000";
    println!("Echo Agent (File Store) listening on http://{}", addr);
    println!("  Agent card: http://{}/.well-known/agent.json", addr);
    println!("  A2A endpoint: http://{}/a2a", addr);
    println!("  Tasks stored in: ./tasks");
    println!("\nTo visualize tasks, run:");
    println!("  cargo run --example tui_monitor -- ./tasks");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
