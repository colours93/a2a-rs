//! Axum integration — ready-made HTTP routes for A2A servers.
//!
//! Provides an [`a2a_router`] function that creates an axum `Router` with:
//! - `POST /a2a` — JSON-RPC 2.0 dispatch for all A2A methods
//! - `GET /.well-known/agent.json` — agent card discovery
//!
//! Mirrors Python SDK's `JSONRPCApplication` from
//! `a2a.server.apps.jsonrpc.jsonrpc_app`.
//!
//! # Supported JSON-RPC Methods
//!
//! | Method | Description |
//! |--------|-------------|
//! | `message/send` | Send a message and get a task or message |
//! | `message/stream` | Send a message with SSE streaming |
//! | `tasks/get` | Retrieve a task by ID |
//! | `tasks/list` | List tasks with filtering |
//! | `tasks/cancel` | Cancel a running task |
//! | `tasks/subscribe` | Subscribe to task updates (SSE) |
//! | `tasks/resubscribe` | Re-subscribe to a running task's stream |
//! | `tasks/pushNotificationConfig/set` | Set push notification config |
//! | `tasks/pushNotificationConfig/get` | Get push notification config |
//! | `tasks/pushNotificationConfig/list` | List push notification configs |
//! | `tasks/pushNotificationConfig/delete` | Delete push notification config |
//!
//! # Example
//!
//! ```rust,ignore
//! use a2a_rs::server::{a2a_router, DefaultRequestHandler, InMemoryTaskStore};
//! use a2a_rs::types::AgentCard;
//! use std::sync::Arc;
//!
//! let handler = Arc::new(DefaultRequestHandler::new(executor, store));
//! let app = a2a_router(handler, agent_card);
//!
//! let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
//! axum::serve(listener, app).await?;
//! ```

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::Router;
use futures::stream::Stream;
use serde_json::Value;
use tokio::sync::broadcast;
use tracing::{debug, error, warn};

use crate::error::{self, A2AError};
use crate::types::{AgentCard, JsonRpcError as A2AJsonRpcError, StreamResponse};

use super::request_handler::{
    CancelTaskParams, GetTaskParams, RequestHandler, SendMessageConfiguration, SendMessageParams,
    SubscribeToTaskParams,
};
use super::task_store::TaskListParams;

/// Shared state for the axum routes.
struct AppState {
    handler: Arc<dyn RequestHandler>,
    agent_card: AgentCard,
}

/// Create an axum Router with A2A protocol routes.
///
/// # Routes
///
/// - `POST /a2a` — JSON-RPC 2.0 dispatch for all A2A methods
/// - `GET /.well-known/agent.json` — agent card discovery endpoint (current)
/// - `GET /.well-known/agent` — deprecated agent card path (with warning)
///
/// # Parameters
///
/// - `handler` — the request handler implementing A2A logic
/// - `agent_card` — the agent card to serve at the well-known endpoint
pub fn a2a_router(handler: Arc<dyn RequestHandler>, agent_card: AgentCard) -> Router {
    let state = Arc::new(AppState {
        handler,
        agent_card,
    });

    Router::new()
        .route("/.well-known/agent.json", get(handle_agent_card))
        .route("/.well-known/agent", get(handle_agent_card_deprecated))
        .route("/a2a", post(handle_jsonrpc))
        .with_state(state)
}

/// Serve the agent card at the well-known endpoint.
async fn handle_agent_card(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(&state.agent_card).into_response()
}

/// Serve the agent card at the deprecated path (with warning).
///
/// Mirrors Python SDK's support for `/.well-known/agent` alongside `/.well-known/agent.json`.
async fn handle_agent_card_deprecated(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    warn!(
        "Deprecated agent card endpoint '/.well-known/agent' accessed. \
         Please use '/.well-known/agent.json' instead."
    );
    Json(&state.agent_card).into_response()
}

/// JSON-RPC 2.0 request envelope.
#[derive(Debug, serde::Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

/// JSON-RPC 2.0 response envelope.
#[derive(Debug, serde::Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<A2AJsonRpcError>,
}

impl JsonRpcResponse {
    fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Option<Value>, code: i64, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(A2AJsonRpcError {
                code,
                message,
                data: None,
            }),
        }
    }

    fn from_a2a_error(id: Option<Value>, err: A2AError) -> Self {
        let rpc_err: A2AJsonRpcError = err.into();
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(rpc_err),
        }
    }
}

