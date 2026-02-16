//! Utility functions for creating A2A Artifact objects.

use crate::types::{Artifact, Part};
use crate::utils::parts::get_text_parts;
use serde_json::Value;
use uuid::Uuid;

/// Creates a new Artifact object with a generated artifact_id.
///
/// # Arguments
///
/// * `parts` - The list of `Part` objects forming the artifact's content.
/// * `name` - The human-readable name of the artifact.
/// * `description` - An optional description of the artifact.
///
/// # Returns
///
/// A new `Artifact` object with a generated artifact_id.
///
/// # Example
///
/// ```
/// use a2a_rs::types::Part;
/// use a2a_rs::utils::new_artifact;
///
/// let parts = vec![
///     Part::Text { text: "Sample text".to_string(), metadata: None },
/// ];
/// let artifact = new_artifact(parts, "My Artifact", Some("This is a test artifact."));
/// assert_eq!(artifact.name, Some("My Artifact".to_string()));
/// ```
pub fn new_artifact(
    parts: Vec<Part>,
    name: impl Into<String>,
    description: Option<impl Into<String>>,
) -> Artifact {
    Artifact {
        artifact_id: Uuid::new_v4().to_string(),
        parts,
        name: Some(name.into()),
        description: description.map(|d| d.into()),
        metadata: None,
        extensions: None,
    }
}

/// Creates a new Artifact object containing only a single text Part.
///
/// # Arguments
///
/// * `name` - The human-readable name of the artifact.
/// * `text` - The text content of the artifact.
/// * `description` - An optional description of the artifact.
///
/// # Returns
///
/// A new `Artifact` object with a generated artifact_id.
///
/// # Example
///
/// ```
/// use a2a_rs::utils::new_text_artifact;
///
/// let artifact = new_text_artifact("Text Artifact", "Hello, world!", Some("A greeting"));
/// assert_eq!(artifact.name, Some("Text Artifact".to_string()));
/// ```
pub fn new_text_artifact(
    name: impl Into<String>,
    text: impl Into<String>,
    description: Option<impl Into<String>>,
) -> Artifact {
    let part = Part::Text {
        text: text.into(),
        metadata: None,
    };
    new_artifact(vec![part], name, description)
}

/// Creates a new Artifact object containing only a single data Part.
///
/// # Arguments
///
/// * `name` - The human-readable name of the artifact.
/// * `data` - The structured data content of the artifact.
/// * `description` - An optional description of the artifact.
///
/// # Returns
///
/// A new `Artifact` object with a generated artifact_id.
///
/// # Example
///
/// ```
/// use a2a_rs::utils::new_data_artifact;
/// use serde_json::json;
///
/// let data = json!({"key": "value", "number": 123});
/// let artifact = new_data_artifact("Data Artifact", data, Some("Sample data"));
/// assert_eq!(artifact.name, Some("Data Artifact".to_string()));
/// ```
pub fn new_data_artifact(
    name: impl Into<String>,
    data: Value,
    description: Option<impl Into<String>>,
) -> Artifact {
    let part = Part::Data {
        data,
        metadata: None,
    };
    new_artifact(vec![part], name, description)
}

/// Extracts and joins all text content from an Artifact's parts.
///
/// # Arguments
///
/// * `artifact` - The `Artifact` object.
/// * `delimiter` - The string to use when joining text from multiple text Parts.
///
/// # Returns
///
/// A single string containing all text content, or an empty string if no text parts are found.
///
/// # Example
///
/// ```
/// use a2a_rs::types::Part;
/// use a2a_rs::utils::{new_artifact, get_artifact_text};
///
/// let parts = vec![
///     Part::Text { text: "First line".to_string(), metadata: None },
///     Part::Text { text: "Second line".to_string(), metadata: None },
/// ];
/// let artifact = new_artifact(parts, "Multi-line", None::<String>);
/// let text = get_artifact_text(&artifact, "\n");
/// assert_eq!(text, "First line\nSecond line");
/// ```
pub fn get_artifact_text(artifact: &Artifact, delimiter: &str) -> String {
    get_text_parts(&artifact.parts).join(delimiter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_new_artifact_generates_id() {
        let parts = vec![Part::Text {
            text: "Sample text".to_string(),
            metadata: None,
        }];
        let artifact = new_artifact(parts, "test_artifact", None::<String>);
        // Verify it's a valid UUID format
        assert!(Uuid::parse_str(&artifact.artifact_id).is_ok());
    }

    #[test]
    fn test_new_text_artifact() {
        let artifact = new_text_artifact("My Artifact", "Hello, world!", Some("A greeting"));
        assert_eq!(artifact.name, Some("My Artifact".to_string()));
        assert_eq!(artifact.description, Some("A greeting".to_string()));
        assert_eq!(artifact.parts.len(), 1);
    }

    #[test]
    fn test_new_data_artifact() {
        let data = json!({"key": "value"});
        let artifact = new_data_artifact("Data Artifact", data.clone(), None::<String>);
        assert_eq!(artifact.name, Some("Data Artifact".to_string()));
        assert_eq!(artifact.parts.len(), 1);
    }

    #[test]
    fn test_get_artifact_text_empty() {
        let artifact = new_artifact(vec![], "Empty", None::<String>);
        assert_eq!(get_artifact_text(&artifact, "\n"), "");
    }
}
