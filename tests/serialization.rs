//! Exhaustive serialization roundtrip tests for every A2A type.
//!
//! These tests verify:
//! - Correct camelCase field names in JSON output
//! - Successful roundtrip (serialize -> deserialize -> equal)
//! - Correct discriminator tags (kind, type)
//! - Optional fields are omitted when None
//!
//! Wire format follows the Python SDK (a2a-python) as the source of truth.

use a2a_rs::types::*;
use serde_json::json;

// ============================================================================
// TaskState
// ============================================================================

#[test]
fn task_state_all_variants_serialize() {
    let cases = vec![
        (TaskState::Submitted, "submitted"),
        (TaskState::Working, "working"),
        (TaskState::Completed, "completed"),
        (TaskState::Failed, "failed"),
        (TaskState::Canceled, "canceled"),
        (TaskState::InputRequired, "input-required"),
        (TaskState::Rejected, "rejected"),
        (TaskState::AuthRequired, "auth-required"),
        (TaskState::Unknown, "unknown"),
    ];

    for (state, expected) in cases {
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, format!("\"{}\"", expected));

        // Roundtrip
        let deserialized: TaskState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, state);
    }
}

// ============================================================================
// Role
// ============================================================================

#[test]
fn role_serialization() {
    assert_eq!(serde_json::to_string(&Role::User).unwrap(), r#""user""#);
    assert_eq!(serde_json::to_string(&Role::Agent).unwrap(), r#""agent""#);
    assert_eq!(
        serde_json::to_string(&Role::Unspecified).unwrap(),
        r#""unspecified""#
    );

    // Roundtrip
    let user: Role = serde_json::from_str(r#""user""#).unwrap();
    assert_eq!(user, Role::User);
    let agent: Role = serde_json::from_str(r#""agent""#).unwrap();
    assert_eq!(agent, Role::Agent);
}

// ============================================================================
// Part (Python SDK style -- "kind" discriminator, nested file object)
// ============================================================================

#[test]
fn text_part_serialization() {
    let part = Part::text("Hello world");
    let json = serde_json::to_value(&part).unwrap();

    // Must have "kind": "text" discriminator (Python SDK format)
    assert_eq!(json["kind"], "text");
    assert_eq!(json["text"], "Hello world");
    // metadata should be omitted when None
    assert!(json.get("metadata").is_none());

    // Roundtrip
    let decoded: Part = serde_json::from_value(json).unwrap();
    match decoded {
        Part::Text { text, metadata } => {
            assert_eq!(text, "Hello world");
            assert!(metadata.is_none());
        }
        _ => panic!("Expected Text part"),
    }
}

#[test]
fn text_part_with_metadata() {
    let part = Part::Text {
        text: "hello".to_string(),
        metadata: Some(json!({"source": "test"})),
    };
    let json = serde_json::to_value(&part).unwrap();
    assert_eq!(json["kind"], "text");
    assert_eq!(json["metadata"]["source"], "test");

    // Roundtrip
    let decoded: Part = serde_json::from_value(json).unwrap();
    match decoded {
        Part::Text { metadata, .. } => {
            assert!(metadata.is_some());
            assert_eq!(metadata.unwrap()["source"], "test");
        }
        _ => panic!("Expected Text part"),
    }
}

#[test]
fn file_part_uri_serialization() {
    let part = Part::file_from_uri(
        "https://example.com/file.pdf",
        None,
        Some("application/pdf".into()),
    );
    let json = serde_json::to_value(&part).unwrap();

    // Must have "kind": "file" and nested "file" object (Python SDK format)
    assert_eq!(json["kind"], "file");
    assert!(json.get("file").is_some());
    assert_eq!(json["file"]["uri"], "https://example.com/file.pdf");
    assert_eq!(json["file"]["mimeType"], "application/pdf");

    // Roundtrip
    let decoded: Part = serde_json::from_value(json).unwrap();
    match decoded {
        Part::File {
            file: FileContent::Uri(f),
            ..
        } => {
            assert_eq!(f.uri, "https://example.com/file.pdf");
            assert_eq!(f.mime_type.unwrap(), "application/pdf");
        }
        _ => panic!("Expected File(Uri) part"),
    }
}

#[test]
fn file_part_bytes_serialization() {
    let part = Part::file_from_bytes(
        "SGVsbG8=",
        Some("hello.txt".into()),
        Some("text/plain".into()),
    );
    let json = serde_json::to_value(&part).unwrap();

    // Must have "kind": "file" and nested "file" object (Python SDK format)
    assert_eq!(json["kind"], "file");
    assert!(json.get("file").is_some());
    assert_eq!(json["file"]["bytes"], "SGVsbG8=");
    assert_eq!(json["file"]["name"], "hello.txt");
    assert_eq!(json["file"]["mimeType"], "text/plain");

    // Roundtrip
    let decoded: Part = serde_json::from_value(json).unwrap();
    match decoded {
        Part::File {
            file: FileContent::Bytes(f),
            ..
        } => {
            assert_eq!(f.bytes, "SGVsbG8=");
            assert_eq!(f.name.unwrap(), "hello.txt");
            assert_eq!(f.mime_type.unwrap(), "text/plain");
        }
        _ => panic!("Expected File(Bytes) part"),
    }
}

#[test]
fn data_part_serialization() {
    let part = Part::data(json!({"key": "value", "nested": {"a": 1}}));
    let json_val = serde_json::to_value(&part).unwrap();

    // Must have "kind": "data" discriminator
    assert_eq!(json_val["kind"], "data");
    assert_eq!(json_val["data"]["key"], "value");
    assert_eq!(json_val["data"]["nested"]["a"], 1);

    // Roundtrip
    let decoded: Part = serde_json::from_value(json_val).unwrap();
    match decoded {
        Part::Data { data, .. } => {
            assert_eq!(data["key"], "value");
        }
        _ => panic!("Expected Data part"),
    }
}

#[test]
fn text_part_json_exact_match() {
    // Verify exact JSON matches Python SDK output
    let part = Part::text("hello");
    let json_str = serde_json::to_string(&part).unwrap();
    assert!(json_str.contains(r#""kind":"text""#));
    assert!(json_str.contains(r#""text":"hello""#));
}

#[test]
fn file_part_bytes_json_exact_match() {
    // Verify exact JSON matches Python SDK output
    let part = Part::file_from_bytes(
        "SGVsbG8=",
        Some("hello.txt".into()),
        Some("text/plain".into()),
    );
    let json_str = serde_json::to_string(&part).unwrap();
    assert!(json_str.contains(r#""kind":"file""#));
    assert!(json_str.contains(r#""file":"#));
    assert!(json_str.contains(r#""bytes":"SGVsbG8=""#));
    assert!(json_str.contains(r#""mimeType":"text/plain""#));
    assert!(json_str.contains(r#""name":"hello.txt""#));
}

#[test]
fn file_part_uri_json_exact_match() {
    // Verify exact JSON matches Python SDK output
    let part = Part::file_from_uri(
        "https://example.com/file.pdf",
        None,
        Some("application/pdf".into()),
    );
    let json_str = serde_json::to_string(&part).unwrap();
    assert!(json_str.contains(r#""kind":"file""#));
    assert!(json_str.contains(r#""file":"#));
    assert!(json_str.contains(r#""uri":"https://example.com/file.pdf""#));
}

#[test]
fn part_deserialization_from_python_sdk_json() {
    // Verify we can deserialize JSON produced by the Python SDK
    let text_json = json!({"kind": "text", "text": "hello"});
    let text: Part = serde_json::from_value(text_json).unwrap();
    assert!(matches!(text, Part::Text { .. }));

    let file_bytes_json = json!({
        "kind": "file",
        "file": {"bytes": "SGVsbG8=", "mimeType": "text/plain", "name": "hello.txt"}
    });
    let file_bytes: Part = serde_json::from_value(file_bytes_json).unwrap();
    assert!(matches!(file_bytes, Part::File { .. }));

    let file_uri_json = json!({
        "kind": "file",
        "file": {"uri": "https://example.com/file.pdf", "mimeType": "application/pdf"}
    });
    let file_uri: Part = serde_json::from_value(file_uri_json).unwrap();
    assert!(matches!(file_uri, Part::File { .. }));

    let data_json = json!({"kind": "data", "data": {"key": "value"}});
    let data: Part = serde_json::from_value(data_json).unwrap();
    assert!(matches!(data, Part::Data { .. }));
}

// ============================================================================
// Message
// ============================================================================

#[test]
fn message_camel_case_fields() {
    let msg = Message::user("msg-1", "Hello");
    let json = serde_json::to_value(&msg).unwrap();

    // Verify camelCase
    assert!(json.get("messageId").is_some());
    assert!(json.get("message_id").is_none()); // NOT snake_case
    assert_eq!(json["messageId"], "msg-1");
    assert_eq!(json["role"], "user");
    // Python SDK: Message has "kind": "message"
    assert_eq!(json["kind"], "message");
    assert!(json["parts"].is_array());
}

#[test]
fn message_optional_fields_omitted() {
    let msg = Message::user("m1", "Hello");
    let json = serde_json::to_value(&msg).unwrap();

    // These optional fields should be omitted
    assert!(json.get("contextId").is_none());
    assert!(json.get("taskId").is_none());
    assert!(json.get("metadata").is_none());
    assert!(json.get("extensions").is_none());
    assert!(json.get("referenceTaskIds").is_none());
}

#[test]
fn message_with_all_fields() {
    let msg = Message {
        message_id: "m1".to_string(),
        role: Role::Agent,
        kind: "message".to_string(),
        parts: vec![Part::text("Hello")],
        context_id: Some("ctx-1".to_string()),
        task_id: Some("task-1".to_string()),
        metadata: Some(json!({"key": "val"})),
        extensions: Some(vec!["ext1".to_string()]),
        reference_task_ids: Some(vec!["ref-1".to_string()]),
    };
    let json = serde_json::to_value(&msg).unwrap();

    assert_eq!(json["contextId"], "ctx-1");
    assert_eq!(json["taskId"], "task-1");
    assert_eq!(json["metadata"]["key"], "val");
    assert_eq!(json["extensions"][0], "ext1");
    assert_eq!(json["referenceTaskIds"][0], "ref-1");

    // Roundtrip
    let decoded: Message = serde_json::from_value(json).unwrap();
    assert_eq!(decoded.context_id.unwrap(), "ctx-1");
    assert_eq!(decoded.task_id.unwrap(), "task-1");
}

#[test]
fn message_kind_defaults_on_deserialize() {
    let json = json!({
        "messageId": "m1",
        "role": "user",
        "parts": [{"kind": "text", "text": "hello"}]
    });
    let msg: Message = serde_json::from_value(json).unwrap();
    assert_eq!(msg.message_id, "m1");
    assert_eq!(msg.kind, "message");
}

// ============================================================================
// Task
// ============================================================================

#[test]
fn task_camel_case_fields() {
    let task = Task {
        id: "t1".to_string(),
        context_id: "ctx1".to_string(),
        kind: "task".to_string(),
        status: TaskStatus::new(TaskState::Working),
        artifacts: None,
        history: None,
        metadata: None,
    };
    let json = serde_json::to_value(&task).unwrap();

    assert_eq!(json["id"], "t1");
    assert_eq!(json["contextId"], "ctx1");
    assert!(json.get("context_id").is_none()); // NOT snake_case
    assert_eq!(json["status"]["state"], "working");
    // Python SDK: Task has "kind": "task"
    assert_eq!(json["kind"], "task");
}

#[test]
fn task_optional_fields_omitted() {
    let task = Task {
        id: "t1".to_string(),
        context_id: "ctx1".to_string(),
        kind: "task".to_string(),
        status: TaskStatus::new(TaskState::Submitted),
        artifacts: None,
        history: None,
        metadata: None,
    };
    let json = serde_json::to_value(&task).unwrap();

    assert!(json.get("artifacts").is_none());
    assert!(json.get("history").is_none());
    assert!(json.get("metadata").is_none());
}

#[test]
fn task_with_artifacts_and_history() {
    let task = Task {
        id: "t1".to_string(),
        context_id: "ctx1".to_string(),
        kind: "task".to_string(),
        status: TaskStatus::new(TaskState::Completed),
        artifacts: Some(vec![Artifact {
            artifact_id: "a1".to_string(),
            name: Some("output.txt".to_string()),
            description: None,
            parts: vec![Part::text("content")],
            metadata: None,
            extensions: None,
        }]),
        history: Some(vec![Message::user("m1", "Hello")]),
        metadata: Some(json!({"custom": true})),
    };
    let json = serde_json::to_value(&task).unwrap();

    assert_eq!(json["artifacts"][0]["artifactId"], "a1");
    assert_eq!(json["artifacts"][0]["name"], "output.txt");
    assert_eq!(json["history"][0]["messageId"], "m1");
    assert_eq!(json["metadata"]["custom"], true);

    // Roundtrip
    let decoded: Task = serde_json::from_value(json).unwrap();
    assert_eq!(decoded.artifacts.unwrap().len(), 1);
    assert_eq!(decoded.history.unwrap().len(), 1);
}

#[test]
fn task_deserializes_without_kind() {
    let json = json!({
        "id": "t1",
        "contextId": "ctx1",
        "status": {"state": "submitted"}
    });
    let task: Task = serde_json::from_value(json).unwrap();
    assert_eq!(task.id, "t1");
    assert_eq!(task.kind, "task");
}

// ============================================================================
// TaskStatus
// ============================================================================

#[test]
fn task_status_serialization() {
    let status = TaskStatus {
        state: TaskState::Working,
        message: Some(Message::agent("m1", "Processing")),
        timestamp: Some("2025-01-01T00:00:00Z".to_string()),
    };
    let json = serde_json::to_value(&status).unwrap();

    assert_eq!(json["state"], "working");
    assert_eq!(json["message"]["messageId"], "m1");
    assert_eq!(json["timestamp"], "2025-01-01T00:00:00Z");
}

// ============================================================================
// Artifact
// ============================================================================

#[test]
fn artifact_camel_case_fields() {
    let artifact = Artifact {
        artifact_id: "a1".to_string(),
        name: Some("result.json".to_string()),
        description: Some("The result".to_string()),
        parts: vec![Part::data(json!({"result": 42}))],
        metadata: Some(json!({"generated": true})),
        extensions: Some(vec!["ext-1".to_string()]),
    };
    let json = serde_json::to_value(&artifact).unwrap();

    assert_eq!(json["artifactId"], "a1");
    assert!(json.get("artifact_id").is_none()); // NOT snake_case
    assert_eq!(json["name"], "result.json");
    assert_eq!(json["description"], "The result");
    assert!(json["parts"][0]["data"]["result"].is_number());
}

// ============================================================================
// StreamResponse (Python SDK: flat with "kind" discrimination)
// ============================================================================

#[test]
fn stream_response_task_variant_roundtrip() {
    let task = Task {
        id: "t1".to_string(),
        context_id: "ctx1".to_string(),
        kind: "task".to_string(),
        status: TaskStatus::new(TaskState::Completed),
        artifacts: None,
        history: None,
        metadata: None,
    };
    let sr = StreamResponse::Task(task);
    let json = serde_json::to_value(&sr).unwrap();
    // Python SDK: flat with "kind": "task"
    assert_eq!(json["kind"], "task");
    assert_eq!(json["id"], "t1");

    let decoded: StreamResponse = serde_json::from_value(json).unwrap();
    match decoded {
        StreamResponse::Task(t) => assert_eq!(t.id, "t1"),
        _ => panic!("Expected Task variant"),
    }
}

#[test]
fn stream_response_message_variant_roundtrip() {
    let msg = Message::agent("m1", "Hello!");
    let sr = StreamResponse::Message(msg);
    let json = serde_json::to_value(&sr).unwrap();
    // Python SDK: flat with "kind": "message"
    assert_eq!(json["kind"], "message");
    assert_eq!(json["messageId"], "m1");

    let decoded: StreamResponse = serde_json::from_value(json).unwrap();
    match decoded {
        StreamResponse::Message(m) => assert_eq!(m.message_id, "m1"),
        _ => panic!("Expected Message variant"),
    }
}

#[test]
fn stream_response_status_update_roundtrip() {
    let event = TaskStatusUpdateEvent {
        task_id: "t1".to_string(),
        context_id: "ctx1".to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus::new(TaskState::Completed),
        r#final: true,
        metadata: None,
    };
    let sr = StreamResponse::StatusUpdate(event);
    let json = serde_json::to_value(&sr).unwrap();

    // Python SDK: flat with "kind": "status-update"
    assert_eq!(json["kind"], "status-update");
    assert_eq!(json["taskId"], "t1");
    assert_eq!(json["contextId"], "ctx1");
    assert_eq!(json["final"], true);

    let decoded: StreamResponse = serde_json::from_value(json).unwrap();
    match decoded {
        StreamResponse::StatusUpdate(e) => {
            assert_eq!(e.task_id, "t1");
            assert_eq!(e.r#final, true);
        }
        _ => panic!("Expected StatusUpdate variant"),
    }
}

#[test]
fn stream_response_artifact_update_roundtrip() {
    let event = TaskArtifactUpdateEvent {
        task_id: "t1".to_string(),
        context_id: "ctx1".to_string(),
        kind: "artifact-update".to_string(),
        artifact: Artifact {
            artifact_id: "a1".to_string(),
            name: Some("output".to_string()),
            description: None,
            parts: vec![Part::text("content")],
            metadata: None,
            extensions: None,
        },
        append: Some(false),
        last_chunk: Some(true),
        metadata: None,
    };
    let sr = StreamResponse::ArtifactUpdate(event);
    let json = serde_json::to_value(&sr).unwrap();

    // Python SDK: flat with "kind": "artifact-update"
    assert_eq!(json["kind"], "artifact-update");
    assert_eq!(json["taskId"], "t1");
    assert_eq!(json["lastChunk"], true);

    let decoded: StreamResponse = serde_json::from_value(json).unwrap();
    match decoded {
        StreamResponse::ArtifactUpdate(e) => {
            assert_eq!(e.artifact.artifact_id, "a1");
            assert_eq!(e.last_chunk, Some(true));
        }
        _ => panic!("Expected ArtifactUpdate variant"),
    }
}

#[test]
fn stream_response_unknown_kind_errors() {
    let json = json!({
        "kind": "unknownType",
        "data": {}
    });
    let result: Result<StreamResponse, _> = serde_json::from_value(json);
    assert!(result.is_err());
}

// ============================================================================
// SendMessageResponse (Python SDK: flat with "kind" discrimination)
// ============================================================================

#[test]
fn send_message_response_task_roundtrip() {
    let task = Task {
        id: "t1".to_string(),
        context_id: "ctx1".to_string(),
        kind: "task".to_string(),
        status: TaskStatus::new(TaskState::Submitted),
        artifacts: None,
        history: None,
        metadata: None,
    };
    let resp = SendMessageResponse::Task(task);
    let json = serde_json::to_value(&resp).unwrap();
    // Python SDK: flat with "kind": "task"
    assert_eq!(json["kind"], "task");
    assert_eq!(json["id"], "t1");

    let decoded: SendMessageResponse = serde_json::from_value(json).unwrap();
    match decoded {
        SendMessageResponse::Task(t) => assert_eq!(t.id, "t1"),
        _ => panic!("Expected Task"),
    }
}

#[test]
fn send_message_response_message_roundtrip() {
    let msg = Message::agent("m1", "Direct response");
    let resp = SendMessageResponse::Message(msg);
    let json = serde_json::to_value(&resp).unwrap();
    // Python SDK: flat with "kind": "message"
    assert_eq!(json["kind"], "message");
    assert_eq!(json["messageId"], "m1");

    let decoded: SendMessageResponse = serde_json::from_value(json).unwrap();
    match decoded {
        SendMessageResponse::Message(m) => assert_eq!(m.message_id, "m1"),
        _ => panic!("Expected Message"),
    }
}

// ============================================================================
// TaskStatusUpdateEvent
// ============================================================================

#[test]
fn task_status_update_event_final_field() {
    let event = TaskStatusUpdateEvent {
        task_id: "t1".to_string(),
        context_id: "ctx1".to_string(),
        kind: "status-update".to_string(),
        status: TaskStatus::new(TaskState::Completed),
        r#final: true,
        metadata: None,
    };
    let json = serde_json::to_value(&event).unwrap();

    // Python SDK: "final" is required bool
    assert_eq!(json["final"], true);
    assert_eq!(json["kind"], "status-update");
    assert_eq!(json["taskId"], "t1");
    assert_eq!(json["contextId"], "ctx1");
}

// ============================================================================
// AgentCard
// ============================================================================

#[test]
fn agent_card_serialization() {
    let card = AgentCard {
        name: "Test Agent".to_string(),
        description: "A test agent".to_string(),
        version: "1.0.0".to_string(),
        url: "http://localhost:8080/a2a".to_string(),
        supported_interfaces: vec![AgentInterface {
            url: "http://localhost:8080/a2a".to_string(),
            transport: "JSONRPC".to_string(),
            tenant: None,
            protocol_version: Some("0.3".to_string()),
        }],
        provider: Some(AgentProvider {
            organization: "Test Org".to_string(),
            url: "https://test.org".to_string(),
        }),
        documentation_url: Some("https://docs.test.org".to_string()),
        capabilities: AgentCapabilities {
            streaming: Some(true),
            push_notifications: Some(false),
            extensions: None,
            state_transition_history: None,
        },
        security_schemes: None,
        security_requirements: vec![],
        default_input_modes: vec!["text/plain".to_string()],
        default_output_modes: vec!["text/plain".to_string(), "application/json".to_string()],
        skills: vec![AgentSkill {
            id: "code".to_string(),
            name: "Code Gen".to_string(),
            description: "Generates code".to_string(),
            tags: vec!["coding".to_string()],
            examples: Some(vec!["Write hello world".to_string()]),
            input_modes: None,
            output_modes: None,
            security_requirements: None,
            security: None,
        }],
        signatures: None,
        icon_url: None,
        additional_interfaces: None,
        preferred_transport: None,
        protocol_version: Some("0.3".to_string()),
        supports_authenticated_extended_card: None,
        security: None,
    };
    let json = serde_json::to_value(&card).unwrap();

    // Verify camelCase
    assert_eq!(json["name"], "Test Agent");
    // Python SDK: uses "transport" not "protocolBinding"
    assert_eq!(json["supportedInterfaces"][0]["transport"], "JSONRPC");
    assert_eq!(json["documentationUrl"], "https://docs.test.org");
    assert_eq!(json["defaultInputModes"][0], "text/plain");
    assert_eq!(json["defaultOutputModes"][1], "application/json");
    assert_eq!(json["capabilities"]["streaming"], true);
    assert_eq!(json["capabilities"]["pushNotifications"], false);
    assert_eq!(json["skills"][0]["id"], "code");
    assert_eq!(json["protocolVersion"], "0.3");

    // Roundtrip
    let decoded: AgentCard = serde_json::from_value(json).unwrap();
    assert_eq!(decoded.name, "Test Agent");
    assert_eq!(decoded.supported_interfaces[0].transport, "JSONRPC");
}

// ============================================================================
// SecurityScheme (Python SDK: tagged union with "type" field)
// ============================================================================

#[test]
fn security_scheme_api_key_roundtrip() {
    let scheme = SecurityScheme::ApiKey {
        description: Some("API key auth".to_string()),
        location: ApiKeyLocation::Header,
        name: "X-API-Key".to_string(),
    };
    let json = serde_json::to_value(&scheme).unwrap();

    // Python SDK: {"type": "apiKey", "in": "header", "name": "X-API-Key"}
    assert_eq!(json["type"], "apiKey");
    assert_eq!(json["in"], "header");
    assert_eq!(json["name"], "X-API-Key");
    assert_eq!(json["description"], "API key auth");

    let decoded: SecurityScheme = serde_json::from_value(json).unwrap();
    match decoded {
        SecurityScheme::ApiKey { name, location, .. } => {
            assert_eq!(name, "X-API-Key");
            assert_eq!(location, ApiKeyLocation::Header);
        }
        _ => panic!("Expected ApiKey"),
    }
}

#[test]
fn security_scheme_http_roundtrip() {
    let scheme = SecurityScheme::Http {
        description: None,
        scheme: "bearer".to_string(),
        bearer_format: Some("JWT".to_string()),
    };
    let json = serde_json::to_value(&scheme).unwrap();

    // Python SDK: {"type": "http", "scheme": "bearer", "bearerFormat": "JWT"}
    assert_eq!(json["type"], "http");
    assert_eq!(json["scheme"], "bearer");
    assert_eq!(json["bearerFormat"], "JWT");

    let decoded: SecurityScheme = serde_json::from_value(json).unwrap();
    match decoded {
        SecurityScheme::Http {
            scheme,
            bearer_format,
            ..
        } => {
            assert_eq!(scheme, "bearer");
            assert_eq!(bearer_format, Some("JWT".to_string()));
        }
        _ => panic!("Expected Http"),
    }
}

#[test]
fn security_scheme_openid_connect_roundtrip() {
    let scheme = SecurityScheme::OpenIdConnect {
        description: None,
        open_id_connect_url: "https://auth.example.com/.well-known/openid-configuration"
            .to_string(),
    };
    let json = serde_json::to_value(&scheme).unwrap();

    // Python SDK: {"type": "openIdConnect", "openIdConnectUrl": "..."}
    assert_eq!(json["type"], "openIdConnect");
    assert!(json["openIdConnectUrl"].is_string());

    let decoded: SecurityScheme = serde_json::from_value(json).unwrap();
    match decoded {
        SecurityScheme::OpenIdConnect {
            open_id_connect_url,
            ..
        } => {
            assert!(open_id_connect_url.contains("openid-configuration"));
        }
        _ => panic!("Expected OpenIdConnect"),
    }
}

#[test]
fn security_scheme_mutual_tls() {
    let scheme = SecurityScheme::MutualTls {
        description: Some("mTLS auth".to_string()),
    };
    let json = serde_json::to_value(&scheme).unwrap();
    // Python SDK: {"type": "mutualTLS", "description": "mTLS auth"}
    assert_eq!(json["type"], "mutualTLS");
    assert_eq!(json["description"], "mTLS auth");

    let decoded: SecurityScheme = serde_json::from_value(json).unwrap();
    match decoded {
        SecurityScheme::MutualTls { description } => {
            assert_eq!(description.unwrap(), "mTLS auth");
        }
        _ => panic!("Expected MutualTls"),
    }
}

#[test]
fn security_scheme_oauth2_roundtrip() {
    use std::collections::HashMap;
    let mut scopes = HashMap::new();
    scopes.insert("read".to_string(), "Read access".to_string());
    scopes.insert("write".to_string(), "Write access".to_string());

    let scheme = SecurityScheme::OAuth2 {
        description: Some("OAuth 2.0 auth".to_string()),
        flows: OAuthFlows {
            authorization_code: Some(AuthorizationCodeOAuthFlow {
                authorization_url: "https://auth.example.com/authorize".to_string(),
                token_url: "https://auth.example.com/token".to_string(),
                refresh_url: Some("https://auth.example.com/refresh".to_string()),
                scopes,
            }),
            client_credentials: None,
            implicit: None,
            password: None,
        },
        oauth2_metadata_url: Some(
            "https://auth.example.com/.well-known/oauth-authorization-server".to_string(),
        ),
    };
    let json = serde_json::to_value(&scheme).unwrap();

    // Python SDK: {"type": "oauth2", "flows": {...}, "oauth2MetadataUrl": "..."}
    assert_eq!(json["type"], "oauth2");
    assert_eq!(json["description"], "OAuth 2.0 auth");
    assert!(json["flows"]["authorizationCode"].is_object());
    assert_eq!(
        json["flows"]["authorizationCode"]["authorizationUrl"],
        "https://auth.example.com/authorize"
    );
    assert_eq!(
        json["flows"]["authorizationCode"]["tokenUrl"],
        "https://auth.example.com/token"
    );
    assert_eq!(
        json["flows"]["authorizationCode"]["refreshUrl"],
        "https://auth.example.com/refresh"
    );
    assert!(json["flows"]["authorizationCode"]["scopes"].is_object());
    assert_eq!(
        json["oauth2MetadataUrl"],
        "https://auth.example.com/.well-known/oauth-authorization-server"
    );

    let decoded: SecurityScheme = serde_json::from_value(json).unwrap();
    match decoded {
        SecurityScheme::OAuth2 {
            flows,
            oauth2_metadata_url,
            ..
        } => {
            assert!(flows.authorization_code.is_some());
            assert_eq!(
                oauth2_metadata_url.unwrap(),
                "https://auth.example.com/.well-known/oauth-authorization-server"
            );
        }
        _ => panic!("Expected OAuth2"),
    }
}

#[test]
fn security_scheme_all_five_variants() {
    println!("\n=== SecurityScheme Serialization Test ===\n");

    // 1. ApiKey
    let api_key = SecurityScheme::ApiKey {
        description: Some("API key authentication".to_string()),
        location: ApiKeyLocation::Header,
        name: "X-API-Key".to_string(),
    };
    let api_key_json = serde_json::to_value(&api_key).unwrap();
    println!("1. ApiKey:");
    println!("{}\n", serde_json::to_string_pretty(&api_key_json).unwrap());

    // Verify discriminator and field names
    assert_eq!(api_key_json["type"], "apiKey");
    assert_eq!(api_key_json["in"], "header");
    assert_eq!(api_key_json["name"], "X-API-Key");
    assert!(
        api_key_json.get("location").is_none(),
        "Must use 'in' not 'location'"
    );

    // 2. Http
    let http = SecurityScheme::Http {
        description: Some("Bearer token authentication".to_string()),
        scheme: "bearer".to_string(),
        bearer_format: Some("JWT".to_string()),
    };
    let http_json = serde_json::to_value(&http).unwrap();
    println!("2. Http:");
    println!("{}\n", serde_json::to_string_pretty(&http_json).unwrap());

    assert_eq!(http_json["type"], "http");
    assert_eq!(http_json["scheme"], "bearer");
    assert_eq!(http_json["bearerFormat"], "JWT");

    // 3. OAuth2
    use std::collections::HashMap;
    let mut auth_code_scopes = HashMap::new();
    auth_code_scopes.insert("read".to_string(), "Read access".to_string());
    auth_code_scopes.insert("write".to_string(), "Write access".to_string());

    let mut client_creds_scopes = HashMap::new();
    client_creds_scopes.insert("admin".to_string(), "Admin access".to_string());

    let oauth2 = SecurityScheme::OAuth2 {
        description: Some("OAuth 2.0 authorization".to_string()),
        flows: OAuthFlows {
            authorization_code: Some(AuthorizationCodeOAuthFlow {
                authorization_url: "https://auth.example.com/authorize".to_string(),
                token_url: "https://auth.example.com/token".to_string(),
                refresh_url: Some("https://auth.example.com/refresh".to_string()),
                scopes: auth_code_scopes,
            }),
            client_credentials: Some(ClientCredentialsOAuthFlow {
                token_url: "https://auth.example.com/token".to_string(),
                refresh_url: None,
                scopes: client_creds_scopes,
            }),
            implicit: None,
            password: None,
        },
        oauth2_metadata_url: Some(
            "https://auth.example.com/.well-known/oauth-authorization-server".to_string(),
        ),
    };
    let oauth2_json = serde_json::to_value(&oauth2).unwrap();
    println!("3. OAuth2:");
    println!("{}\n", serde_json::to_string_pretty(&oauth2_json).unwrap());

    assert_eq!(oauth2_json["type"], "oauth2");
    assert!(oauth2_json["flows"].is_object());
    assert!(oauth2_json["flows"]["authorizationCode"].is_object());
    assert!(oauth2_json["flows"]["clientCredentials"].is_object());
    assert_eq!(
        oauth2_json["oauth2MetadataUrl"],
        "https://auth.example.com/.well-known/oauth-authorization-server"
    );

    // 4. OpenIdConnect
    let openid = SecurityScheme::OpenIdConnect {
        description: Some("OpenID Connect authentication".to_string()),
        open_id_connect_url: "https://auth.example.com/.well-known/openid-configuration"
            .to_string(),
    };
    let openid_json = serde_json::to_value(&openid).unwrap();
    println!("4. OpenIdConnect:");
    println!("{}\n", serde_json::to_string_pretty(&openid_json).unwrap());

    assert_eq!(openid_json["type"], "openIdConnect");
    assert_eq!(
        openid_json["openIdConnectUrl"],
        "https://auth.example.com/.well-known/openid-configuration"
    );

    // 5. MutualTLS
    let mutual_tls = SecurityScheme::MutualTls {
        description: Some("Mutual TLS authentication".to_string()),
    };
    let mutual_tls_json = serde_json::to_value(&mutual_tls).unwrap();
    println!("5. MutualTLS:");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&mutual_tls_json).unwrap()
    );

    assert_eq!(mutual_tls_json["type"], "mutualTLS");
    assert_eq!(mutual_tls_json["description"], "Mutual TLS authentication");

    println!("=== All 5 SecurityScheme variants passed ===\n");
}

// ============================================================================
// JSON-RPC types
// ============================================================================

#[test]
fn json_rpc_request_serialization() {
    let req = JsonRpcRequest::new(1i64, "message/send", Some(json!({"message": {}})));
    let json = serde_json::to_value(&req).unwrap();

    assert_eq!(json["jsonrpc"], "2.0");
    assert_eq!(json["id"], 1);
    assert_eq!(json["method"], "message/send");
    assert!(json["params"].is_object());
}

#[test]
fn json_rpc_request_notification() {
    let req = JsonRpcRequest::notification("tasks/update", None);
    let json = serde_json::to_value(&req).unwrap();

    assert_eq!(json["jsonrpc"], "2.0");
    assert!(json.get("id").is_none());
    assert_eq!(json["method"], "tasks/update");
}

#[test]
fn json_rpc_response_success() {
    let resp = JsonRpcResponse::success(Some(JsonRpcId::Number(1)), json!({"id": "t1"}));
    let json = serde_json::to_value(&resp).unwrap();

    assert_eq!(json["jsonrpc"], "2.0");
    assert_eq!(json["id"], 1);
    assert!(json["result"]["id"].is_string());
    assert!(json.get("error").is_none());
}

#[test]
fn json_rpc_response_error() {
    let err = JsonRpcError {
        code: -32001,
        message: "Task not found".to_string(),
        data: Some(json!({"task_id": "t1"})),
    };
    let resp = JsonRpcResponse::error(Some(JsonRpcId::String("req-1".to_string())), err);
    let json = serde_json::to_value(&resp).unwrap();

    assert_eq!(json["jsonrpc"], "2.0");
    assert_eq!(json["id"], "req-1");
    assert_eq!(json["error"]["code"], -32001);
    assert_eq!(json["error"]["message"], "Task not found");
    assert_eq!(json["error"]["data"]["task_id"], "t1");
    assert!(json.get("result").is_none());
}

#[test]
fn json_rpc_id_all_variants() {
    // String
    let id: JsonRpcId = "abc".into();
    assert_eq!(serde_json::to_string(&id).unwrap(), "\"abc\"");

    // Number
    let id: JsonRpcId = 42i64.into();
    assert_eq!(serde_json::to_string(&id).unwrap(), "42");

    // i32 -> i64
    let id: JsonRpcId = 7i32.into();
    assert_eq!(serde_json::to_string(&id).unwrap(), "7");

    // Null
    let id = JsonRpcId::Null;
    assert_eq!(serde_json::to_string(&id).unwrap(), "null");
}

// ============================================================================
// Request/Response param types
// ============================================================================

#[test]
fn send_message_params_serialization() {
    let params = SendMessageParams {
        message: Message::user("m1", "Hello"),
        configuration: Some(SendMessageConfiguration {
            accepted_output_modes: Some(vec!["text/plain".to_string()]),
            push_notification_config: None,
            history_length: Some(10),
            blocking: Some(true),
        }),
        metadata: None,
        tenant: None,
    };
    let json = serde_json::to_value(&params).unwrap();

    assert_eq!(json["message"]["messageId"], "m1");
    assert_eq!(
        json["configuration"]["acceptedOutputModes"][0],
        "text/plain"
    );
    assert_eq!(json["configuration"]["historyLength"], 10);
    assert_eq!(json["configuration"]["blocking"], true);
}

#[test]
fn get_task_params_serialization() {
    let params = GetTaskParams {
        id: "t1".to_string(),
        history_length: Some(5),
        metadata: None,
        tenant: None,
    };
    let json = serde_json::to_value(&params).unwrap();

    assert_eq!(json["id"], "t1");
    assert_eq!(json["historyLength"], 5);
}

#[test]
fn cancel_task_params_serialization() {
    let params = CancelTaskParams {
        id: "t1".to_string(),
        metadata: Some(json!({"reason": "user request"})),
        tenant: None,
    };
    let json = serde_json::to_value(&params).unwrap();

    assert_eq!(json["id"], "t1");
    assert_eq!(json["metadata"]["reason"], "user request");
}

#[test]
fn list_tasks_params_serialization() {
    let params = ListTasksParams {
        context_id: Some("ctx1".to_string()),
        status: Some(TaskState::Working),
        page_size: Some(10),
        page_token: Some("token-1".to_string()),
        history_length: None,
        status_timestamp_after: None,
        include_artifacts: Some(true),
        tenant: None,
    };
    let json = serde_json::to_value(&params).unwrap();

    assert_eq!(json["contextId"], "ctx1");
    assert_eq!(json["status"], "working");
    assert_eq!(json["pageSize"], 10);
    assert_eq!(json["pageToken"], "token-1");
    assert_eq!(json["includeArtifacts"], true);
}

// ============================================================================
// PushNotificationConfig
// ============================================================================

#[test]
fn push_notification_config_roundtrip() {
    let config = PushNotificationConfig {
        id: Some("pnc-1".to_string()),
        url: "https://example.com/webhook".to_string(),
        token: Some("secret".to_string()),
        authentication: Some(PushNotificationAuthenticationInfo {
            schemes: vec!["Bearer".to_string()],
            credentials: Some("my-token".to_string()),
        }),
    };
    let json = serde_json::to_value(&config).unwrap();

    assert_eq!(json["id"], "pnc-1");
    assert_eq!(json["url"], "https://example.com/webhook");
    assert_eq!(json["token"], "secret");
    // Python SDK: plural "schemes" as list
    assert_eq!(json["authentication"]["schemes"], json!(["Bearer"]));
    assert_eq!(json["authentication"]["credentials"], "my-token");

    let decoded: PushNotificationConfig = serde_json::from_value(json).unwrap();
    assert_eq!(decoded.url, "https://example.com/webhook");
    assert_eq!(
        decoded.authentication.unwrap().credentials.unwrap(),
        "my-token"
    );
}

// ============================================================================
// Part field names (Python SDK naming: uri, bytes, name, mimeType)
// ============================================================================

#[test]
fn file_part_field_names_python_sdk() {
    // FileWithUri uses "uri" (not "url"), "mimeType" (not "mediaType"), "name" (not "filename")
    let part = Part::file_from_uri(
        "https://example.com/test.txt",
        Some("test.txt".to_string()),
        Some("text/plain".to_string()),
    );
    let json = serde_json::to_value(&part).unwrap();

    assert_eq!(json["kind"], "file");
    assert_eq!(json["file"]["uri"], "https://example.com/test.txt");
    assert_eq!(json["file"]["name"], "test.txt");
    assert_eq!(json["file"]["mimeType"], "text/plain");
    // Verify correct field names (not old proto names)
    assert!(
        json["file"].get("url").is_none(),
        "Must use 'uri' not 'url'"
    );
    assert!(
        json["file"].get("filename").is_none(),
        "Must use 'name' not 'filename'"
    );
    assert!(
        json["file"].get("mediaType").is_none(),
        "Must use 'mimeType' not 'mediaType'"
    );
    assert!(
        json["file"].get("mime_type").is_none(),
        "Must use camelCase 'mimeType'"
    );
}

// ============================================================================
// ApiKeyLocation
// ============================================================================

#[test]
fn api_key_location_all_variants() {
    let cases = vec![
        (ApiKeyLocation::Cookie, "cookie"),
        (ApiKeyLocation::Header, "header"),
        (ApiKeyLocation::Query, "query"),
    ];

    for (variant, expected) in cases {
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, format!("\"{}\"", expected));

        let decoded: ApiKeyLocation = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, variant);
    }
}

// ============================================================================
// Wire-Format Spot-Checks (Python SDK compliance)
// ============================================================================

#[test]
fn spot_check_security_requirement_wire_format() {
    // Python SDK: SecurityRequirement is HashMap<String, Vec<String>>
    use std::collections::HashMap;
    let req: SecurityRequirement = HashMap::from([("oauth".to_string(), vec!["read".to_string()])]);

    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["oauth"], json!(["read"]));

    let decoded: SecurityRequirement = serde_json::from_value(json).unwrap();
    assert_eq!(decoded["oauth"], vec!["read".to_string()]);
}

#[test]
fn spot_check_push_notification_auth_schemes_plural() {
    // Python SDK: PushNotificationAuthenticationInfo uses `schemes` (plural, list)
    let auth = PushNotificationAuthenticationInfo {
        schemes: vec!["Bearer".to_string()],
        credentials: Some("token123".to_string()),
    };
    let json = serde_json::to_value(&auth).unwrap();

    assert!(
        json.get("scheme").is_none(),
        "Must not have singular 'scheme' field"
    );
    assert_eq!(json["schemes"], json!(["Bearer"]));
    assert_eq!(json["credentials"], "token123");

    let decoded: PushNotificationAuthenticationInfo = serde_json::from_value(json).unwrap();
    assert_eq!(decoded.schemes, vec!["Bearer"]);
}

#[test]
fn spot_check_push_notification_config_param_field_names() {
    // DeleteTaskPushNotificationConfigParams uses `id` (task) + `pushNotificationConfigId` (config)
    let params = DeleteTaskPushNotificationConfigParams {
        id: "task-123".to_string(),
        push_notification_config_id: "config-456".to_string(),
        metadata: None,
    };
    let json = serde_json::to_value(&params).unwrap();

    assert_eq!(json["id"], "task-123");
    assert_eq!(json["pushNotificationConfigId"], "config-456");
    assert!(json.get("taskId").is_none(), "Must not have 'taskId' field");

    // GetTaskPushNotificationConfigParams
    let params = GetTaskPushNotificationConfigParams {
        id: "task-789".to_string(),
        push_notification_config_id: Some("config-abc".to_string()),
        metadata: None,
    };
    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["id"], "task-789");
    assert_eq!(json["pushNotificationConfigId"], "config-abc");

    // ListTaskPushNotificationConfigParams
    let params = ListTaskPushNotificationConfigParams {
        id: "task-def".to_string(),
        metadata: None,
    };
    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["id"], "task-def");
    assert!(json.get("taskId").is_none(), "Must not have 'taskId' field");
}

#[test]
fn spot_check_agent_card_defaults_on_deserialize() {
    // AgentCard should default preferredTransport to "JSONRPC" and protocolVersion to "0.3.0"
    let json = json!({
        "name": "Test",
        "description": "Test agent",
        "version": "1.0",
        "url": "http://localhost/a2a",
        "supportedInterfaces": [{
            "url": "http://localhost/a2a",
            "transport": "JSONRPC"
        }],
        "capabilities": {},
        "defaultInputModes": ["text/plain"],
        "defaultOutputModes": ["text/plain"],
        "skills": [],
        "securityRequirements": []
    });

    let card: AgentCard = serde_json::from_value(json).unwrap();
    assert_eq!(card.preferred_transport, Some("JSONRPC".to_string()));
    assert_eq!(card.protocol_version, Some("0.3.0".to_string()));
}

#[test]
fn spot_check_role_unspecified() {
    // Role should support "unspecified" variant (Python SDK format)
    let role = Role::Unspecified;
    let json = serde_json::to_value(&role).unwrap();
    assert_eq!(json, "unspecified");

    let decoded: Role = serde_json::from_str(r#""unspecified""#).unwrap();
    assert_eq!(decoded, Role::Unspecified);
    assert_eq!(format!("{}", decoded), "unspecified");
}

#[test]
fn spot_check_agent_interface_transport_field() {
    // Python SDK: AgentInterface uses "transport", protocol_version is optional
    let iface = AgentInterface {
        url: "http://localhost/a2a".to_string(),
        transport: "JSONRPC".to_string(),
        tenant: None,
        protocol_version: Some("0.3".to_string()),
    };
    let json = serde_json::to_value(&iface).unwrap();
    assert_eq!(json["transport"], "JSONRPC");
    assert!(
        json.get("protocolBinding").is_none(),
        "Must not use 'protocolBinding'"
    );

    // Deserializing without protocolVersion should succeed (it's optional)
    let missing = json!({
        "url": "http://localhost/a2a",
        "transport": "JSONRPC"
    });
    let result: Result<AgentInterface, _> = serde_json::from_value(missing);
    assert!(
        result.is_ok(),
        "Missing protocolVersion should succeed (it's optional)"
    );
    assert_eq!(result.unwrap().protocol_version, None);
}

#[test]
fn spot_check_tenant_on_param_structs() {
    // SendMessageParams should serialize tenant when present
    let params = SendMessageParams {
        message: Message::user("m1", "Hello"),
        configuration: None,
        metadata: None,
        tenant: Some("acme-corp".to_string()),
    };
    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["tenant"], "acme-corp");

    // tenant should be omitted when None
    let params = SendMessageParams {
        message: Message::user("m2", "Hello"),
        configuration: None,
        metadata: None,
        tenant: None,
    };
    let json = serde_json::to_value(&params).unwrap();
    assert!(
        json.get("tenant").is_none(),
        "tenant must be omitted when None"
    );
}
