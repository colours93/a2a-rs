//! Multi-Turn Conversation — demonstrates context_id for multi-turn conversations.
//!
//! Shows how to use `context_id` to group related messages into a single
//! conversation thread. The echo agent will process each message independently,
//! but the conversation history is tracked via the shared context.
//!
//! Run the echo agent first:
//! ```sh
//! cargo run --example echo_agent
//! ```
//!
//! Then in another terminal:
//! ```sh
//! cargo run --example multi_turn
//! ```

use a2a_rs::client::{A2AClient, SendMessageResponse};
use a2a_rs::types::{Part, Task};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a local A2A agent.
    let client = A2AClient::from_url("http://localhost:3000").await?;

    let card = client.get_card()?;
    println!("Multi-turn conversation with: {}", card.name);
    println!();

    // --- Turn 1: Initial message ---
    println!("=== Turn 1 ===");
    let task1 = expect_task(client.send_text("What is the A2A protocol?").await?);

    println!("Task: {}", task1.id);
    println!("Context: {}", task1.context_id);
    println!("Status: {}", task1.status.state);
    print_task_output(&task1);

    // Save the context_id from the first turn.
    let context_id = task1.context_id.clone();
    println!();

    // --- Turn 2: Follow-up in the same context ---
    println!("=== Turn 2 (same context: {}) ===", &context_id[..8]);
    let task2 = expect_task(
        client
            .send_text_in_context("Tell me more about streaming.", &context_id)
            .await?,
    );

    println!("Task: {}", task2.id);
    println!("Context: {}", task2.context_id);
    println!("Status: {}", task2.status.state);
    print_task_output(&task2);
    println!();

    // --- Turn 3: Another follow-up ---
    println!("=== Turn 3 (same context: {}) ===", &context_id[..8]);
    let task3 = expect_task(
        client
            .send_text_in_context("Thanks, that's helpful!", &context_id)
            .await?,
    );

    println!("Task: {}", task3.id);
    println!("Context: {}", task3.context_id);
    println!("Status: {}", task3.status.state);
    print_task_output(&task3);
    println!();

    // --- New context: independent conversation ---
    println!("=== New conversation (different context) ===");
    let task4 = expect_task(client.send_text("This is a fresh conversation.").await?);

    println!("Task: {}", task4.id);
    println!("Context: {}", task4.context_id);
    println!("Status: {}", task4.status.state);
    print_task_output(&task4);

    // Verify contexts are different.
    assert_ne!(
        task1.context_id, task4.context_id,
        "New conversation should have a different context_id"
    );
    println!();
    println!("Context IDs:");
    println!("  Turns 1-3 shared context: {}", &task1.context_id[..8]);
    println!("  Turn 4 new context:       {}", &task4.context_id[..8]);

    Ok(())
}

/// Extract a Task from a SendMessageResponse, panicking if it's a Message.
fn expect_task(response: SendMessageResponse) -> Task {
    match response {
        SendMessageResponse::Task(task) => task,
        SendMessageResponse::Message(_) => panic!("Expected a Task response, got a Message"),
    }
}

fn print_task_output(task: &Task) {
    if let Some(artifacts) = &task.artifacts {
        for artifact in artifacts {
            for part in &artifact.parts {
                if let Part::Text { text, .. } = part {
                    println!("  -> {}", text);
                }
            }
        }
    }
    if let Some(msg) = &task.status.message {
        for part in &msg.parts {
            if let Part::Text { text, .. } = part {
                println!("  Agent: {}", text);
            }
        }
    }
}
