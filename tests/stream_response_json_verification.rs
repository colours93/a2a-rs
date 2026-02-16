//! Verification test: StreamResponse serializes to flat JSON with "kind" field,
//! NOT wrapper keys like {"task": {...}}.
//!
//! This test shows the actual JSON output for all 4 StreamResponse variants.

use a2a_rs::types::*;
use serde_json::json;

#[test]
fn stream_response_all_variants_produce_flat_json_with_kind() {
    // VARIANT 1: StreamResponse::Task
    let task = Task {
        id: "task-1".to_string(),
        context_id: "ctx-1".to_string(),
        kind: "task".to_string(),
        status: TaskStatus::new(TaskState::Completed),
        artifacts: None,
        history: None,
        metadata: None,
    };
    let sr_task = StreamResponse::Task(task);
    let json_task = serde_json::to_value(&sr_task).unwrap();

    println!("\n=== StreamResponse::Task ===");
    println!("{}", serde_json::to_string_pretty(&json_task).unwrap());

    // Verify: flat JSON with "kind": "task", NOT {"task": {...}}
    assert_eq!(json_task["kind"], "task", "Must have 'kind' field");
    assert_eq!(
        json_task["id"], "task-1",
        "Task fields must be at top level"
    );
    assert_eq!(
        json_task["contextId"], "ctx-1",
        "Task fields must be at top level"
    );
    assert!(
        json_task.get("Task").is_none(),
        "Must NOT have wrapper key 'Task'"
    );
    assert!(
        json_task.get("task").is_none(),
        "Must NOT have wrapper key 'task'"
    );

    // VARIANT 2: StreamResponse::Message
    let msg = Message::user("msg-1", "Hello world");
    let sr_msg = StreamResponse::Message(msg);
    let json_msg = serde_json::to_value(&sr_msg).unwrap();

    println!("\n=== StreamResponse::Message ===");
    println!("{}", serde_json::to_string_pretty(&json_msg).unwrap());

    // Verify: flat JSON with "kind": "message", NOT {"message": {...}}
    assert_eq!(json_msg["kind"], "message", "Must have 'kind' field");
    assert_eq!(
        json_msg["messageId"], "msg-1",
        "Message fields must be at top level"
    );
    assert_eq!(
        json_msg["role"], "user",
        "Message fields must be at top level"
    );
    assert!(
        json_msg.get("Message").is_none(),
        "Must NOT have wrapper key 'Message'"
    );
    assert!(
        json_msg.get("message").is_none(),
        "Must NOT have wrapper key 'message'"
    );

    // VARIANT 3: StreamResponse::StatusUpdate
    let status_event = TaskStatusUpdateEvent {
        task_id: "task-2".to_string(),
        context_id: "ctx-2".to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus::new(TaskState::Working),
        r#final: false,
        metadata: Some(json!({"step": "processing"})),
    };
    let sr_status = StreamResponse::StatusUpdate(status_event);
    let json_status = serde_json::to_value(&sr_status).unwrap();

    println!("\n=== StreamResponse::StatusUpdate ===");
    println!("{}", serde_json::to_string_pretty(&json_status).unwrap());

    // Verify: flat JSON with "kind": "status-update", NOT {"status_update": {...}}
    assert_eq!(
        json_status["kind"], "status-update",
        "Must have 'kind' field"
    );
    assert_eq!(
        json_status["taskId"], "task-2",
        "Event fields must be at top level"
    );
    assert_eq!(
        json_status["contextId"], "ctx-2",
        "Event fields must be at top level"
    );
    assert_eq!(
        json_status["final"], false,
        "Event fields must be at top level"
    );
    assert!(
        json_status.get("StatusUpdate").is_none(),
        "Must NOT have wrapper key 'StatusUpdate'"
    );
    assert!(
        json_status.get("status_update").is_none(),
        "Must NOT have wrapper key 'status_update'"
    );

    // VARIANT 4: StreamResponse::ArtifactUpdate
    let artifact_event = TaskArtifactUpdateEvent {
        task_id: "task-3".to_string(),
        context_id: "ctx-3".to_string(),
        kind: "artifact-update".to_string(),
        artifact: Artifact {
            artifact_id: "artifact-1".to_string(),
            name: Some("result.json".to_string()),
            description: Some("Generated output".to_string()),
            parts: vec![Part::data(json!({"result": "success", "value": 42}))],
            metadata: None,
            extensions: None,
        },
        append: Some(false),
        last_chunk: Some(true),
        metadata: None,
    };
    let sr_artifact = StreamResponse::ArtifactUpdate(artifact_event);
    let json_artifact = serde_json::to_value(&sr_artifact).unwrap();

    println!("\n=== StreamResponse::ArtifactUpdate ===");
    println!("{}", serde_json::to_string_pretty(&json_artifact).unwrap());

    // Verify: flat JSON with "kind": "artifact-update", NOT {"artifact_update": {...}}
    assert_eq!(
        json_artifact["kind"], "artifact-update",
        "Must have 'kind' field"
    );
    assert_eq!(
        json_artifact["taskId"], "task-3",
        "Event fields must be at top level"
    );
    assert_eq!(
        json_artifact["contextId"], "ctx-3",
        "Event fields must be at top level"
    );
    assert_eq!(
        json_artifact["lastChunk"], true,
        "Event fields must be at top level"
    );
    assert_eq!(
        json_artifact["artifact"]["artifactId"], "artifact-1",
        "Artifact must be nested object"
    );
    assert!(
        json_artifact.get("ArtifactUpdate").is_none(),
        "Must NOT have wrapper key 'ArtifactUpdate'"
    );
    assert!(
        json_artifact.get("artifact_update").is_none(),
        "Must NOT have wrapper key 'artifact_update'"
    );

    println!("\n=== VERIFICATION PASSED ===");
    println!("All 4 StreamResponse variants produce flat JSON with 'kind' discriminator.");
    println!("No wrapper keys like {{\"task\": {{...}}}} detected.");
}

