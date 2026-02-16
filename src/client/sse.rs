//! Server-Sent Events (SSE) stream handling for A2A streaming responses.
//!
//! Parses SSE `data:` lines from HTTP responses and deserializes them into
//! [`StreamResponse`] events (status updates, artifact updates, task snapshots,
//! and direct messages).

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::Stream;
use tokio::sync::mpsc;

use crate::error::{A2AError, A2AResult};
use crate::types::StreamResponse;

/// A stream of A2A server-sent events.
///
/// Wraps a raw HTTP response and parses SSE `data:` lines into typed
/// [`StreamResponse`] values. Supports both pull-based (`next()`) and
/// push-based (`Stream` trait) consumption.
///
/// # Example
///
/// ```no_run
/// # async fn example(mut stream: a2a_rs::client::SseStream) {
/// while let Some(event) = stream.next().await {
///     match event {
///         Ok(response) => println!("Got event: {:?}", response),
///         Err(e) => eprintln!("Stream error: {}", e),
///     }
/// }
/// # }
/// ```
pub struct SseStream {
    receiver: mpsc::Receiver<A2AResult<StreamResponse>>,
    /// Background task handle — kept alive so the parsing task runs to completion.
    _task: tokio::task::JoinHandle<()>,
}

impl std::fmt::Debug for SseStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SseStream").finish_non_exhaustive()
    }
}

impl SseStream {
    /// Create an `SseStream` from a raw `reqwest::Response`.
    ///
    /// Spawns a background task that reads the response body as SSE lines
    /// and sends parsed events through a channel.
    pub(crate) fn from_response(response: reqwest::Response) -> Self {
        let (tx, rx) = mpsc::channel(64);

        let task = tokio::spawn(async move {
            if let Err(e) = parse_sse_stream(response, &tx).await {
                // Send the final error and then stop. Ignore send failures
                // (receiver may have been dropped).
                let _ = tx.send(Err(e)).await;
            }
        });

        Self {
            receiver: rx,
            _task: task,
        }
    }

    /// Get the next event from the stream.
    ///
    /// Returns `None` when the stream is exhausted (server closed the connection
    /// or sent a terminal event). Returns `Some(Err(...))` on parse or transport
    /// errors.
    pub async fn next(&mut self) -> Option<A2AResult<StreamResponse>> {
        self.receiver.recv().await
    }

    /// Convert this stream into a `futures::Stream`.
    ///
    /// This consumes the `SseStream` and returns an impl `Stream` that yields
    /// `A2AResult<StreamResponse>` items.
    pub fn into_stream(self) -> SseStreamAdapter {
        SseStreamAdapter {
            receiver: self.receiver,
            _task: self._task,
        }
    }
}

/// Adapter that implements `futures::Stream` for an [`SseStream`].
///
/// Created by [`SseStream::into_stream()`].
pub struct SseStreamAdapter {
    receiver: mpsc::Receiver<A2AResult<StreamResponse>>,
    _task: tokio::task::JoinHandle<()>,
}

impl Stream for SseStreamAdapter {
    type Item = A2AResult<StreamResponse>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

/// Parse an SSE response body line-by-line, sending parsed events to `tx`.
async fn parse_sse_stream(
    response: reqwest::Response,
    tx: &mpsc::Sender<A2AResult<StreamResponse>>,
) -> A2AResult<()> {
    use futures::StreamExt;

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result
            .map_err(|e| A2AError::Transport(format!("error reading SSE stream: {e}")))?;

        let text = std::str::from_utf8(&chunk)
            .map_err(|e| A2AError::Transport(format!("invalid UTF-8 in SSE stream: {e}")))?;

        buffer.push_str(text);

        // Process complete lines from the buffer.
        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim_end_matches('\r').to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if let Some(event) = parse_sse_line(&line)? {
                if tx.send(Ok(event)).await.is_err() {
                    // Receiver dropped — stop parsing.
                    return Ok(());
                }
            }
        }
    }

    // Process any remaining data in the buffer (no trailing newline).
    if !buffer.trim().is_empty() {
        if let Some(event) = parse_sse_line(buffer.trim())? {
            let _ = tx.send(Ok(event)).await;
        }
    }

    Ok(())
}

/// Parse a single SSE line. Returns `Some(event)` for `data:` lines with
/// valid JSON, `None` for comments, empty lines, and keep-alive signals.
///
/// Handles two formats:
/// 1. **Raw events** — the data is a `StreamResponse` directly (status update,
///    artifact update, task, or message).
/// 2. **JSON-RPC wrapped** — the data is a full JSON-RPC response with
///    `jsonrpc`, `id`, and `result` fields (as sent by the Python SDK).
///    In this case, the `result` field is extracted and parsed as a
///    `StreamResponse`.
fn parse_sse_line(line: &str) -> A2AResult<Option<StreamResponse>> {
    // Empty line = event boundary (we process data lines individually).
    if line.is_empty() {
        return Ok(None);
    }

    // SSE comments (lines starting with ':') are keep-alive signals.
    if line.starts_with(':') {
        return Ok(None);
    }

    // We only care about `data:` lines.
    if let Some(data) = line.strip_prefix("data:") {
        let data = data.trim();

        // Empty data field — skip.
        if data.is_empty() {
            return Ok(None);
        }

        // "[DONE]" is a common sentinel for stream completion.
        if data == "[DONE]" {
            return Ok(None);
        }

        // Parse the JSON.
        let value: serde_json::Value = serde_json::from_str(data).map_err(|e| {
            A2AError::InvalidJson(format!(
                "failed to parse SSE event data: {e} (data: {data})"
            ))
        })?;

        // Detect JSON-RPC wrapper: has "jsonrpc" field.
        let event_value = if value.get("jsonrpc").is_some() {
            // JSON-RPC wrapped response — check for error.
            if let Some(error) = value.get("error") {
                let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
                let message = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown error")
                    .to_string();
                let data = error.get("data").cloned();
                return Err(A2AError::JsonRpc {
                    code,
                    message,
                    data,
                });
            }
            // Extract the `result` field.
            value.get("result").cloned().ok_or_else(|| {
                A2AError::InvalidJson(format!(
                    "JSON-RPC SSE response has neither 'result' nor 'error': {data}"
                ))
            })?
        } else {
            // Raw event — parse directly.
            value
        };

        let event: StreamResponse = serde_json::from_value(event_value).map_err(|e| {
            A2AError::InvalidJson(format!(
                "failed to parse SSE event as StreamResponse: {e} (data: {data})"
            ))
        })?;

        return Ok(Some(event));
    }

    // Other SSE fields (event:, id:, retry:) — ignore for now.
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_line() {
        assert!(parse_sse_line("").unwrap().is_none());
    }

    #[test]
    fn test_parse_comment() {
        assert!(parse_sse_line(": keepalive").unwrap().is_none());
    }

    #[test]
    fn test_parse_done_sentinel() {
        assert!(parse_sse_line("data: [DONE]").unwrap().is_none());
    }

    #[test]
    fn test_parse_empty_data() {
        assert!(parse_sse_line("data:").unwrap().is_none());
        assert!(parse_sse_line("data:  ").unwrap().is_none());
    }

    #[test]
    fn test_parse_non_data_field() {
        assert!(parse_sse_line("event: update").unwrap().is_none());
        assert!(parse_sse_line("id: 123").unwrap().is_none());
        assert!(parse_sse_line("retry: 5000").unwrap().is_none());
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse_sse_line("data: {not valid json}");
        assert!(result.is_err());
    }
}
