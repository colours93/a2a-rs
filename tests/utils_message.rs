//! Tests for utils::message module
//! Ported from reference/a2a-python/tests/utils/test_message.py

use a2a_rs::types::{Message, Part, Role};
use a2a_rs::utils::{get_message_text, new_agent_parts_message, new_agent_text_message};
use serde_json::json;
use uuid::Uuid;

// TestNewAgentTextMessage class tests

#[test]
fn test_new_agent_text_message_basic() {
    // Setup
    let text = "Hello, I'm an agent";

    // Exercise
    let message = new_agent_text_message(text, None::<String>, None::<String>);

    // Verify
    assert_eq!(message.role, Role::Agent);
    assert_eq!(message.parts.len(), 1);
    match &message.parts[0] {
        Part::Text {
            text: part_text, ..
        } => {
            assert_eq!(part_text, text);
        }
        _ => panic!("Expected text part"),
    }
    assert!(Uuid::parse_str(&message.message_id).is_ok());
    assert!(message.task_id.is_none());
    assert!(message.context_id.is_none());
}

#[test]
fn test_new_agent_text_message_with_context_id() {
    // Setup
    let text = "Message with context";
    let context_id = "test-context-id";

    // Exercise
    let message = new_agent_text_message(text, Some(context_id), None::<String>);

    // Verify
    assert_eq!(message.role, Role::Agent);
    match &message.parts[0] {
        Part::Text {
            text: part_text, ..
        } => {
            assert_eq!(part_text, text);
        }
        _ => panic!("Expected text part"),
    }
    assert!(Uuid::parse_str(&message.message_id).is_ok());
    assert_eq!(message.context_id, Some(context_id.to_string()));
    assert!(message.task_id.is_none());
}

#[test]
fn test_new_agent_text_message_with_task_id() {
    // Setup
    let text = "Message with task id";
    let task_id = "test-task-id";

    // Exercise
    let message = new_agent_text_message(text, None::<String>, Some(task_id));

    // Verify
    assert_eq!(message.role, Role::Agent);
    match &message.parts[0] {
        Part::Text {
            text: part_text, ..
        } => {
            assert_eq!(part_text, text);
        }
        _ => panic!("Expected text part"),
    }
    assert!(Uuid::parse_str(&message.message_id).is_ok());
    assert_eq!(message.task_id, Some(task_id.to_string()));
    assert!(message.context_id.is_none());
}

#[test]
fn test_new_agent_text_message_with_both_ids() {
    // Setup
    let text = "Message with both ids";
    let context_id = "test-context-id";
    let task_id = "test-task-id";

    // Exercise
    let message = new_agent_text_message(text, Some(context_id), Some(task_id));

    // Verify
    assert_eq!(message.role, Role::Agent);
    match &message.parts[0] {
        Part::Text {
            text: part_text, ..
        } => {
            assert_eq!(part_text, text);
        }
        _ => panic!("Expected text part"),
    }
    assert!(Uuid::parse_str(&message.message_id).is_ok());
    assert_eq!(message.context_id, Some(context_id.to_string()));
    assert_eq!(message.task_id, Some(task_id.to_string()));
}

#[test]
fn test_new_agent_text_message_empty_text() {
    // Setup
    let text = "";

    // Exercise
    let message = new_agent_text_message(text, None::<String>, None::<String>);

    // Verify
    assert_eq!(message.role, Role::Agent);
    match &message.parts[0] {
        Part::Text {
            text: part_text, ..
        } => {
            assert_eq!(part_text, "");
        }
        _ => panic!("Expected text part"),
    }
    assert!(Uuid::parse_str(&message.message_id).is_ok());
}

// TestNewAgentPartsMessage class tests

#[test]
fn test_new_agent_parts_message() {
    // Setup
    let parts = vec![
        Part::Text {
            text: "Here is some text.".to_string(),
            metadata: None,
        },
        Part::Data {
            data: json!({"product_id": 123, "quantity": 2}),
            metadata: None,
        },
    ];
    let context_id = "ctx-multi-part";
    let task_id = "task-multi-part";

    // Exercise
    let message = new_agent_parts_message(parts.clone(), Some(context_id), Some(task_id));

    // Verify
    assert_eq!(message.role, Role::Agent);
    assert_eq!(message.parts.len(), parts.len());
    assert_eq!(message.context_id, Some(context_id.to_string()));
    assert_eq!(message.task_id, Some(task_id.to_string()));
    assert!(Uuid::parse_str(&message.message_id).is_ok());
}

// TestGetMessageText class tests

#[test]
fn test_get_message_text_single_part() {
    // Setup
    let message = Message {
        role: Role::Agent,
        kind: "message".to_string(),
        parts: vec![Part::Text {
            text: "Hello world".to_string(),
            metadata: None,
        }],
        message_id: "test-message-id".to_string(),
        context_id: None,
        task_id: None,
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    };

    // Exercise
    let result = get_message_text(&message, "\n");

    // Verify
    assert_eq!(result, "Hello world");
}

#[test]
fn test_get_message_text_multiple_parts() {
    // Setup
    let message = Message {
        role: Role::Agent,
        kind: "message".to_string(),
        parts: vec![
            Part::Text {
                text: "First line".to_string(),
                metadata: None,
            },
            Part::Text {
                text: "Second line".to_string(),
                metadata: None,
            },
            Part::Text {
                text: "Third line".to_string(),
                metadata: None,
            },
        ],
        message_id: "test-message-id".to_string(),
        context_id: None,
        task_id: None,
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    };

    // Exercise
    let result = get_message_text(&message, "\n");

    // Verify - default delimiter is newline
    assert_eq!(result, "First line\nSecond line\nThird line");
}

#[test]
fn test_get_message_text_custom_delimiter() {
    // Setup
    let message = Message {
        role: Role::Agent,
        kind: "message".to_string(),
        parts: vec![
            Part::Text {
                text: "First part".to_string(),
                metadata: None,
            },
            Part::Text {
                text: "Second part".to_string(),
                metadata: None,
            },
            Part::Text {
                text: "Third part".to_string(),
                metadata: None,
            },
        ],
        message_id: "test-message-id".to_string(),
        context_id: None,
        task_id: None,
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    };

    // Exercise
    let result = get_message_text(&message, " | ");

    // Verify
    assert_eq!(result, "First part | Second part | Third part");
}

#[test]
fn test_get_message_text_empty_parts() {
    // Setup
    let message = Message {
        role: Role::Agent,
        kind: "message".to_string(),
        parts: vec![],
        message_id: "test-message-id".to_string(),
        context_id: None,
        task_id: None,
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    };

    // Exercise
    let result = get_message_text(&message, "\n");

    // Verify
    assert_eq!(result, "");
}
