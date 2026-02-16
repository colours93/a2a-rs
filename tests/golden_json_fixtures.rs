//! Golden JSON Fixture Tests — Spec Compliance Verification
//!
//! These tests verify that a2a-rs serialization/deserialization matches the
//! A2A v0.3 wire format as defined by:
//! - Proto spec: specification/a2a.proto
//! - Python SDK: a2a-python/src/a2a/types.py (reference JSON-RPC implementation)
//!
//! Every golden JSON string is hand-derived from the spec, NOT from the Rust code.
//! Tests work in both directions:
//! 1. Deserialize golden JSON → Rust type (verify we accept spec-compliant input)
//! 2. Serialize Rust type → JSON → compare against golden (verify we produce spec-compliant output)

use a2a_rs::types::*;
use serde_json::{json, Value};

// ============================================================================
// Helper: compare JSON values ignoring key order
// ============================================================================

fn assert_json_eq(actual: &Value, expected: &Value) {
    assert_eq!(
        actual,
        expected,
        "\n\nACTUAL:\n{}\n\nEXPECTED:\n{}\n",
        serde_json::to_string_pretty(actual).unwrap(),
        serde_json::to_string_pretty(expected).unwrap(),
    );
}

/// Serialize a value and compare against golden JSON.
fn assert_serializes_to<T: serde::Serialize>(value: &T, expected: &Value) {
    let actual = serde_json::to_value(value).unwrap();
    assert_json_eq(&actual, expected);
}

/// Deserialize golden JSON and re-serialize, checking round-trip matches golden.
fn assert_round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(golden: &Value) {
    let deserialized: T = serde_json::from_value(golden.clone())
        .unwrap_or_else(|e| panic!("Failed to deserialize golden JSON: {e}\nJSON: {golden}"));
    let reserialized = serde_json::to_value(&deserialized).unwrap();
    assert_json_eq(&reserialized, golden);
}

// ============================================================================
// 1. TaskState — kebab-case strings
// ============================================================================

#[test]
fn golden_task_state_submitted() {
    let state = TaskState::Submitted;
    let json = serde_json::to_value(&state).unwrap();
    assert_eq!(json, json!("submitted"));
}

#[test]
fn golden_task_state_working() {
    let json = serde_json::to_value(&TaskState::Working).unwrap();
    assert_eq!(json, json!("working"));
}

#[test]
fn golden_task_state_completed() {
    let json = serde_json::to_value(&TaskState::Completed).unwrap();
    assert_eq!(json, json!("completed"));
}

#[test]
fn golden_task_state_failed() {
    let json = serde_json::to_value(&TaskState::Failed).unwrap();
    assert_eq!(json, json!("failed"));
}

#[test]
fn golden_task_state_canceled() {
    let json = serde_json::to_value(&TaskState::Canceled).unwrap();
    assert_eq!(json, json!("canceled"));
}

#[test]
fn golden_task_state_input_required() {
    // Proto: TASK_STATE_INPUT_REQUIRED → kebab-case: "input-required"
    let json = serde_json::to_value(&TaskState::InputRequired).unwrap();
    assert_eq!(json, json!("input-required"));
}

#[test]
fn golden_task_state_rejected() {
    let json = serde_json::to_value(&TaskState::Rejected).unwrap();
    assert_eq!(json, json!("rejected"));
}

#[test]
fn golden_task_state_auth_required() {
    // Proto: TASK_STATE_AUTH_REQUIRED → kebab-case: "auth-required"
    let json = serde_json::to_value(&TaskState::AuthRequired).unwrap();
    assert_eq!(json, json!("auth-required"));
}

#[test]
fn golden_task_state_deserialize_all() {
    // Verify all 8 proto-defined states round-trip from JSON strings
    let cases = vec![
        ("\"submitted\"", TaskState::Submitted),
        ("\"working\"", TaskState::Working),
        ("\"completed\"", TaskState::Completed),
        ("\"failed\"", TaskState::Failed),
        ("\"canceled\"", TaskState::Canceled),
        ("\"input-required\"", TaskState::InputRequired),
        ("\"rejected\"", TaskState::Rejected),
        ("\"auth-required\"", TaskState::AuthRequired),
    ];
    for (json_str, expected) in cases {
        let deserialized: TaskState = serde_json::from_str(json_str)
            .unwrap_or_else(|e| panic!("Failed to deserialize {json_str}: {e}"));
        assert_eq!(deserialized, expected, "Mismatch for {json_str}");
    }
}

// ============================================================================
// 2. Role — lowercase strings
// ============================================================================

#[test]
fn golden_role_user() {
    let json = serde_json::to_value(&Role::User).unwrap();
    assert_eq!(json, json!("user"));
}

#[test]
fn golden_role_agent() {
    let json = serde_json::to_value(&Role::Agent).unwrap();
    assert_eq!(json, json!("agent"));
}

#[test]
fn golden_role_deserialize() {
    let user: Role = serde_json::from_str("\"user\"").unwrap();
    assert_eq!(user, Role::User);
    let agent: Role = serde_json::from_str("\"agent\"").unwrap();
    assert_eq!(agent, Role::Agent);
}

// ============================================================================
// 3. Part — discriminated by "kind" field
// ============================================================================

#[test]
fn golden_text_part() {
    let golden = json!({
        "kind": "text",
        "text": "Hello, world!"
    });
    let part: Part = serde_json::from_value(golden.clone()).unwrap();
    match &part {
        Part::Text { text, metadata } => {
            assert_eq!(text, "Hello, world!");
            assert!(metadata.is_none());
        }
        _ => panic!("Expected Text part"),
    }
    assert_serializes_to(&part, &golden);
}

#[test]
fn golden_text_part_with_metadata() {
    let golden = json!({
        "kind": "text",
        "text": "Hello",
        "metadata": {"source": "test"}
    });
    let part: Part = serde_json::from_value(golden.clone()).unwrap();
    match &part {
        Part::Text { text, metadata } => {
            assert_eq!(text, "Hello");
            assert_eq!(metadata.as_ref().unwrap()["source"], "test");
        }
        _ => panic!("Expected Text part"),
    }
    assert_serializes_to(&part, &golden);
}

#[test]
fn golden_file_part_with_bytes() {
    // File part with base64-encoded bytes
    let golden = json!({
        "kind": "file",
        "file": {
            "bytes": "SGVsbG8gV29ybGQ=",
            "mimeType": "text/plain",
            "name": "hello.txt"
        }
    });
    let part: Part = serde_json::from_value(golden.clone()).unwrap();
    match &part {
        Part::File { file, metadata } => {
            match file {
                FileContent::Bytes(fb) => {
                    assert_eq!(fb.bytes, "SGVsbG8gV29ybGQ=");
                    assert_eq!(fb.mime_type.as_deref(), Some("text/plain"));
                    assert_eq!(fb.name.as_deref(), Some("hello.txt"));
                }
                _ => panic!("Expected FileContent::Bytes"),
            }
            assert!(metadata.is_none());
        }
        _ => panic!("Expected File part"),
    }
    assert_serializes_to(&part, &golden);
}

#[test]
fn golden_file_part_with_uri() {
    let golden = json!({
        "kind": "file",
        "file": {
            "uri": "https://example.com/doc.pdf",
            "mimeType": "application/pdf"
        }
    });
    let part: Part = serde_json::from_value(golden.clone()).unwrap();
    match &part {
        Part::File { file, .. } => match file {
            FileContent::Uri(fu) => {
                assert_eq!(fu.uri, "https://example.com/doc.pdf");
                assert_eq!(fu.mime_type.as_deref(), Some("application/pdf"));
            }
            _ => panic!("Expected FileContent::Uri"),
        },
        _ => panic!("Expected File part"),
    }
    assert_serializes_to(&part, &golden);
}

#[test]
fn golden_data_part() {
    let golden = json!({
        "kind": "data",
        "data": {"key": "value", "count": 42}
    });
    let part: Part = serde_json::from_value(golden.clone()).unwrap();
    match &part {
        Part::Data { data, metadata } => {
            assert_eq!(data["key"], "value");
            assert_eq!(data["count"], 42);
            assert!(metadata.is_none());
        }
        _ => panic!("Expected Data part"),
    }
    assert_serializes_to(&part, &golden);
}

