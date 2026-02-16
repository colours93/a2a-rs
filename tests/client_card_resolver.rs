//! Port of Python SDK tests/client/test_card_resolver.py
//!
//! Tests for the CardResolver agent card discovery mechanism.
//!
//! Many Python tests use httpx mocking (respx, MagicMock). Those that require
//! a live HTTP server are skipped here with comments. Tests that can be done
//! purely with construction logic and URL resolution are included.
//!
//! Skipped (need HTTP mock server like wiremock-rs):
//! - test_get_agent_card_success_* (HTTP GET mocks)
//! - test_get_agent_card_http_status_error (HTTP error mocks)
//! - test_get_agent_card_json_decode_error (JSON parse mocks)
//! - test_get_agent_card_request_error (network error mocks)
//! - test_get_agent_card_validation_error (validation mocks)
//! - test_get_agent_card_logs_success (logging capture)
//! - test_get_agent_card_with_signature_verifier
//! - test_get_agent_card_returns_agent_card_instance
//! - test_get_agent_card_different_status_codes (parametrized HTTP errors)

use a2a_rs::client::CardResolver;
use a2a_rs::types::*;

fn make_card(name: &str, interfaces: Vec<AgentInterface>) -> AgentCard {
    AgentCard {
        name: name.to_string(),
        description: "test".to_string(),
        version: "1.0".to_string(),
        url: "http://example.com".to_string(),
        supported_interfaces: interfaces,
        capabilities: AgentCapabilities::default(),
        default_input_modes: vec![],
        default_output_modes: vec![],
        skills: vec![],
        provider: None,
        documentation_url: None,
        security_schemes: None,
        security_requirements: vec![],
        supports_authenticated_extended_card: None,
        icon_url: None,
        additional_interfaces: None,
        preferred_transport: None,
        protocol_version: None,
        signatures: None,
        security: None,
    }
}

fn jsonrpc_interface(url: &str) -> AgentInterface {
    AgentInterface {
        url: url.to_string(),
        transport: "JSONRPC".to_string(),
        protocol_version: Some("0.3".to_string()),
        tenant: None,
    }
}

// ============================================================================
// Construction tests
// ============================================================================

#[test]
fn test_card_resolver_default_construction() {
    let resolver = CardResolver::new();
    let _ = format!("{:?}", resolver);
}

#[test]
fn test_card_resolver_with_custom_path() {
    let resolver = CardResolver::new().with_card_path("/custom/agent/card");
    let _ = format!("{:?}", resolver);
}

#[test]
fn test_card_resolver_with_reqwest_client() {
    let client = reqwest::Client::new();
    let resolver = CardResolver::with_client(client);
    let _ = format!("{:?}", resolver);
}

// ============================================================================
// get_a2a_url — URL extraction from AgentCard
// ============================================================================

#[test]
fn test_get_a2a_url_with_jsonrpc_interface() {
    let card = make_card(
        "TestAgent",
        vec![jsonrpc_interface("http://example.com/a2a")],
    );
    assert_eq!(
        CardResolver::get_a2a_url(&card),
        Some("http://example.com/a2a".to_string())
    );
}

#[test]
fn test_get_a2a_url_case_insensitive() {
    let card = make_card(
        "TestAgent",
        vec![AgentInterface {
            url: "http://example.com/rpc".to_string(),
            transport: "jsonrpc".to_string(),
            protocol_version: Some("0.3".to_string()),
            tenant: None,
        }],
    );
    assert_eq!(
        CardResolver::get_a2a_url(&card),
        Some("http://example.com/rpc".to_string())
    );
}

#[test]
fn test_get_a2a_url_no_jsonrpc_interface() {
    let card = make_card(
        "TestAgent",
        vec![AgentInterface {
            url: "http://example.com/grpc".to_string(),
            transport: "gRPC".to_string(),
            protocol_version: Some("0.3".to_string()),
            tenant: None,
        }],
    );
    assert!(CardResolver::get_a2a_url(&card).is_none());
}

#[test]
fn test_get_a2a_url_empty_interfaces() {
    let card = make_card("TestAgent", vec![]);
    assert!(CardResolver::get_a2a_url(&card).is_none());
}

