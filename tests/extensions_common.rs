//! Tests for extensions utility functions
//! Ported from reference/a2a-python/tests/extensions/test_common.py

use a2a_rs::types::{AgentCapabilities, AgentCard, AgentExtension, AgentInterface};
use a2a_rs::utils::{
    find_extension_by_uri, get_requested_extensions, update_extension_header, HTTP_EXTENSION_HEADER,
};
use std::collections::{HashMap, HashSet};

// Test get_requested_extensions

#[test]
fn test_get_requested_extensions_empty() {
    assert_eq!(get_requested_extensions(&vec![]), HashSet::new());
}

#[test]
fn test_get_requested_extensions_single() {
    let result = get_requested_extensions(&vec!["foo".to_string()]);
    assert_eq!(result, vec!["foo"].into_iter().map(String::from).collect());
}

#[test]
fn test_get_requested_extensions_multiple() {
    let result = get_requested_extensions(&vec!["foo".to_string(), "bar".to_string()]);
    let expected: HashSet<String> = vec!["foo", "bar"].into_iter().map(String::from).collect();
    assert_eq!(result, expected);
}

#[test]
fn test_get_requested_extensions_comma_separated() {
    let result = get_requested_extensions(&vec!["foo, bar".to_string()]);
    let expected: HashSet<String> = vec!["foo", "bar"].into_iter().map(String::from).collect();
    assert_eq!(result, expected);
}

#[test]
fn test_get_requested_extensions_comma_no_space() {
    let result = get_requested_extensions(&vec!["foo,bar".to_string()]);
    let expected: HashSet<String> = vec!["foo", "bar"].into_iter().map(String::from).collect();
    assert_eq!(result, expected);
}