// ============================================================================
// 4. Message — camelCase fields, "kind": "message"
// ============================================================================

#[test]
fn golden_message_minimal() {
    let golden = json!({
        "messageId": "msg-001",
        "role": "user",
        "kind": "message",
        "parts": [{"kind": "text", "text": "Hello agent"}]
    });
    let msg: Message = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(msg.message_id, "msg-001");
    assert_eq!(msg.role, Role::User);
    assert_eq!(msg.kind, "message");
    assert_eq!(msg.parts.len(), 1);
    assert!(msg.context_id.is_none());
    assert!(msg.task_id.is_none());
    assert_serializes_to(&msg, &golden);
}

#[test]
fn golden_message_full() {
    let golden = json!({
        "messageId": "msg-002",
        "role": "agent",
        "kind": "message",
        "parts": [{"kind": "text", "text": "Response"}],
        "contextId": "ctx-1",
        "taskId": "task-1",
        "metadata": {"model": "gpt-4"},
        "extensions": ["urn:a2a:ext:streaming"],
        "referenceTaskIds": ["task-0"]
    });
    let msg: Message = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(msg.message_id, "msg-002");
    assert_eq!(msg.role, Role::Agent);
    assert_eq!(msg.context_id.as_deref(), Some("ctx-1"));
    assert_eq!(msg.task_id.as_deref(), Some("task-1"));
    assert_eq!(msg.extensions.as_ref().unwrap().len(), 1);
    assert_eq!(msg.reference_task_ids.as_ref().unwrap(), &["task-0"]);
    assert_serializes_to(&msg, &golden);
}

// ============================================================================
// 5. TaskStatus — camelCase
// ============================================================================

#[test]
fn golden_task_status_minimal() {
    let golden = json!({
        "state": "working"
    });
    let status: TaskStatus = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(status.state, TaskState::Working);
    assert!(status.message.is_none());
    assert!(status.timestamp.is_none());
    assert_serializes_to(&status, &golden);
}

#[test]
fn golden_task_status_with_message_and_timestamp() {
    let golden = json!({
        "state": "completed",
        "message": {
            "messageId": "m1",
            "role": "agent",
            "kind": "message",
            "parts": [{"kind": "text", "text": "Done!"}]
        },
        "timestamp": "2024-01-15T10:30:00Z"
    });
    let status: TaskStatus = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(status.state, TaskState::Completed);
    assert!(status.message.is_some());
    assert_eq!(status.timestamp.as_deref(), Some("2024-01-15T10:30:00Z"));
    assert_serializes_to(&status, &golden);
}

// ============================================================================
// 6. Task — camelCase, "kind": "task"
// ============================================================================

#[test]
fn golden_task_minimal() {
    let golden = json!({
        "id": "task-001",
        "contextId": "ctx-001",
        "kind": "task",
        "status": {
            "state": "submitted"
        }
    });
    let task: Task = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(task.id, "task-001");
    assert_eq!(task.context_id, "ctx-001");
    assert_eq!(task.kind, "task");
    assert_eq!(task.status.state, TaskState::Submitted);
    assert!(task.artifacts.is_none());
    assert!(task.history.is_none());
    assert!(task.metadata.is_none());
    assert_serializes_to(&task, &golden);
}

#[test]
fn golden_task_full() {
    let golden = json!({
        "id": "task-002",
        "contextId": "ctx-002",
        "kind": "task",
        "status": {
            "state": "completed",
            "timestamp": "2024-01-15T12:00:00Z"
        },
        "artifacts": [{
            "artifactId": "art-1",
            "parts": [{"kind": "text", "text": "Result data"}],
            "name": "output",
            "description": "The output artifact"
        }],
        "history": [{
            "messageId": "m1",
            "role": "user",
            "kind": "message",
            "parts": [{"kind": "text", "text": "Do something"}]
        }],
        "metadata": {"priority": "high"}
    });
    let task: Task = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(task.id, "task-002");
    assert_eq!(task.artifacts.as_ref().unwrap().len(), 1);
    assert_eq!(task.history.as_ref().unwrap().len(), 1);
    assert_serializes_to(&task, &golden);
}

// ============================================================================
// 7. Artifact — camelCase, artifactId
// ============================================================================

#[test]
fn golden_artifact() {
    let golden = json!({
        "artifactId": "art-001",
        "name": "code_output",
        "description": "Generated code",
        "parts": [
            {"kind": "text", "text": "fn main() {}"},
            {"kind": "data", "data": {"language": "rust"}}
        ],
        "extensions": ["urn:a2a:ext:code"]
    });
    let artifact: Artifact = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(artifact.artifact_id, "art-001");
    assert_eq!(artifact.name.as_deref(), Some("code_output"));
    assert_eq!(artifact.parts.len(), 2);
    assert_eq!(artifact.extensions.as_ref().unwrap(), &["urn:a2a:ext:code"]);
    assert_serializes_to(&artifact, &golden);
}

// ============================================================================
// 8. TaskStatusUpdateEvent — "kind": "status-update", camelCase
// ============================================================================

