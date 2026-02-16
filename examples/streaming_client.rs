//! Streaming Client — connects to an A2A agent and streams SSE events.
//!
//! Demonstrates the `message/stream` method which returns real-time status
//! updates and artifact updates via Server-Sent Events.
//!
//! Run the echo agent first:
//! ```sh
//! cargo run --example echo_agent
//! ```
//!
//! Then in another terminal:
//! ```sh
//! cargo run --example streaming_client
//! ```

use a2a_rs::client::A2AClient;
use a2a_rs::types::StreamResponse;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to a local A2A agent.
    let client = A2AClient::from_url("http://localhost:3000").await?;

    let card = client.get_card()?;
    println!("Streaming from: {} (v{})", card.name, card.version);
    println!();

    // Send a message via the streaming endpoint.
    let mut stream = client
        .send_text_stream("Tell me something interesting about Rust!")
        .await?;

    println!("--- Stream started ---");

    // Process each SSE event as it arrives.
    while let Some(event) = stream.next().await {
        match event? {
            StreamResponse::StatusUpdate(update) => {
                println!(
                    "[status] {} (final: {:?})",
                    update.status.state, update.r#final
                );
                if let Some(msg) = &update.status.message {
                    for part in &msg.parts {
                        if let a2a_rs::types::Part::Text { text, .. } = part {
                            println!("  message: {}", text);
                        }
                    }
                }
            }
            StreamResponse::ArtifactUpdate(update) => {
                println!(
                    "[artifact] {} (append: {:?}, last_chunk: {:?})",
                    update.artifact.name.as_deref().unwrap_or("unnamed"),
                    update.append,
                    update.last_chunk,
                );
                for part in &update.artifact.parts {
                    match part {
                        a2a_rs::types::Part::Text { text, .. } => {
                            println!("  content: {}", text);
                        }
                        a2a_rs::types::Part::File { file, .. } => match file {
                            a2a_rs::types::FileContent::Uri(f) => {
                                println!(
                                    "  file (uri): {} ({})",
                                    f.uri,
                                    f.mime_type.as_deref().unwrap_or("unknown type")
                                );
                            }
                            a2a_rs::types::FileContent::Bytes(f) => {
                                println!(
                                    "  file (bytes): {} bytes ({})",
                                    f.bytes.len(),
                                    f.mime_type.as_deref().unwrap_or("unknown type")
                                );
                            }
                        },
                        a2a_rs::types::Part::Data { data, .. } => {
                            println!("  data: {}", data);
                        }
                    }
                }
            }
            StreamResponse::Task(task) => {
                println!("[task] {} — status: {}", task.id, task.status.state);
            }
            StreamResponse::Message(msg) => {
                println!("[message] {} (role: {})", msg.message_id, msg.role);
                for part in &msg.parts {
                    if let a2a_rs::types::Part::Text { text, .. } = part {
                        println!("  {}", text);
                    }
                }
            }
        }
    }

    println!("--- Stream ended ---");

    Ok(())
}
