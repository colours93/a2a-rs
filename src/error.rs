//! A2A error types — JSON-RPC error codes + A2A-specific errors.
//!
//! Mirrors the Python SDK's error handling:
//! - Standard JSON-RPC 2.0 errors (-32700 through -32603)
//! - A2A-specific errors (-32001 through -32007)

use crate::types::JsonRpcError;

// ---------------------------------------------------------------------------
// Standard JSON-RPC 2.0 error codes
// ---------------------------------------------------------------------------

/// Invalid JSON was received by the server.
pub const PARSE_ERROR: i64 = -32700;

/// The JSON sent is not a valid Request object.
pub const INVALID_REQUEST: i64 = -32600;

/// The method does not exist / is not available.
pub const METHOD_NOT_FOUND: i64 = -32601;

/// Invalid method parameter(s).
pub const INVALID_PARAMS: i64 = -32602;

/// Internal JSON-RPC error.
pub const INTERNAL_ERROR: i64 = -32603;

// ---------------------------------------------------------------------------
// A2A-specific error codes
// ---------------------------------------------------------------------------

/// The requested task was not found.
pub const TASK_NOT_FOUND: i64 = -32001;

/// The task cannot be canceled in its current state.
pub const TASK_NOT_CANCELABLE: i64 = -32002;

/// Push notifications are not supported by this agent.
pub const PUSH_NOTIFICATION_NOT_SUPPORTED: i64 = -32003;

/// The requested operation is not supported.
pub const UNSUPPORTED_OPERATION: i64 = -32004;

/// The content type is not supported.
pub const CONTENT_TYPE_NOT_SUPPORTED: i64 = -32005;

/// The agent returned an invalid response.
pub const INVALID_AGENT_RESPONSE: i64 = -32006;

/// Authenticated extended card is not configured.
pub const AUTHENTICATED_EXTENDED_CARD_NOT_CONFIGURED: i64 = -32007;

// ---------------------------------------------------------------------------
// A2AError enum
// ---------------------------------------------------------------------------

/// Unified error type for all A2A and JSON-RPC errors.
///
/// Each variant carries an optional human-readable message and optional
/// structured data payload (matching the Python SDK pattern).
///
/// Also includes transport/client-side error variants that are not part of
/// the A2A spec but are needed for a complete Rust SDK.
#[derive(Debug, Clone, thiserror::Error)]
pub enum A2AError {
    // -- A2A protocol errors (map to JSON-RPC error codes) --
    //
    // Each variant carries a human-readable message and an optional structured
    // `data` payload, matching the Python SDK pattern where every error type
    // has `message: str | None` and `data: Any | None = None`.
    /// Invalid JSON payload (code -32700).
    #[error("Parse error: {message}")]
    ParseError {
        /// Human-readable error message.
        message: String,
        /// Optional structured error data (e.g. parse error details).
        data: Option<serde_json::Value>,
    },

    /// Request payload validation error (code -32600).
    #[error("Invalid request: {message}")]
    InvalidRequest {
        /// Human-readable error message.
        message: String,
        /// Optional structured error data (e.g. validation errors).
        data: Option<serde_json::Value>,
    },

    /// Method not found (code -32601).
    #[error("Method not found: {message}")]
    MethodNotFound {
        /// Human-readable error message.
        message: String,
        /// Optional structured error data.
        data: Option<serde_json::Value>,
    },

    /// Invalid parameters (code -32602).
    #[error("Invalid params: {message}")]
    InvalidParams {
        /// Human-readable error message.
        message: String,
        /// Optional structured error data (e.g. validation errors).
        data: Option<serde_json::Value>,
    },

    /// Internal error (code -32603).
    #[error("Internal error: {message}")]
    InternalError {
        /// Human-readable error message.
        message: String,
        /// Optional structured error data.
        data: Option<serde_json::Value>,
    },

    /// Task not found (code -32001).
    #[error("Task not found: {message}")]
    TaskNotFound {
        /// Human-readable error message.
        message: String,
        /// Optional structured error data.
        data: Option<serde_json::Value>,
    },