#[test]
fn golden_task_status_update_event() {
    let golden = json!({
        "taskId": "task-001",
        "contextId": "ctx-001",
        "kind": "status-update",
        "status": {
            "state": "working"
        },
        "final": false
    });
    let event: TaskStatusUpdateEvent = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(event.task_id, "task-001");
    assert_eq!(event.context_id, "ctx-001");
    assert_eq!(event.kind, "status-update");
    assert_eq!(event.status.state, TaskState::Working);
    assert!(!event.r#final);
    assert_serializes_to(&event, &golden);
}

#[test]
fn golden_task_status_update_event_final() {
    let golden = json!({
        "taskId": "task-002",
        "contextId": "ctx-002",
        "kind": "status-update",
        "status": {
            "state": "completed"
        },
        "final": true
    });
    let event: TaskStatusUpdateEvent = serde_json::from_value(golden.clone()).unwrap();
    assert!(event.r#final);
    assert_eq!(event.status.state, TaskState::Completed);
    assert_serializes_to(&event, &golden);
}

// ============================================================================
// 9. TaskArtifactUpdateEvent — "kind": "artifact-update", camelCase
// ============================================================================

#[test]
fn golden_task_artifact_update_event() {
    let golden = json!({
        "taskId": "task-001",
        "contextId": "ctx-001",
        "kind": "artifact-update",
        "artifact": {
            "artifactId": "art-1",
            "parts": [{"kind": "text", "text": "chunk 1"}]
        },
        "append": false,
        "lastChunk": true
    });
    let event: TaskArtifactUpdateEvent = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(event.task_id, "task-001");
    assert_eq!(event.kind, "artifact-update");
    assert_eq!(event.artifact.artifact_id, "art-1");
    assert_eq!(event.append, Some(false));
    assert_eq!(event.last_chunk, Some(true));
    assert_serializes_to(&event, &golden);
}

// ============================================================================
// 10. StreamResponse — flat discriminator via "kind"
// ============================================================================

#[test]
fn golden_stream_response_task() {
    let golden = json!({
        "id": "task-001",
        "contextId": "ctx-001",
        "kind": "task",
        "status": {"state": "submitted"}
    });
    let sr: StreamResponse = serde_json::from_value(golden.clone()).unwrap();
    assert!(matches!(sr, StreamResponse::Task(_)));
    assert_serializes_to(&sr, &golden);
}

#[test]
fn golden_stream_response_message() {
    let golden = json!({
        "messageId": "msg-1",
        "role": "agent",
        "kind": "message",
        "parts": [{"kind": "text", "text": "Hello"}]
    });
    let sr: StreamResponse = serde_json::from_value(golden.clone()).unwrap();
    assert!(matches!(sr, StreamResponse::Message(_)));
    assert_serializes_to(&sr, &golden);
}

#[test]
fn golden_stream_response_status_update() {
    let golden = json!({
        "taskId": "t1",
        "contextId": "c1",
        "kind": "status-update",
        "status": {"state": "working"},
        "final": false
    });
    let sr: StreamResponse = serde_json::from_value(golden.clone()).unwrap();
    assert!(matches!(sr, StreamResponse::StatusUpdate(_)));
    assert_serializes_to(&sr, &golden);
}

#[test]
fn golden_stream_response_artifact_update() {
    let golden = json!({
        "taskId": "t1",
        "contextId": "c1",
        "kind": "artifact-update",
        "artifact": {
            "artifactId": "a1",
            "parts": [{"kind": "text", "text": "data"}]
        }
    });
    let sr: StreamResponse = serde_json::from_value(golden.clone()).unwrap();
    assert!(matches!(sr, StreamResponse::ArtifactUpdate(_)));
    assert_serializes_to(&sr, &golden);
}

// ============================================================================
// 11. SendMessageResponse — flat discriminator via "kind"
// ============================================================================

#[test]
fn golden_send_message_response_task() {
    let golden = json!({
        "id": "task-001",
        "contextId": "ctx-001",
        "kind": "task",
        "status": {"state": "working"}
    });
    let resp: SendMessageResponse = serde_json::from_value(golden.clone()).unwrap();
    assert!(matches!(resp, SendMessageResponse::Task(_)));
    assert_serializes_to(&resp, &golden);
}

#[test]
fn golden_send_message_response_message() {
    let golden = json!({
        "messageId": "m1",
        "role": "agent",
        "kind": "message",
        "parts": [{"kind": "text", "text": "Direct reply"}]
    });
    let resp: SendMessageResponse = serde_json::from_value(golden.clone()).unwrap();
    assert!(matches!(resp, SendMessageResponse::Message(_)));
    assert_serializes_to(&resp, &golden);
}

// ============================================================================
// 12. SecurityScheme — discriminated by "type" field
// ============================================================================

#[test]
fn golden_security_scheme_api_key() {
    let golden = json!({
        "type": "apiKey",
        "in": "header",
        "name": "X-API-Key"
    });
    let scheme: SecurityScheme = serde_json::from_value(golden.clone()).unwrap();
    match &scheme {
        SecurityScheme::ApiKey { location, name, .. } => {
            assert_eq!(*location, ApiKeyLocation::Header);
            assert_eq!(name, "X-API-Key");
        }
        _ => panic!("Expected ApiKey"),
    }
    assert_serializes_to(&scheme, &golden);
}

#[test]
fn golden_security_scheme_api_key_query() {
    let golden = json!({
        "type": "apiKey",
        "in": "query",
        "name": "api_key"
    });
    let scheme: SecurityScheme = serde_json::from_value(golden.clone()).unwrap();
    match &scheme {
        SecurityScheme::ApiKey { location, .. } => {
            assert_eq!(*location, ApiKeyLocation::Query);
        }
        _ => panic!("Expected ApiKey"),
    }
    assert_serializes_to(&scheme, &golden);
}

#[test]
fn golden_security_scheme_api_key_cookie() {
    let golden = json!({
        "type": "apiKey",
        "in": "cookie",
        "name": "session"
    });
    let scheme: SecurityScheme = serde_json::from_value(golden.clone()).unwrap();
    match &scheme {
        SecurityScheme::ApiKey { location, .. } => {
            assert_eq!(*location, ApiKeyLocation::Cookie);
        }
        _ => panic!("Expected ApiKey"),
    }
    assert_serializes_to(&scheme, &golden);
}

#[test]
fn golden_security_scheme_http_bearer() {
    let golden = json!({
        "type": "http",
        "scheme": "bearer",
        "bearerFormat": "JWT"
    });
    let scheme: SecurityScheme = serde_json::from_value(golden.clone()).unwrap();
    match &scheme {
        SecurityScheme::Http {
            scheme,
            bearer_format,
            ..
        } => {
            assert_eq!(scheme, "bearer");
            assert_eq!(bearer_format.as_deref(), Some("JWT"));
        }
        _ => panic!("Expected Http"),
    }
    assert_serializes_to(&scheme, &golden);
}

#[test]
fn golden_security_scheme_oauth2() {
    let golden = json!({
        "type": "oauth2",
        "flows": {
            "authorizationCode": {
                "authorizationUrl": "https://auth.example.com/authorize",
                "tokenUrl": "https://auth.example.com/token",
                "scopes": {
                    "read": "Read access",
                    "write": "Write access"
                }
            }
        }
    });
    let scheme: SecurityScheme = serde_json::from_value(golden.clone()).unwrap();
    match &scheme {
        SecurityScheme::OAuth2 { flows, .. } => {
            let ac = flows.authorization_code.as_ref().unwrap();
            assert_eq!(ac.authorization_url, "https://auth.example.com/authorize");
            assert_eq!(ac.token_url, "https://auth.example.com/token");
            assert_eq!(ac.scopes.len(), 2);
        }
        _ => panic!("Expected OAuth2"),
    }
    assert_serializes_to(&scheme, &golden);
}

#[test]
fn golden_security_scheme_openid_connect() {
    let golden = json!({
        "type": "openIdConnect",
        "openIdConnectUrl": "https://auth.example.com/.well-known/openid-configuration"
    });
    let scheme: SecurityScheme = serde_json::from_value(golden.clone()).unwrap();
    match &scheme {
        SecurityScheme::OpenIdConnect {
            open_id_connect_url,
            ..
        } => {
            assert_eq!(
                open_id_connect_url,
                "https://auth.example.com/.well-known/openid-configuration"
            );
        }
        _ => panic!("Expected OpenIdConnect"),
    }
    assert_serializes_to(&scheme, &golden);
}

#[test]
fn golden_security_scheme_mutual_tls() {
    let golden = json!({
        "type": "mutualTLS"
    });
    let scheme: SecurityScheme = serde_json::from_value(golden.clone()).unwrap();
    assert!(matches!(scheme, SecurityScheme::MutualTls { .. }));
    assert_serializes_to(&scheme, &golden);
}

// ============================================================================
// 13. AgentInterface — uses "transport" (Python SDK), NOT "protocolBinding" (proto)
// ============================================================================

#[test]
fn golden_agent_interface() {
    let golden = json!({
        "url": "https://api.example.com/a2a",
        "transport": "JSONRPC"
    });
    let iface: AgentInterface = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(iface.url, "https://api.example.com/a2a");
    assert_eq!(iface.transport, "JSONRPC");
    assert_serializes_to(&iface, &golden);
}

#[test]
fn golden_agent_interface_with_version() {
    let golden = json!({
        "url": "https://grpc.example.com/a2a",
        "transport": "GRPC",
        "protocolVersion": "0.3"
    });
    let iface: AgentInterface = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(iface.transport, "GRPC");
    assert_eq!(iface.protocol_version.as_deref(), Some("0.3"));
    assert_serializes_to(&iface, &golden);
}

// ============================================================================
// 14. AgentCapabilities — camelCase
// ============================================================================

#[test]
fn golden_agent_capabilities_empty() {
    let golden = json!({});
    let caps: AgentCapabilities = serde_json::from_value(golden.clone()).unwrap();
    assert!(caps.streaming.is_none());
    assert!(caps.push_notifications.is_none());
    assert_serializes_to(&caps, &golden);
}

#[test]
fn golden_agent_capabilities_full() {
    let golden = json!({
        "streaming": true,
        "pushNotifications": false,
        "stateTransitionHistory": true
    });
    let caps: AgentCapabilities = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(caps.streaming, Some(true));
    assert_eq!(caps.push_notifications, Some(false));
    assert_eq!(caps.state_transition_history, Some(true));
    assert_serializes_to(&caps, &golden);
}

// ============================================================================
// 15. AgentSkill — camelCase
// ============================================================================

#[test]
fn golden_agent_skill_minimal() {
    let golden = json!({
        "id": "code-gen",
        "name": "Code Generation",
        "description": "Generates code in various languages",
        "tags": ["coding", "generation"]
    });
    let skill: AgentSkill = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(skill.id, "code-gen");
    assert_eq!(skill.tags, vec!["coding", "generation"]);
    assert_serializes_to(&skill, &golden);
}

#[test]
fn golden_agent_skill_full() {
    let golden = json!({
        "id": "translate",
        "name": "Translation",
        "description": "Translates text between languages",
        "tags": ["nlp", "translation"],
        "examples": ["Translate this to French", "What is hello in Spanish?"],
        "inputModes": ["text/plain"],
        "outputModes": ["text/plain"]
    });
    let skill: AgentSkill = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(skill.examples.as_ref().unwrap().len(), 2);
    assert_eq!(skill.input_modes.as_ref().unwrap(), &["text/plain"]);
    assert_serializes_to(&skill, &golden);
}

// ============================================================================
// 16. AgentProvider — camelCase
// ============================================================================

#[test]
fn golden_agent_provider() {
    let golden = json!({
        "organization": "Acme Corp",
        "url": "https://acme.example.com"
    });
    let provider: AgentProvider = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(provider.organization, "Acme Corp");
    assert_eq!(provider.url, "https://acme.example.com");
    assert_serializes_to(&provider, &golden);
}

// ============================================================================
// 17. PushNotificationConfig — camelCase
// ============================================================================

#[test]
fn golden_push_notification_config_minimal() {
    let golden = json!({
        "url": "https://hooks.example.com/notify"
    });
    let config: PushNotificationConfig = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(config.url, "https://hooks.example.com/notify");
    assert!(config.id.is_none());
    assert!(config.token.is_none());
    assert_serializes_to(&config, &golden);
}

#[test]
fn golden_push_notification_config_full() {
    let golden = json!({
        "id": "pn-1",
        "url": "https://hooks.example.com/notify",
        "token": "verify-token-123",
        "authentication": {
            "schemes": ["Bearer"],
            "credentials": "secret-token"
        }
    });
    let config: PushNotificationConfig = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(config.id.as_deref(), Some("pn-1"));
    let auth = config.authentication.as_ref().unwrap();
    assert_eq!(auth.schemes, vec!["Bearer"]);
    assert_eq!(auth.credentials.as_deref(), Some("secret-token"));
    assert_serializes_to(&config, &golden);
}

// ============================================================================
// 18. TaskPushNotificationConfig — camelCase
// ============================================================================

#[test]
fn golden_task_push_notification_config() {
    let golden = json!({
        "id": "tpnc-1",
        "taskId": "task-001",
        "pushNotificationConfig": {
            "url": "https://hooks.example.com/notify"
        }
    });
    let config: TaskPushNotificationConfig = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(config.id.as_deref(), Some("tpnc-1"));
    assert_eq!(config.task_id, "task-001");
    assert_eq!(
        config.push_notification_config.url,
        "https://hooks.example.com/notify"
    );
    assert_serializes_to(&config, &golden);
}

// ============================================================================
// 19. JSON-RPC Request/Response
// ============================================================================

#[test]
fn golden_jsonrpc_request() {
    let golden = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "message/send",
        "params": {
            "message": {
                "messageId": "m1",
                "role": "user",
                "kind": "message",
                "parts": [{"kind": "text", "text": "Hello"}]
            }
        }
    });
    let req: JsonRpcRequest = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(req.jsonrpc, "2.0");
    assert_eq!(req.method, "message/send");
    assert!(req.id.is_some());
    assert!(req.params.is_some());
}

