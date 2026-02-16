//! Tests for utils::task module
//! Ported from reference/a2a-python/tests/utils/test_task.py

use a2a_rs::types::{Artifact, Message, Part, Role, TaskState};
use a2a_rs::utils::{completed_task, new_task, new_text_artifact};
use uuid::Uuid;

#[test]
fn test_new_task_status() {
    let message = Message {
        role: Role::User,
        kind: "message".to_string(),
        parts: vec![Part::Text {
            text: "test message".to_string(),
            metadata: None,
        }],
        message_id: Uuid::new_v4().to_string(),
        context_id: None,
        task_id: None,
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    };
    let task = new_task(message).unwrap();
    assert_eq!(task.status.state, TaskState::Submitted);
}

#[test]
fn test_new_task_generates_ids() {
    let message = Message {
        role: Role::User,
        kind: "message".to_string(),
        parts: vec![Part::Text {
            text: "test message".to_string(),
            metadata: None,
        }],
        message_id: Uuid::new_v4().to_string(),
        context_id: None,
        task_id: None,
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    };
    let task = new_task(message).unwrap();

    // Verify IDs are valid UUIDs
    assert!(Uuid::parse_str(&task.id).is_ok());
    assert!(Uuid::parse_str(&task.context_id).is_ok());
}

#[test]
fn test_new_task_uses_provided_ids() {
    let task_id = Uuid::new_v4().to_string();
    let context_id = Uuid::new_v4().to_string();

    let message = Message {
        role: Role::User,
        kind: "message".to_string(),
        parts: vec![Part::Text {
            text: "test message".to_string(),
            metadata: None,
        }],
        message_id: Uuid::new_v4().to_string(),
        task_id: Some(task_id.clone()),
        context_id: Some(context_id.clone()),
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    };
    let task = new_task(message).unwrap();

    assert_eq!(task.id, task_id);
    assert_eq!(task.context_id, context_id);
}

#[test]
fn test_new_task_initial_message_in_history() {
    let message = Message {
        role: Role::User,
        kind: "message".to_string(),
        parts: vec![Part::Text {
            text: "test message".to_string(),
            metadata: None,
        }],
        message_id: Uuid::new_v4().to_string(),
        context_id: None,
        task_id: None,
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    };
    let msg_id = message.message_id.clone();
    let task = new_task(message).unwrap();

    assert_eq!(task.history.as_ref().unwrap().len(), 1);
    assert_eq!(task.history.as_ref().unwrap()[0].message_id, msg_id);
}

#[test]
fn test_completed_task_status() {
    let task_id = Uuid::new_v4().to_string();
    let context_id = Uuid::new_v4().to_string();
    let artifacts = vec![new_text_artifact("test", "some content", None::<String>)];

    let task = completed_task(&task_id, &context_id, artifacts, None).unwrap();

    assert_eq!(task.status.state, TaskState::Completed);
}

#[test]
fn test_completed_task_assigns_ids_and_artifacts() {
    let task_id = Uuid::new_v4().to_string();
    let context_id = Uuid::new_v4().to_string();
    let artifact = new_text_artifact("test", "some content", None::<String>);
    let artifact_id = artifact.artifact_id.clone();
    let artifacts = vec![artifact];

    let task = completed_task(&task_id, &context_id, artifacts, None).unwrap();

    assert_eq!(task.id, task_id);
    assert_eq!(task.context_id, context_id);
    assert_eq!(task.artifacts.as_ref().unwrap().len(), 1);
    assert_eq!(task.artifacts.as_ref().unwrap()[0].artifact_id, artifact_id);
}

#[test]
fn test_completed_task_empty_history_if_not_provided() {
    let task_id = Uuid::new_v4().to_string();
    let context_id = Uuid::new_v4().to_string();
    let artifacts = vec![new_text_artifact("test", "some content", None::<String>)];

    let task = completed_task(&task_id, &context_id, artifacts, None).unwrap();

    assert!(task.history.is_none() || task.history.as_ref().unwrap().is_empty());
}

#[test]
fn test_completed_task_uses_provided_history() {
    let task_id = Uuid::new_v4().to_string();
    let context_id = Uuid::new_v4().to_string();
    let artifacts = vec![new_text_artifact("test", "some content", None::<String>)];

    let history = vec![
        Message {
            role: Role::User,
            kind: "message".to_string(),
            parts: vec![Part::Text {
                text: "Hello".to_string(),
                metadata: None,
            }],
            message_id: Uuid::new_v4().to_string(),
            context_id: None,
            task_id: None,
            metadata: None,
            extensions: None,
            reference_task_ids: None,
        },
        Message {
            role: Role::Agent,
            kind: "message".to_string(),
            parts: vec![Part::Text {
                text: "Hi there".to_string(),
                metadata: None,
            }],
            message_id: Uuid::new_v4().to_string(),
            context_id: None,
            task_id: None,
            metadata: None,
            extensions: None,
            reference_task_ids: None,
        },
    ];

    let task = completed_task(&task_id, &context_id, artifacts, Some(history.clone())).unwrap();

    let task_history = task.history.as_ref().unwrap();
    assert_eq!(task_history.len(), 2);
    assert_eq!(task_history[0].message_id, history[0].message_id);
    assert_eq!(task_history[1].message_id, history[1].message_id);
}

#[test]
fn test_new_task_invalid_message_empty_parts() {
    let message = Message {
        role: Role::User,
        kind: "message".to_string(),
        parts: vec![],
        message_id: Uuid::new_v4().to_string(),
        context_id: None,
        task_id: None,
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    };

    let result = new_task(message);
    assert!(result.is_err());
}

#[test]
fn test_new_task_invalid_message_empty_content() {
    let message = Message {
        role: Role::User,
        kind: "message".to_string(),
        parts: vec![Part::Text {
            text: "".to_string(),
            metadata: None,
        }],
        message_id: Uuid::new_v4().to_string(),
        context_id: None,
        task_id: None,
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    };

    let result = new_task(message);
    assert!(result.is_err());
}

#[test]
fn test_completed_task_empty_artifacts() {
    let result = completed_task("task-123", "ctx-456", vec![], None);
    assert!(result.is_err());
    match result {
        Err(e) => {
            assert!(e.to_string().contains("artifacts must be a non-empty list"));
        }
        Ok(_) => panic!("Expected error"),
    }
}
