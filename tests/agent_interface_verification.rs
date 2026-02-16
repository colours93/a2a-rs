//! Verification test for AgentInterface "transport" field compliance.
//!
//! This test explicitly verifies that AgentInterface uses "transport" field
//! (as per Python SDK) instead of "protocolBinding" (old proto field name).

use a2a_rs::types::AgentInterface;

#[test]
fn agent_interface_uses_transport_field() {
    println!("\n=== AgentInterface Serialization Verification ===\n");

    // Create AgentInterface instance with all fields
    let interface = AgentInterface {
        url: "http://localhost:7420/a2a".to_string(),
        transport: "JSONRPC".to_string(),
        tenant: Some("acme-corp".to_string()),
        protocol_version: Some("0.3".to_string()),
    };

    // Serialize to JSON
    let json = serde_json::to_value(&interface).unwrap();

    println!("AgentInterface JSON output:");
    println!("{}\n", serde_json::to_string_pretty(&json).unwrap());

    // Verify field names
    assert_eq!(
        json["url"], "http://localhost:7420/a2a",
        "url field must be present"
    );
    assert_eq!(
        json["transport"], "JSONRPC",
        "transport field must be present"
    );
    assert_eq!(
        json["tenant"], "acme-corp",
        "tenant field must be present when Some"
    );
    assert_eq!(
        json["protocolVersion"], "0.3",
        "protocolVersion field must be present when Some"
    );

    // CRITICAL: Verify "transport" exists and "protocolBinding" does NOT exist
    assert!(
        json.get("transport").is_some(),
        "MUST have 'transport' field (Python SDK format)"
    );
    assert!(
        json.get("protocolBinding").is_none(),
        "MUST NOT have 'protocolBinding' field (old proto name)"
    );

    println!("✓ VERIFIED: AgentInterface uses 'transport' field (not 'protocolBinding')");
    println!("✓ Field naming matches Python SDK specification\n");

    // Test 2: Minimal interface (optional fields omitted)
    let minimal = AgentInterface {
        url: "https://example.com/api/a2a".to_string(),
        transport: "HTTP+JSON".to_string(),
        tenant: None,
        protocol_version: None,
    };

    let json_minimal = serde_json::to_value(&minimal).unwrap();

    println!("Minimal AgentInterface JSON (optional fields omitted):");
    println!("{}\n", serde_json::to_string_pretty(&json_minimal).unwrap());

    assert_eq!(json_minimal["url"], "https://example.com/api/a2a");
    assert_eq!(json_minimal["transport"], "HTTP+JSON");
    assert!(
        json_minimal.get("tenant").is_none(),
        "Optional tenant should be omitted"
    );
    assert!(
        json_minimal.get("protocolVersion").is_none(),
        "Optional protocolVersion should be omitted"
    );

    println!("✓ VERIFIED: Optional fields correctly omitted when None\n");

    // Test 3: Deserialization roundtrip
    let json_input = serde_json::json!({
        "url": "http://agent.example.com",
        "transport": "GRPC",
        "protocolVersion": "0.3"
    });

    let deserialized: AgentInterface = serde_json::from_value(json_input.clone()).unwrap();

    println!("Deserialization test:");
    println!(
        "Input JSON: {}",
        serde_json::to_string(&json_input).unwrap()
    );
    println!(
        "Deserialized: url={}, transport={}, protocolVersion={:?}\n",
        deserialized.url, deserialized.transport, deserialized.protocol_version
    );

    assert_eq!(deserialized.url, "http://agent.example.com");
    assert_eq!(deserialized.transport, "GRPC");
    assert_eq!(deserialized.protocol_version, Some("0.3".to_string()));

    println!("✓ VERIFIED: Deserialization roundtrip successful\n");

    // Test 4: Verify old "protocolBinding" field name is NOT recognized
    let json_with_old_field = serde_json::json!({
        "url": "http://example.com",
        "protocolBinding": "JSONRPC",  // OLD field name (should be ignored)
        "transport": "HTTP"
    });

    let result: Result<AgentInterface, _> = serde_json::from_value(json_with_old_field);
    if let Ok(parsed) = result {
        // If it parses, it should use "transport", not "protocolBinding"
        assert_eq!(
            parsed.transport, "HTTP",
            "Should read 'transport' field, not 'protocolBinding'"
        );
        println!("✓ VERIFIED: 'protocolBinding' field is ignored (deprecated field name)\n");
    }

    println!("=== All AgentInterface verification tests PASSED ===\n");
}

#[test]
fn agent_interface_in_agent_card_context() {
    println!("\n=== AgentInterface in AgentCard Context ===\n");

    use a2a_rs::types::{AgentCapabilities, AgentCard, AgentSkill};

    let card = AgentCard {
        name: "Test Agent".to_string(),
        description: "A test agent".to_string(),
        version: "1.0.0".to_string(),
        url: "http://localhost:8080/a2a".to_string(),
        supported_interfaces: vec![
            AgentInterface {
                url: "http://localhost:8080/a2a".to_string(),
                transport: "JSONRPC".to_string(),
                tenant: None,
                protocol_version: Some("0.3".to_string()),
            },
            AgentInterface {
                url: "http://localhost:8080/grpc".to_string(),
                transport: "GRPC".to_string(),
                tenant: Some("enterprise".to_string()),
                protocol_version: Some("0.3".to_string()),
            },
        ],
        provider: None,
        documentation_url: None,
        capabilities: AgentCapabilities {
            streaming: Some(true),
            push_notifications: Some(false),
            extensions: None,
            state_transition_history: None,
        },
        security_schemes: None,
        security_requirements: vec![],
        default_input_modes: vec!["text/plain".to_string()],
        default_output_modes: vec!["text/plain".to_string()],
        skills: vec![AgentSkill {
            id: "general".to_string(),
            name: "General".to_string(),
            description: "General purpose".to_string(),
            tags: vec![],
            examples: None,
            input_modes: None,
            output_modes: None,
            security_requirements: None,
            security: None,
        }],
        signatures: None,
        icon_url: None,
        additional_interfaces: None,
        preferred_transport: None,
        protocol_version: Some("0.3".to_string()),
        supports_authenticated_extended_card: None,
        security: None,
    };

    let json = serde_json::to_value(&card).unwrap();

    println!("AgentCard with multiple supportedInterfaces:");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&json["supportedInterfaces"]).unwrap()
    );

    // Verify all interfaces use "transport" field
    let interfaces = json["supportedInterfaces"].as_array().unwrap();
    assert_eq!(interfaces.len(), 2);

    for (i, interface_json) in interfaces.iter().enumerate() {
        assert!(
            interface_json.get("transport").is_some(),
            "Interface {} must have 'transport' field",
            i
        );
        assert!(
            interface_json.get("protocolBinding").is_none(),
            "Interface {} must NOT have 'protocolBinding' field",
            i
        );
    }

    println!("✓ VERIFIED: All interfaces in supportedInterfaces array use 'transport' field");
    println!("✓ Interface 0: transport={}", interfaces[0]["transport"]);
    println!("✓ Interface 1: transport={}\n", interfaces[1]["transport"]);

    println!("=== AgentCard verification PASSED ===\n");
}