/// Main JSON-RPC dispatch handler.
///
/// Parses the incoming JSON-RPC request, routes to the appropriate handler
/// method, and returns either a JSON response or an SSE stream.
///
/// Mirrors Python SDK's `_handle_requests` method routing.
async fn handle_jsonrpc(
    State(state): State<Arc<AppState>>,
    Json(request): Json<JsonRpcRequest>,
) -> Response {
    // Validate JSON-RPC version.
    if request.jsonrpc != "2.0" {
        return Json(JsonRpcResponse::error(
            request.id,
            error::INVALID_REQUEST,
            "Invalid JSON-RPC version — must be \"2.0\"".to_string(),
        ))
        .into_response();
    }

    debug!(method = %request.method, "JSON-RPC request received");

    match request.method.as_str() {
        "message/send" => handle_message_send(state, request).await,
        "message/stream" => handle_message_stream(state, request).await,
        "tasks/get" => handle_tasks_get(state, request).await,
        "tasks/list" => handle_tasks_list(state, request).await,
        "tasks/cancel" => handle_tasks_cancel(state, request).await,
        "tasks/subscribe" => handle_tasks_subscribe(state, request).await,
        "tasks/resubscribe" => handle_tasks_resubscribe(state, request).await,
        "tasks/pushNotificationConfig/set" => {
            handle_push_notification_config_set(state, request).await
        }
        "tasks/pushNotificationConfig/get" => {
            handle_push_notification_config_get(state, request).await
        }
        "tasks/pushNotificationConfig/list" => {
            handle_push_notification_config_list(state, request).await
        }
        "tasks/pushNotificationConfig/delete" => {
            handle_push_notification_config_delete(state, request).await
        }
        "agent/authenticatedExtendedCard" => {
            // Return the agent card as the extended card.
            // Mirrors Python SDK's get_authenticated_extended_card method.
            handle_authenticated_extended_card(state, request).await
        }
        method => {
            warn!(method = %method, "Unknown JSON-RPC method");
            Json(JsonRpcResponse::error(
                request.id,
                error::METHOD_NOT_FOUND,
                format!("Method not found: {}", method),
            ))
            .into_response()
        }
    }
}

/// Parse `SendMessageParams` from JSON-RPC params.
fn parse_send_message_params(params: Value) -> Result<SendMessageParams, String> {
    let obj = params.as_object().ok_or("params must be an object")?;

    let message: crate::types::Message = serde_json::from_value(
        obj.get("message")
            .cloned()
            .ok_or("missing 'message' field")?,
    )
    .map_err(|e| format!("invalid message: {}", e))?;

    let configuration = obj
        .get("configuration")
        .and_then(|v| {
            if v.is_null() {
                None
            } else {
                Some(parse_send_config(v.clone()))
            }
        })
        .transpose()?;

    let metadata = obj
        .get("metadata")
        .and_then(|v| if v.is_null() { None } else { Some(v.clone()) });

    let tenant = obj.get("tenant").and_then(|v| v.as_str().map(String::from));

    Ok(SendMessageParams {
        message,
        configuration,
        metadata,
        tenant,
    })
}

/// Parse `SendMessageConfiguration` from a JSON value.
fn parse_send_config(value: Value) -> Result<SendMessageConfiguration, String> {
    let obj = value.as_object().ok_or("configuration must be an object")?;

    Ok(SendMessageConfiguration {
        accepted_output_modes: obj.get("acceptedOutputModes").and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
        }),
        blocking: obj.get("blocking").and_then(|v| v.as_bool()),
        history_length: obj
            .get("historyLength")
            .and_then(|v| v.as_u64().map(|n| n as usize)),
        push_notification_config: obj.get("pushNotificationConfig").cloned(),
    })
}

/// Handle `message/send` — synchronous execution.
///
/// Returns either a Task or Message in the response, matching Python SDK's
/// `SendMessageResponse` which is `Task | Message`.
async fn handle_message_send(state: Arc<AppState>, request: JsonRpcRequest) -> Response {
    let params = match parse_send_message_params(request.params) {
        Ok(p) => p,
        Err(e) => {
            return Json(JsonRpcResponse::error(
                request.id,
                error::INVALID_PARAMS,
                format!("Invalid params: {}", e),
            ))
            .into_response();
        }
    };

    match state.handler.on_message_send(params).await {
        Ok(response) => {
            let result = serde_json::to_value(&response);
            match result {
                Ok(v) => Json(JsonRpcResponse::success(request.id, v)).into_response(),
                Err(e) => {
                    error!(error = %e, "Failed to serialize response");
                    Json(JsonRpcResponse::error(
                        request.id,
                        error::INTERNAL_ERROR,
                        format!("Internal error: {}", e),
                    ))
                    .into_response()
                }
            }
        }
        Err(e) => Json(JsonRpcResponse::from_a2a_error(request.id, e)).into_response(),
    }
}

