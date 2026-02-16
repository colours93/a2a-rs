//! Hello Client — the simplest possible A2A client.
//!
//! Sends a text message to an A2A agent and prints the response.
//!
//! Run the echo agent first:
//! ```sh
//! cargo run --example echo_agent
//! ```
//!
//! Then in another terminal:
//! ```sh
//! cargo run --example hello_client
//! ```

use a2a_rs::client::{A2AClient, SendMessageResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a local A2A agent. This resolves the agent card from
    // /.well-known/agent-card.json and discovers the JSON-RPC endpoint.
    let client = A2AClient::from_url("http://localhost:3000").await?;

    // Print the agent card info.
    let card = client.get_card()?;
    println!("Connected to: {} (v{})", card.name, card.version);
    println!("Description: {}", card.description);
    println!("Skills:");
    for skill in &card.skills {
        println!("  - {} ({})", skill.name, skill.description);
    }
    println!();

    // Send a simple text message.
    let response = client.send_text("Hello from a2a-rs!").await?;

    match response {
        SendMessageResponse::Task(task) => {
            println!("Task ID: {}", task.id);
            println!("Status: {}", task.status.state);

            // Print artifacts if any.
            if let Some(artifacts) = &task.artifacts {
                for artifact in artifacts {
                    println!(
                        "Artifact: {}",
                        artifact.name.as_deref().unwrap_or("unnamed")
                    );
                    for part in &artifact.parts {
                        match part {
                            a2a_rs::types::Part::Text { text, .. } => {
                                println!("  {}", text);
                            }
                            _ => {
                                println!("  (non-text part)");
                            }
                        }
                    }
                }
            }

            // Print the status message if any.
            if let Some(msg) = &task.status.message {
                for part in &msg.parts {
                    if let a2a_rs::types::Part::Text { text, .. } = part {
                        println!("Agent says: {}", text);
                    }
                }
            }
        }
        SendMessageResponse::Message(msg) => {
            println!("Direct message from agent:");
            for part in &msg.parts {
                if let a2a_rs::types::Part::Text { text, .. } = part {
                    println!("  {}", text);
                }
            }
        }
    }

    Ok(())
}
