//! Verify TaskStatusUpdateEvent and TaskArtifactUpdateEvent kind fields.
//!
//! This example serializes both event types to JSON and verifies:
//! - Both have "kind" discriminator field
//! - TaskStatusUpdateEvent has required "final" field (bool, not Option)
//!
//! Run with: cargo run --example verify_event_kinds

use a2a_rs::types::{
    Artifact, Part, TaskArtifactUpdateEvent, TaskState, TaskStatus, TaskStatusUpdateEvent,
};

fn main() {
    println!("=== Verifying A2A Event Serialization ===\n");

    // 1. TaskStatusUpdateEvent
    println!("1. TaskStatusUpdateEvent");
    println!("{}", "-".repeat(60));

    let status_event = TaskStatusUpdateEvent {
        task_id: "task-123".to_string(),
        context_id: "ctx-456".to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus::new(TaskState::Working),
        r#final: false,
        metadata: None,
    };

    let status_json = serde_json::to_string_pretty(&status_event).unwrap();
    println!("{}\n", status_json);

    // Verify fields
    let status_value: serde_json::Value = serde_json::from_str(&status_json).unwrap();
    assert_eq!(
        status_value["kind"], "status-update",
        "kind field must be 'status-update'"
    );
    assert_eq!(
        status_value["final"], false,
        "final field must be present as bool"
    );
    assert_eq!(
        status_value["taskId"], "task-123",
        "taskId field must be present"
    );
    assert_eq!(
        status_value["contextId"], "ctx-456",
        "contextId field must be present"
    );
    println!("✓ kind field: {}", status_value["kind"]);
    println!(
        "✓ final field: {} (type: bool, required)",
        status_value["final"]
    );
    println!("✓ taskId field: {}", status_value["taskId"]);
    println!("✓ contextId field: {}", status_value["contextId"]);

    // Test with final=true
    let status_event_final = TaskStatusUpdateEvent {
        task_id: "task-789".to_string(),
        context_id: "ctx-456".to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus::new(TaskState::Completed),
        r#final: true,
        metadata: None,
    };

    let status_json_final = serde_json::to_string_pretty(&status_event_final).unwrap();
    let status_value_final: serde_json::Value = serde_json::from_str(&status_json_final).unwrap();
    assert_eq!(
        status_value_final["final"], true,
        "final field must be present when true"
    );
    println!("✓ final=true case: {}", status_value_final["final"]);

    println!("\n");

    // 2. TaskArtifactUpdateEvent
    println!("2. TaskArtifactUpdateEvent");
    println!("{}", "-".repeat(60));

    let artifact_event = TaskArtifactUpdateEvent {
        task_id: "task-123".to_string(),
        context_id: "ctx-456".to_string(),
        kind: "artifact-update".to_string(),
        artifact: Artifact {
            artifact_id: "art-001".to_string(),
            name: Some("output.txt".to_string()),
            description: Some("Generated output file".to_string()),
            parts: vec![Part::Text {
                text: "Hello from the agent!".to_string(),
                metadata: None,
            }],
            metadata: None,
            extensions: None,
        },
        append: None,
        last_chunk: None,
        metadata: None,
    };

    let artifact_json = serde_json::to_string_pretty(&artifact_event).unwrap();
    println!("{}\n", artifact_json);

    // Verify fields
    let artifact_value: serde_json::Value = serde_json::from_str(&artifact_json).unwrap();
    assert_eq!(
        artifact_value["kind"], "artifact-update",
        "kind field must be 'artifact-update'"
    );
    assert_eq!(
        artifact_value["taskId"], "task-123",
        "taskId field must be present"
    );
    assert_eq!(
        artifact_value["contextId"], "ctx-456",
        "contextId field must be present"
    );
    println!("✓ kind field: {}", artifact_value["kind"]);
    println!("✓ taskId field: {}", artifact_value["taskId"]);
    println!("✓ contextId field: {}", artifact_value["contextId"]);
    println!(
        "✓ artifact field present: {}",
        artifact_value["artifact"]["artifactId"]
    );

    println!("\n");

    // Summary
    println!("{}", "=".repeat(60));
    println!("✓ All verifications passed!");
    println!("  - TaskStatusUpdateEvent has 'kind' field");
    println!("  - TaskStatusUpdateEvent has required 'final' bool field (not Option)");
    println!("  - TaskArtifactUpdateEvent has 'kind' field");
    println!("  - Both serialize to camelCase JSON per A2A spec");
}
