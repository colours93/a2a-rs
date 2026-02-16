//! High-level A2A client for interacting with remote agents.
//!
//! Mirrors the Python SDK's `Client` / `BaseClient` architecture, providing
//! typed methods for every JSON-RPC method in the A2A v0.3 specification.

use serde::Serialize;

use crate::error::{A2AError, A2AResult};
use crate::types::{
    AgentCard, CancelTaskParams, GetTaskParams, GetTaskPushNotificationConfigParams, JsonRpcId,
    JsonRpcRequest, JsonRpcResponse, ListTasksParams, ListTasksResponse, Message, Part, Role,
    SendMessageConfiguration, SendMessageParams, SendMessageResponse,
    SetTaskPushNotificationConfigParams, Task, TaskIdParams, TaskPushNotificationConfig,
};

use super::card_resolver::CardResolver;
use super::sse::SseStream;
use super::transport::{JsonRpcTransport, Transport};

/// Client for interacting with A2A-compatible agents.
///
/// Provides typed methods for all A2A JSON-RPC methods:
/// - `message/send` — send a message and get a task or message back
/// - `message/stream` — send a message and stream status/artifact updates
/// - `tasks/get` — retrieve a task by ID
/// - `tasks/list` — list tasks with filtering
/// - `tasks/cancel` — cancel a running task
/// - `tasks/resubscribe` — resubscribe to task update events
/// - `tasks/pushNotificationConfig/set` — set push notification config
/// - `tasks/pushNotificationConfig/get` — get push notification config
///
/// # Construction
///
/// The client can be created in several ways:
///
/// ```no_run
/// use a2a_rs::client::A2AClient;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // From a base URL (resolves agent card, discovers endpoint):
/// let client = A2AClient::from_url("http://localhost:7420").await?;
///
/// // With custom transport:
/// use a2a_rs::client::JsonRpcTransport;
/// let transport = JsonRpcTransport::new("http://localhost:7420/a2a");
/// let client = A2AClient::with_transport(Box::new(transport));
/// # Ok(())
/// # }
/// ```
pub struct A2AClient {
    transport: Box<dyn Transport>,
    agent_card: Option<AgentCard>,
}

impl std::fmt::Debug for A2AClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("A2AClient")
            .field("agent_card", &self.agent_card)
            .finish_non_exhaustive()
    }
}

impl A2AClient {
    /// Create a client from a base URL.
    ///
    /// This will:
    /// 1. Resolve the agent card from `{url}/.well-known/agent-card.json`
    ///    (falls back to `/.well-known/agent.json` for older agents)
    /// 2. Extract the JSON-RPC endpoint URL from the card
    /// 3. Create a `JsonRpcTransport` pointing to that endpoint
    ///
    /// # Errors
    ///
    /// Returns an error if the agent card cannot be fetched or parsed,
    /// or if no JSON-RPC interface is found in the card.
    pub async fn from_url(url: &str) -> A2AResult<Self> {
        let resolver = CardResolver::new();
        let card = resolver.resolve(url).await?;
        Self::from_card(card)
    }

    /// Create a client from an already-resolved agent card.
    ///
    /// Extracts the JSON-RPC endpoint URL from the card's
    /// `supported_interfaces` and creates a transport for it.
    ///
    /// # Errors
    ///
    /// Returns an error if the card does not contain a JSON-RPC interface.
    pub fn from_card(card: AgentCard) -> A2AResult<Self> {
        let url = CardResolver::get_a2a_url(&card).ok_or_else(|| {
            A2AError::Transport(format!(
                "agent card for '{}' has no JSONRPC interface in supported_interfaces",
                card.name
            ))
        })?;

        let transport = JsonRpcTransport::new(url);

        Ok(Self {
            transport: Box::new(transport),
            agent_card: Some(card),
        })
    }

    /// Create a client with a custom transport.
    ///
    /// Use this when you need custom HTTP configuration, authentication,
    /// or a non-HTTP transport implementation.
    pub fn with_transport(transport: Box<dyn Transport>) -> Self {
        Self {
            transport,
            agent_card: None,
        }
    }

