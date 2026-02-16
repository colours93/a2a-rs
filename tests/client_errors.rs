//! Port of Python SDK tests/client/test_errors.py
//!
//! Tests the A2A client error types: A2AError variants that correspond to
//! A2AClientError, A2AClientHTTPError, and A2AClientJSONError in Python.
//!
//! In the Rust SDK, these are unified under A2AError enum variants:
//! - A2AError::Transport / A2AError::Other → base client error
//! - A2AError::Http { status, body }       → HTTP error
//! - A2AError::InvalidJson(msg)            → JSON error

use a2a_rs::error::A2AError;

// ============================================================================
// TestA2AClientError (base error)
// ============================================================================

#[test]
fn test_base_error_instantiation() {
    let error = A2AError::Transport("Test error message".to_string());
    let msg = format!("{}", error);
    assert!(msg.contains("Test error message"));
}

#[test]
fn test_other_error_instantiation() {
    let error = A2AError::Other("Generic client error".to_string());
    let msg = format!("{}", error);
    assert!(msg.contains("Generic client error"));
}

// ============================================================================
// TestA2AClientHTTPError → A2AError::Http
// ============================================================================

#[test]
fn test_http_error_instantiation() {
    let error = A2AError::Http {
        status: 404,
        body: "Not Found".to_string(),
    };
    match &error {
        A2AError::Http { status, body } => {
            assert_eq!(*status, 404);
            assert_eq!(body, "Not Found");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn test_http_error_message_formatting() {
    let error = A2AError::Http {
        status: 500,
        body: "Internal Server Error".to_string(),
    };
    let msg = format!("{}", error);
    assert!(msg.contains("500"));
    assert!(msg.contains("Internal Server Error"));
}

#[test]
fn test_http_error_with_empty_body() {
    let error = A2AError::Http {
        status: 403,
        body: String::new(),
    };
    match &error {
        A2AError::Http { status, body } => {
            assert_eq!(*status, 403);
            assert!(body.is_empty());
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn test_http_error_with_various_status_codes() {
    let test_cases = vec![
        (200u16, "OK"),
        (201, "Created"),
        (400, "Bad Request"),
        (401, "Unauthorized"),
        (403, "Forbidden"),
        (404, "Not Found"),
        (500, "Internal Server Error"),
        (503, "Service Unavailable"),
    ];

    for (status_code, message) in test_cases {
        let error = A2AError::Http {
            status: status_code,
            body: message.to_string(),
        };
        match &error {
            A2AError::Http { status, body } => {
                assert_eq!(*status, status_code);
                assert_eq!(body, message);
            }
            _ => panic!("wrong variant"),
        }
        let display = format!("{}", error);
        assert!(display.contains(&status_code.to_string()));
        assert!(display.contains(message));
    }
}

// ============================================================================
// TestA2AClientJSONError → A2AError::InvalidJson
// ============================================================================

#[test]
fn test_json_error_instantiation() {
    let error = A2AError::InvalidJson("Invalid JSON format".to_string());
    let msg = format!("{}", error);
    assert!(msg.contains("Invalid JSON format"));
}

#[test]
fn test_json_error_message_formatting() {
    let error = A2AError::InvalidJson("Missing required field".to_string());
    let msg = format!("{}", error);
    assert!(msg.contains("Missing required field"));
}

#[test]
fn test_json_error_with_empty_message() {
    let error = A2AError::InvalidJson(String::new());
    let msg = format!("{}", error);
    assert!(msg.contains("Invalid JSON"));
}

#[test]
fn test_json_error_with_various_messages() {
    let test_messages = vec![
        "Malformed JSON",
        "Missing required fields",
        "Invalid data type",
        "Unexpected JSON structure",
        "Empty JSON object",
    ];

    for message in test_messages {
        let error = A2AError::InvalidJson(message.to_string());
        let display = format!("{}", error);
        assert!(display.contains(message));
    }
}

// ============================================================================
// TestExceptionHierarchy — Rust uses enum variants instead of class hierarchy
// ============================================================================

#[test]
fn test_error_variant_discrimination() {
    // In Rust, we use match instead of isinstance checks
    let http_err = A2AError::Http {
        status: 404,
        body: "Not Found".to_string(),
    };
    let json_err = A2AError::InvalidJson("Invalid JSON".to_string());
    let transport_err = A2AError::Transport("connection error".to_string());

    assert!(matches!(http_err, A2AError::Http { .. }));
    assert!(matches!(json_err, A2AError::InvalidJson(_)));
    assert!(matches!(transport_err, A2AError::Transport(_)));
}

#[test]
fn test_all_client_errors_implement_display() {
    // Equivalent to testing that all exceptions can be caught as base Exception
    let errors: Vec<A2AError> = vec![
        A2AError::Http {
            status: 404,
            body: "Not Found".to_string(),
        },
        A2AError::InvalidJson("Invalid JSON".to_string()),
        A2AError::Transport("transport error".to_string()),
        A2AError::Timeout("request timed out".to_string()),
        A2AError::Other("generic error".to_string()),
    ];

    for error in errors {
        // All variants implement Display (via thiserror)
        let msg = format!("{}", error);
        assert!(!msg.is_empty());
    }
}

// ============================================================================
// TestExceptionRaising — Rust uses Result instead of exceptions
// ============================================================================

#[test]
fn test_http_error_in_result() {
    let result: Result<(), A2AError> = Err(A2AError::Http {
        status: 429,
        body: "Too Many Requests".to_string(),
    });

    match result {
        Err(A2AError::Http { status, body }) => {
            assert_eq!(status, 429);
            assert_eq!(body, "Too Many Requests");
        }
        _ => panic!("expected Http error"),
    }
}

#[test]
fn test_json_error_in_result() {
    let result: Result<(), A2AError> = Err(A2AError::InvalidJson("Invalid format".to_string()));

    match result {
        Err(A2AError::InvalidJson(msg)) => {
            assert!(msg.contains("Invalid format"));
        }
        _ => panic!("expected InvalidJson error"),
    }
}

#[test]
fn test_transport_error_in_result() {
    let result: Result<(), A2AError> = Err(A2AError::Transport("Generic client error".to_string()));

    match result {
        Err(A2AError::Transport(msg)) => {
            assert!(msg.contains("Generic client error"));
        }
        _ => panic!("expected Transport error"),
    }
}

// ============================================================================
// Parametrized tests (matching Python's @pytest.mark.parametrize)
// ============================================================================

#[test]
fn test_http_error_parametrized() {
    let cases = vec![
        (400u16, "Bad Request"),
        (404, "Not Found"),
        (500, "Server Error"),
    ];

    for (status_code, message) in cases {
        let error = A2AError::Http {
            status: status_code,
            body: message.to_string(),
        };
        match &error {
            A2AError::Http { status, body } => {
                assert_eq!(*status, status_code);
                assert_eq!(body, message);
            }
            _ => panic!("wrong variant"),
        }
        let display = format!("{}", error);
        assert!(display.contains(&status_code.to_string()));
        assert!(display.contains(message));
    }
}

#[test]
fn test_json_error_parametrized() {
    let cases = vec!["Missing field", "Invalid type", "Parsing failed"];

    for message in cases {
        let error = A2AError::InvalidJson(message.to_string());
        let display = format!("{}", error);
        assert!(display.contains(message));
    }
}

// ============================================================================
// Additional Rust-specific tests: error code mapping
// ============================================================================

#[test]
fn test_http_error_maps_to_internal_error_code() {
    let error = A2AError::Http {
        status: 500,
        body: "error".to_string(),
    };
    // Client-side errors map to internal error code
    assert_eq!(error.code(), a2a_rs::error::INTERNAL_ERROR);
}

#[test]
fn test_json_error_maps_to_internal_error_code() {
    let error = A2AError::InvalidJson("bad json".to_string());
    assert_eq!(error.code(), a2a_rs::error::INTERNAL_ERROR);
}

#[test]
fn test_timeout_error() {
    let error = A2AError::Timeout("Client Request timed out".to_string());
    let msg = format!("{}", error);
    assert!(msg.contains("Client Request timed out"));
    assert_eq!(error.code(), a2a_rs::error::INTERNAL_ERROR);
}