    /// Task cannot be canceled (code -32002).
    #[error("Task not cancelable: {message}")]
    TaskNotCancelable {
        /// Human-readable error message.
        message: String,
        /// Optional structured error data.
        data: Option<serde_json::Value>,
    },

    /// Push notifications not supported (code -32003).
    #[error("Push notification not supported: {message}")]
    PushNotificationNotSupported {
        /// Human-readable error message.
        message: String,
        /// Optional structured error data.
        data: Option<serde_json::Value>,
    },

    /// Operation not supported (code -32004).
    #[error("Unsupported operation: {message}")]
    UnsupportedOperation {
        /// Human-readable error message.
        message: String,
        /// Optional structured error data.
        data: Option<serde_json::Value>,
    },

    /// Content type not supported (code -32005).
    #[error("Content type not supported: {message}")]
    ContentTypeNotSupported {
        /// Human-readable error message.
        message: String,
        /// Optional structured error data.
        data: Option<serde_json::Value>,
    },

    /// Invalid agent response (code -32006).
    #[error("Invalid agent response: {message}")]
    InvalidAgentResponse {
        /// Human-readable error message.
        message: String,
        /// Optional structured error data.
        data: Option<serde_json::Value>,
    },

    /// Authenticated extended card not configured (code -32007).
    #[error("Authenticated extended card not configured: {message}")]
    AuthenticatedExtendedCardNotConfigured {
        /// Human-readable error message.
        message: String,
        /// Optional structured error data.
        data: Option<serde_json::Value>,
    },

    // -- Client/transport-side errors (not A2A error codes) --
    /// Transport-level error (connection failed, request failed, etc.).
    #[error("Transport error: {0}")]
    Transport(String),

    /// Request or stream timed out.
    #[error("Timeout: {0}")]
    Timeout(String),

    /// HTTP error with status code and response body.
    #[error("HTTP {status}: {body}")]
    Http {
        /// HTTP status code.
        status: u16,
        /// Response body text.
        body: String,
    },

    /// Invalid JSON received from remote (parse or deserialization failure).
    #[error("Invalid JSON: {0}")]
    InvalidJson(String),

    /// A JSON-RPC error response was received from the remote agent.
    #[error("JSON-RPC error {code}: {message}")]
    JsonRpc {
        /// JSON-RPC error code.
        code: i64,
        /// Error message.
        message: String,
        /// Optional structured error data.
        data: Option<serde_json::Value>,
    },

    /// Catch-all for errors that don't fit other categories.
    #[error("{0}")]
    Other(String),
}

/// Convenience result type for A2A operations.
pub type A2AResult<T> = Result<T, A2AError>;

impl A2AError {
    // -- Convenience constructors (message-only, no data) --