    /// Create a client from a direct endpoint URL (skips agent card resolution).
    ///
    /// This is a convenience method when you already know the A2A endpoint
    /// and don't need the agent card.
    pub fn from_endpoint(url: &str) -> Self {
        let transport = JsonRpcTransport::new(url);
        Self {
            transport: Box::new(transport),
            agent_card: None,
        }
    }

    // ──────────────────────────────────────────────────
    // Core A2A JSON-RPC Methods
    // ──────────────────────────────────────────────────

    /// Send a message to the agent (`message/send`).
    ///
    /// The agent processes the message and returns either a [`Task`] or a
    /// direct [`Message`]. For long-running tasks, poll with [`get_task()`] or
    /// use [`send_message_stream()`] for real-time updates.
    ///
    /// [`get_task()`]: Self::get_task
    /// [`send_message_stream()`]: Self::send_message_stream
    pub async fn send_message(&self, params: SendMessageParams) -> A2AResult<SendMessageResponse> {
        let request = build_request("message/send", &params)?;
        let response = self.transport.send(&request).await?;
        parse_result(response)
    }

    /// Send a message with streaming (`message/stream`).
    ///
    /// Returns an SSE stream that yields [`crate::types::StreamResponse`] events as the
    /// agent processes the message. Events include status updates, artifact
    /// updates, and the final task snapshot.
    pub async fn send_message_stream(&self, params: SendMessageParams) -> A2AResult<SseStream> {
        let request = build_request("message/stream", &params)?;
        self.transport.send_stream(&request).await
    }

    /// Get the current state of a task (`tasks/get`).
    pub async fn get_task(&self, params: GetTaskParams) -> A2AResult<Task> {
        let request = build_request("tasks/get", &params)?;
        let response = self.transport.send(&request).await?;
        parse_result(response)
    }

    /// List tasks with optional filtering (`tasks/list`).
    pub async fn list_tasks(&self, params: ListTasksParams) -> A2AResult<ListTasksResponse> {
        let request = build_request("tasks/list", &params)?;
        let response = self.transport.send(&request).await?;
        parse_result(response)
    }

    /// Cancel a running task (`tasks/cancel`).
    pub async fn cancel_task(&self, params: CancelTaskParams) -> A2AResult<Task> {
        let request = build_request("tasks/cancel", &params)?;
        let response = self.transport.send(&request).await?;
        parse_result(response)
    }

    /// Resubscribe to a task's event stream (`tasks/resubscribe`).
    ///
    /// Returns an SSE stream of [`crate::types::StreamResponse`] events for
    /// the given task. Use this to reconnect to a task's event stream after
    /// a disconnection.
    ///
    /// Python SDK ref: `Client.resubscribe()`
    pub async fn resubscribe(&self, params: TaskIdParams) -> A2AResult<SseStream> {
        let request = build_request("tasks/resubscribe", &params)?;
        self.transport.send_stream(&request).await
    }

    /// Set push notification configuration for a task
    /// (`tasks/pushNotificationConfig/set`).
    ///
    /// Python SDK ref: `Client.set_task_callback()`
    pub async fn set_task_callback(
        &self,
        params: SetTaskPushNotificationConfigParams,
    ) -> A2AResult<TaskPushNotificationConfig> {
        let request = build_request("tasks/pushNotificationConfig/set", &params)?;
        let response = self.transport.send(&request).await?;
        parse_result(response)
    }

    /// Get push notification configuration for a task
    /// (`tasks/pushNotificationConfig/get`).
    ///
    /// Python SDK ref: `Client.get_task_callback()`
    pub async fn get_task_callback(
        &self,
        params: GetTaskPushNotificationConfigParams,
    ) -> A2AResult<TaskPushNotificationConfig> {
        let request = build_request("tasks/pushNotificationConfig/get", &params)?;
        let response = self.transport.send(&request).await?;
        parse_result(response)
    }

