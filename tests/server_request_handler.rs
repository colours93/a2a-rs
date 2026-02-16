//! Tests for DefaultRequestHandler — ported from Python SDK's
//! tests/server/request_handlers/ directory.

use std::sync::Arc;

use a2a_rs::error::A2AError;
use a2a_rs::server::{
    AgentExecutor, DefaultRequestHandler, EventQueue, InMemoryTaskStore, RequestContext,
    RequestHandler, TaskStore,
};
use a2a_rs::types::*;
use async_trait::async_trait;

mod common;

// ---- Test agent executors ----

/// Agent that immediately completes with a text message.
struct ImmediateCompleteAgent;

#[async_trait]
impl AgentExecutor for ImmediateCompleteAgent {
    async fn execute(
        &self,
        context: RequestContext,
        event_queue: EventQueue,
    ) -> a2a_rs::error::A2AResult<()> {
        let updater =
            a2a_rs::server::TaskUpdater::new(event_queue, context.task_id, context.context_id);
        updater.complete_with_text("Done!").await
    }

    async fn cancel(
        &self,
        context: RequestContext,
        event_queue: EventQueue,
    ) -> a2a_rs::error::A2AResult<()> {
        let updater =
            a2a_rs::server::TaskUpdater::new(event_queue, context.task_id, context.context_id);
        updater.cancel(None).await
    }
}

/// Agent that fails immediately.
struct ImmediateFailAgent;

#[async_trait]
impl AgentExecutor for ImmediateFailAgent {
    async fn execute(
        &self,
        _context: RequestContext,
        _event_queue: EventQueue,
    ) -> a2a_rs::error::A2AResult<()> {
        Err(A2AError::InternalError {
            message: "Agent crashed".to_string(),
            data: None,
        })
    }

    async fn cancel(
        &self,
        _context: RequestContext,
        _event_queue: EventQueue,
    ) -> a2a_rs::error::A2AResult<()> {
        Ok(())
    }
}

fn make_handler(executor: Arc<dyn AgentExecutor>) -> DefaultRequestHandler {
    let store: Arc<dyn TaskStore> = Arc::new(InMemoryTaskStore::new());
    DefaultRequestHandler::new(executor, store)
}

fn make_send_params(text: &str) -> a2a_rs::server::SendMessageParams {
    a2a_rs::server::SendMessageParams {
        message: Message::user("m1", text),
        configuration: None,
        metadata: None,
        tenant: None,
    }
}

fn make_send_params_with_task_id(text: &str, task_id: &str) -> a2a_rs::server::SendMessageParams {
    let mut msg = Message::user("m1", text);
    msg.task_id = Some(task_id.to_string());
    a2a_rs::server::SendMessageParams {
        message: msg,
        configuration: None,
        metadata: None,
        tenant: None,
    }
}

// ---- on_message_send tests ----

#[tokio::test]
async fn test_message_send_creates_task_and_completes() {
    let handler = make_handler(Arc::new(ImmediateCompleteAgent));
    let params = make_send_params("Hello");

    let response = handler.on_message_send(params).await.unwrap();
    match response {
        SendMessageResponse::Task(task) => {
            assert_eq!(task.status.state, TaskState::Completed);
            assert!(task.history.is_some());
        }
        _ => panic!("Expected Task response"),
    }
}

#[tokio::test]
async fn test_message_send_agent_failure_results_in_failed_task() {
    let handler = make_handler(Arc::new(ImmediateFailAgent));
    let params = make_send_params("Hello");

    let response = handler.on_message_send(params).await.unwrap();
    match response {
        SendMessageResponse::Task(task) => {
            assert_eq!(task.status.state, TaskState::Failed);
        }
        _ => panic!("Expected Task response"),
    }
}

#[tokio::test]
async fn test_message_send_with_nonexistent_task_id_errors() {
    let handler = make_handler(Arc::new(ImmediateCompleteAgent));
    let params = make_send_params_with_task_id("Hello", "nonexistent");

    let result = handler.on_message_send(params).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        A2AError::TaskNotFound { .. } => {}
        other => panic!("Expected TaskNotFound, got: {:?}", other),
    }
}

// ---- on_get_task tests ----

#[tokio::test]
async fn test_get_task_returns_task() {
    let handler = make_handler(Arc::new(ImmediateCompleteAgent));
    let params = make_send_params("Hello");

    let response = handler.on_message_send(params).await.unwrap();
    let task_id = match &response {
        SendMessageResponse::Task(t) => t.id.clone(),
        _ => panic!("Expected Task"),
    };

    let get_params = a2a_rs::server::GetTaskParams {
        id: task_id,
        history_length: None,
        metadata: None,
        tenant: None,
    };
    let task = handler.on_get_task(get_params).await.unwrap();
    assert_eq!(task.status.state, TaskState::Completed);
}