#[test]
fn test_get_a2a_url_multiple_interfaces_picks_first_jsonrpc() {
    let card = make_card(
        "TestAgent",
        vec![
            AgentInterface {
                url: "http://example.com/grpc".to_string(),
                transport: "gRPC".to_string(),
                protocol_version: Some("0.3".to_string()),
                tenant: None,
            },
            jsonrpc_interface("http://example.com/rpc"),
            jsonrpc_interface("http://example.com/rpc2"),
        ],
    );
    assert_eq!(
        CardResolver::get_a2a_url(&card),
        Some("http://example.com/rpc".to_string())
    );
}

// ============================================================================
// Agent card JSON deserialization (mirrors Python validation tests)
// ============================================================================

#[test]
fn test_valid_agent_card_deserialization() {
    let json = serde_json::json!({
        "name": "TestAgent",
        "description": "A test agent",
        "version": "1.0.0",
        "url": "https://example.com/a2a",
        "supportedInterfaces": [{
            "url": "https://example.com/a2a",
            "transport": "JSONRPC",
            "protocolVersion": "0.3"
        }],
        "capabilities": {},
        "defaultInputModes": ["text/plain"],
        "defaultOutputModes": ["text/plain"],
        "skills": [{
            "id": "test-skill",
            "name": "Test Skill",
            "description": "A skill for testing",
            "tags": ["test"]
        }]
    });

    let card: AgentCard = serde_json::from_value(json).unwrap();
    assert_eq!(card.name, "TestAgent");
    assert_eq!(card.description, "A test agent");
    assert_eq!(card.version, "1.0.0");
    assert_eq!(card.skills.len(), 1);
    assert_eq!(card.skills[0].id, "test-skill");
}

#[test]
fn test_invalid_agent_card_deserialization_fails() {
    let json = serde_json::json!({
        "invalid_field": "value",
        "name": "Test Agent"
    });
    let result: Result<AgentCard, _> = serde_json::from_value(json);
    assert!(result.is_err());
}

#[test]
fn test_agent_card_with_multiple_skills() {
    let json = serde_json::json!({
        "name": "Hello World Agent",
        "description": "Just a hello world agent",
        "version": "1.0.0",
        "url": "http://localhost:9999/",
        "supportedInterfaces": [{
            "url": "http://localhost:9999/",
            "transport": "JSONRPC",
            "protocolVersion": "0.3"
        }],
        "capabilities": {},
        "defaultInputModes": ["text"],
        "defaultOutputModes": ["text"],
        "skills": [
            {
                "id": "hello_world",
                "name": "Returns hello world",
                "description": "just returns hello world",
                "tags": ["hello world"],
                "examples": ["hi", "hello world"]
            },
            {
                "id": "extended_skill",
                "name": "Super Greet",
                "description": "A more enthusiastic greeting.",
                "tags": ["extended"],
                "examples": ["super hi"]
            }
        ]
    });

    let card: AgentCard = serde_json::from_value(json).unwrap();
    assert_eq!(card.skills.len(), 2);
    assert_eq!(card.skills[0].id, "hello_world");
    assert_eq!(card.skills[1].id, "extended_skill");
}

// ============================================================================
// A2AClient::from_card — tests that card→client construction works
// ============================================================================

#[test]
fn test_client_from_card_with_jsonrpc() {
    let card = make_card(
        "TestAgent",
        vec![jsonrpc_interface("http://example.com/a2a")],
    );
    let client = a2a_rs::client::A2AClient::from_card(card);
    assert!(client.is_ok());
}

#[test]
fn test_client_from_card_without_jsonrpc_fails() {
    let card = make_card(
        "TestAgent",
        vec![AgentInterface {
            url: "http://example.com/grpc".to_string(),
            transport: "gRPC".to_string(),
            protocol_version: Some("0.3".to_string()),
            tenant: None,
        }],
    );
    let client = a2a_rs::client::A2AClient::from_card(card);
    assert!(client.is_err());
}

#[test]
fn test_client_from_endpoint_skips_card_resolution() {
    let client = a2a_rs::client::A2AClient::from_endpoint("http://example.com/a2a");
    assert!(client.get_card().is_err());
}

#[test]
fn test_client_from_card_caches_card() {
    let card = make_card(
        "CachedBot",
        vec![jsonrpc_interface("http://example.com/a2a")],
    );
    let client = a2a_rs::client::A2AClient::from_card(card).unwrap();
    let cached = client.get_card().unwrap();
    assert_eq!(cached.name, "CachedBot");
}
