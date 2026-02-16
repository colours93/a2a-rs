//! Extension utility functions.
//!
//! Helpers for working with A2A protocol extensions (AgentExtension).

use crate::types::{AgentCard, AgentExtension};
use std::collections::{HashMap, HashSet};

/// HTTP header for A2A extensions.
pub const HTTP_EXTENSION_HEADER: &str = "X-A2A-Extensions";

/// Parse requested extensions from HTTP header values.
///
/// Handles comma-separated values as occurs in HTTP headers.
/// Strips whitespace and filters empty strings.
///
/// # Example
/// ```
/// use a2a_rs::utils::get_requested_extensions;
///
/// let exts = get_requested_extensions(&vec!["foo,bar".to_string(), "baz".to_string()]);
/// assert_eq!(exts, vec!["foo", "bar", "baz"].into_iter().map(String::from).collect());
/// ```
pub fn get_requested_extensions(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .flat_map(|v| v.split(','))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// Find an AgentExtension in an AgentCard by URI.
///
/// Returns `None` if no extension with the given URI is found.
///
/// # Example
/// ```
/// use a2a_rs::types::{AgentCard, AgentExtension, AgentCapabilities};
/// use a2a_rs::utils::find_extension_by_uri;
///
/// let ext = AgentExtension {
///     uri: "foo".to_string(),
///     description: Some("Foo extension".to_string()),
///     required: None,
///     params: None,
/// };
/// // ... create card with this extension ...
/// # let card = AgentCard { name: "test".to_string(), description: "test".to_string(), version: "1.0".to_string(), url: "https://example.com".to_string(), supported_interfaces: vec![], capabilities: AgentCapabilities { extensions: Some(vec![ext.clone()]), ..Default::default() }, default_input_modes: vec![], default_output_modes: vec![], skills: vec![], provider: None, documentation_url: None, security_schemes: None, security_requirements: vec![], signatures: None, icon_url: None, additional_interfaces: None, preferred_transport: None, protocol_version: None, supports_authenticated_extended_card: None, security: None };
/// let found = find_extension_by_uri(&card, "foo");
/// assert_eq!(found.map(|e| &e.uri), Some(&"foo".to_string()));
/// ```
pub fn find_extension_by_uri<'a>(card: &'a AgentCard, uri: &str) -> Option<&'a AgentExtension> {
    card.capabilities
        .extensions
        .as_ref()?
        .iter()
        .find(|ext| ext.uri == uri)
}

/// Update HTTP kwargs with the X-A2A-Extensions header.
///
/// If `extensions` is `Some`, sets the header to a comma-separated list.
/// If `extensions` is `None`, the header is not modified.
///
/// Returns a new HashMap with the updated headers.
///
/// # Example
/// ```
/// use a2a_rs::utils::update_extension_header;
/// use std::collections::HashMap;
///
/// let mut kwargs: HashMap<String, HashMap<String, String>> = HashMap::new();
/// let result = update_extension_header(Some(kwargs), Some(&vec!["ext1".to_string(), "ext2".to_string()]));
/// let headers = result.get("headers").unwrap();
/// assert_eq!(headers.get("X-A2A-Extensions"), Some(&"ext1,ext2".to_string()));
/// ```
pub fn update_extension_header(
    http_kwargs: Option<HashMap<String, HashMap<String, String>>>,
    extensions: Option<&Vec<String>>,
) -> HashMap<String, HashMap<String, String>> {
    let mut kwargs = http_kwargs.unwrap_or_default();

    if let Some(exts) = extensions {
        let headers = kwargs.entry("headers".to_string()).or_default();
        headers.insert(HTTP_EXTENSION_HEADER.to_string(), exts.join(","));
    }

    kwargs
}