#[tokio::test]
async fn test_get_task_not_found() {
    let handler = make_handler(Arc::new(ImmediateCompleteAgent));

    let get_params = a2a_rs::server::GetTaskParams {
        id: "nonexistent".to_string(),
        history_length: None,
        metadata: None,
        tenant: None,
    };
    let result = handler.on_get_task(get_params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_task_with_history_length() {
    let handler = make_handler(Arc::new(ImmediateCompleteAgent));
    let params = make_send_params("Hello");

    let response = handler.on_message_send(params).await.unwrap();
    let task_id = match &response {
        SendMessageResponse::Task(t) => t.id.clone(),
        _ => panic!("Expected Task"),
    };

    let get_params = a2a_rs::server::GetTaskParams {
        id: task_id,
        history_length: Some(1),
        metadata: None,
        tenant: None,
    };
    let task = handler.on_get_task(get_params).await.unwrap();
    if let Some(history) = &task.history {
        assert!(history.len() <= 1);
    }
}

// ---- on_cancel_task tests ----

#[tokio::test]
async fn test_cancel_nonexistent_task_errors() {
    let handler = make_handler(Arc::new(ImmediateCompleteAgent));

    let cancel_params = a2a_rs::server::CancelTaskParams {
        id: "nonexistent".to_string(),
        metadata: None,
        tenant: None,
    };
    let result = handler.on_cancel_task(cancel_params).await;
    assert!(result.is_err());
}

// ---- on_subscribe_to_task tests ----

#[tokio::test]
async fn test_subscribe_nonexistent_task_errors() {
    let handler = make_handler(Arc::new(ImmediateCompleteAgent));

    let sub_params = a2a_rs::server::SubscribeToTaskParams {
        id: "nonexistent".to_string(),
        metadata: None,
        tenant: None,
    };
    let result = handler.on_subscribe_to_task(sub_params).await;
    assert!(result.is_err());
}

// ---- Push notification defaults ----

#[tokio::test]
async fn test_push_notification_set_unsupported() {
    let handler = make_handler(Arc::new(ImmediateCompleteAgent));
    let result = handler
        .on_set_task_push_notification_config(serde_json::json!({}))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_push_notification_get_unsupported() {
    let handler = make_handler(Arc::new(ImmediateCompleteAgent));
    let result = handler
        .on_get_task_push_notification_config(serde_json::json!({}))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_push_notification_list_unsupported() {
    let handler = make_handler(Arc::new(ImmediateCompleteAgent));
    let result = handler
        .on_list_task_push_notification_config(serde_json::json!({}))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_push_notification_delete_unsupported() {
    let handler = make_handler(Arc::new(ImmediateCompleteAgent));
    let result = handler
        .on_delete_task_push_notification_config(serde_json::json!({}))
        .await;
    assert!(result.is_err());
}

// ---- on_resubscribe_to_task default ----

#[tokio::test]
async fn test_resubscribe_default_unsupported() {
    // The default trait implementation returns UnsupportedOperation,
    // but DefaultRequestHandler overrides it. Test a custom handler.
    struct MinimalHandler;

    #[async_trait]
    impl RequestHandler for MinimalHandler {
        async fn on_message_send(
            &self,
            _p: a2a_rs::server::SendMessageParams,
        ) -> a2a_rs::error::A2AResult<SendMessageResponse> {
            unimplemented!()
        }
        async fn on_message_send_stream(
            &self,
            _p: a2a_rs::server::SendMessageParams,
        ) -> a2a_rs::error::A2AResult<tokio::sync::broadcast::Receiver<StreamResponse>> {
            unimplemented!()
        }
        async fn on_get_task(
            &self,
            _p: a2a_rs::server::GetTaskParams,
        ) -> a2a_rs::error::A2AResult<Task> {
            unimplemented!()
        }
        async fn on_list_tasks(
            &self,
            _p: a2a_rs::server::TaskListParams,
        ) -> a2a_rs::error::A2AResult<a2a_rs::server::TaskListResponse> {
            unimplemented!()
        }
        async fn on_cancel_task(
            &self,
            _p: a2a_rs::server::CancelTaskParams,
        ) -> a2a_rs::error::A2AResult<Task> {
            unimplemented!()
        }
        async fn on_subscribe_to_task(
            &self,
            _p: a2a_rs::server::SubscribeToTaskParams,
        ) -> a2a_rs::error::A2AResult<tokio::sync::broadcast::Receiver<StreamResponse>> {
            unimplemented!()
        }
    }

    let handler = MinimalHandler;
    let params = a2a_rs::server::SubscribeToTaskParams {
        id: "t1".to_string(),
        metadata: None,
        tenant: None,
    };
    let result = handler.on_resubscribe_to_task(params).await;
    assert!(result.is_err());
}

// ---- on_list_tasks ----

#[tokio::test]
async fn test_list_tasks_empty() {
    let handler = make_handler(Arc::new(ImmediateCompleteAgent));
    let params = a2a_rs::server::TaskListParams {
        context_id: None,
        status: None,
        page_size: None,
        page_token: None,
    };
    let result = handler.on_list_tasks(params).await.unwrap();
    assert!(result.tasks.is_empty());
}

#[tokio::test]
async fn test_list_tasks_after_send() {
    let handler = make_handler(Arc::new(ImmediateCompleteAgent));
    let send_params = make_send_params("Hello");
    handler.on_message_send(send_params).await.unwrap();

    let list_params = a2a_rs::server::TaskListParams {
        context_id: None,
        status: None,
        page_size: None,
        page_token: None,
    };
    let result = handler.on_list_tasks(list_params).await.unwrap();
    assert_eq!(result.tasks.len(), 1);
}