#[test]
fn golden_jsonrpc_response_success() {
    let golden = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "id": "task-001",
            "contextId": "ctx-001",
            "kind": "task",
            "status": {"state": "submitted"}
        }
    });
    let resp: JsonRpcResponse = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(resp.jsonrpc, "2.0");
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

#[test]
fn golden_jsonrpc_response_error() {
    let golden = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32001,
            "message": "Task not found"
        }
    });
    let resp: JsonRpcResponse = serde_json::from_value(golden.clone()).unwrap();
    assert!(resp.result.is_none());
    let err = resp.error.as_ref().unwrap();
    assert_eq!(err.code, -32001);
    assert_eq!(err.message, "Task not found");
}

#[test]
fn golden_jsonrpc_error_with_data() {
    let golden = json!({
        "jsonrpc": "2.0",
        "id": "req-1",
        "error": {
            "code": -32600,
            "message": "Invalid request",
            "data": {"detail": "Missing method field"}
        }
    });
    let resp: JsonRpcResponse = serde_json::from_value(golden.clone()).unwrap();
    let err = resp.error.as_ref().unwrap();
    assert_eq!(err.code, -32600);
    assert!(err.data.is_some());
}

// ============================================================================
// 20. SendMessageParams — camelCase
// ============================================================================

#[test]
fn golden_send_message_params_minimal() {
    let golden = json!({
        "message": {
            "messageId": "m1",
            "role": "user",
            "kind": "message",
            "parts": [{"kind": "text", "text": "Hello"}]
        }
    });
    let params: SendMessageParams = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(params.message.message_id, "m1");
    assert!(params.configuration.is_none());
    assert_serializes_to(&params, &golden);
}

#[test]
fn golden_send_message_params_with_config() {
    let golden = json!({
        "message": {
            "messageId": "m1",
            "role": "user",
            "kind": "message",
            "parts": [{"kind": "text", "text": "Hello"}]
        },
        "configuration": {
            "acceptedOutputModes": ["text/plain", "application/json"],
            "historyLength": 10,
            "blocking": true
        }
    });
    let params: SendMessageParams = serde_json::from_value(golden.clone()).unwrap();
    let config = params.configuration.as_ref().unwrap();
    assert_eq!(
        config.accepted_output_modes.as_ref().unwrap(),
        &["text/plain", "application/json"]
    );
    assert_eq!(config.history_length, Some(10));
    assert_eq!(config.blocking, Some(true));
    assert_serializes_to(&params, &golden);
}

// ============================================================================
// 21. GetTaskParams — camelCase
// ============================================================================

#[test]
fn golden_get_task_params() {
    let golden = json!({
        "id": "task-001",
        "historyLength": 5
    });
    let params: GetTaskParams = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(params.id, "task-001");
    assert_eq!(params.history_length, Some(5));
    assert_serializes_to(&params, &golden);
}

// ============================================================================
// 22. CancelTaskParams
// ============================================================================

#[test]
fn golden_cancel_task_params() {
    let golden = json!({
        "id": "task-001"
    });
    let params: CancelTaskParams = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(params.id, "task-001");
    assert_serializes_to(&params, &golden);
}

// ============================================================================
// 23. ListTasksParams — camelCase, all optional filters
// ============================================================================

#[test]
fn golden_list_tasks_params_empty() {
    let golden = json!({});
    let params: ListTasksParams = serde_json::from_value(golden.clone()).unwrap();
    assert!(params.context_id.is_none());
    assert!(params.status.is_none());
    assert_serializes_to(&params, &golden);
}

#[test]
fn golden_list_tasks_params_full() {
    let golden = json!({
        "contextId": "ctx-1",
        "status": "working",
        "pageSize": 20,
        "pageToken": "abc123",
        "historyLength": 3,
        "includeArtifacts": true
    });
    let params: ListTasksParams = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(params.context_id.as_deref(), Some("ctx-1"));
    assert_eq!(params.status, Some(TaskState::Working));
    assert_eq!(params.page_size, Some(20));
    assert_serializes_to(&params, &golden);
}

// ============================================================================
// 24. ListTasksResponse — camelCase
// ============================================================================

#[test]
fn golden_list_tasks_response() {
    let golden = json!({
        "tasks": [{
            "id": "task-1",
            "contextId": "ctx-1",
            "kind": "task",
            "status": {"state": "completed"}
        }],
        "nextPageToken": "",
        "pageSize": 50,
        "totalSize": 1
    });
    let resp: ListTasksResponse = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(resp.tasks.len(), 1);
    assert_eq!(resp.next_page_token, "");
    assert_eq!(resp.page_size, 50);
    assert_eq!(resp.total_size, 1);
    assert_serializes_to(&resp, &golden);
}

// ============================================================================
// 25. OAuthFlows — all-optional structure (Python SDK pattern, not proto oneof)
// ============================================================================

#[test]
fn golden_oauth_flows_authorization_code() {
    let golden = json!({
        "authorizationCode": {
            "authorizationUrl": "https://auth.example.com/authorize",
            "tokenUrl": "https://auth.example.com/token",
            "refreshUrl": "https://auth.example.com/refresh",
            "scopes": {"read": "Read access"}
        }
    });
    let flows: OAuthFlows = serde_json::from_value(golden.clone()).unwrap();
    let ac = flows.authorization_code.as_ref().unwrap();
    assert_eq!(ac.authorization_url, "https://auth.example.com/authorize");
    assert_eq!(
        ac.refresh_url.as_deref(),
        Some("https://auth.example.com/refresh")
    );
    assert!(flows.client_credentials.is_none());
    assert_serializes_to(&flows, &golden);
}