    /// Get the cached agent card.
    ///
    /// If the card was already resolved during construction, returns the
    /// cached copy. Otherwise, returns an error (use [`from_url()`] to
    /// auto-resolve, or fetch manually with [`CardResolver`]).
    ///
    /// To refresh the card from the server (including fetching the
    /// authenticated extended card if supported), use [`get_card_from_server()`].
    ///
    /// [`from_url()`]: Self::from_url
    /// [`get_card_from_server()`]: Self::get_card_from_server
    /// [`CardResolver`]: super::CardResolver
    pub fn get_card(&self) -> A2AResult<&AgentCard> {
        self.agent_card.as_ref().ok_or_else(|| {
            A2AError::Transport(
                "no agent card available; use A2AClient::from_url() to auto-resolve".to_string(),
            )
        })
    }

    /// Fetch the agent card from the server, updating the cached copy.
    ///
    /// If the agent supports authenticated extended cards
    /// (`supports_authenticated_extended_card` is `true`), this will make a
    /// `getAuthenticatedExtendedCard` JSON-RPC call to fetch the full card.
    /// Otherwise, it returns the already-cached card (or fetches the public
    /// card if none is cached).
    ///
    /// Python SDK ref: `BaseClient.get_card()` in `base_client.py`
    pub async fn get_card_from_server(&mut self) -> A2AResult<&AgentCard> {
        // If we don't have a card yet, we can't know the base URL to fetch from.
        let card = self.agent_card.as_ref().ok_or_else(|| {
            A2AError::Transport(
                "no agent card available; use A2AClient::from_url() first".to_string(),
            )
        })?;

        // Check if we need to fetch the extended card.
        let needs_extended = card.supports_authenticated_extended_card.unwrap_or(false);

        if !needs_extended {
            return Ok(self.agent_card.as_ref().unwrap());
        }

        // Make the JSON-RPC call to get the authenticated extended card.
        let request = build_request("getAuthenticatedExtendedCard", &serde_json::json!({}))?;
        let response = self.transport.send(&request).await?;
        let extended_card: AgentCard = parse_result(response)?;

        self.agent_card = Some(extended_card);
        Ok(self.agent_card.as_ref().unwrap())
    }

    /// Close the client and release any held resources.
    ///
    /// Python SDK ref: `BaseClient.close()` in `base_client.py`
    pub async fn close(self) -> A2AResult<()> {
        self.transport.close().await
    }

    // ──────────────────────────────────────────────────
    // Convenience Helpers
    // ──────────────────────────────────────────────────

    /// Convenience: send a text message and get back the response.
    ///
    /// Creates a simple user message with a single text part and sends it
    /// via `message/send`.
    pub async fn send_text(&self, text: &str) -> A2AResult<SendMessageResponse> {
        let params = build_text_message_params(text);
        self.send_message(params).await
    }

    /// Convenience: send a text message and stream responses.
    ///
    /// Creates a simple user message with a single text part and sends it
    /// via `message/stream`.
    pub async fn send_text_stream(&self, text: &str) -> A2AResult<SseStream> {
        let params = build_text_message_params(text);
        self.send_message_stream(params).await
    }

    /// Convenience: send a text message with a specific context ID.
    ///
    /// Useful for continuing a conversation within an existing context.
    pub async fn send_text_in_context(
        &self,
        text: &str,
        context_id: &str,
    ) -> A2AResult<SendMessageResponse> {
        let message = Message {
            message_id: uuid::Uuid::new_v4().to_string(),
            role: Role::User,
            kind: "message".to_string(),
            parts: vec![Part::text(text)],
            context_id: Some(context_id.to_string()),
            task_id: None,
            reference_task_ids: None,
            metadata: None,
            extensions: None,
        };
        let params = SendMessageParams {
            message,
            configuration: None,
            metadata: None,
            tenant: None,
        };
        self.send_message(params).await
    }

