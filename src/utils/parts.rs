//! Utility functions for working with A2A Part objects.

use crate::types::{FileContent, Part};
use serde_json::Value;

/// Extracts text content from all text Parts in a list.
///
/// # Arguments
///
/// * `parts` - A slice of `Part` objects.
///
/// # Returns
///
/// A vector of strings containing the text content from any text Parts found.
///
/// # Example
///
/// ```
/// use a2a_rs::types::Part;
/// use a2a_rs::utils::get_text_parts;
///
/// let parts = vec![
///     Part::Text { text: "Hello".to_string(), metadata: None },
///     Part::Text { text: "World".to_string(), metadata: None },
/// ];
/// let texts = get_text_parts(&parts);
/// assert_eq!(texts, vec!["Hello", "World"]);
/// ```
pub fn get_text_parts(parts: &[Part]) -> Vec<String> {
    parts
        .iter()
        .filter_map(|part| match part {
            Part::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Extracts data content from all data Parts in a list.
///
/// # Arguments
///
/// * `parts` - A slice of `Part` objects.
///
/// # Returns
///
/// A vector of `serde_json::Value` objects containing the data from any data Parts found.
///
/// # Example
///
/// ```
/// use a2a_rs::types::Part;
/// use a2a_rs::utils::get_data_parts;
/// use serde_json::json;
///
/// let parts = vec![
///     Part::Data { data: json!({"key": "value"}), metadata: None },
/// ];
/// let data = get_data_parts(&parts);
/// assert_eq!(data, vec![json!({"key": "value"})]);
/// ```
pub fn get_data_parts(parts: &[Part]) -> Vec<Value> {
    parts
        .iter()
        .filter_map(|part| match part {
            Part::Data { data, .. } => Some(data.clone()),
            _ => None,
        })
        .collect()
}

/// Extracts file content from all file Parts in a list.
///
/// # Arguments
///
/// * `parts` - A slice of `Part` objects.
///
/// # Returns
///
/// A vector of `FileContent` objects containing the file data from any file Parts found.
///
/// # Example
///
/// ```
/// use a2a_rs::types::{Part, FileContent, FileWithUri};
/// use a2a_rs::utils::get_file_parts;
///
/// let file = FileContent::Uri(FileWithUri {
///     uri: "file://path/to/file".to_string(),
///     mime_type: Some("text/plain".to_string()),
///     name: None,
/// });
/// let parts = vec![
///     Part::File { file: file.clone(), metadata: None },
/// ];
/// let files = get_file_parts(&parts);
/// assert_eq!(files.len(), 1);
/// ```
pub fn get_file_parts(parts: &[Part]) -> Vec<FileContent> {
    parts
        .iter()
        .filter_map(|part| match part {
            Part::File { file, .. } => Some(file.clone()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get_text_parts_empty() {
        let parts: Vec<Part> = vec![];
        assert_eq!(get_text_parts(&parts), Vec::<String>::new());
    }

    #[test]
    fn test_get_data_parts_empty() {
        let parts: Vec<Part> = vec![];
        assert_eq!(get_data_parts(&parts), Vec::<Value>::new());
    }

    #[test]
    fn test_get_file_parts_empty() {
        let parts: Vec<Part> = vec![];
        assert_eq!(get_file_parts(&parts), Vec::<FileContent>::new());
    }
}
