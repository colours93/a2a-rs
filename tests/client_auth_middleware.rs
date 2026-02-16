//! Port of Python SDK tests/client/test_auth_middleware.py
//!
//! Tests for authentication middleware / interceptor behavior.
//!
//! Python has AuthInterceptor, InMemoryContextCredentialStore, and various
//! security schemes (APIKey, OAuth2, OIDC, Bearer). The Rust SDK doesn't
//! have an interceptor/middleware system yet — these tests verify that the
//! types used in auth (SecurityScheme, AgentCard.security) serialize correctly.
//!
//! Skipped tests (require interceptor system not in Rust SDK):
//! - test_auth_interceptor_skips_when_no_agent_card
//! - test_in_memory_context_credential_store
//! - test_client_with_simple_interceptor
//! - test_auth_interceptor_variants (all 4 scheme types)
//! - test_auth_interceptor_skips_when_scheme_not_in_security_schemes

use a2a_rs::types::*;

// ============================================================================
// SecurityScheme serialization (used in auth middleware)
// ============================================================================

#[test]
fn test_api_key_security_scheme() {
    let scheme = SecurityScheme::ApiKey {
        description: None,
        location: ApiKeyLocation::Header,
        name: "X-API-Key".to_string(),
    };

    let json = serde_json::to_value(&scheme).unwrap();
    assert_eq!(json["type"], "apiKey");
    assert_eq!(json["name"], "X-API-Key");
    assert_eq!(json["in"], "header");
}

#[test]
fn test_http_bearer_security_scheme() {
    let scheme = SecurityScheme::Http {
        description: None,
        scheme: "bearer".to_string(),
        bearer_format: None,
    };

    let json = serde_json::to_value(&scheme).unwrap();
    assert_eq!(json["type"], "http");
    assert_eq!(json["scheme"], "bearer");
}

#[test]
fn test_oauth2_security_scheme() {
    let scheme = SecurityScheme::OAuth2 {
        description: None,
        flows: OAuthFlows {
            authorization_code: Some(AuthorizationCodeOAuthFlow {
                authorization_url: "http://provider.com/auth".to_string(),
                token_url: "http://provider.com/token".to_string(),
                scopes: [("read".to_string(), "Read scope".to_string())].into(),
                refresh_url: None,
            }),
            implicit: None,
            client_credentials: None,
            password: None,
        },
        oauth2_metadata_url: None,
    };

    let json = serde_json::to_value(&scheme).unwrap();
    assert_eq!(json["type"], "oauth2");
    assert!(json["flows"]["authorizationCode"]["authorizationUrl"].is_string());
    assert!(json["flows"]["authorizationCode"]["tokenUrl"].is_string());
}

#[test]
fn test_openid_connect_security_scheme() {
    let scheme = SecurityScheme::OpenIdConnect {
        description: None,
        open_id_connect_url: "http://provider.com/.well-known/openid-configuration".to_string(),
    };

    let json = serde_json::to_value(&scheme).unwrap();
    assert_eq!(json["type"], "openIdConnect");
    assert!(json["openIdConnectUrl"]
        .as_str()
        .unwrap()
        .contains("openid-configuration"));
}

// ============================================================================
// Agent card with security schemes
// ============================================================================

#[test]
fn test_agent_card_with_security_schemes() {
    let json = serde_json::json!({
        "name": "Secured Agent",
        "description": "An agent with auth",
        "version": "1.0",
        "url": "http://agent.com/rpc",
        "supportedInterfaces": [{
            "url": "http://agent.com/rpc",
            "transport": "JSONRPC",
            "protocolVersion": "0.3"
        }],
        "capabilities": {},
        "defaultInputModes": [],
        "defaultOutputModes": [],
        "skills": [],
        "securitySchemes": {
            "apikey": {
                "type": "apiKey",
                "name": "X-API-Key",
                "in": "header"
            },
            "bearer": {
                "type": "http",
                "scheme": "bearer"
            }
        },
        "security": [{"apikey": []}]
    });

    let card: AgentCard = serde_json::from_value(json).unwrap();
    assert_eq!(card.name, "Secured Agent");
    assert!(card.security_schemes.is_some());
    let schemes = card.security_schemes.unwrap();
    assert_eq!(schemes.len(), 2);
    assert!(card.security.is_some());
    assert_eq!(card.security.unwrap().len(), 1);
}

// ============================================================================
// Security scheme roundtrip
// ============================================================================

#[test]
fn test_api_key_scheme_roundtrip() {
    let scheme = SecurityScheme::ApiKey {
        description: Some("API key for auth".to_string()),
        location: ApiKeyLocation::Header,
        name: "X-API-Key".to_string(),
    };

    let json = serde_json::to_value(&scheme).unwrap();
    let decoded: SecurityScheme = serde_json::from_value(json).unwrap();

    match decoded {
        SecurityScheme::ApiKey { name, location, .. } => {
            assert_eq!(name, "X-API-Key");
            assert_eq!(location, ApiKeyLocation::Header);
        }
        _ => panic!("expected ApiKey"),
    }
}

#[test]
fn test_http_scheme_roundtrip() {
    let scheme = SecurityScheme::Http {
        description: None,
        scheme: "bearer".to_string(),
        bearer_format: Some("JWT".to_string()),
    };

    let json = serde_json::to_value(&scheme).unwrap();
    let decoded: SecurityScheme = serde_json::from_value(json).unwrap();

    match decoded {
        SecurityScheme::Http {
            scheme,
            bearer_format,
            ..
        } => {
            assert_eq!(scheme, "bearer");
            assert_eq!(bearer_format.as_deref(), Some("JWT"));
        }
        _ => panic!("expected Http"),
    }
}

// ============================================================================
// ApiKeyLocation variants
// ============================================================================

#[test]
fn test_api_key_location_variants() {
    let cases = vec![
        (ApiKeyLocation::Header, "header"),
        (ApiKeyLocation::Query, "query"),
        (ApiKeyLocation::Cookie, "cookie"),
    ];

    for (location, expected) in cases {
        let json = serde_json::to_value(&location).unwrap();
        assert_eq!(json.as_str().unwrap(), expected);

        let decoded: ApiKeyLocation = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, location);
    }
}