#[test]
fn golden_oauth_flows_client_credentials() {
    let golden = json!({
        "clientCredentials": {
            "tokenUrl": "https://auth.example.com/token",
            "scopes": {"admin": "Admin access"}
        }
    });
    let flows: OAuthFlows = serde_json::from_value(golden.clone()).unwrap();
    assert!(flows.client_credentials.is_some());
    assert!(flows.authorization_code.is_none());
    assert_serializes_to(&flows, &golden);
}

// ============================================================================
// 26. AgentCardSignature — camelCase
// ============================================================================

#[test]
fn golden_agent_card_signature() {
    let golden = json!({
        "protected": "eyJhbGciOiJSUzI1NiJ9",
        "signature": "dGVzdC1zaWduYXR1cmU"
    });
    let sig: AgentCardSignature = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(sig.protected, "eyJhbGciOiJSUzI1NiJ9");
    assert_eq!(sig.signature, "dGVzdC1zaWduYXR1cmU");
    assert!(sig.header.is_none());
    assert_serializes_to(&sig, &golden);
}

// ============================================================================
// 27. AgentExtension — camelCase
// ============================================================================

#[test]
fn golden_agent_extension() {
    let golden = json!({
        "uri": "urn:a2a:ext:streaming",
        "description": "Streaming support",
        "required": true
    });
    let ext: AgentExtension = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(ext.uri, "urn:a2a:ext:streaming");
    assert_eq!(ext.required, Some(true));
    assert_serializes_to(&ext, &golden);
}

// ============================================================================
// 28. Full AgentCard — comprehensive test
// ============================================================================

#[test]
fn golden_agent_card_minimal() {
    let golden = json!({
        "name": "Test Agent",
        "description": "A test agent",
        "version": "1.0.0",
        "url": "https://agent.example.com",
        "supportedInterfaces": [{
            "url": "https://agent.example.com/a2a",
            "transport": "JSONRPC"
        }],
        "capabilities": {},
        "defaultInputModes": ["text/plain"],
        "defaultOutputModes": ["text/plain"],
        "skills": [{
            "id": "echo",
            "name": "Echo",
            "description": "Echoes input",
            "tags": ["utility"]
        }]
    });
    let card: AgentCard = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(card.name, "Test Agent");
    assert_eq!(card.version, "1.0.0");
    assert_eq!(card.url, "https://agent.example.com");
    assert_eq!(card.supported_interfaces.len(), 1);
    assert_eq!(card.skills.len(), 1);
    assert_eq!(card.default_input_modes, vec!["text/plain"]);
    // Note: round-trip may add default fields like preferredTransport/protocolVersion
}

// ============================================================================
// 29. Error Codes — spec-defined values
// ============================================================================

#[test]
fn golden_error_codes() {
    use a2a_rs::error::*;
    // JSON-RPC standard errors
    assert_eq!(PARSE_ERROR, -32700);
    assert_eq!(INVALID_REQUEST, -32600);
    assert_eq!(METHOD_NOT_FOUND, -32601);
    assert_eq!(INVALID_PARAMS, -32602);

    // A2A-specific errors
    assert_eq!(TASK_NOT_FOUND, -32001);
    assert_eq!(TASK_NOT_CANCELABLE, -32002);
    assert_eq!(PUSH_NOTIFICATION_NOT_SUPPORTED, -32003);
    assert_eq!(UNSUPPORTED_OPERATION, -32004);
    assert_eq!(CONTENT_TYPE_NOT_SUPPORTED, -32005);
}

// ============================================================================
// 30. A2AError → JsonRpcError conversion
// ============================================================================

#[test]
fn golden_a2a_error_to_jsonrpc_error() {
    use a2a_rs::error::A2AError;

    let error = A2AError::task_not_found("task-001");
    let rpc_err: JsonRpcError = error.into();
    assert_eq!(rpc_err.code, -32001);
    assert!(rpc_err.message.contains("task-001"));
}

#[test]
fn golden_a2a_error_task_not_cancelable() {
    use a2a_rs::error::A2AError;

    let error = A2AError::TaskNotCancelable {
        message: "task-001".into(),
        data: None,
    };
    let rpc_err: JsonRpcError = error.into();
    assert_eq!(rpc_err.code, -32002);
}

#[test]
fn golden_a2a_error_push_not_supported() {
    use a2a_rs::error::A2AError;

    let error = A2AError::PushNotificationNotSupported {
        message: "not supported".into(),
        data: None,
    };
    let rpc_err: JsonRpcError = error.into();
    assert_eq!(rpc_err.code, -32003);
}

#[test]
fn golden_a2a_error_unsupported_operation() {
    use a2a_rs::error::A2AError;

    let error = A2AError::UnsupportedOperation {
        message: "streaming".into(),
        data: None,
    };
    let rpc_err: JsonRpcError = error.into();
    assert_eq!(rpc_err.code, -32004);
}

#[test]
fn golden_a2a_error_content_type_not_supported() {
    use a2a_rs::error::A2AError;

    let error = A2AError::ContentTypeNotSupported {
        message: "video/mp4".into(),
        data: None,
    };
    let rpc_err: JsonRpcError = error.into();
    assert_eq!(rpc_err.code, -32005);
}

// ============================================================================
// 31. Push Notification Param Types — camelCase
// ============================================================================

#[test]
fn golden_create_task_push_notification_config_params() {
    let golden = json!({
        "taskId": "task-001",
        "configId": "cfg-1",
        "config": {
            "url": "https://hooks.example.com/notify"
        }
    });
    let params: CreateTaskPushNotificationConfigParams =
        serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(params.task_id, "task-001");
    assert_eq!(params.config_id, "cfg-1");
    assert_eq!(params.config.url, "https://hooks.example.com/notify");
    assert_serializes_to(&params, &golden);
}

// ============================================================================
// 32. Cross-type integration: Full JSON-RPC message/send request
// ============================================================================

#[test]
fn golden_full_message_send_jsonrpc() {
    // A complete JSON-RPC message/send request as it would appear on the wire
    let golden = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "message/send",
        "params": {
            "message": {
                "messageId": "msg-001",
                "role": "user",
                "kind": "message",
                "parts": [
                    {"kind": "text", "text": "Generate a Rust function"},
                    {"kind": "data", "data": {"language": "rust", "style": "idiomatic"}}
                ]
            },
            "configuration": {
                "acceptedOutputModes": ["text/plain", "application/json"],
                "blocking": true,
                "historyLength": 5
            }
        }
    });

    let req: JsonRpcRequest = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(req.method, "message/send");

    // Parse the params as SendMessageParams
    let params: SendMessageParams = serde_json::from_value(req.params.unwrap()).unwrap();
    assert_eq!(params.message.parts.len(), 2);
    assert!(matches!(params.message.parts[0], Part::Text { .. }));
    assert!(matches!(params.message.parts[1], Part::Data { .. }));
    assert_eq!(params.configuration.as_ref().unwrap().blocking, Some(true));
}

#[test]
fn golden_full_message_send_response_jsonrpc() {
    // Complete JSON-RPC response with a Task result
    let golden = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "id": "task-001",
            "contextId": "ctx-001",
            "kind": "task",
            "status": {
                "state": "working",
                "timestamp": "2024-01-15T10:30:00Z"
            }
        }
    });

    let resp: JsonRpcResponse = serde_json::from_value(golden.clone()).unwrap();
    let result = resp.result.unwrap();

    // The result should parse as a SendMessageResponse (Task variant)
    let smr: SendMessageResponse = serde_json::from_value(result).unwrap();
    assert!(matches!(smr, SendMessageResponse::Task(_)));
}

// ============================================================================
// 32b. JSON-RPC 2.0 envelope for EVERY A2A method
// ============================================================================

/// Helper: wrap params in a JSON-RPC 2.0 request envelope
fn jsonrpc_request(id: impl Into<JsonRpcId>, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": serde_json::to_value(id.into()).unwrap(),
        "method": method,
        "params": params
    })
}

/// Helper: wrap result in a JSON-RPC 2.0 success response
fn jsonrpc_success(id: impl Into<JsonRpcId>, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": serde_json::to_value(id.into()).unwrap(),
        "result": result
    })
}

