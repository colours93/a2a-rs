//! Tests for RequestContext, ServerCallContext, and SimpleRequestContextBuilder
//! — ported from Python SDK's tests/server/agent_execution/ directory.

use std::collections::HashSet;
use std::sync::Arc;

use a2a_rs::server::{
    InMemoryTaskStore, RequestContext, RequestContextBuilder, ServerCallContext,
    SimpleRequestContextBuilder, TaskStore,
};
use a2a_rs::types::*;

// ============================================================
// RequestContext tests
// ============================================================

fn make_context(text: &str) -> RequestContext {
    RequestContext {
        task_id: "t1".to_string(),
        context_id: "c1".to_string(),
        message: Some(Message::user("m1", text)),
        task: None,
        configuration: None,
        related_tasks: Vec::new(),
        metadata: None,
        call_context: None,
    }
}

#[test]
fn test_request_context_get_user_input_single_part() {
    let ctx = make_context("Hello world");
    assert_eq!(ctx.get_user_input(" "), "Hello world");
}

#[test]
fn test_request_context_get_user_input_multiple_parts() {
    let ctx = RequestContext {
        task_id: "t1".to_string(),
        context_id: "c1".to_string(),
        message: Some(Message {
            message_id: "m1".to_string(),
            role: Role::User,
            kind: "message".to_string(),
            parts: vec![Part::text("Hello"), Part::text("World")],
            context_id: None,
            task_id: None,
            metadata: None,
            extensions: None,
            reference_task_ids: None,
        }),
        task: None,
        configuration: None,
        related_tasks: Vec::new(),
        metadata: None,
        call_context: None,
    };
    assert_eq!(ctx.get_user_input(" "), "Hello World");
    assert_eq!(ctx.get_user_input(", "), "Hello, World");
}

#[test]
fn test_request_context_get_user_input_no_message() {
    let ctx = RequestContext {
        task_id: "t1".to_string(),
        context_id: "c1".to_string(),
        message: None,
        task: None,
        configuration: None,
        related_tasks: Vec::new(),
        metadata: None,
        call_context: None,
    };
    assert_eq!(ctx.get_user_input(" "), "");
}

#[test]
fn test_request_context_get_user_input_non_text_parts_ignored() {
    let ctx = RequestContext {
        task_id: "t1".to_string(),
        context_id: "c1".to_string(),
        message: Some(Message {
            message_id: "m1".to_string(),
            role: Role::User,
            kind: "message".to_string(),
            parts: vec![
                Part::text("Hello"),
                Part::Data {
                    data: serde_json::json!({"key": "value"}),
                    metadata: None,
                },
                Part::text("World"),
            ],
            context_id: None,
            task_id: None,
            metadata: None,
            extensions: None,
            reference_task_ids: None,
        }),
        task: None,
        configuration: None,
        related_tasks: Vec::new(),
        metadata: None,
        call_context: None,
    };
    assert_eq!(ctx.get_user_input(" "), "Hello World");
}

#[test]
fn test_request_context_attach_related_task() {
    let mut ctx = make_context("Hello");
    assert!(ctx.related_tasks.is_empty());

    let task = Task {
        id: "related-1".to_string(),
        context_id: "c1".to_string(),
        kind: "task".to_string(),
        status: TaskStatus {
            state: TaskState::Completed,
            message: None,
            timestamp: None,
        },
        artifacts: None,
        history: None,
        metadata: None,
    };

    ctx.attach_related_task(task);
    assert_eq!(ctx.related_tasks.len(), 1);
    assert_eq!(ctx.related_tasks[0].id, "related-1");
}

#[test]
fn test_request_context_add_activated_extension() {
    let mut ctx = make_context("Hello");
    ctx.call_context = Some(ServerCallContext::default());

    ctx.add_activated_extension("urn:extension:foo".to_string());

    let activated = &ctx.call_context.as_ref().unwrap().activated_extensions;
    assert!(activated.contains("urn:extension:foo"));
}

#[test]
fn test_request_context_add_activated_extension_no_context() {
    let mut ctx = make_context("Hello");
    // No call_context — should be a no-op
    ctx.add_activated_extension("urn:extension:foo".to_string());
    assert!(ctx.call_context.is_none());
}

