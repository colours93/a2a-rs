//! Verify SendMessageResponse serialization uses flat kind discrimination.
//!
//! This example demonstrates that both SendMessageResponse variants
//! (Task and Message) serialize to flat JSON with a "kind" discriminator field.

use a2a_rs::types::*;

fn main() {
    println!("=== SendMessageResponse Serialization Verification ===\n");

    // Test 1: Task variant
    println!("1. SendMessageResponse::Task variant:");
    let task = Task {
        id: "task-123".to_string(),
        context_id: "ctx-abc".to_string(),
        kind: "task".to_string(),
        status: TaskStatus {
            state: TaskState::Working,
            message: Some(Message::agent("msg-1", "Processing your request")),
            timestamp: Some("2025-01-15T10:30:00Z".to_string()),
        },
        artifacts: None,
        history: None,
        metadata: None,
    };

    let response_task = SendMessageResponse::Task(task);
    let json_task = serde_json::to_string_pretty(&response_task).unwrap();

    println!("{}\n", json_task);

    // Verify flat format with "kind" field
    let parsed_task: serde_json::Value = serde_json::from_str(&json_task).unwrap();
    assert_eq!(
        parsed_task["kind"], "task",
        "Task variant must have kind='task'"
    );
    assert_eq!(
        parsed_task["id"], "task-123",
        "Task fields must be at top level"
    );
    println!("✓ Task variant uses flat format with kind='task'\n");

    // Test 2: Message variant
    println!("2. SendMessageResponse::Message variant:");
    let message = Message {
        message_id: "msg-456".to_string(),
        role: Role::Agent,
        kind: "message".to_string(),
        parts: vec![
            Part::text("Here is your direct response"),
            Part::data(serde_json::json!({"status": "success", "data": [1, 2, 3]})),
        ],
        context_id: Some("ctx-xyz".to_string()),
        task_id: None,
        metadata: Some(serde_json::json!({"response_type": "direct"})),
        extensions: None,
        reference_task_ids: None,
    };

    let response_msg = SendMessageResponse::Message(message);
    let json_msg = serde_json::to_string_pretty(&response_msg).unwrap();

    println!("{}\n", json_msg);

    // Verify flat format with "kind" field
    let parsed_msg: serde_json::Value = serde_json::from_str(&json_msg).unwrap();
    assert_eq!(
        parsed_msg["kind"], "message",
        "Message variant must have kind='message'"
    );
    assert_eq!(
        parsed_msg["messageId"], "msg-456",
        "Message fields must be at top level"
    );
    println!("✓ Message variant uses flat format with kind='message'\n");

    // Test 3: Roundtrip deserialization
    println!("3. Roundtrip deserialization test:");

    let task_roundtrip: SendMessageResponse = serde_json::from_str(&json_task).unwrap();
    match task_roundtrip {
        SendMessageResponse::Task(t) => {
            println!("✓ Task deserialized correctly: id={}", t.id);
            assert_eq!(t.id, "task-123");
        }
        _ => panic!("Expected Task variant"),
    }

    let msg_roundtrip: SendMessageResponse = serde_json::from_str(&json_msg).unwrap();
    match msg_roundtrip {
        SendMessageResponse::Message(m) => {
            println!(
                "✓ Message deserialized correctly: messageId={}",
                m.message_id
            );
            assert_eq!(m.message_id, "msg-456");
        }
        _ => panic!("Expected Message variant"),
    }

    println!("\n=== All Verification Tests Passed ===");
    println!("\nConclusion:");
    println!("- SendMessageResponse::Task serializes with kind='task' at top level");
    println!("- SendMessageResponse::Message serializes with kind='message' at top level");
    println!("- Both variants use FLAT discrimination (no wrapper keys)");
    println!("- Deserialization correctly reconstructs the enum variants");
}
