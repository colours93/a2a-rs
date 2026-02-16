//! Port of Python SDK tests/client/test_client_factory.py
//!
//! Tests for client factory / construction patterns.
//!
//! Python's ClientFactory selects transports (JsonRpc, REST, gRPC) based on
//! agent card preferences. In Rust, the A2AClient only supports JSON-RPC,
//! so we test the construction patterns that exist.
//!
//! Skipped tests (Python-specific or require features not in Rust SDK):
//! - test_client_factory_selects_secondary_transport_url (REST transport)
//! - test_client_factory_server_preference (REST preference)
//! - test_client_factory_connect_with_url (requires network mock)
//! - test_client_factory_connect_with_url_and_client_config (requires network)
//! - test_client_factory_connect_with_resolver_args (requires network)
//! - test_client_factory_connect_resolver_args_without_client (requires network)
//! - test_client_factory_connect_with_extra_transports (custom transport registration)
//! - test_client_factory_connect_with_consumers_and_interceptors (no interceptors in Rust)

use a2a_rs::client::A2AClient;
use a2a_rs::types::*;

fn make_card(name: &str, url: &str, transport: &str) -> AgentCard {
    AgentCard {
        name: name.to_string(),
        description: "test".to_string(),
        version: "1.0".to_string(),
        url: url.to_string(),
        supported_interfaces: vec![AgentInterface {
            url: url.to_string(),
            transport: transport.to_string(),
            protocol_version: Some("0.3".to_string()),
            tenant: None,
        }],
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
        preferred_transport: Some(transport.to_string()),
        protocol_version: None,
        signatures: None,
        security: None,
    }
}

// ============================================================================
// Equivalent of test_client_factory_selects_preferred_transport
// ============================================================================

#[test]
fn test_client_selects_jsonrpc_transport() {
    let card = make_card("Test Agent", "http://primary-url.com", "JSONRPC");
    let client = A2AClient::from_card(card);
    assert!(client.is_ok());
}

// ============================================================================
// Equivalent of test_client_factory_no_compatible_transport
// ============================================================================

#[test]
fn test_client_no_compatible_transport_errors() {
    let card = make_card("Test Agent", "http://primary-url.com", "gRPC");
    let result = A2AClient::from_card(card);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("JSONRPC"));
}

// ============================================================================
// Equivalent of test_client_factory_connect_with_agent_card
// ============================================================================

#[test]
fn test_client_from_card_stores_card() {
    let card = make_card("Test Agent", "http://primary-url.com", "JSONRPC");
    let client = A2AClient::from_card(card).unwrap();
    let cached = client.get_card().unwrap();
    assert_eq!(cached.name, "Test Agent");
}

// ============================================================================
// Construction variants
// ============================================================================

#[test]
fn test_client_from_endpoint() {
    let client = A2AClient::from_endpoint("http://primary-url.com");
    // No card cached when using from_endpoint
    assert!(client.get_card().is_err());
}

#[test]
fn test_client_with_transport() {
    use a2a_rs::client::{JsonRpcTransport, Transport};

    let transport = JsonRpcTransport::new("http://primary-url.com");
    let client = A2AClient::with_transport(Box::new(transport));
    // No card cached when using with_transport
    assert!(client.get_card().is_err());
}

// ============================================================================
// Multiple interfaces — Rust picks first JSONRPC
// ============================================================================

#[test]
fn test_client_picks_jsonrpc_from_multiple_interfaces() {
    let card = AgentCard {
        name: "Multi-Interface Agent".to_string(),
        description: "test".to_string(),
        version: "1.0".to_string(),
        url: "http://primary.com".to_string(),
        supported_interfaces: vec![
            AgentInterface {
                url: "http://grpc.com".to_string(),
                transport: "gRPC".to_string(),
                protocol_version: Some("0.3".to_string()),
                tenant: None,
            },
            AgentInterface {
                url: "http://jsonrpc.com/a2a".to_string(),
                transport: "JSONRPC".to_string(),
                protocol_version: Some("0.3".to_string()),
                tenant: None,
            },
        ],
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
    };

    let client = A2AClient::from_card(card);
    assert!(client.is_ok());
}