#[test]
fn test_request_context_requested_extensions() {
    let mut ctx = make_context("Hello");
    ctx.call_context = Some(ServerCallContext {
        state: Default::default(),
        requested_extensions: {
            let mut s = HashSet::new();
            s.insert("urn:ext:a".to_string());
            s.insert("urn:ext:b".to_string());
            s
        },
        activated_extensions: HashSet::new(),
    });

    let exts = ctx.requested_extensions();
    assert_eq!(exts.len(), 2);
    assert!(exts.contains("urn:ext:a"));
    assert!(exts.contains("urn:ext:b"));
}

#[test]
fn test_request_context_requested_extensions_empty_without_context() {
    let ctx = make_context("Hello");
    assert!(ctx.requested_extensions().is_empty());
}

// ============================================================
// ServerCallContext tests
// ============================================================

#[test]
fn test_server_call_context_default() {
    let ctx = ServerCallContext::default();
    assert!(ctx.state.is_empty());
    assert!(ctx.requested_extensions.is_empty());
    assert!(ctx.activated_extensions.is_empty());
}

#[test]
fn test_server_call_context_state() {
    let mut ctx = ServerCallContext::default();
    ctx.state
        .insert("user_id".to_string(), serde_json::json!("u1"));
    assert_eq!(ctx.state["user_id"], "u1");
}

// ============================================================
// SimpleRequestContextBuilder tests
// ============================================================

#[tokio::test]
async fn test_simple_builder_basic() {
    let builder = SimpleRequestContextBuilder::new(None, false);

    let msg = Message::user("m1", "Hello");
    let params = SendMessageParams {
        message: msg,
        configuration: None,
        metadata: None,
        tenant: None,
    };

    let ctx = builder
        .build(Some(&params), Some("t1"), Some("c1"), None, None)
        .await
        .unwrap();

    assert_eq!(ctx.task_id, "t1");
    assert_eq!(ctx.context_id, "c1");
    assert!(ctx.message.is_some());
    assert!(ctx.related_tasks.is_empty());
}

#[tokio::test]
async fn test_simple_builder_resolves_ids_from_message() {
    let builder = SimpleRequestContextBuilder::new(None, false);

    let mut msg = Message::user("m1", "Hello");
    msg.task_id = Some("msg-task".to_string());
    msg.context_id = Some("msg-ctx".to_string());
    let params = SendMessageParams {
        message: msg,
        configuration: None,
        metadata: None,
        tenant: None,
    };

    // When task_id and context_id are not explicitly provided, fall back to message
    let ctx = builder
        .build(Some(&params), None, None, None, None)
        .await
        .unwrap();

    assert_eq!(ctx.task_id, "msg-task");
    assert_eq!(ctx.context_id, "msg-ctx");
}

#[tokio::test]
async fn test_simple_builder_resolves_ids_from_task() {
    let builder = SimpleRequestContextBuilder::new(None, false);

    let task = Task {
        id: "task-id".to_string(),
        context_id: "task-ctx".to_string(),
        kind: "task".to_string(),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: None,
        },
        artifacts: None,
        history: None,
        metadata: None,
    };

    let ctx = builder
        .build(None, None, None, Some(&task), None)
        .await
        .unwrap();

    assert_eq!(ctx.task_id, "task-id");
    assert_eq!(ctx.context_id, "task-ctx");
}

