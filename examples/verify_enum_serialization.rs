use a2a_rs::types::{Role, TaskState};
use serde_json;

fn main() {
    println!("=== TaskState Enum Serialization ===");
    println!("Expected format: kebab-case");
    println!();

    let task_states = [
        TaskState::Submitted,
        TaskState::Working,
        TaskState::Completed,
        TaskState::Failed,
        TaskState::Canceled,
        TaskState::InputRequired,
        TaskState::Rejected,
        TaskState::AuthRequired,
        TaskState::Unknown,
    ];

    for state in &task_states {
        let json = serde_json::to_string(state).unwrap();
        println!("{:?} => {}", state, json);
    }

    println!();
    println!("=== Role Enum Serialization ===");
    println!("Expected format: lowercase");
    println!();

    let roles = [Role::User, Role::Agent, Role::Unspecified];

    for role in &roles {
        let json = serde_json::to_string(role).unwrap();
        println!("{:?} => {}", role, json);
    }

    println!();
    println!("=== Complete JSON Objects ===");
    println!();

    // Create a sample object with each enum
    let task_state_json = serde_json::json!({
        "submitted": TaskState::Submitted,
        "working": TaskState::Working,
        "completed": TaskState::Completed,
        "failed": TaskState::Failed,
        "canceled": TaskState::Canceled,
        "input_required": TaskState::InputRequired,
        "rejected": TaskState::Rejected,
        "auth_required": TaskState::AuthRequired,
        "unknown": TaskState::Unknown,
    });

    println!("TaskState JSON:");
    println!(
        "{}",
        serde_json::to_string_pretty(&task_state_json).unwrap()
    );
    println!();

    let role_json = serde_json::json!({
        "user": Role::User,
        "agent": Role::Agent,
        "unspecified": Role::Unspecified,
    });

    println!("Role JSON:");
    println!("{}", serde_json::to_string_pretty(&role_json).unwrap());
    println!();

    // Verify deserialization works correctly
    println!("=== Deserialization Verification ===");
    println!();

    let test_cases = [
        ("\"submitted\"", "TaskState::Submitted"),
        ("\"working\"", "TaskState::Working"),
        ("\"completed\"", "TaskState::Completed"),
        ("\"failed\"", "TaskState::Failed"),
        ("\"canceled\"", "TaskState::Canceled"),
        ("\"input-required\"", "TaskState::InputRequired"),
        ("\"rejected\"", "TaskState::Rejected"),
        ("\"auth-required\"", "TaskState::AuthRequired"),
        ("\"unknown\"", "TaskState::Unknown"),
    ];

    for (json_str, expected) in &test_cases {
        match serde_json::from_str::<TaskState>(json_str) {
            Ok(state) => println!("{} => {:?} ✓", json_str, state),
            Err(e) => println!("{} => ERROR: {} ✗", json_str, e),
        }
    }

    println!();

    let role_test_cases = [
        ("\"user\"", "Role::User"),
        ("\"agent\"", "Role::Agent"),
        ("\"unspecified\"", "Role::Unspecified"),
    ];

    for (json_str, expected) in &role_test_cases {
        match serde_json::from_str::<Role>(json_str) {
            Ok(role) => println!("{} => {:?} ✓", json_str, role),
            Err(e) => println!("{} => ERROR: {} ✗", json_str, e),
        }
    }

    println!();
    println!("=== Python SDK Compatibility Check ===");
    println!();
    println!("TaskState variants (kebab-case):");
    println!("  - submitted");
    println!("  - working");
    println!("  - completed");
    println!("  - failed");
    println!("  - canceled");
    println!("  - input-required");
    println!("  - rejected");
    println!("  - auth-required");
    println!("  - unknown");
    println!();
    println!("Role variants (lowercase):");
    println!("  - user");
    println!("  - agent");
    println!("  - unspecified");
}
