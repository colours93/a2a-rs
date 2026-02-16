//! Task #15: Verify PushNotificationAuthenticationInfo uses "schemes" (plural)
//!
//! This test serializes a PushNotificationAuthenticationInfo instance and
//! verifies the JSON output has "schemes" as an array field (not singular "scheme").

use a2a_rs::types::PushNotificationAuthenticationInfo;

#[test]
fn verify_schemes_plural_in_json() {
    println!("\n=== Task #15: PushNotificationAuthenticationInfo Serialization ===\n");

    // Create instance with multiple schemes
    let auth_info = PushNotificationAuthenticationInfo {
        schemes: vec!["Bearer".to_string(), "Basic".to_string()],
        credentials: Some("my-secret-token".to_string()),
    };

    // Serialize to JSON
    let json_value = serde_json::to_value(&auth_info).unwrap();
    let json_string = serde_json::to_string_pretty(&json_value).unwrap();

    println!("Serialized JSON:");
    println!("{}\n", json_string);

    // Verify the JSON structure
    println!("Verification:");

    // 1. Must have "schemes" field (plural)
    assert!(
        json_value.get("schemes").is_some(),
        "✗ FAIL: Missing 'schemes' field"
    );
    println!("✓ Has 'schemes' field (plural)");

    // 2. Must NOT have singular "scheme" field
    assert!(
        json_value.get("scheme").is_none(),
        "✗ FAIL: Should not have singular 'scheme' field"
    );
    println!("✓ Does NOT have singular 'scheme' field");

    // 3. schemes must be an array
    assert!(
        json_value["schemes"].is_array(),
        "✗ FAIL: 'schemes' is not an array"
    );
    println!("✓ 'schemes' is an array");

    // 4. Array should have both values
    let schemes_array = json_value["schemes"].as_array().unwrap();
    assert_eq!(schemes_array.len(), 2);
    assert_eq!(schemes_array[0], "Bearer");
    assert_eq!(schemes_array[1], "Basic");
    println!("✓ Array contains correct values: [\"Bearer\", \"Basic\"]");

    // 5. credentials should be present
    assert_eq!(json_value["credentials"], "my-secret-token");
    println!("✓ 'credentials' field present with correct value");

    println!("\n=== All checks passed! ===\n");

    // Roundtrip test
    let decoded: PushNotificationAuthenticationInfo = serde_json::from_value(json_value).unwrap();
    assert_eq!(decoded.schemes, vec!["Bearer", "Basic"]);
    assert_eq!(decoded.credentials, Some("my-secret-token".to_string()));
    println!("✓ Roundtrip deserialization successful\n");
}

#[test]
fn verify_single_scheme_serialization() {
    println!("\n=== Single Scheme Test ===\n");

    let auth_info = PushNotificationAuthenticationInfo {
        schemes: vec!["Bearer".to_string()],
        credentials: None,
    };

    let json_value = serde_json::to_value(&auth_info).unwrap();
    let json_string = serde_json::to_string_pretty(&json_value).unwrap();

    println!("Single scheme JSON:");
    println!("{}\n", json_string);

    // Even with a single value, it should still be an array
    assert!(json_value["schemes"].is_array());
    assert_eq!(json_value["schemes"].as_array().unwrap().len(), 1);
    assert_eq!(json_value["schemes"][0], "Bearer");

    // credentials should be omitted when None
    assert!(json_value.get("credentials").is_none());
    println!("✓ Single scheme serialized as array");
    println!("✓ credentials omitted when None\n");
}
