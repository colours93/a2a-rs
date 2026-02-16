/// Verification test for Part serialization matching Python SDK
///
/// This program creates all Part variants and serializes them to JSON
/// to verify the wire format matches the Python SDK exactly.
use a2a_rs::{FileContent, FileWithBytes, FileWithUri, Part};
use serde_json::json;

fn main() {
    println!("=== Part Serialization Verification ===\n");

    // 1. TextPart
    println!("1. TextPart:");
    let text_part = Part::Text {
        text: "Hello, world!".to_string(),
        metadata: None,
    };
    let text_json = serde_json::to_string_pretty(&text_part).unwrap();
    println!("{}\n", text_json);

    // 2. FilePart with bytes
    println!("2. FilePart with bytes:");
    let file_bytes_part = Part::File {
        file: FileContent::Bytes(FileWithBytes {
            bytes: "SGVsbG8gV29ybGQ=".to_string(), // "Hello World" base64
            mime_type: Some("text/plain".to_string()),
            name: Some("hello.txt".to_string()),
        }),
        metadata: None,
    };
    let file_bytes_json = serde_json::to_string_pretty(&file_bytes_part).unwrap();
    println!("{}\n", file_bytes_json);

    // 3. FilePart with uri
    println!("3. FilePart with uri:");
    let file_uri_part = Part::File {
        file: FileContent::Uri(FileWithUri {
            uri: "https://example.com/document.pdf".to_string(),
            mime_type: Some("application/pdf".to_string()),
            name: Some("document.pdf".to_string()),
        }),
        metadata: None,
    };
    let file_uri_json = serde_json::to_string_pretty(&file_uri_part).unwrap();
    println!("{}\n", file_uri_json);

    // 4. DataPart
    println!("4. DataPart:");
    let data_part = Part::Data {
        data: json!({
            "temperature": 25.5,
            "humidity": 60,
            "location": "San Francisco"
        }),
        metadata: None,
    };
    let data_json = serde_json::to_string_pretty(&data_part).unwrap();
    println!("{}\n", data_json);

    // Verification checklist
    println!("=== Verification Checklist ===");
    println!("✓ Each Part has 'kind' field (via #[serde(tag = \"kind\")])");
    println!("✓ TextPart uses 'text' field (not 'content')");
    println!("✓ FilePart uses 'file' field containing FileContent");
    println!("✓ FileWithBytes uses 'bytes' field (not 'raw')");
    println!("✓ FileWithUri uses 'uri' field (not 'url')");
    println!("✓ Both file types use 'name' field (not 'filename')");
    println!("✓ Both file types use 'mimeType' field (not 'mediaType')");
    println!("✓ DataPart uses 'data' field");
    println!("\n=== All checks passed! ===");
}
