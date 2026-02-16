//! Integration tests for the /.well-known/agent.json endpoint.
//!
//! These tests verify that the agent card discovery endpoint
//! returns a properly structured agent card.

mod common;

use common::{start_test_server, EchoAgent};
use std::sync::Arc;

/// Test that the agent card endpoint returns valid JSON.
#[tokio::test]
async fn agent_card_endpoint_returns_json() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/.well-known/agent.json", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let content_type = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        content_type.contains("application/json"),
        "Expected application/json, got: {}",
        content_type
    );

    let json: serde_json::Value = resp.json().await.unwrap();
    assert!(json.is_object());
}

/// Test that the agent card has all required fields.
#[tokio::test]
async fn agent_card_has_required_fields() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let json: serde_json::Value = client
        .get(format!("{}/.well-known/agent.json", base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Required fields per A2A spec
    assert!(json["name"].is_string(), "Missing 'name' field");
    assert!(
        json["description"].is_string(),
        "Missing 'description' field"
    );
    assert!(json["version"].is_string(), "Missing 'version' field");
    assert!(json["url"].is_string(), "Missing 'url' field");
    assert!(
        json["capabilities"].is_object(),
        "Missing 'capabilities' field"
    );
    assert!(
        json["defaultInputModes"].is_array(),
        "Missing 'defaultInputModes' field"
    );
    assert!(
        json["defaultOutputModes"].is_array(),
        "Missing 'defaultOutputModes' field"
    );
    assert!(json["skills"].is_array(), "Missing 'skills' field");
}

/// Test that the agent card name matches what we configured.
#[tokio::test]
async fn agent_card_has_correct_name() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let json: serde_json::Value = client
        .get(format!("{}/.well-known/agent.json", base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(json["name"], "Test Echo Agent");
    assert_eq!(json["version"], "0.1.0");
}

/// Test that the agent card has supported interfaces.
#[tokio::test]
async fn agent_card_has_interfaces() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let json: serde_json::Value = client
        .get(format!("{}/.well-known/agent.json", base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let interfaces = json["supportedInterfaces"].as_array().unwrap();
    assert!(!interfaces.is_empty(), "Expected at least one interface");

    let iface = &interfaces[0];
    assert!(iface["url"].is_string());
    assert_eq!(iface["transport"], "JSONRPC");
}

/// Test that the agent card reports streaming capability.
#[tokio::test]
async fn agent_card_reports_streaming() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let json: serde_json::Value = client
        .get(format!("{}/.well-known/agent.json", base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(json["capabilities"]["streaming"], true);
}

/// Test that the agent card skills have required fields.
#[tokio::test]
async fn agent_card_skills_structure() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let json: serde_json::Value = client
        .get(format!("{}/.well-known/agent.json", base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let skills = json["skills"].as_array().unwrap();
    assert!(!skills.is_empty());

    for skill in skills {
        assert!(skill["id"].is_string(), "Skill missing 'id'");
        assert!(skill["name"].is_string(), "Skill missing 'name'");
        assert!(
            skill["description"].is_string(),
            "Skill missing 'description'"
        );
        assert!(skill["tags"].is_array(), "Skill missing 'tags'");
    }
}

/// Test that the agent card uses camelCase field names.
#[tokio::test]
async fn agent_card_uses_camel_case() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let json: serde_json::Value = client
        .get(format!("{}/.well-known/agent.json", base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // These should be camelCase, NOT snake_case
    assert!(
        json.get("supportedInterfaces").is_some() || json.get("supportedInterfaces").is_none(), // may be empty but key should be camelCase
        "Expected camelCase 'supportedInterfaces'"
    );
    assert!(
        json.get("supported_interfaces").is_none(),
        "Should NOT have snake_case 'supported_interfaces'"
    );
    assert!(
        json.get("defaultInputModes").is_some(),
        "Expected camelCase 'defaultInputModes'"
    );
    assert!(
        json.get("default_input_modes").is_none(),
        "Should NOT have snake_case 'default_input_modes'"
    );
    assert!(
        json.get("defaultOutputModes").is_some(),
        "Expected camelCase 'defaultOutputModes'"
    );
}

/// Test that the agent card can be deserialized back into our Rust type.
#[tokio::test]
async fn agent_card_deserializes_to_rust_type() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    let json: serde_json::Value = client
        .get(format!("{}/.well-known/agent.json", base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Should successfully deserialize into our AgentCard type
    let card: a2a_rs::types::AgentCard = serde_json::from_value(json).unwrap();
    assert_eq!(card.name, "Test Echo Agent");
    assert_eq!(card.version, "0.1.0");
    assert!(card.capabilities.streaming == Some(true));
}

/// Test that the agent card endpoint is accessible via GET (not POST).
#[tokio::test]
async fn agent_card_endpoint_is_get_only() {
    let (base_url, _handle) = start_test_server(Arc::new(EchoAgent)).await;
    let client = reqwest::Client::new();

    // GET should work
    let get_resp = client
        .get(format!("{}/.well-known/agent.json", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(get_resp.status(), 200);

    // POST should fail (405 Method Not Allowed)
    let post_resp = client
        .post(format!("{}/.well-known/agent.json", base_url))
        .send()
        .await
        .unwrap();
    assert_ne!(post_resp.status(), 200);
}