    /// Convenience: send a text message with configuration options.
    pub async fn send_text_with_config(
        &self,
        text: &str,
        config: SendMessageConfiguration,
    ) -> A2AResult<SendMessageResponse> {
        let message = Message {
            message_id: uuid::Uuid::new_v4().to_string(),
            role: Role::User,
            kind: "message".to_string(),
            parts: vec![Part::text(text)],
            context_id: None,
            task_id: None,
            reference_task_ids: None,
            metadata: None,
            extensions: None,
        };
        let params = SendMessageParams {
            message,
            configuration: Some(config),
            metadata: None,
            tenant: None,
        };
        self.send_message(params).await
    }

    /// Convenience: get a task by ID with optional history length.
    pub async fn get_task_by_id(
        &self,
        task_id: &str,
        history_length: Option<i32>,
    ) -> A2AResult<Task> {
        self.get_task(GetTaskParams {
            id: task_id.to_string(),
            history_length,
            metadata: None,
            tenant: None,
        })
        .await
    }

    /// Convenience: cancel a task by ID.
    pub async fn cancel_task_by_id(&self, task_id: &str) -> A2AResult<Task> {
        self.cancel_task(CancelTaskParams {
            id: task_id.to_string(),
            metadata: None,
            tenant: None,
        })
        .await
    }

    /// Convenience: resubscribe to a task by ID.
    pub async fn resubscribe_by_id(&self, task_id: &str) -> A2AResult<SseStream> {
        self.resubscribe(TaskIdParams {
            id: task_id.to_string(),
            metadata: None,
        })
        .await
    }
}

// ──────────────────────────────────────────────────
// Internal helpers
// ──────────────────────────────────────────────────

/// Build a JSON-RPC request with a random UUID ID.
fn build_request(method: &str, params: &impl Serialize) -> A2AResult<JsonRpcRequest> {
    let params_value = serde_json::to_value(params)
        .map_err(|e| A2AError::Transport(format!("failed to serialize request params: {e}")))?;

    Ok(JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(JsonRpcId::String(uuid::Uuid::new_v4().to_string())),
        method: method.to_string(),
        params: Some(params_value),
    })
}

/// Parse the `result` field from a JSON-RPC response into the expected type.
///
/// If the response contains an error, converts it into an [`A2AError::JsonRpc`].
fn parse_result<T: serde::de::DeserializeOwned>(response: JsonRpcResponse) -> A2AResult<T> {
    // Check for JSON-RPC error.
    if let Some(error) = response.error {
        return Err(A2AError::JsonRpc {
            code: error.code,
            message: error.message,
            data: error.data,
        });
    }

    // Extract the result field.
    let result = response.result.ok_or_else(|| {
        A2AError::InvalidJson("JSON-RPC response has neither 'result' nor 'error'".to_string())
    })?;

    serde_json::from_value(result)
        .map_err(|e| A2AError::InvalidJson(format!("failed to deserialize response result: {e}")))
}

/// Build a simple text message params struct.
fn build_text_message_params(text: &str) -> SendMessageParams {
    let message = create_text_message(Role::User, text);

    SendMessageParams {
        message,
        configuration: None,
        metadata: None,
        tenant: None,
    }
}

/// Create a [`Message`] containing a single text part.
///
/// This is a convenience helper matching the Python SDK's
/// `helpers.create_text_message_object(role, content)`.
///
/// # Example
///
/// ```
/// use a2a_rs::client::create_text_message;
/// use a2a_rs::types::Role;
///
/// let msg = create_text_message(Role::User, "Hello, agent!");
/// assert_eq!(msg.role, Role::User);
/// assert_eq!(msg.parts.len(), 1);
/// ```
pub fn create_text_message(role: Role, content: &str) -> Message {
    Message {
        message_id: uuid::Uuid::new_v4().to_string(),
        role,
        kind: "message".to_string(),
        parts: vec![Part::text(content)],
        context_id: None,
        task_id: None,
        reference_task_ids: None,
        metadata: None,
        extensions: None,
    }
}