    /// Create a `ParseError` with a message and no data.
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::ParseError {
            message: message.into(),
            data: None,
        }
    }

    /// Create an `InvalidRequest` with a message and no data.
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest {
            message: message.into(),
            data: None,
        }
    }

    /// Create a `MethodNotFound` with a message and no data.
    pub fn method_not_found(message: impl Into<String>) -> Self {
        Self::MethodNotFound {
            message: message.into(),
            data: None,
        }
    }

    /// Create an `InvalidParams` with a message and no data.
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::InvalidParams {
            message: message.into(),
            data: None,
        }
    }

    /// Create an `InternalError` with a message and no data.
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError {
            message: message.into(),
            data: None,
        }
    }

    /// Create a `TaskNotFound` with a message and no data.
    pub fn task_not_found(message: impl Into<String>) -> Self {
        Self::TaskNotFound {
            message: message.into(),
            data: None,
        }
    }

    /// Create a `TaskNotCancelable` with a message and no data.
    pub fn task_not_cancelable(message: impl Into<String>) -> Self {
        Self::TaskNotCancelable {
            message: message.into(),
            data: None,
        }
    }

    /// Create a `PushNotificationNotSupported` with a message and no data.
    pub fn push_notification_not_supported(message: impl Into<String>) -> Self {
        Self::PushNotificationNotSupported {
            message: message.into(),
            data: None,
        }
    }

    /// Create an `UnsupportedOperation` with a message and no data.
    pub fn unsupported_operation(message: impl Into<String>) -> Self {
        Self::UnsupportedOperation {
            message: message.into(),
            data: None,
        }
    }

    /// Create a `ContentTypeNotSupported` with a message and no data.
    pub fn content_type_not_supported(message: impl Into<String>) -> Self {
        Self::ContentTypeNotSupported {
            message: message.into(),
            data: None,
        }
    }

    /// Create an `InvalidAgentResponse` with a message and no data.
    pub fn invalid_agent_response(message: impl Into<String>) -> Self {
        Self::InvalidAgentResponse {
            message: message.into(),
            data: None,
        }
    }

    /// Create an `AuthenticatedExtendedCardNotConfigured` with a message and no data.
    pub fn authenticated_extended_card_not_configured(message: impl Into<String>) -> Self {
        Self::AuthenticatedExtendedCardNotConfigured {
            message: message.into(),
            data: None,
        }
    }

    /// Returns the JSON-RPC error code for this error variant.
    ///
    /// For transport/client-side errors that don't map to A2A codes,
    /// returns -32603 (internal error).
    pub fn code(&self) -> i64 {
        match self {
            A2AError::ParseError { .. } => PARSE_ERROR,
            A2AError::InvalidRequest { .. } => INVALID_REQUEST,
            A2AError::MethodNotFound { .. } => METHOD_NOT_FOUND,
            A2AError::InvalidParams { .. } => INVALID_PARAMS,
            A2AError::InternalError { .. } => INTERNAL_ERROR,
            A2AError::TaskNotFound { .. } => TASK_NOT_FOUND,
            A2AError::TaskNotCancelable { .. } => TASK_NOT_CANCELABLE,
            A2AError::PushNotificationNotSupported { .. } => PUSH_NOTIFICATION_NOT_SUPPORTED,
            A2AError::UnsupportedOperation { .. } => UNSUPPORTED_OPERATION,
            A2AError::ContentTypeNotSupported { .. } => CONTENT_TYPE_NOT_SUPPORTED,
            A2AError::InvalidAgentResponse { .. } => INVALID_AGENT_RESPONSE,
            A2AError::AuthenticatedExtendedCardNotConfigured { .. } => {
                AUTHENTICATED_EXTENDED_CARD_NOT_CONFIGURED
            }
            // Client/transport errors map to internal error.
            A2AError::Transport(_)
            | A2AError::Timeout(_)
            | A2AError::Http { .. }
            | A2AError::InvalidJson(_)
            | A2AError::Other(_) => INTERNAL_ERROR,
            A2AError::JsonRpc { code, .. } => *code,
        }
    }

    /// Returns the default human-readable message for this error variant
    /// (matching the Python SDK defaults).
    pub fn default_message(&self) -> &str {
        match self {
            A2AError::ParseError { .. } => "Invalid JSON payload",
            A2AError::InvalidRequest { .. } => "Request payload validation error",
            A2AError::MethodNotFound { .. } => "Method not found",
            A2AError::InvalidParams { .. } => "Invalid parameters",
            A2AError::InternalError { .. } => "Internal error",
            A2AError::TaskNotFound { .. } => "Task not found",
            A2AError::TaskNotCancelable { .. } => "Task cannot be canceled",
            A2AError::PushNotificationNotSupported { .. } => "Push Notification is not supported",
            A2AError::UnsupportedOperation { .. } => "This operation is not supported",
            A2AError::ContentTypeNotSupported { .. } => "Incompatible content types",
            A2AError::InvalidAgentResponse { .. } => "Invalid agent response",
            A2AError::AuthenticatedExtendedCardNotConfigured { .. } => {
                "Authenticated Extended Card is not configured"
            }
            A2AError::Transport(_) => "Transport error",
            A2AError::Timeout(_) => "Request timed out",
            A2AError::Http { .. } => "HTTP error",
            A2AError::InvalidJson(_) => "Invalid JSON",
            A2AError::JsonRpc { .. } => "JSON-RPC error",
            A2AError::Other(_) => "Error",
        }
    }
}