#[test]
fn stream_response_roundtrip_all_variants() {
    // Ensure all 4 variants can be serialized and deserialized
    let variants = vec![
        StreamResponse::Task(Task {
            id: "t1".to_string(),
            context_id: "c1".to_string(),
            kind: "task".to_string(),
            status: TaskStatus::new(TaskState::Submitted),
            artifacts: None,
            history: None,
            metadata: None,
        }),
        StreamResponse::Message(Message::agent("m1", "Response")),
        StreamResponse::StatusUpdate(TaskStatusUpdateEvent {
            task_id: "t2".to_string(),
            context_id: "c2".to_string(),
            kind: "status-update".to_string(),
            status: TaskStatus::new(TaskState::Working),
            r#final: false,
            metadata: None,
        }),
        StreamResponse::ArtifactUpdate(TaskArtifactUpdateEvent {
            task_id: "t3".to_string(),
            context_id: "c3".to_string(),
            kind: "artifact-update".to_string(),
            artifact: Artifact {
                artifact_id: "a1".to_string(),
                name: None,
                description: None,
                parts: vec![Part::text("output")],
                metadata: None,
                extensions: None,
            },
            append: None,
            last_chunk: None,
            metadata: None,
        }),
    ];

    for variant in variants {
        let json = serde_json::to_value(&variant).unwrap();
        let decoded: StreamResponse = serde_json::from_value(json).unwrap();

        // Verify variant type matches
        match (&variant, &decoded) {
            (StreamResponse::Task(_), StreamResponse::Task(_)) => {}
            (StreamResponse::Message(_), StreamResponse::Message(_)) => {}
            (StreamResponse::StatusUpdate(_), StreamResponse::StatusUpdate(_)) => {}
            (StreamResponse::ArtifactUpdate(_), StreamResponse::ArtifactUpdate(_)) => {}
            _ => panic!("Roundtrip changed variant type"),
        }
    }
}
