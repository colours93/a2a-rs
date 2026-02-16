//! Integration tests for SSE streaming via message/stream.
//!
//! These tests verify that the server correctly streams SSE events
//! for streaming requests.

mod common;

use common::{start_test_server, EchoAgent, SlowEchoAgent};
use std::sync::Arc;

/// Test that message/stream returns an SSE response with proper content type.
#[tokio::test]
async fn message_stream_returns_sse() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "message/stream",
        "params": {
            "message": {
                "messageId": "m1",
                "role": "user",
                "parts": [{"kind": "text", "text": "Stream this"}]
            }
        }
    });

    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    // Should be an SSE response (text/event-stream)
    let content_type = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        content_type.contains("text/event-stream"),
        "Expected text/event-stream, got: {}",
        content_type
    );

    // Read the full body as text and verify it contains SSE events
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("event:") || body.contains("data:"),
        "Expected SSE events in body: {}",
        body
    );
}

/// Test that SSE stream includes statusUpdate and done events.
#[tokio::test]
async fn message_stream_contains_status_events() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "message/stream",
        "params": {
            "message": {
                "messageId": "m1",
                "role": "user",
                "parts": [{"kind": "text", "text": "Hello streaming"}]
            }
        }
    });

    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    let body = resp.text().await.unwrap();

    // Should contain at least a working status and a completed (final) status
    assert!(
        body.contains("statusUpdate"),
        "Expected statusUpdate event in SSE stream: {}",
        body
    );
    assert!(
        body.contains("done"),
        "Expected done event in SSE stream: {}",
        body
    );
}

/// Test that SSE stream from slow echo agent contains artifactUpdate events.
#[tokio::test]
async fn message_stream_with_artifacts() {
    let (base_url, _handle) = start_test_server(Arc::new(SlowEchoAgent)).await;
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "message/stream",
        "params": {
            "message": {
                "messageId": "m1",
                "role": "user",
                "parts": [{"kind": "text", "text": "Stream with artifacts"}]
            }
        }
    });

    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    let body = resp.text().await.unwrap();

    // Should contain artifactUpdate events
    assert!(
        body.contains("artifactUpdate"),
        "Expected artifactUpdate event in SSE stream: {}",
        body
    );
}

/// Parse SSE events from the raw text body into structured data.
fn parse_sse_events(body: &str) -> Vec<(String, String)> {
    let mut events = Vec::new();
    let mut current_event = String::new();
    let mut current_data = String::new();

    for line in body.lines() {
        if line.starts_with("event:") {
            current_event = line.trim_start_matches("event:").trim().to_string();
        } else if line.starts_with("data:") {
            current_data = line.trim_start_matches("data:").trim().to_string();
        } else if line.is_empty() && !current_event.is_empty() {
            events.push((current_event.clone(), current_data.clone()));
            current_event.clear();
            current_data.clear();
        }
    }
    // Capture last event if no trailing newline
    if !current_event.is_empty() {
        events.push((current_event, current_data));
    }

    events
}

/// Test that SSE events are properly formatted JSON in the data field.
#[tokio::test]
async fn sse_events_have_valid_json_data() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "message/stream",
        "params": {
            "message": {
                "messageId": "m1",
                "role": "user",
                "parts": [{"kind": "text", "text": "Test JSON parsing"}]
            }
        }
    });

    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    let raw_body = resp.text().await.unwrap();
    let events = parse_sse_events(&raw_body);

    assert!(!events.is_empty(), "Expected at least one SSE event");

    for (event_type, data) in &events {
        if event_type == "done" {
            // Done event has empty data
            continue;
        }
        // All other events should have valid JSON data
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(data);
        assert!(
            parsed.is_ok(),
            "Event '{}' has invalid JSON data: {}. Error: {}",
            event_type,
            data,
            parsed.unwrap_err()
        );

        let json = parsed.unwrap();

        // Events are wrapped in a JSON-RPC response envelope.
        // Verify envelope structure first.
        assert_eq!(json["jsonrpc"], "2.0", "Expected JSON-RPC 2.0 envelope");
        assert!(
            json["result"].is_object(),
            "Expected 'result' field in envelope"
        );

        let result = &json["result"];

        // Python SDK: StreamResponse uses flat kind discrimination.
        // Status update events have "kind": "status-update" at top level.
        if event_type == "statusUpdate" {
            assert_eq!(
                result["kind"], "status-update",
                "Expected kind: status-update"
            );
            assert!(result["taskId"].is_string());
            assert!(result["contextId"].is_string());
            assert!(result["status"]["state"].is_string());
        }
    }
}

/// Test that the final SSE event before 'done' has final=true.
#[tokio::test]
async fn sse_final_event_has_final_true() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "message/stream",
        "params": {
            "message": {
                "messageId": "m1",
                "role": "user",
                "parts": [{"kind": "text", "text": "Check final flag"}]
            }
        }
    });

    let resp = client
        .post(format!("{}/a2a", base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    let raw_body = resp.text().await.unwrap();
    let events = parse_sse_events(&raw_body);

    // Find the last statusUpdate before 'done'
    let status_updates: Vec<_> = events.iter().filter(|(t, _)| t == "statusUpdate").collect();

    assert!(
        !status_updates.is_empty(),
        "Expected at least one statusUpdate event"
    );

    let last_update = status_updates.last().unwrap();
    let json: serde_json::Value = serde_json::from_str(&last_update.1).unwrap();

    // Events are wrapped in a JSON-RPC response envelope.
    // Python SDK: flat kind-based format, so result IS the status update directly.
    let result = &json["result"];
    assert_eq!(
        result["final"], true,
        "Last statusUpdate event should have final=true"
    );
}