/// Handle `message/stream` — SSE streaming.
///
/// Mirrors Python SDK's `JSONRPCHandler.on_message_send_stream` which:
/// 1. Validates streaming is supported via agent card capabilities
/// 2. Wraps each event in a JSON-RPC success response envelope
/// 3. Catches errors and yields them as JSON-RPC error responses
async fn handle_message_stream(state: Arc<AppState>, request: JsonRpcRequest) -> Response {
    // Check streaming capability (mirrors Python SDK's @validate decorator).
    if !state.agent_card.capabilities.streaming.unwrap_or(false) {
        return Json(JsonRpcResponse::error(
            request.id,
            error::UNSUPPORTED_OPERATION,
            "Streaming is not supported by the agent".to_string(),
        ))
        .into_response();
    }

    let params = match parse_send_message_params(request.params) {
        Ok(p) => p,
        Err(e) => {
            return Json(JsonRpcResponse::error(
                request.id,
                error::INVALID_PARAMS,
                format!("Invalid params: {}", e),
            ))
            .into_response();
        }
    };

    match state.handler.on_message_send_stream(params).await {
        Ok(rx) => {
            let stream = make_sse_stream(request.id, rx);
            Sse::new(stream)
                .keep_alive(KeepAlive::default())
                .into_response()
        }
        Err(e) => Json(JsonRpcResponse::from_a2a_error(request.id, e)).into_response(),
    }
}

/// Handle `tasks/get`.
async fn handle_tasks_get(state: Arc<AppState>, request: JsonRpcRequest) -> Response {
    let params = match parse_get_task_params(request.params) {
        Ok(p) => p,
        Err(e) => {
            return Json(JsonRpcResponse::error(
                request.id,
                error::INVALID_PARAMS,
                format!("Invalid params: {}", e),
            ))
            .into_response();
        }
    };

    match state.handler.on_get_task(params).await {
        Ok(task) => match serde_json::to_value(&task) {
            Ok(v) => Json(JsonRpcResponse::success(request.id, v)).into_response(),
            Err(e) => Json(JsonRpcResponse::error(
                request.id,
                error::INTERNAL_ERROR,
                format!("Internal error: {}", e),
            ))
            .into_response(),
        },
        Err(e) => Json(JsonRpcResponse::from_a2a_error(request.id, e)).into_response(),
    }
}

/// Handle `tasks/list`.
async fn handle_tasks_list(state: Arc<AppState>, request: JsonRpcRequest) -> Response {
    let params = match parse_list_tasks_params(request.params) {
        Ok(p) => p,
        Err(e) => {
            return Json(JsonRpcResponse::error(
                request.id,
                error::INVALID_PARAMS,
                format!("Invalid params: {}", e),
            ))
            .into_response();
        }
    };

    match state.handler.on_list_tasks(params).await {
        Ok(response) => match serde_json::to_value(&response.tasks) {
            Ok(v) => {
                let mut result = serde_json::Map::new();
                result.insert("tasks".to_string(), v);
                if let Some(token) = response.next_page_token {
                    result.insert("nextPageToken".to_string(), Value::String(token));
                }
                Json(JsonRpcResponse::success(request.id, Value::Object(result))).into_response()
            }
            Err(e) => Json(JsonRpcResponse::error(
                request.id,
                error::INTERNAL_ERROR,
                format!("Internal error: {}", e),
            ))
            .into_response(),
        },
        Err(e) => Json(JsonRpcResponse::from_a2a_error(request.id, e)).into_response(),
    }
}

/// Handle `tasks/cancel`.
async fn handle_tasks_cancel(state: Arc<AppState>, request: JsonRpcRequest) -> Response {
    let params = match parse_cancel_task_params(request.params) {
        Ok(p) => p,
        Err(e) => {
            return Json(JsonRpcResponse::error(
                request.id,
                error::INVALID_PARAMS,
                format!("Invalid params: {}", e),
            ))
            .into_response();
        }
    };

    match state.handler.on_cancel_task(params).await {
        Ok(task) => match serde_json::to_value(&task) {
            Ok(v) => Json(JsonRpcResponse::success(request.id, v)).into_response(),
            Err(e) => Json(JsonRpcResponse::error(
                request.id,
                error::INTERNAL_ERROR,
                format!("Internal error: {}", e),
            ))
            .into_response(),
        },
        Err(e) => Json(JsonRpcResponse::from_a2a_error(request.id, e)).into_response(),
    }
}

