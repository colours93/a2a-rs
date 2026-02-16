//! Transport layer for A2A client communication.
//!
//! Provides the `Transport` trait for abstracting over different communication
//! protocols, and `JsonRpcTransport` for the standard JSON-RPC over HTTP binding.

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::error::{A2AError, A2AResult};
use crate::types::{JsonRpcRequest, JsonRpcResponse};

use super::sse::SseStream;

/// Transport abstraction for A2A communication.
///
/// Implementations handle the low-level details of sending JSON-RPC requests
/// and receiving responses (or SSE streams) over a particular protocol binding.
///
/// Python SDK ref: `ClientTransport` (abstract base class in `transports/base.py`)
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send a JSON-RPC request and receive a JSON-RPC response.
    async fn send(&self, request: &JsonRpcRequest) -> A2AResult<JsonRpcResponse>;

    /// Send a JSON-RPC request and receive an SSE event stream.
    ///
    /// Used for streaming methods like `message/stream` and `tasks/subscribe`.
    async fn send_stream(&self, request: &JsonRpcRequest) -> A2AResult<SseStream>;

    /// Close the transport and release any held resources.
    ///
    /// Python SDK ref: `ClientTransport.close()`, `JsonRpcTransport.close()`
    ///
    /// The default implementation is a no-op. Override if your transport holds
    /// resources (e.g., persistent connections) that need explicit cleanup.
    async fn close(&self) -> A2AResult<()> {
        Ok(())
    }
}

/// Configuration for [`JsonRpcTransport`].
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Request timeout. Defaults to 60 seconds.
    pub timeout: Duration,
    /// Additional HTTP headers to include on every request.
    pub headers: HashMap<String, String>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(60),
            headers: HashMap::new(),
        }
    }
}

/// JSON-RPC over HTTP transport using `reqwest`.
///
/// This is the standard transport for the A2A JSON-RPC protocol binding.
/// It sends POST requests with `Content-Type: application/json` and parses
/// the response as a JSON-RPC result or error.
///
/// For streaming methods, the response is interpreted as an SSE event stream.
///
/// # Example
///
/// ```no_run
/// use a2a_rs::client::JsonRpcTransport;
///
/// let transport = JsonRpcTransport::new("http://localhost:7420/a2a");
/// ```
#[derive(Debug, Clone)]
pub struct JsonRpcTransport {
    client: reqwest::Client,
    url: String,
}

impl JsonRpcTransport {
    /// Create a new transport targeting the given A2A endpoint URL.
    ///
    /// Uses default configuration (60s timeout, no extra headers).
    pub fn new(url: impl Into<String>) -> Self {
        Self::with_config(url, TransportConfig::default())
    }

    /// Create a new transport with custom configuration.
    pub fn with_config(url: impl Into<String>, config: TransportConfig) -> Self {
        let mut default_headers = HeaderMap::new();
        for (key, value) in &config.headers {
            if let (Ok(name), Ok(val)) = (
                HeaderName::from_bytes(key.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                default_headers.insert(name, val);
            }
        }

        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .default_headers(default_headers)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            url: url.into(),
        }
    }

    /// Create a new transport with an existing `reqwest::Client`.
    ///
    /// Useful when you want to share a connection pool or configure TLS
    /// settings externally.
    pub fn with_client(url: impl Into<String>, client: reqwest::Client) -> Self {
        Self {
            client,
            url: url.into(),
        }
    }

    /// Returns the URL this transport sends requests to.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Create a transport with a custom timeout (builder-style).
    pub fn with_timeout(self, timeout: Duration) -> Self {
        let mut config = TransportConfig::default();
        config.timeout = timeout;
        Self::with_config(self.url, config)
    }

    /// Add a custom header (builder-style).
    pub fn with_header(self, key: &str, value: &str) -> Self {
        // Rebuild the client with the new header
        let mut config = TransportConfig::default();
        config.headers.insert(key.to_string(), value.to_string());
        Self::with_config(self.url, config)
    }
}

#[async_trait]
impl Transport for JsonRpcTransport {
    async fn send(&self, request: &JsonRpcRequest) -> A2AResult<JsonRpcResponse> {
        let body = serde_json::to_vec(request).map_err(|e| {
            A2AError::Transport(format!("failed to serialize JSON-RPC request: {e}"))
        })?;

        let response = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    A2AError::Timeout(format!("request timed out: {e}"))
                } else if e.is_connect() {
                    A2AError::Transport(format!("connection failed: {e}"))
                } else {
                    A2AError::Transport(format!("HTTP request failed: {e}"))
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(A2AError::Http {
                status: status.as_u16(),
                body: body_text,
            });
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| A2AError::Transport(format!("failed to read response body: {e}")))?;

        let rpc_response: JsonRpcResponse = serde_json::from_slice(&bytes).map_err(|e| {
            A2AError::InvalidJson(format!("failed to parse JSON-RPC response: {e}"))
        })?;

        Ok(rpc_response)
    }

    async fn send_stream(&self, request: &JsonRpcRequest) -> A2AResult<SseStream> {
        let body = serde_json::to_vec(request).map_err(|e| {
            A2AError::Transport(format!("failed to serialize JSON-RPC request: {e}"))
        })?;

        let response = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .body(body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    A2AError::Timeout(format!("stream request timed out: {e}"))
                } else if e.is_connect() {
                    A2AError::Transport(format!("stream connection failed: {e}"))
                } else {
                    A2AError::Transport(format!("stream HTTP request failed: {e}"))
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(A2AError::Http {
                status: status.as_u16(),
                body: body_text,
            });
        }

        Ok(SseStream::from_response(response))
    }
}