impl From<A2AError> for JsonRpcError {
    fn from(err: A2AError) -> Self {
        let code = err.code();
        let message = err.to_string();
        // Preserve structured data from protocol error variants and JsonRpc variant.
        let data = match &err {
            A2AError::ParseError { data, .. }
            | A2AError::InvalidRequest { data, .. }
            | A2AError::MethodNotFound { data, .. }
            | A2AError::InvalidParams { data, .. }
            | A2AError::InternalError { data, .. }
            | A2AError::TaskNotFound { data, .. }
            | A2AError::TaskNotCancelable { data, .. }
            | A2AError::PushNotificationNotSupported { data, .. }
            | A2AError::UnsupportedOperation { data, .. }
            | A2AError::ContentTypeNotSupported { data, .. }
            | A2AError::InvalidAgentResponse { data, .. }
            | A2AError::AuthenticatedExtendedCardNotConfigured { data, .. }
            | A2AError::JsonRpc { data, .. } => data.clone(),
            _ => None,
        };
        JsonRpcError {
            code,
            message,
            data,
        }
    }
}

impl From<serde_json::Error> for A2AError {
    fn from(err: serde_json::Error) -> Self {
        A2AError::ParseError {
            message: err.to_string(),
            data: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_match_spec() {
        assert_eq!(PARSE_ERROR, -32700);
        assert_eq!(INVALID_REQUEST, -32600);
        assert_eq!(METHOD_NOT_FOUND, -32601);
        assert_eq!(INVALID_PARAMS, -32602);
        assert_eq!(INTERNAL_ERROR, -32603);
        assert_eq!(TASK_NOT_FOUND, -32001);
        assert_eq!(TASK_NOT_CANCELABLE, -32002);
        assert_eq!(PUSH_NOTIFICATION_NOT_SUPPORTED, -32003);
        assert_eq!(UNSUPPORTED_OPERATION, -32004);
        assert_eq!(CONTENT_TYPE_NOT_SUPPORTED, -32005);
        assert_eq!(INVALID_AGENT_RESPONSE, -32006);
        assert_eq!(AUTHENTICATED_EXTENDED_CARD_NOT_CONFIGURED, -32007);
    }

    #[test]
    fn a2a_error_to_json_rpc_error() {
        let err = A2AError::TaskNotFound {
            message: "task-123".to_string(),
            data: None,
        };
        let rpc_err: JsonRpcError = err.into();
        assert_eq!(rpc_err.code, -32001);
        assert!(rpc_err.message.contains("task-123"));
        assert!(rpc_err.data.is_none());
    }

    #[test]
    fn transport_error_maps_to_internal() {
        let err = A2AError::Transport("connection refused".to_string());
        assert_eq!(err.code(), INTERNAL_ERROR);
    }

    #[test]
    fn json_rpc_error_preserves_code() {
        let err = A2AError::JsonRpc {
            code: -32001,
            message: "Task not found".to_string(),
            data: None,
        };
        assert_eq!(err.code(), -32001);
    }

    #[test]
    fn push_notification_not_supported_variant() {
        let err = A2AError::PushNotificationNotSupported {
            message: "Push Notification is not supported".to_string(),
            data: None,
        };
        assert_eq!(err.code(), PUSH_NOTIFICATION_NOT_SUPPORTED);
        assert!(format!("{}", err).contains("Push notification not supported"));
    }

    #[test]
    fn protocol_error_data_propagates_to_json_rpc() {
        let validation_data = serde_json::json!([
            {"loc": ["params", "message"], "msg": "field required", "type": "value_error.missing"}
        ]);
        let err = A2AError::InvalidParams {
            message: "Invalid parameters".to_string(),
            data: Some(validation_data.clone()),
        };
        let rpc_err: JsonRpcError = err.into();
        assert_eq!(rpc_err.code, INVALID_PARAMS);
        assert_eq!(rpc_err.data, Some(validation_data));
    }

    #[test]
    fn convenience_constructor_sets_data_none() {
        let err = A2AError::task_not_found("task-abc");
        match &err {
            A2AError::TaskNotFound { message, data } => {
                assert_eq!(message, "task-abc");
                assert!(data.is_none());
            }
            _ => panic!("wrong variant"),
        }
        // Also verify it converts correctly.
        let rpc_err: JsonRpcError = err.into();
        assert_eq!(rpc_err.code, TASK_NOT_FOUND);
        assert!(rpc_err.data.is_none());
    }
}