/// Helper: wrap error in a JSON-RPC 2.0 error response
fn jsonrpc_error(id: impl Into<JsonRpcId>, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": serde_json::to_value(id.into()).unwrap(),
        "error": {
            "code": code,
            "message": message
        }
    })
}

// -- message/send request --

#[test]
fn golden_jsonrpc2_message_send_request() {
    let wire = jsonrpc_request(
        JsonRpcId::Number(1),
        "message/send",
        json!({
            "message": {
                "messageId": "m1",
                "role": "user",
                "kind": "message",
                "parts": [{"kind": "text", "text": "Hello"}]
            }
        }),
    );

    let req: JsonRpcRequest = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(req.jsonrpc, "2.0");
    assert_eq!(req.id, Some(JsonRpcId::Number(1)));
    assert_eq!(req.method, "message/send");

    let params: SendMessageParams = serde_json::from_value(req.params.unwrap()).unwrap();
    assert_eq!(params.message.message_id, "m1");
    assert_eq!(params.message.role, Role::User);
}

// -- message/send response (Task) --

#[test]
fn golden_jsonrpc2_message_send_response_task() {
    let wire = jsonrpc_success(
        JsonRpcId::Number(1),
        json!({
            "id": "task-001",
            "contextId": "ctx-001",
            "kind": "task",
            "status": {"state": "working"}
        }),
    );

    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    assert_eq!(resp.jsonrpc, "2.0");
    assert!(resp.error.is_none());
    let result: SendMessageResponse = serde_json::from_value(resp.result.unwrap()).unwrap();
    match result {
        SendMessageResponse::Task(t) => {
            assert_eq!(t.id, "task-001");
            assert_eq!(t.status.state, TaskState::Working);
        }
        _ => panic!("Expected Task variant"),
    }
}

// -- message/send response (Message) --

#[test]
fn golden_jsonrpc2_message_send_response_message() {
    let wire = jsonrpc_success(
        JsonRpcId::Number(2),
        json!({
            "messageId": "m2",
            "role": "agent",
            "kind": "message",
            "parts": [{"kind": "text", "text": "Direct reply, no task created"}]
        }),
    );

    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    let result: SendMessageResponse = serde_json::from_value(resp.result.unwrap()).unwrap();
    assert!(matches!(result, SendMessageResponse::Message(_)));
}

// -- message/stream request (same params as message/send) --

#[test]
fn golden_jsonrpc2_message_stream_request() {
    let wire = jsonrpc_request(
        JsonRpcId::Number(3),
        "message/stream",
        json!({
            "message": {
                "messageId": "m1",
                "role": "user",
                "kind": "message",
                "parts": [{"kind": "text", "text": "Stream me a response"}]
            },
            "configuration": {
                "acceptedOutputModes": ["text/plain"],
                "blocking": false
            }
        }),
    );

    let req: JsonRpcRequest = serde_json::from_value(wire).unwrap();
    assert_eq!(req.method, "message/stream");
    let params: SendMessageParams = serde_json::from_value(req.params.unwrap()).unwrap();
    assert_eq!(params.configuration.as_ref().unwrap().blocking, Some(false));
}

// -- message/stream SSE events (each line is a StreamResponse) --

#[test]
fn golden_jsonrpc2_stream_sse_status_update() {
    // Each SSE `data:` line in message/stream is a JSON-RPC response
    // with result being a StreamResponse variant
    let wire = jsonrpc_success(
        JsonRpcId::Number(3),
        json!({
            "taskId": "task-001",
            "contextId": "ctx-001",
            "kind": "status-update",
            "status": {"state": "working"},
            "final": false
        }),
    );

    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    let sr: StreamResponse = serde_json::from_value(resp.result.unwrap()).unwrap();
    assert!(matches!(sr, StreamResponse::StatusUpdate(_)));
}

#[test]
fn golden_jsonrpc2_stream_sse_artifact_update() {
    let wire = jsonrpc_success(
        JsonRpcId::Number(3),
        json!({
            "taskId": "task-001",
            "contextId": "ctx-001",
            "kind": "artifact-update",
            "artifact": {
                "artifactId": "a1",
                "parts": [{"kind": "text", "text": "partial output..."}]
            },
            "append": true,
            "lastChunk": false
        }),
    );

    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    let sr: StreamResponse = serde_json::from_value(resp.result.unwrap()).unwrap();
    match sr {
        StreamResponse::ArtifactUpdate(e) => {
            assert_eq!(e.append, Some(true));
            assert_eq!(e.last_chunk, Some(false));
        }
        _ => panic!("Expected ArtifactUpdate"),
    }
}

#[test]
fn golden_jsonrpc2_stream_sse_final_task() {
    // Final SSE event is typically a complete Task snapshot
    let wire = jsonrpc_success(
        JsonRpcId::Number(3),
        json!({
            "id": "task-001",
            "contextId": "ctx-001",
            "kind": "task",
            "status": {"state": "completed"},
            "artifacts": [{
                "artifactId": "a1",
                "parts": [{"kind": "text", "text": "Final result"}]
            }]
        }),
    );

    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    let sr: StreamResponse = serde_json::from_value(resp.result.unwrap()).unwrap();
    match sr {
        StreamResponse::Task(t) => {
            assert_eq!(t.status.state, TaskState::Completed);
            assert_eq!(t.artifacts.as_ref().unwrap().len(), 1);
        }
        _ => panic!("Expected Task"),
    }
}

// -- tasks/get --

#[test]
fn golden_jsonrpc2_tasks_get_request() {
    let wire = jsonrpc_request(
        JsonRpcId::Number(4),
        "tasks/get",
        json!({
            "id": "task-001",
            "historyLength": 10
        }),
    );

    let req: JsonRpcRequest = serde_json::from_value(wire).unwrap();
    assert_eq!(req.method, "tasks/get");
    let params: GetTaskParams = serde_json::from_value(req.params.unwrap()).unwrap();
    assert_eq!(params.id, "task-001");
    assert_eq!(params.history_length, Some(10));
}

#[test]
fn golden_jsonrpc2_tasks_get_response() {
    let wire = jsonrpc_success(
        JsonRpcId::Number(4),
        json!({
            "id": "task-001",
            "contextId": "ctx-001",
            "kind": "task",
            "status": {"state": "completed"},
            "artifacts": [{
                "artifactId": "a1",
                "parts": [{"kind": "text", "text": "Result"}]
            }],
            "history": [
                {
                    "messageId": "m1",
                    "role": "user",
                    "kind": "message",
                    "parts": [{"kind": "text", "text": "Do something"}]
                },
                {
                    "messageId": "m2",
                    "role": "agent",
                    "kind": "message",
                    "parts": [{"kind": "text", "text": "Done"}]
                }
            ]
        }),
    );

    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    let task: Task = serde_json::from_value(resp.result.unwrap()).unwrap();
    assert_eq!(task.history.as_ref().unwrap().len(), 2);
    assert_eq!(task.artifacts.as_ref().unwrap().len(), 1);
}

// -- tasks/list --

#[test]
fn golden_jsonrpc2_tasks_list_request() {
    let wire = jsonrpc_request(
        JsonRpcId::Number(5),
        "tasks/list",
        json!({
            "contextId": "ctx-001",
            "status": "working",
            "pageSize": 20
        }),
    );

    let req: JsonRpcRequest = serde_json::from_value(wire).unwrap();
    assert_eq!(req.method, "tasks/list");
    let params: ListTasksParams = serde_json::from_value(req.params.unwrap()).unwrap();
    assert_eq!(params.context_id.as_deref(), Some("ctx-001"));
    assert_eq!(params.status, Some(TaskState::Working));
}

#[test]
fn golden_jsonrpc2_tasks_list_response() {
    let wire = jsonrpc_success(
        JsonRpcId::Number(5),
        json!({
            "tasks": [
                {
                    "id": "task-1",
                    "contextId": "ctx-001",
                    "kind": "task",
                    "status": {"state": "completed"}
                },
                {
                    "id": "task-2",
                    "contextId": "ctx-001",
                    "kind": "task",
                    "status": {"state": "working"}
                }
            ],
            "nextPageToken": "tok-abc",
            "pageSize": 20,
            "totalSize": 42
        }),
    );

    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    let list: ListTasksResponse = serde_json::from_value(resp.result.unwrap()).unwrap();
    assert_eq!(list.tasks.len(), 2);
    assert_eq!(list.next_page_token, "tok-abc");
    assert_eq!(list.total_size, 42);
}

