//! Tests for utils::artifact module
//! Ported from reference/a2a-python/tests/utils/test_artifact.py

use a2a_rs::types::{Artifact, Part};
use a2a_rs::utils::{get_artifact_text, new_artifact, new_data_artifact, new_text_artifact};
use serde_json::json;
use uuid::Uuid;

// TestArtifact class tests

#[test]
fn test_new_artifact_generates_id() {
    let parts = vec![Part::Text {
        text: "Sample text".to_string(),
        metadata: None,
    }];
    let artifact = new_artifact(parts, "test_artifact", None::<String>);

    // Verify it's a valid UUID
    assert!(Uuid::parse_str(&artifact.artifact_id).is_ok());
}

#[test]
fn test_new_artifact_assigns_parts_name_description() {
    let parts = vec![Part::Text {
        text: "Sample text".to_string(),
        metadata: None,
    }];
    let name = "My Artifact";
    let description = "This is a test artifact.";

    let artifact = new_artifact(parts.clone(), name, Some(description));

    assert_eq!(artifact.parts.len(), parts.len());
    assert_eq!(artifact.name, Some(name.to_string()));
    assert_eq!(artifact.description, Some(description.to_string()));
}

#[test]
fn test_new_artifact_empty_description_if_not_provided() {
    let parts = vec![Part::Text {
        text: "Another sample".to_string(),
        metadata: None,
    }];
    let name = "Artifact_No_Desc";

    let artifact = new_artifact(parts, name, None::<String>);

    assert_eq!(artifact.description, None);
}

#[test]
fn test_new_text_artifact_creates_single_text_part() {
    let text = "This is a text artifact.";
    let name = "Text_Artifact";

    let artifact = new_text_artifact(name, text, None::<String>);

    assert_eq!(artifact.parts.len(), 1);
    match &artifact.parts[0] {
        Part::Text { .. } => (),
        _ => panic!("Expected text part"),
    }
}

#[test]
fn test_new_text_artifact_part_contains_provided_text() {
    let text = "Hello, world!";
    let name = "Greeting_Artifact";

    let artifact = new_text_artifact(name, text, None::<String>);

    match &artifact.parts[0] {
        Part::Text {
            text: part_text, ..
        } => {
            assert_eq!(part_text, text);
        }
        _ => panic!("Expected text part"),
    }
}

#[test]
fn test_new_text_artifact_assigns_name_description() {
    let text = "Some content.";
    let name = "Named_Text_Artifact";
    let description = "Description for text artifact.";

    let artifact = new_text_artifact(name, text, Some(description));

    assert_eq!(artifact.name, Some(name.to_string()));
    assert_eq!(artifact.description, Some(description.to_string()));
}

#[test]
fn test_new_data_artifact_creates_single_data_part() {
    let sample_data = json!({"key": "value", "number": 123});
    let name = "Data_Artifact";

    let artifact = new_data_artifact(name, sample_data, None::<String>);

    assert_eq!(artifact.parts.len(), 1);
    match &artifact.parts[0] {
        Part::Data { .. } => (),
        _ => panic!("Expected data part"),
    }
}

#[test]
fn test_new_data_artifact_part_contains_provided_data() {
    let sample_data = json!({"content": "test_data", "is_valid": true});
    let name = "Structured_Data_Artifact";

    let artifact = new_data_artifact(name, sample_data.clone(), None::<String>);

    match &artifact.parts[0] {
        Part::Data { data, .. } => {
            assert_eq!(data, &sample_data);
        }
        _ => panic!("Expected data part"),
    }
}

#[test]
fn test_new_data_artifact_assigns_name_description() {
    let sample_data = json!({"info": "some details"});
    let name = "Named_Data_Artifact";
    let description = "Description for data artifact.";

    let artifact = new_data_artifact(name, sample_data, Some(description));

    assert_eq!(artifact.name, Some(name.to_string()));
    assert_eq!(artifact.description, Some(description.to_string()));
}

// TestGetArtifactText class tests

#[test]
fn test_get_artifact_text_single_part() {
    // Setup
    let artifact = Artifact {
        name: Some("test-artifact".to_string()),
        parts: vec![Part::Text {
            text: "Hello world".to_string(),
            metadata: None,
        }],
        artifact_id: "test-artifact-id".to_string(),
        description: None,
        metadata: None,
        extensions: None,
    };

    // Exercise
    let result = get_artifact_text(&artifact, "\n");

    // Verify
    assert_eq!(result, "Hello world");
}

#[test]
fn test_get_artifact_text_multiple_parts() {
    // Setup
    let artifact = Artifact {
        name: Some("test-artifact".to_string()),
        parts: vec![
            Part::Text {
                text: "First line".to_string(),
                metadata: None,
            },
            Part::Text {
                text: "Second line".to_string(),
                metadata: None,
            },
            Part::Text {
                text: "Third line".to_string(),
                metadata: None,
            },
        ],
        artifact_id: "test-artifact-id".to_string(),
        description: None,
        metadata: None,
        extensions: None,
    };

    // Exercise
    let result = get_artifact_text(&artifact, "\n");

    // Verify - default delimiter is newline
    assert_eq!(result, "First line\nSecond line\nThird line");
}

#[test]
fn test_get_artifact_text_custom_delimiter() {
    // Setup
    let artifact = Artifact {
        name: Some("test-artifact".to_string()),
        parts: vec![
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
        ],
        artifact_id: "test-artifact-id".to_string(),
        description: None,
        metadata: None,
        extensions: None,
    };

    // Exercise
    let result = get_artifact_text(&artifact, " | ");

    // Verify
    assert_eq!(result, "First part | Second part | Third part");
}

#[test]
fn test_get_artifact_text_empty_parts() {
    // Setup
    let artifact = Artifact {
        name: Some("test-artifact".to_string()),
        parts: vec![],
        artifact_id: "test-artifact-id".to_string(),
        description: None,
        metadata: None,
        extensions: None,
    };

    // Exercise
    let result = get_artifact_text(&artifact, "\n");

    // Verify
    assert_eq!(result, "");
}