#[tokio::test]
async fn test_simple_builder_explicit_ids_take_precedence() {
    let builder = SimpleRequestContextBuilder::new(None, false);

    let mut msg = Message::user("m1", "Hello");
    msg.task_id = Some("msg-task".to_string());
    let params = SendMessageParams {
        message: msg,
        configuration: None,
        metadata: None,
        tenant: None,
    };

    let ctx = builder
        .build(
            Some(&params),
            Some("explicit-task"),
            Some("explicit-ctx"),
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(ctx.task_id, "explicit-task");
    assert_eq!(ctx.context_id, "explicit-ctx");
}

#[tokio::test]
async fn test_simple_builder_populates_referred_tasks() {
    let store: Arc<dyn TaskStore> = Arc::new(InMemoryTaskStore::new());

    // Save a task to be referenced
    let ref_task = Task {
        id: "ref-1".to_string(),
        context_id: "c1".to_string(),
        kind: "task".to_string(),
        status: TaskStatus {
            state: TaskState::Completed,
            message: None,
            timestamp: None,
        },
        artifacts: None,
        history: None,
        metadata: None,
    };
    store.save(ref_task).await.unwrap();

    let builder = SimpleRequestContextBuilder::new(Some(store), true);

    let mut msg = Message::user("m1", "Hello");
    msg.reference_task_ids = Some(vec!["ref-1".to_string(), "nonexistent".to_string()]);
    let params = SendMessageParams {
        message: msg,
        configuration: None,
        metadata: None,
        tenant: None,
    };

    let ctx = builder
        .build(Some(&params), Some("t1"), Some("c1"), None, None)
        .await
        .unwrap();

    // Should have found ref-1 but not nonexistent
    assert_eq!(ctx.related_tasks.len(), 1);
    assert_eq!(ctx.related_tasks[0].id, "ref-1");
}

#[tokio::test]
async fn test_simple_builder_no_populate_when_disabled() {
    let store: Arc<dyn TaskStore> = Arc::new(InMemoryTaskStore::new());

    let ref_task = Task {
        id: "ref-1".to_string(),
        context_id: "c1".to_string(),
        kind: "task".to_string(),
        status: TaskStatus {
            state: TaskState::Completed,
            message: None,
            timestamp: None,
        },
        artifacts: None,
        history: None,
        metadata: None,
    };
    store.save(ref_task).await.unwrap();

    // should_populate_referred_tasks = false
    let builder = SimpleRequestContextBuilder::new(Some(store), false);

    let mut msg = Message::user("m1", "Hello");
    msg.reference_task_ids = Some(vec!["ref-1".to_string()]);
    let params = SendMessageParams {
        message: msg,
        configuration: None,
        metadata: None,
        tenant: None,
    };

    let ctx = builder
        .build(Some(&params), Some("t1"), Some("c1"), None, None)
        .await
        .unwrap();

    assert!(ctx.related_tasks.is_empty());
}

#[tokio::test]
async fn test_simple_builder_default() {
    let builder = SimpleRequestContextBuilder::default();
    let ctx = builder
        .build(None, Some("t1"), Some("c1"), None, None)
        .await
        .unwrap();
    assert_eq!(ctx.task_id, "t1");
}

#[tokio::test]
async fn test_simple_builder_with_call_context() {
    let builder = SimpleRequestContextBuilder::new(None, false);

    let call_ctx = ServerCallContext {
        state: Default::default(),
        requested_extensions: {
            let mut s = HashSet::new();
            s.insert("urn:ext:test".to_string());
            s
        },
        activated_extensions: HashSet::new(),
    };

    let ctx = builder
        .build(None, Some("t1"), Some("c1"), None, Some(call_ctx))
        .await
        .unwrap();

    assert!(ctx.call_context.is_some());
    assert!(ctx.requested_extensions().contains("urn:ext:test"));
}

#[tokio::test]
async fn test_simple_builder_passes_configuration() {
    let builder = SimpleRequestContextBuilder::new(None, false);

    let msg = Message::user("m1", "Hello");
    let params = SendMessageParams {
        message: msg,
        configuration: Some(SendMessageConfiguration {
            accepted_output_modes: Some(vec!["text/plain".to_string()]),
            push_notification_config: None,
            history_length: Some(5),
            blocking: Some(true),
        }),
        metadata: Some(serde_json::json!({"key": "value"})),
        tenant: None,
    };

    let ctx = builder
        .build(Some(&params), Some("t1"), Some("c1"), None, None)
        .await
        .unwrap();

    assert!(ctx.configuration.is_some());
    let config = ctx.configuration.unwrap();
    assert_eq!(config.blocking, Some(true));
    assert_eq!(ctx.metadata.unwrap()["key"], "value");
}