// -- tasks/cancel --

#[test]
fn golden_jsonrpc2_tasks_cancel_request() {
    let wire = jsonrpc_request(
        JsonRpcId::Number(6),
        "tasks/cancel",
        json!({
            "id": "task-001"
        }),
    );

    let req: JsonRpcRequest = serde_json::from_value(wire).unwrap();
    assert_eq!(req.method, "tasks/cancel");
    let params: CancelTaskParams = serde_json::from_value(req.params.unwrap()).unwrap();
    assert_eq!(params.id, "task-001");
}

#[test]
fn golden_jsonrpc2_tasks_cancel_response() {
    // Cancel returns the Task in its new state
    let wire = jsonrpc_success(
        JsonRpcId::Number(6),
        json!({
            "id": "task-001",
            "contextId": "ctx-001",
            "kind": "task",
            "status": {"state": "canceled"}
        }),
    );

    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    let task: Task = serde_json::from_value(resp.result.unwrap()).unwrap();
    assert_eq!(task.status.state, TaskState::Canceled);
}

// -- tasks/subscribe --

#[test]
fn golden_jsonrpc2_tasks_subscribe_request() {
    let wire = jsonrpc_request(
        JsonRpcId::Number(7),
        "tasks/subscribe",
        json!({
            "id": "task-001"
        }),
    );

    let req: JsonRpcRequest = serde_json::from_value(wire).unwrap();
    assert_eq!(req.method, "tasks/subscribe");
    let params: SubscribeToTaskParams = serde_json::from_value(req.params.unwrap()).unwrap();
    assert_eq!(params.id, "task-001");
}

// -- JSON-RPC 2.0 error responses for each A2A error code --

#[test]
fn golden_jsonrpc2_error_parse_error() {
    let wire = jsonrpc_error(JsonRpcId::Null, -32700, "Parse error");
    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    assert!(resp.result.is_none());
    assert_eq!(resp.error.as_ref().unwrap().code, -32700);
}

#[test]
fn golden_jsonrpc2_error_invalid_request() {
    let wire = jsonrpc_error(JsonRpcId::Null, -32600, "Invalid request");
    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    assert_eq!(resp.error.as_ref().unwrap().code, -32600);
}

#[test]
fn golden_jsonrpc2_error_method_not_found() {
    let wire = jsonrpc_error(JsonRpcId::Number(1), -32601, "Method not found");
    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    assert_eq!(resp.error.as_ref().unwrap().code, -32601);
}

#[test]
fn golden_jsonrpc2_error_invalid_params() {
    let wire = jsonrpc_error(JsonRpcId::Number(1), -32602, "Invalid params");
    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    assert_eq!(resp.error.as_ref().unwrap().code, -32602);
}

#[test]
fn golden_jsonrpc2_error_task_not_found() {
    let wire = jsonrpc_error(JsonRpcId::Number(4), -32001, "Task not found: task-999");
    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    assert_eq!(resp.error.as_ref().unwrap().code, -32001);
    assert!(resp.error.as_ref().unwrap().message.contains("task-999"));
}

#[test]
fn golden_jsonrpc2_error_task_not_cancelable() {
    let wire = jsonrpc_error(JsonRpcId::Number(6), -32002, "Task cannot be canceled");
    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    assert_eq!(resp.error.as_ref().unwrap().code, -32002);
}

#[test]
fn golden_jsonrpc2_error_push_not_supported() {
    let wire = jsonrpc_error(
        JsonRpcId::Number(1),
        -32003,
        "Push notifications not supported",
    );
    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    assert_eq!(resp.error.as_ref().unwrap().code, -32003);
}

#[test]
fn golden_jsonrpc2_error_unsupported_operation() {
    let wire = jsonrpc_error(JsonRpcId::Number(1), -32004, "Unsupported operation");
    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    assert_eq!(resp.error.as_ref().unwrap().code, -32004);
}

#[test]
fn golden_jsonrpc2_error_content_type_not_supported() {
    let wire = jsonrpc_error(JsonRpcId::Number(1), -32005, "Content type not supported");
    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    assert_eq!(resp.error.as_ref().unwrap().code, -32005);
}

// -- JSON-RPC 2.0 error with structured data field --

#[test]
fn golden_jsonrpc2_error_with_data() {
    let wire = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32001,
            "message": "Task not found",
            "data": {
                "taskId": "task-999",
                "suggestion": "Task may have expired"
            }
        }
    });

    let resp: JsonRpcResponse = serde_json::from_value(wire).unwrap();
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32001);
    let data = err.data.unwrap();
    assert_eq!(data["taskId"], "task-999");
}

// -- JSON-RPC 2.0 notification (no id) --

#[test]
fn golden_jsonrpc2_notification_no_id() {
    // JSON-RPC 2.0 notifications have no "id" field
    let wire = json!({
        "jsonrpc": "2.0",
        "method": "tasks/pushNotification",
        "params": {
            "taskId": "task-001",
            "contextId": "ctx-001",
            "kind": "status-update",
            "status": {"state": "completed"},
            "final": true
        }
    });

    let req: JsonRpcRequest = serde_json::from_value(wire).unwrap();
    assert!(req.id.is_none(), "Notification must not have an id");
    assert_eq!(req.method, "tasks/pushNotification");
}

// -- JSON-RPC 2.0 string id (both string and number IDs are valid) --

#[test]
fn golden_jsonrpc2_string_id() {
    let wire = json!({
        "jsonrpc": "2.0",
        "id": "req-abc-123",
        "method": "tasks/get",
        "params": {"id": "task-001"}
    });

    let req: JsonRpcRequest = serde_json::from_value(wire).unwrap();
    assert_eq!(req.id, Some(JsonRpcId::String("req-abc-123".into())));
}

// -- message/send with context continuation (multi-turn) --

#[test]
fn golden_jsonrpc2_message_send_with_context() {
    // Second turn: client provides contextId and/or taskId to continue
    let wire = jsonrpc_request(
        JsonRpcId::Number(10),
        "message/send",
        json!({
            "message": {
                "messageId": "m3",
                "role": "user",
                "kind": "message",
                "parts": [{"kind": "text", "text": "Follow up question"}],
                "contextId": "ctx-001",
                "taskId": "task-001"
            }
        }),
    );

    let req: JsonRpcRequest = serde_json::from_value(wire).unwrap();
    let params: SendMessageParams = serde_json::from_value(req.params.unwrap()).unwrap();
    assert_eq!(params.message.context_id.as_deref(), Some("ctx-001"));
    assert_eq!(params.message.task_id.as_deref(), Some("task-001"));
}

// -- message/send with push notification config --

#[test]
fn golden_jsonrpc2_message_send_with_push_config() {
    let wire = jsonrpc_request(
        JsonRpcId::Number(11),
        "message/send",
        json!({
            "message": {
                "messageId": "m1",
                "role": "user",
                "kind": "message",
                "parts": [{"kind": "text", "text": "Long running task"}]
            },
            "configuration": {
                "pushNotificationConfig": {
                    "url": "https://hooks.example.com/a2a-updates",
                    "authentication": {
                        "schemes": ["Bearer"],
                        "credentials": "my-webhook-token"
                    }
                }
            }
        }),
    );

    let req: JsonRpcRequest = serde_json::from_value(wire).unwrap();
    let params: SendMessageParams = serde_json::from_value(req.params.unwrap()).unwrap();
    let pnc = params
        .configuration
        .unwrap()
        .push_notification_config
        .unwrap();
    assert_eq!(pnc.url, "https://hooks.example.com/a2a-updates");
    assert_eq!(pnc.authentication.unwrap().schemes, vec!["Bearer"]);
}

// ============================================================================
// 33. Field name casing verification — ensure snake_case proto → camelCase JSON
// ============================================================================

