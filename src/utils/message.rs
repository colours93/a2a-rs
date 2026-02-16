//! Utility functions for creating and handling A2A Message objects.

use crate::types::{Message, Part, Role};
use crate::utils::parts::get_text_parts;
use uuid::Uuid;

/// Creates a new agent message containing a single text Part.
///
/// # Arguments
///
/// * `text` - The text content of the message.
/// * `context_id` - The context ID for the message.
/// * `task_id` - The task ID for the message.
///
/// # Returns
///
/// A new `Message` object with role 'agent'.
///
/// # Example
///
/// ```
/// use a2a_rs::utils::new_agent_text_message;
///
/// let message = new_agent_text_message("Hello, I'm an agent", None::<String>, None::<String>);
/// assert_eq!(message.role, a2a_rs::types::Role::Agent);
/// ```
pub fn new_agent_text_message(
    text: impl Into<String>,
    context_id: Option<impl Into<String>>,
    task_id: Option<impl Into<String>>,
) -> Message {
    let part = Part::Text {
        text: text.into(),
        metadata: None,
    };
    Message {
        message_id: Uuid::new_v4().to_string(),
        role: Role::Agent,
        kind: "message".to_string(),
        parts: vec![part],
        context_id: context_id.map(|id| id.into()),
        task_id: task_id.map(|id| id.into()),
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    }
}

/// Creates a new agent message containing a list of Parts.
///
/// # Arguments
///
/// * `parts` - The list of `Part` objects for the message content.
/// * `context_id` - The context ID for the message.
/// * `task_id` - The task ID for the message.
///
/// # Returns
///
/// A new `Message` object with role 'agent'.
///
/// # Example
///
/// ```
/// use a2a_rs::types::Part;
/// use a2a_rs::utils::new_agent_parts_message;
///
/// let parts = vec![
///     Part::Text { text: "Hello".to_string(), metadata: None },
/// ];
/// let message = new_agent_parts_message(parts, None::<String>, None::<String>);
/// assert_eq!(message.role, a2a_rs::types::Role::Agent);
/// ```
pub fn new_agent_parts_message(
    parts: Vec<Part>,
    context_id: Option<impl Into<String>>,
    task_id: Option<impl Into<String>>,
) -> Message {
    Message {
        message_id: Uuid::new_v4().to_string(),
        role: Role::Agent,
        kind: "message".to_string(),
        parts,
        context_id: context_id.map(|id| id.into()),
        task_id: task_id.map(|id| id.into()),
        metadata: None,
        extensions: None,
        reference_task_ids: None,
    }
}

/// Extracts and joins all text content from a Message's parts.
///
/// # Arguments
///
/// * `message` - The `Message` object.
/// * `delimiter` - The string to use when joining text from multiple text Parts.
///
/// # Returns
///
/// A single string containing all text content, or an empty string if no text parts are found.
///
/// # Example
///
/// ```
/// use a2a_rs::utils::{new_agent_text_message, get_message_text};
///
/// let message = new_agent_text_message("Hello, world!", None::<String>, None::<String>);
/// let text = get_message_text(&message, "\n");
/// assert_eq!(text, "Hello, world!");
/// ```
pub fn get_message_text(message: &Message, delimiter: &str) -> String {
    get_text_parts(&message.parts).join(delimiter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Role;
    use uuid::Uuid;

    #[test]
    fn test_new_agent_text_message_basic() {
        let message = new_agent_text_message("Hello", None::<String>, None::<String>);
        assert_eq!(message.role, Role::Agent);
        assert_eq!(message.parts.len(), 1);
        assert!(Uuid::parse_str(&message.message_id).is_ok());
        assert!(message.context_id.is_none());
        assert!(message.task_id.is_none());
    }

    #[test]
    fn test_new_agent_parts_message() {
        let parts = vec![Part::Text {
            text: "Test".to_string(),
            metadata: None,
        }];
        let message = new_agent_parts_message(parts, Some("ctx-1"), Some("task-1"));
        assert_eq!(message.role, Role::Agent);
        assert_eq!(message.context_id, Some("ctx-1".to_string()));
        assert_eq!(message.task_id, Some("task-1".to_string()));
    }

    #[test]
    fn test_get_message_text_empty() {
        let message = new_agent_parts_message(vec![], None::<String>, None::<String>);
        assert_eq!(get_message_text(&message, "\n"), "");
    }
}
