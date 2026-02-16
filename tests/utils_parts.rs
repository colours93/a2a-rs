//! Tests for utils::parts module
//! Ported from reference/a2a-python/tests/utils/test_parts.py

use a2a_rs::types::{FileContent, FileWithBytes, FileWithUri, Part};
use a2a_rs::utils::{get_data_parts, get_file_parts, get_text_parts};
use serde_json::json;

// TestGetTextParts class tests

#[test]
fn test_get_text_parts_single_text_part() {
    // Setup
    let parts = vec![Part::Text {
        text: "Hello world".to_string(),
        metadata: None,
    }];

    // Exercise
    let result = get_text_parts(&parts);

    // Verify
    assert_eq!(result, vec!["Hello world"]);
}

#[test]
fn test_get_text_parts_multiple_text_parts() {
    // Setup
    let parts = vec![
        Part::Text {
            text: "First part".to_string(),
            metadata: None,
        },
        Part::Text {
            text: "Second part".to_string(),
            metadata: None,
        },
        Part::Text {
            text: "Third part".to_string(),
            metadata: None,
        },
    ];

    // Exercise
    let result = get_text_parts(&parts);

    // Verify
    assert_eq!(result, vec!["First part", "Second part", "Third part"]);
}

#[test]
fn test_get_text_parts_empty_list() {
    // Setup
    let parts: Vec<Part> = vec![];

    // Exercise
    let result = get_text_parts(&parts);

    // Verify
    assert_eq!(result, Vec::<String>::new());
}

// TestGetDataParts class tests

#[test]
fn test_get_data_parts_single_data_part() {
    // Setup
    let parts = vec![Part::Data {
        data: json!({"key": "value"}),
        metadata: None,
    }];

    // Exercise
    let result = get_data_parts(&parts);

    // Verify
    assert_eq!(result, vec![json!({"key": "value"})]);
}

#[test]
fn test_get_data_parts_multiple_data_parts() {
    // Setup
    let parts = vec![
        Part::Data {
            data: json!({"key1": "value1"}),
            metadata: None,
        },
        Part::Data {
            data: json!({"key2": "value2"}),
            metadata: None,
        },
    ];

    // Exercise
    let result = get_data_parts(&parts);

    // Verify
    assert_eq!(
        result,
        vec![json!({"key1": "value1"}), json!({"key2": "value2"})]
    );
}

#[test]
fn test_get_data_parts_mixed_parts() {
    // Setup
    let parts = vec![
        Part::Text {
            text: "some text".to_string(),
            metadata: None,
        },
        Part::Data {
            data: json!({"key1": "value1"}),
            metadata: None,
        },
        Part::Data {
            data: json!({"key2": "value2"}),
            metadata: None,
        },
    ];

    // Exercise
    let result = get_data_parts(&parts);

    // Verify
    assert_eq!(
        result,
        vec![json!({"key1": "value1"}), json!({"key2": "value2"})]
    );
}

#[test]
fn test_get_data_parts_no_data_parts() {
    // Setup
    let parts = vec![Part::Text {
        text: "some text".to_string(),
        metadata: None,
    }];

    // Exercise
    let result = get_data_parts(&parts);

    // Verify
    assert_eq!(result, Vec::<serde_json::Value>::new());
}

#[test]
fn test_get_data_parts_empty_list() {
    // Setup
    let parts: Vec<Part> = vec![];

    // Exercise
    let result = get_data_parts(&parts);

    // Verify
    assert_eq!(result, Vec::<serde_json::Value>::new());
}

// TestGetFileParts class tests

#[test]
fn test_get_file_parts_single_file_part() {
    // Setup
    let file_with_uri = FileContent::Uri(FileWithUri {
        uri: "file://path/to/file".to_string(),
        mime_type: Some("text/plain".to_string()),
        name: None,
    });
    let parts = vec![Part::File {
        file: file_with_uri.clone(),
        metadata: None,
    }];

    // Exercise
    let result = get_file_parts(&parts);

    // Verify
    assert_eq!(result.len(), 1);
    match &result[0] {
        FileContent::Uri(f) => {
            assert_eq!(f.uri, "file://path/to/file");
            assert_eq!(f.mime_type, Some("text/plain".to_string()));
        }
        _ => panic!("Expected FileWithUri"),
    }
}

#[test]
fn test_get_file_parts_multiple_file_parts() {
    // Setup
    let file_with_uri1 = FileContent::Uri(FileWithUri {
        uri: "file://path/to/file1".to_string(),
        mime_type: Some("text/plain".to_string()),
        name: None,
    });
    let file_with_bytes = FileContent::Bytes(FileWithBytes {
        bytes: "ZmlsZSBjb250ZW50".to_string(), // base64 for "file content"
        mime_type: Some("application/octet-stream".to_string()),
        name: None,
    });
    let parts = vec![
        Part::File {
            file: file_with_uri1,
            metadata: None,
        },
        Part::File {
            file: file_with_bytes,
            metadata: None,
        },
    ];

    // Exercise
    let result = get_file_parts(&parts);

    // Verify
    assert_eq!(result.len(), 2);
}

#[test]
fn test_get_file_parts_mixed_parts() {
    // Setup
    let file_with_uri = FileContent::Uri(FileWithUri {
        uri: "file://path/to/file".to_string(),
        mime_type: Some("text/plain".to_string()),
        name: None,
    });
    let parts = vec![
        Part::Text {
            text: "some text".to_string(),
            metadata: None,
        },
        Part::File {
            file: file_with_uri,
            metadata: None,
        },
    ];

    // Exercise
    let result = get_file_parts(&parts);

    // Verify
    assert_eq!(result.len(), 1);
}

#[test]
fn test_get_file_parts_no_file_parts() {
    // Setup
    let parts = vec![
        Part::Text {
            text: "some text".to_string(),
            metadata: None,
        },
        Part::Data {
            data: json!({"key": "value"}),
            metadata: None,
        },
    ];

    // Exercise
    let result = get_file_parts(&parts);

    // Verify
    assert_eq!(result, Vec::<FileContent>::new());
}

#[test]
fn test_get_file_parts_empty_list() {
    // Setup
    let parts: Vec<Part> = vec![];

    // Exercise
    let result = get_file_parts(&parts);

    // Verify
    assert_eq!(result, Vec::<FileContent>::new());
}