#[test]
fn test_get_requested_extensions_mixed() {
    let result = get_requested_extensions(&vec!["foo".to_string(), "bar,baz".to_string()]);
    let expected: HashSet<String> = vec!["foo", "bar", "baz"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(result, expected);
}

#[test]
fn test_get_requested_extensions_empty_segments() {
    let result = get_requested_extensions(&vec!["foo,, bar".to_string(), "baz".to_string()]);
    let expected: HashSet<String> = vec!["foo", "bar", "baz"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(result, expected);
}

#[test]
fn test_get_requested_extensions_with_spaces() {
    let result = get_requested_extensions(&vec![" foo , bar ".to_string(), "baz".to_string()]);
    let expected: HashSet<String> = vec!["foo", "bar", "baz"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(result, expected);
}

// Test find_extension_by_uri

fn create_test_card_with_extensions(extensions: Option<Vec<AgentExtension>>) -> AgentCard {
    AgentCard {
        name: "Test Agent".to_string(),
        description: "Test Agent Description".to_string(),
        version: "1.0".to_string(),
        url: "http://test.com".to_string(),
        supported_interfaces: vec![AgentInterface {
            url: "http://test.com".to_string(),
            transport: "JSONRPC".to_string(),
            protocol_version: Some("0.3".to_string()),
            tenant: None,
        }],
        capabilities: AgentCapabilities {
            extensions,
            ..Default::default()
        },
        default_input_modes: vec!["text/plain".to_string()],
        default_output_modes: vec!["text/plain".to_string()],
        skills: vec![],
        provider: None,
        documentation_url: None,
        security_schemes: None,
        security_requirements: vec![],
        signatures: None,
        icon_url: None,
        additional_interfaces: None,
        preferred_transport: None,
        protocol_version: None,
        supports_authenticated_extended_card: None,
        security: None,
    }
}

#[test]
fn test_find_extension_by_uri() {
    let ext1 = AgentExtension {
        uri: "foo".to_string(),
        description: Some("The Foo extension".to_string()),
        required: None,
        params: None,
    };
    let ext2 = AgentExtension {
        uri: "bar".to_string(),
        description: Some("The Bar extension".to_string()),
        required: None,
        params: None,
    };
    let card = create_test_card_with_extensions(Some(vec![ext1.clone(), ext2.clone()]));

    let found_foo = find_extension_by_uri(&card, "foo");
    assert!(found_foo.is_some());
    assert_eq!(found_foo.unwrap().uri, "foo");

    let found_bar = find_extension_by_uri(&card, "bar");
    assert!(found_bar.is_some());
    assert_eq!(found_bar.unwrap().uri, "bar");

    let found_baz = find_extension_by_uri(&card, "baz");
    assert!(found_baz.is_none());
}

#[test]
fn test_find_extension_by_uri_no_extensions() {
    let card = create_test_card_with_extensions(None);
    let found = find_extension_by_uri(&card, "foo");
    assert!(found.is_none());
}

// Test update_extension_header

#[test]
fn test_update_extension_header_new_extensions_empty_header() {
    let mut headers = HashMap::new();
    headers.insert(HTTP_EXTENSION_HEADER.to_string(), "".to_string());

    let mut http_kwargs = HashMap::new();
    http_kwargs.insert("headers".to_string(), headers);

    let extensions = vec!["ext1".to_string(), "ext2".to_string()];
    let result = update_extension_header(Some(http_kwargs), Some(&extensions));

    let result_headers = result.get("headers").unwrap();
    let header_value = result_headers.get(HTTP_EXTENSION_HEADER).unwrap();
    let actual: HashSet<String> = header_value
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let expected: HashSet<String> = vec!["ext1", "ext2"].into_iter().map(String::from).collect();
    assert_eq!(actual, expected);
}

#[test]
fn test_update_extension_header_extensions_none_existing_header() {
    let mut headers = HashMap::new();
    headers.insert(HTTP_EXTENSION_HEADER.to_string(), "ext1, ext2".to_string());

    let mut http_kwargs = HashMap::new();
    http_kwargs.insert("headers".to_string(), headers.clone());

    let result = update_extension_header(Some(http_kwargs), None);

    // When extensions is None, header should remain unchanged
    let result_headers = result.get("headers").unwrap();
    assert_eq!(
        result_headers.get(HTTP_EXTENSION_HEADER),
        Some(&"ext1, ext2".to_string())
    );
}

#[test]
fn test_update_extension_header_empty_extensions_existing_header() {
    let mut headers = HashMap::new();
    headers.insert(HTTP_EXTENSION_HEADER.to_string(), "ext1".to_string());

    let mut http_kwargs = HashMap::new();
    http_kwargs.insert("headers".to_string(), headers);

    let extensions = vec![];
    let result = update_extension_header(Some(http_kwargs), Some(&extensions));

    let result_headers = result.get("headers").unwrap();
    let header_value = result_headers.get(HTTP_EXTENSION_HEADER).unwrap();
    // Empty extensions should produce empty header value
    assert_eq!(header_value, "");
}

#[test]
fn test_update_extension_header_override_existing() {
    let mut headers = HashMap::new();
    headers.insert(HTTP_EXTENSION_HEADER.to_string(), "ext3".to_string());

    let mut http_kwargs = HashMap::new();
    http_kwargs.insert("headers".to_string(), headers);

    let extensions = vec!["ext1".to_string(), "ext2".to_string()];
    let result = update_extension_header(Some(http_kwargs), Some(&extensions));

    let result_headers = result.get("headers").unwrap();
    let header_value = result_headers.get(HTTP_EXTENSION_HEADER).unwrap();
    let actual: HashSet<String> = header_value
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let expected: HashSet<String> = vec!["ext1", "ext2"].into_iter().map(String::from).collect();
    assert_eq!(actual, expected);
}

#[test]
fn test_update_extension_header_with_other_headers() {
    let mut headers = HashMap::new();
    headers.insert("X_Other".to_string(), "Test".to_string());

    let mut http_kwargs = HashMap::new();
    http_kwargs.insert("headers".to_string(), headers);

    let extensions = vec!["ext".to_string()];
    let result = update_extension_header(Some(http_kwargs), Some(&extensions));

    let result_headers = result.get("headers").unwrap();
    assert_eq!(
        result_headers.get(HTTP_EXTENSION_HEADER),
        Some(&"ext".to_string())
    );
    assert_eq!(result_headers.get("X_Other"), Some(&"Test".to_string()));
}

#[test]
fn test_update_extension_header_no_headers_in_kwargs() {
    let http_kwargs = HashMap::new();
    let extensions = vec!["ext".to_string()];
    let result = update_extension_header(Some(http_kwargs), Some(&extensions));

    let result_headers = result.get("headers").unwrap();
    assert_eq!(
        result_headers.get(HTTP_EXTENSION_HEADER),
        Some(&"ext".to_string())
    );
}

#[test]
fn test_update_extension_header_kwargs_none() {
    let extensions = vec!["ext".to_string()];
    let result = update_extension_header(None, Some(&extensions));

    let result_headers = result.get("headers").unwrap();
    assert_eq!(
        result_headers.get(HTTP_EXTENSION_HEADER),
        Some(&"ext".to_string())
    );
}

#[test]
fn test_update_extension_header_with_other_headers_extensions_none() {
    let mut headers = HashMap::new();
    headers.insert("X_Other".to_string(), "Test".to_string());

    let mut http_kwargs = HashMap::new();
    http_kwargs.insert("headers".to_string(), headers);

    let result = update_extension_header(Some(http_kwargs), None);

    let result_headers = result.get("headers").unwrap();
    assert!(!result_headers.contains_key(HTTP_EXTENSION_HEADER));
    assert_eq!(result_headers.get("X_Other"), Some(&"Test".to_string()));
}
