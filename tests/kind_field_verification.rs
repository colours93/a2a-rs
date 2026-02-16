/// Verification test for Task #8: Ensure Task and Message have "kind" fields in JSON output.
///
/// This test creates instances of Task and Message structs, serializes them to JSON,
/// and verifies that the "kind" discriminator field is present in the output.
use a2a_rs::types::{Message, Part, Role, Task, TaskState, TaskStatus};
use serde_json::json;

#[test]
fn test_task_has_kind_field_in_json() {
    // Create a minimal Task instance
    let task = Task {
        id: "test-task-123".to_string(),
        context_id: "test-context-456".to_string(),
        kind: "task".to_string(),
        status: TaskStatus {
            state: TaskState::Submitted,
            message: None,
            timestamp: Some("2026-02-12T00:00:00Z".to_string()),
        },
        artifacts: None,
        history: None,
        metadata: None,
    };

    // Serialize to JSON
    let json_output = serde_json::to_string_pretty(&task).expect("Failed to serialize Task");
    println!("\n=== Task JSON Output ===");
    println!("{}", json_output);
    println!("========================\n");

    // Parse back to verify structure
    let json_value: serde_json::Value =
        serde_json::from_str(&json_output).expect("Failed to parse JSON");

    // Verify "kind" field exists and has correct value
    assert!(
        json_value.get("kind").is_some(),
        "Task JSON must have 'kind' field"
    );
    assert_eq!(
        json_value["kind"].as_str(),
        Some("task"),
        "Task 'kind' field must be 'task'"
    );

    // Verify other required fields
    assert_eq!(json_value["id"].as_str(), Some("test-task-123"));
    assert_eq!(json_value["contextId"].as_str(), Some("test-context-456"));
    assert!(json_value.get("status").is_some());
}

#[test]
fn test_message_has_kind_field_in_json() {
    // Create a minimal Message instance
    let message = Message {
        message_id: "test-msg-789".to_string(),
        role: Role::User,
        kind: "message".to_string(),
        parts: vec![Part::Text {
            text: "Hello, this is a test message.".to_string(),
            metadata: None,
        }],
        context_id: Some("test-context-456".to_string()),
        task_id: Some("test-task-123".to_string()),
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    };

    // Serialize to JSON
    let json_output = serde_json::to_string_pretty(&message).expect("Failed to serialize Message");
    println!("\n=== Message JSON Output ===");
    println!("{}", json_output);
    println!("===========================\n");

    // Parse back to verify structure
    let json_value: serde_json::Value =
        serde_json::from_str(&json_output).expect("Failed to parse JSON");

    // Verify "kind" field exists and has correct value
    assert!(
        json_value.get("kind").is_some(),
        "Message JSON must have 'kind' field"
    );
    assert_eq!(
        json_value["kind"].as_str(),
        Some("message"),
        "Message 'kind' field must be 'message'"
    );

    // Verify other required fields
    assert_eq!(json_value["messageId"].as_str(), Some("test-msg-789"));
    assert_eq!(json_value["role"].as_str(), Some("user"));
    assert!(json_value.get("parts").is_some());
    assert!(json_value["parts"].is_array());
}

#[test]
fn test_task_and_message_kind_fields_together() {
    // Create both structs and serialize to demonstrate they both have "kind"
    let task = Task {
        id: "combined-task-001".to_string(),
        context_id: "combined-context-002".to_string(),
        kind: "task".to_string(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: Some("2026-02-12T12:00:00Z".to_string()),
        },
        artifacts: None,
        history: None,
        metadata: None,
    };

    let message = Message {
        message_id: "combined-msg-003".to_string(),
        role: Role::Agent,
        kind: "message".to_string(),
        parts: vec![Part::Text {
            text: "Task is in progress.".to_string(),
            metadata: None,
        }],
        context_id: Some("combined-context-002".to_string()),
        task_id: Some("combined-task-001".to_string()),
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    };

    // Create a combined JSON structure to show both
    let combined = json!({
        "task": task,
        "message": message
    });

    let json_output = serde_json::to_string_pretty(&combined).expect("Failed to serialize");
    println!("\n=== Combined Task and Message JSON ===");
    println!("{}", json_output);
    println!("======================================\n");

    // Verify both have "kind" fields
    assert!(
        combined["task"].get("kind").is_some(),
        "Task must have 'kind' field in combined output"
    );
    assert!(
        combined["message"].get("kind").is_some(),
        "Message must have 'kind' field in combined output"
    );

    assert_eq!(combined["task"]["kind"].as_str(), Some("task"));
    assert_eq!(combined["message"]["kind"].as_str(), Some("message"));
}