/// Handle `tasks/subscribe` — SSE streaming for an existing task.
async fn handle_tasks_subscribe(state: Arc<AppState>, request: JsonRpcRequest) -> Response {
    let params = match parse_subscribe_params(request.params) {
        Ok(p) => p,
        Err(e) => {
            return Json(JsonRpcResponse::error(
                request.id,
                error::INVALID_PARAMS,
                format!("Invalid params: {}", e),
            ))
            .into_response();
        }
    };

    match state.handler.on_subscribe_to_task(params).await {
        Ok(rx) => {
            let stream = make_sse_stream(request.id, rx);
            Sse::new(stream)
                .keep_alive(KeepAlive::default())
                .into_response()
        }
        Err(e) => Json(JsonRpcResponse::from_a2a_error(request.id, e)).into_response(),
    }
}

/// Handle `tasks/resubscribe` — re-subscribe to a running task's event stream.
///
/// Mirrors Python SDK's `on_resubscribe_to_task`.
async fn handle_tasks_resubscribe(state: Arc<AppState>, request: JsonRpcRequest) -> Response {
    let params = match parse_subscribe_params(request.params) {
        Ok(p) => p,
        Err(e) => {
            return Json(JsonRpcResponse::error(
                request.id,
                error::INVALID_PARAMS,
                format!("Invalid params: {}", e),
            ))
            .into_response();
        }
    };

    match state.handler.on_resubscribe_to_task(params).await {
        Ok(rx) => {
            let stream = make_sse_stream(request.id, rx);
            Sse::new(stream)
                .keep_alive(KeepAlive::default())
                .into_response()
        }
        Err(e) => Json(JsonRpcResponse::from_a2a_error(request.id, e)).into_response(),
    }
}

/// Handle `tasks/pushNotificationConfig/set`.
async fn handle_push_notification_config_set(
    state: Arc<AppState>,
    request: JsonRpcRequest,
) -> Response {
    match state
        .handler
        .on_set_task_push_notification_config(request.params)
        .await
    {
        Ok(result) => Json(JsonRpcResponse::success(request.id, result)).into_response(),
        Err(e) => Json(JsonRpcResponse::from_a2a_error(request.id, e)).into_response(),
    }
}

/// Handle `tasks/pushNotificationConfig/get`.
async fn handle_push_notification_config_get(
    state: Arc<AppState>,
    request: JsonRpcRequest,
) -> Response {
    match state
        .handler
        .on_get_task_push_notification_config(request.params)
        .await
    {
        Ok(result) => Json(JsonRpcResponse::success(request.id, result)).into_response(),
        Err(e) => Json(JsonRpcResponse::from_a2a_error(request.id, e)).into_response(),
    }
}

/// Handle `tasks/pushNotificationConfig/list`.
async fn handle_push_notification_config_list(
    state: Arc<AppState>,
    request: JsonRpcRequest,
) -> Response {
    match state
        .handler
        .on_list_task_push_notification_config(request.params)
        .await
    {
        Ok(result) => Json(JsonRpcResponse::success(request.id, result)).into_response(),
        Err(e) => Json(JsonRpcResponse::from_a2a_error(request.id, e)).into_response(),
    }
}

/// Handle `tasks/pushNotificationConfig/delete`.
async fn handle_push_notification_config_delete(
    state: Arc<AppState>,
    request: JsonRpcRequest,
) -> Response {
    match state
        .handler
        .on_delete_task_push_notification_config(request.params)
        .await
    {
        Ok(()) => Json(JsonRpcResponse::success(request.id, Value::Null)).into_response(),
        Err(e) => Json(JsonRpcResponse::from_a2a_error(request.id, e)).into_response(),
    }
}

/// Handle `agent/authenticatedExtendedCard` — return the agent card.
///
/// Mirrors Python SDK's `get_authenticated_extended_card` method.
/// For now, returns the public agent card. Extended card support can be
/// added when the `RequestHandler` trait supports it.
async fn handle_authenticated_extended_card(
    state: Arc<AppState>,
    request: JsonRpcRequest,
) -> Response {
    match serde_json::to_value(&state.agent_card) {
        Ok(v) => Json(JsonRpcResponse::success(request.id, v)).into_response(),
        Err(e) => Json(JsonRpcResponse::error(
            request.id,
            error::INTERNAL_ERROR,
            format!("Internal error: {}", e),
        ))
        .into_response(),
    }
}