#[test]
fn golden_verify_camel_case_field_names() {
    // Build a Task with all camelCase fields and verify exact key names
    let task = Task {
        id: "t1".into(),
        context_id: "c1".into(),
        kind: "task".into(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: None,
        },
        artifacts: None,
        history: None,
        metadata: None,
    };
    let json = serde_json::to_value(&task).unwrap();
    let obj = json.as_object().unwrap();

    // These MUST be camelCase, not snake_case
    assert!(
        obj.contains_key("contextId"),
        "Expected 'contextId', not 'context_id'"
    );
    assert!(
        !obj.contains_key("context_id"),
        "Must not use snake_case 'context_id'"
    );
}

#[test]
fn golden_verify_message_camel_case() {
    let msg = Message {
        message_id: "m1".into(),
        role: Role::User,
        kind: "message".into(),
        parts: vec![Part::text("hello")],
        context_id: Some("c1".into()),
        task_id: Some("t1".into()),
        metadata: None,
        extensions: None,
        reference_task_ids: Some(vec!["t0".into()]),
    };
    let json = serde_json::to_value(&msg).unwrap();
    let obj = json.as_object().unwrap();

    assert!(obj.contains_key("messageId"), "Expected 'messageId'");
    assert!(obj.contains_key("contextId"), "Expected 'contextId'");
    assert!(obj.contains_key("taskId"), "Expected 'taskId'");
    assert!(
        obj.contains_key("referenceTaskIds"),
        "Expected 'referenceTaskIds'"
    );

    // Must NOT have snake_case versions
    assert!(!obj.contains_key("message_id"));
    assert!(!obj.contains_key("context_id"));
    assert!(!obj.contains_key("task_id"));
    assert!(!obj.contains_key("reference_task_ids"));
}

#[test]
fn golden_verify_artifact_camel_case() {
    let artifact = Artifact {
        artifact_id: "a1".into(),
        name: None,
        description: None,
        parts: vec![Part::text("x")],
        metadata: None,
        extensions: None,
    };
    let json = serde_json::to_value(&artifact).unwrap();
    let obj = json.as_object().unwrap();

    assert!(obj.contains_key("artifactId"), "Expected 'artifactId'");
    assert!(!obj.contains_key("artifact_id"));
}

#[test]
fn golden_verify_status_update_camel_case() {
    let event = TaskStatusUpdateEvent {
        task_id: "t1".into(),
        context_id: "c1".into(),
        kind: "status-update".into(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: None,
        },
        r#final: false,
        metadata: None,
    };
    let json = serde_json::to_value(&event).unwrap();
    let obj = json.as_object().unwrap();

    assert!(obj.contains_key("taskId"), "Expected 'taskId'");
    assert!(obj.contains_key("contextId"), "Expected 'contextId'");
    assert!(obj.contains_key("final"), "Expected 'final'");
    assert!(!obj.contains_key("task_id"));
    assert!(!obj.contains_key("context_id"));
}

#[test]
fn golden_verify_artifact_update_camel_case() {
    let event = TaskArtifactUpdateEvent {
        task_id: "t1".into(),
        context_id: "c1".into(),
        kind: "artifact-update".into(),
        artifact: Artifact {
            artifact_id: "a1".into(),
            name: None,
            description: None,
            parts: vec![Part::text("x")],
            metadata: None,
            extensions: None,
        },
        append: Some(true),
        last_chunk: Some(false),
        metadata: None,
    };
    let json = serde_json::to_value(&event).unwrap();
    let obj = json.as_object().unwrap();

    assert!(obj.contains_key("taskId"));
    assert!(obj.contains_key("contextId"));
    assert!(obj.contains_key("lastChunk"), "Expected 'lastChunk'");
    assert!(!obj.contains_key("last_chunk"));
}

// ============================================================================
// 34. Negative tests — reject invalid JSON
// ============================================================================

#[test]
fn golden_reject_invalid_task_state() {
    let result = serde_json::from_str::<TaskState>("\"invalid-state\"");
    assert!(result.is_err(), "Should reject unknown task state");
}

#[test]
fn golden_reject_invalid_role() {
    let result = serde_json::from_str::<Role>("\"system\"");
    assert!(result.is_err(), "Should reject unknown role");
}

#[test]
fn golden_reject_part_without_kind() {
    let bad_json = json!({"text": "hello"});
    let result = serde_json::from_value::<Part>(bad_json);
    assert!(result.is_err(), "Part without 'kind' should fail");
}

#[test]
fn golden_reject_part_with_invalid_kind() {
    let bad_json = json!({"kind": "audio", "data": {}});
    let result = serde_json::from_value::<Part>(bad_json);
    assert!(result.is_err(), "Part with unknown kind should fail");
}

#[test]
fn golden_reject_stream_response_without_kind() {
    let bad_json = json!({"id": "t1", "status": {"state": "working"}});
    let result = serde_json::from_value::<StreamResponse>(bad_json);
    assert!(result.is_err(), "StreamResponse without 'kind' should fail");
}

#[test]
fn golden_reject_stream_response_invalid_kind() {
    let bad_json = json!({"kind": "notification", "data": {}});
    let result = serde_json::from_value::<StreamResponse>(bad_json);
    assert!(
        result.is_err(),
        "StreamResponse with unknown kind should fail"
    );
}

#[test]
fn golden_reject_security_scheme_without_type() {
    let bad_json = json!({"name": "X-Key"});
    let result = serde_json::from_value::<SecurityScheme>(bad_json);
    assert!(result.is_err(), "SecurityScheme without 'type' should fail");
}

#[test]
fn golden_reject_security_scheme_invalid_type() {
    let bad_json = json!({"type": "saml"});
    let result = serde_json::from_value::<SecurityScheme>(bad_json);
    assert!(
        result.is_err(),
        "SecurityScheme with unknown type should fail"
    );
}

// ============================================================================
// 35. Optional field omission — ensure skip_serializing_if works
// ============================================================================

#[test]
fn golden_optional_fields_omitted() {
    // When optional fields are None, they must NOT appear in JSON
    let msg = Message {
        message_id: "m1".into(),
        role: Role::User,
        kind: "message".into(),
        parts: vec![Part::text("hi")],
        context_id: None,
        task_id: None,
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    };
    let json = serde_json::to_value(&msg).unwrap();
    let obj = json.as_object().unwrap();

    // These optional fields should be absent, not null
    assert!(
        !obj.contains_key("contextId"),
        "None should be omitted, not null"
    );
    assert!(!obj.contains_key("taskId"));
    assert!(!obj.contains_key("metadata"));
    assert!(!obj.contains_key("extensions"));
    assert!(!obj.contains_key("referenceTaskIds"));
}

#[test]
fn golden_task_optional_fields_omitted() {
    let task = Task {
        id: "t1".into(),
        context_id: "c1".into(),
        kind: "task".into(),
        status: TaskStatus {
            state: TaskState::Submitted,
            message: None,
            timestamp: None,
        },
        artifacts: None,
        history: None,
        metadata: None,
    };
    let json = serde_json::to_value(&task).unwrap();
    let obj = json.as_object().unwrap();

    assert!(!obj.contains_key("artifacts"));
    assert!(!obj.contains_key("history"));
    assert!(!obj.contains_key("metadata"));
}

// ============================================================================
// 36. SubscribeToTaskParams
// ============================================================================

#[test]
fn golden_subscribe_to_task_params() {
    let golden = json!({"id": "task-001"});
    let params: SubscribeToTaskParams = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(params.id, "task-001");
    assert_serializes_to(&params, &golden);
}

// ============================================================================
// 37. JsonRpcId variants
// ============================================================================

#[test]
fn golden_jsonrpc_id_string() {
    let id: JsonRpcId = serde_json::from_str("\"req-1\"").unwrap();
    assert_eq!(id, JsonRpcId::String("req-1".into()));
    assert_eq!(serde_json::to_value(&id).unwrap(), json!("req-1"));
}

#[test]
fn golden_jsonrpc_id_number() {
    let id: JsonRpcId = serde_json::from_str("42").unwrap();
    assert_eq!(id, JsonRpcId::Number(42));
    assert_eq!(serde_json::to_value(&id).unwrap(), json!(42));
}

#[test]
fn golden_jsonrpc_id_null() {
    let id: JsonRpcId = serde_json::from_str("null").unwrap();
    assert_eq!(id, JsonRpcId::Null);
    assert_eq!(serde_json::to_value(&id).unwrap(), json!(null));
}