// ---- Parameter parsing helpers ----

fn parse_get_task_params(params: Value) -> Result<GetTaskParams, String> {
    let obj = params.as_object().ok_or("params must be an object")?;
    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("missing 'id' field")?
        .to_string();
    let history_length = obj
        .get("historyLength")
        .and_then(|v| v.as_u64().map(|n| n as usize));
    let metadata = obj
        .get("metadata")
        .and_then(|v| if v.is_null() { None } else { Some(v.clone()) });

    let tenant = obj.get("tenant").and_then(|v| v.as_str().map(String::from));

    Ok(GetTaskParams {
        id,
        history_length,
        metadata,
        tenant,
    })
}

fn parse_list_tasks_params(params: Value) -> Result<TaskListParams, String> {
    let obj = params.as_object().ok_or("params must be an object")?;

    let context_id = obj
        .get("contextId")
        .and_then(|v| v.as_str().map(String::from));
    let status = obj.get("status").and_then(|v| {
        v.as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v.as_str()
                        .and_then(|s| serde_json::from_value(Value::String(s.to_string())).ok())
                })
                .collect()
        })
    });
    let page_size = obj
        .get("pageSize")
        .and_then(|v| v.as_u64().map(|n| n as usize));
    let page_token = obj
        .get("pageToken")
        .and_then(|v| v.as_str().map(String::from));

    Ok(TaskListParams {
        context_id,
        status,
        page_size,
        page_token,
    })
}

fn parse_cancel_task_params(params: Value) -> Result<CancelTaskParams, String> {
    let obj = params.as_object().ok_or("params must be an object")?;
    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("missing 'id' field")?
        .to_string();
    let metadata = obj
        .get("metadata")
        .and_then(|v| if v.is_null() { None } else { Some(v.clone()) });

    let tenant = obj.get("tenant").and_then(|v| v.as_str().map(String::from));

    Ok(CancelTaskParams {
        id,
        metadata,
        tenant,
    })
}

fn parse_subscribe_params(params: Value) -> Result<SubscribeToTaskParams, String> {
    let obj = params.as_object().ok_or("params must be an object")?;
    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("missing 'id' field")?
        .to_string();
    let metadata = obj
        .get("metadata")
        .and_then(|v| if v.is_null() { None } else { Some(v.clone()) });

    let tenant = obj.get("tenant").and_then(|v| v.as_str().map(String::from));

    Ok(SubscribeToTaskParams {
        id,
        metadata,
        tenant,
    })
}

// ---- SSE streaming ----

/// Create an SSE stream from a broadcast receiver.
///
/// Each `StreamResponse` event is wrapped in a JSON-RPC 2.0 success response
/// envelope before being sent as an SSE event. This mirrors the Python SDK's
/// `JSONRPCHandler.on_message_send_stream` which wraps each event in a
/// `SendStreamingMessageSuccessResponse`.
///
/// The stream ends when the channel is closed or a terminal status update is received.
fn make_sse_stream(
    request_id: Option<Value>,
    mut rx: broadcast::Receiver<StreamResponse>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let is_terminal = match &event {
                        StreamResponse::StatusUpdate(update) => update.r#final,
                        _ => false,
                    };

                    // Determine the event type for SSE.
                    let event_type = match &event {
                        StreamResponse::StatusUpdate(_) => "statusUpdate",
                        StreamResponse::ArtifactUpdate(_) => "artifactUpdate",
                        StreamResponse::Task(_) => "task",
                        StreamResponse::Message(_) => "message",
                    };

                    // Wrap in JSON-RPC response envelope (mirrors Python SDK's
                    // prepare_response_object wrapping in SendStreamingMessageResponse).
                    match serde_json::to_value(&event) {
                        Ok(result_value) => {
                            let rpc_response = JsonRpcResponse::success(
                                request_id.clone(),
                                result_value,
                            );
                            match serde_json::to_string(&rpc_response) {
                                Ok(json) => {
                                    yield Ok(Event::default()
                                        .event(event_type)
                                        .data(json));
                                }
                                Err(e) => {
                                    error!(error = %e, "Failed to serialize SSE JSON-RPC response");
                                }
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to serialize SSE event");
                        }
                    }

                    if is_terminal {
                        // Send a final empty event to signal completion.
                        yield Ok(Event::default().event("done").data(""));
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    yield Ok(Event::default().event("done").data(""));
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(missed = n, "SSE stream lagged — some events were missed");
                    // Continue receiving.
                }
            }
        }
    }
}
