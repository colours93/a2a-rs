//! A2A protocol types — complete coverage of the v0.3 protobuf schema.
//!
//! Reference: <https://github.com/a2aproject/A2A/blob/main/specification/a2a.proto>
//! Python SDK reference: <https://github.com/a2aproject/a2a-python/blob/main/src/a2a/types.py>
//!
//! This module implements ALL message types from the proto spec with correct
//! JSON-RPC serialization (matching the Python SDK wire format).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Enums
// ============================================================================

/// The lifecycle state of a task.
///
/// Serialized as SCREAMING_SNAKE_CASE ProtoJSON strings to match the
/// official A2A proto specification (`enum TaskState`).
///
/// Proto ref: `enum TaskState`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskState {
    /// Task has been received but not yet started.
    Submitted,
    /// Task is actively being processed.
    Working,
    /// Task completed successfully.
    Completed,
    /// Task failed.
    Failed,
    /// Task was canceled.
    Canceled,
    /// Task requires additional input from the user.
    InputRequired,
    /// Task was rejected by the agent.
    Rejected,
    /// Task requires authentication.
    AuthRequired,
    /// Unknown state (not in proto, but present in Python SDK for forward compat).
    Unknown,
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TaskState::Submitted => "submitted",
            TaskState::Working => "working",
            TaskState::Completed => "completed",
            TaskState::Failed => "failed",
            TaskState::Canceled => "canceled",
            TaskState::InputRequired => "input-required",
            TaskState::Rejected => "rejected",
            TaskState::AuthRequired => "auth-required",
            TaskState::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

/// The role of a message sender.
///
/// Serialized as SCREAMING_SNAKE_CASE ProtoJSON strings to match the
/// official A2A proto specification (`enum Role`).
///
/// Proto ref: `enum Role`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Message from the user / client.
    User,
    /// Message from the agent / server.
    Agent,
    /// Unspecified role.
    Unspecified,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::User => write!(f, "user"),
            Role::Agent => write!(f, "agent"),
            Role::Unspecified => write!(f, "unspecified"),
        }
    }
}

/// Location for an API key (header, query, cookie).
///
/// Python SDK ref: `In` enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLocation {
    /// API key sent as a cookie.
    Cookie,
    /// API key sent in an HTTP header.
    Header,
    /// API key sent as a query parameter.
    Query,
}

// ============================================================================
// Core Task Types
// ============================================================================

/// Current status of a task.
///
/// Proto ref: `message TaskStatus`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatus {
    /// The current state.
    pub state: TaskState,

    /// Optional message associated with this status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,

    /// ISO-8601 timestamp of when this status was set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// A task — the primary unit of work in the A2A protocol.
///
/// Python SDK ref: `Task` (has `kind: Literal['task'] = 'task'`)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    /// Unique task identifier.
    pub id: String,

    /// Context identifier (groups related tasks/messages).
    pub context_id: String,

    /// Discriminator field — always "task".
    #[serde(default = "kind_task")]
    pub kind: String,

    /// Current task status.
    pub status: TaskStatus,

    /// Artifacts produced by the task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<Artifact>>,

    /// Message history for this task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<Message>>,

    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Type alias for `Task` — used in client APIs for readability.
pub type A2ATask = Task;

// ============================================================================
// Message & Parts
// ============================================================================

/// A single message in a conversation.
///
/// Python SDK ref: `Message` (has `kind: Literal['message'] = 'message'`)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// Unique message identifier.
    pub message_id: String,

    /// Who sent this message.
    pub role: Role,

    /// Discriminator field — always "message".
    #[serde(default = "kind_message")]
    pub kind: String,

    /// Content parts of the message.
    pub parts: Vec<Part>,

    /// Context this message belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,

    /// Task this message is associated with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,

    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// Protocol extensions active for this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,

    /// IDs of tasks referenced by this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_task_ids: Option<Vec<String>>,
}

/// File content provided as base64-encoded bytes.
///
/// Python SDK ref: `FileWithBytes`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileWithBytes {
    /// Base64-encoded file content.
    pub bytes: String,
    /// MIME type of the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Optional file name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// File content provided as a URI reference.
///
/// Python SDK ref: `FileWithUri`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileWithUri {
    /// URI pointing to the file content.
    pub uri: String,
    /// MIME type of the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Optional file name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// File content — either inline bytes or a URI reference.
///
/// Python SDK ref: `FileWithBytes | FileWithUri` (union in FilePart.file)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FileContent {
    /// File with inline base64-encoded bytes.
    Bytes(FileWithBytes),
    /// File referenced by URI.
    Uri(FileWithUri),
}

/// A content part within a message or artifact.
///
/// Discriminated by the `kind` field, matching the Python SDK's
/// `Part(RootModel[TextPart | FilePart | DataPart])`.
///
/// JSON wire format:
/// - Text: `{"kind": "text", "text": "hello"}`
/// - File (bytes): `{"kind": "file", "file": {"bytes": "SGVsbG8=", "mimeType": "text/plain", "name": "hello.txt"}}`
/// - File (uri): `{"kind": "file", "file": {"uri": "https://example.com/file.pdf", "mimeType": "application/pdf"}}`
/// - Data: `{"kind": "data", "data": {"key": "value"}}`
///
/// Python SDK ref: `Part`, `TextPart`, `FilePart`, `DataPart`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Part {
    /// A text content part. Discriminator: `"text"`.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
        /// Optional metadata associated with this part.
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<serde_json::Value>,
    },
    /// A file content part. Discriminator: `"file"`.
    #[serde(rename = "file")]
    File {
        /// The file content (bytes or URI).
        file: FileContent,
        /// Optional metadata associated with this part.
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<serde_json::Value>,
    },
    /// A structured data content part. Discriminator: `"data"`.
    #[serde(rename = "data")]
    Data {
        /// Arbitrary structured data.
        data: serde_json::Value,
        /// Optional metadata associated with this part.
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<serde_json::Value>,
    },
}

/// Legacy type alias — `TextPart` is now an inline variant of [`Part::Text`].
///
/// For pattern matching, use `Part::Text { text, metadata }` instead.
#[deprecated(note = "Use Part::Text { text, metadata } enum variant instead")]
pub type TextPart = ();

/// Legacy type alias — replaced by [`Part::File`] with [`FileContent::Bytes`].
#[deprecated(
    note = "Use Part::File { file: FileContent::Bytes(FileWithBytes { .. }), .. } instead"
)]
pub type RawPart = ();

/// Legacy type alias — replaced by [`Part::File`] with [`FileContent::Uri`].
#[deprecated(note = "Use Part::File { file: FileContent::Uri(FileWithUri { .. }), .. } instead")]
pub type UrlPart = ();

/// Legacy type alias — `DataPart` is now an inline variant of [`Part::Data`].
///
/// For pattern matching, use `Part::Data { data, metadata }` instead.
#[deprecated(note = "Use Part::Data { data, metadata } enum variant instead")]
pub type DataPart = ();

/// An artifact produced by a task.
///
/// Proto ref: `message Artifact`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    /// Unique artifact identifier.
    pub artifact_id: String,

    /// Human-readable name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Description of the artifact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Content parts of the artifact.
    pub parts: Vec<Part>,

    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// Protocol extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
}

// ============================================================================
// Streaming Events
// ============================================================================

/// Notification that a task's status has changed.
///
/// Python SDK ref: `TaskStatusUpdateEvent` (has `kind: Literal['status-update'] = 'status-update'`)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatusUpdateEvent {
    /// ID of the task whose status changed.
    pub task_id: String,

    /// Context this task belongs to.
    pub context_id: String,

    /// Discriminator field — always "status-update".
    #[serde(default = "kind_status_update")]
    pub kind: String,

    /// The new status.
    pub status: TaskStatus,

    /// Whether this is the final status update for this task.
    ///
    /// **IMPORTANT: Proto v0.3 Discrepancy**
    ///
    /// The official A2A protobuf schema v0.3 **removed** the `final` field from
    /// `TaskStatusUpdateEvent` (it's now reserved field 4, indicating deprecation):
    /// - Proto source: https://github.com/a2aproject/A2A/blob/main/specification/a2a.proto
    ///   (see `message TaskStatusUpdateEvent`, field 4 is reserved)
    ///
    /// However, the JavaScript SDK (a2a-js) **still includes** `final: boolean` in its
    /// JSON-RPC serialization for `TaskStatusUpdateEvent`:
    /// - JS SDK source: https://github.com/a2aproject/a2a-js
    ///
    /// **Our implementation follows the JS SDK pattern** for JSON-RPC compatibility.
    /// This field is required (not optional) and is always serialized in JSON responses.
    ///
    /// Python SDK: `final: bool` (REQUIRED, not optional).
    #[serde(rename = "final")]
    pub r#final: bool,

    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Notification that an artifact has been created or updated.
///
/// Python SDK ref: `TaskArtifactUpdateEvent` (has `kind: Literal['artifact-update'] = 'artifact-update'`)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskArtifactUpdateEvent {
    /// ID of the task that produced the artifact.
    pub task_id: String,

    /// Context this task belongs to.
    pub context_id: String,

    /// Discriminator field — always "artifact-update".
    #[serde(default = "kind_artifact_update")]
    pub kind: String,

    /// The artifact.
    pub artifact: Artifact,

    /// Whether to append to an existing artifact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append: Option<bool>,

    /// Whether this is the last chunk of the artifact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_chunk: Option<bool>,

    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

fn kind_task() -> String {
    "task".to_string()
}

fn kind_message() -> String {
    "message".to_string()
}

fn kind_status_update() -> String {
    "status-update".to_string()
}

fn kind_artifact_update() -> String {
    "artifact-update".to_string()
}

fn default_preferred_transport() -> Option<String> {
    Some("JSONRPC".to_string())
}

fn default_protocol_version() -> Option<String> {
    Some("0.3.0".to_string())
}

/// A streaming response payload.
///
/// Python SDK ref: `SendStreamingMessageSuccessResponse.result` is
/// `Task | Message | TaskStatusUpdateEvent | TaskArtifactUpdateEvent`.
///
/// Each inner type has a `kind` discriminator field that identifies it:
/// - `"task"` -> Task
/// - `"message"` -> Message
/// - `"status-update"` -> TaskStatusUpdateEvent
/// - `"artifact-update"` -> TaskArtifactUpdateEvent
///
/// Serializes FLAT (no wrapper keys) — the `kind` field is the discriminator.
#[derive(Debug, Clone)]
pub enum StreamResponse {
    /// A complete task snapshot.
    Task(Task),

    /// A direct message.
    Message(Message),

    /// A task status update event.
    StatusUpdate(TaskStatusUpdateEvent),

    /// An artifact update event.
    ArtifactUpdate(TaskArtifactUpdateEvent),
}

impl Serialize for StreamResponse {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            StreamResponse::Task(inner) => inner.serialize(serializer),
            StreamResponse::Message(inner) => inner.serialize(serializer),
            StreamResponse::StatusUpdate(inner) => inner.serialize(serializer),
            StreamResponse::ArtifactUpdate(inner) => inner.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for StreamResponse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let kind = value
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| serde::de::Error::custom("missing 'kind' field"))?;

        match kind {
            "task" => {
                let task: Task =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(StreamResponse::Task(task))
            }
            "message" => {
                let msg: Message =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(StreamResponse::Message(msg))
            }
            "status-update" => {
                let event: TaskStatusUpdateEvent =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(StreamResponse::StatusUpdate(event))
            }
            "artifact-update" => {
                let event: TaskArtifactUpdateEvent =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(StreamResponse::ArtifactUpdate(event))
            }
            other => Err(serde::de::Error::custom(format!(
                "unknown kind '{}' — expected one of: task, message, status-update, artifact-update",
                other
            ))),
        }
    }
}

// ============================================================================
// Agent Card & Related Types
// ============================================================================

/// Self-describing manifest for an A2A agent.
///
/// Proto ref: `message AgentCard`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCard {
    /// Human-readable name.
    pub name: String,

    /// Description of the agent's capabilities.
    pub description: String,

    /// Agent version string.
    pub version: String,

    /// Supported transport interfaces.
    #[serde(default)]
    pub supported_interfaces: Vec<AgentInterface>,

    /// Service provider information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<AgentProvider>,

    /// URL to the agent's documentation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,

    /// Agent capabilities.
    pub capabilities: AgentCapabilities,

    /// Named security scheme definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_schemes: Option<HashMap<String, SecurityScheme>>,

    /// Security requirements (references to security_schemes).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub security_requirements: Vec<SecurityRequirement>,

    /// Default MIME types accepted as input.
    pub default_input_modes: Vec<String>,

    /// Default MIME types produced as output.
    pub default_output_modes: Vec<String>,

    /// Skills the agent supports.
    pub skills: Vec<AgentSkill>,

    /// JWS signatures for the agent card.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signatures: Option<Vec<AgentCardSignature>>,

    /// URL to the agent's icon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,

    /// Additional interfaces (Python SDK field, not in proto).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_interfaces: Option<Vec<AgentInterface>>,

    /// Preferred transport protocol (e.g. "JSONRPC", "GRPC", "HTTP+JSON").
    /// Defaults to "JSONRPC" per the Python SDK.
    #[serde(
        default = "default_preferred_transport",
        skip_serializing_if = "Option::is_none"
    )]
    pub preferred_transport: Option<String>,

    /// Protocol version — defaults to "0.3.0" per the Python SDK.
    #[serde(
        default = "default_protocol_version",
        skip_serializing_if = "Option::is_none"
    )]
    pub protocol_version: Option<String>,

    /// Primary URL for the agent (required in Python/JS SDKs).
    pub url: String,

    /// Whether the agent supports authenticated extended card.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_authenticated_extended_card: Option<bool>,

    /// Security (Python SDK shorthand — list of scheme-name to scopes mappings).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<Vec<HashMap<String, Vec<String>>>>,
}

/// A transport interface supported by an agent.
///
/// Python SDK ref: `AgentInterface` — uses `transport: str`, NOT `protocolBinding`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInterface {
    /// URL of the interface endpoint.
    pub url: String,

    /// Transport protocol (e.g. "JSONRPC", "HTTP+JSON", "GRPC").
    ///
    /// Python SDK: `transport: str`
    /// JSON: `"transport"`.
    pub transport: String,

    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,

    /// Protocol version (e.g. "0.3"). Optional since Python SDK doesn't require it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
}

/// Agent capabilities declaration.
///
/// Python SDK ref: `AgentCapabilities`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct AgentCapabilities {
    /// Whether the agent supports streaming responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,

    /// Whether the agent supports push notifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_notifications: Option<bool>,

    /// Protocol extensions supported by the agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<AgentExtension>>,

    /// Whether the agent provides a history of state transitions for a task.
    ///
    /// Present in the Python SDK (generated from the JSON schema).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_transition_history: Option<bool>,
}

/// A protocol extension supported by the agent.
///
/// Proto ref: `message AgentExtension`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentExtension {
    /// URI identifying the extension.
    pub uri: String,

    /// Human-readable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether this extension is required.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    /// Extension-specific parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// A skill that an agent can perform.
///
/// Proto ref: `message AgentSkill`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSkill {
    /// Unique skill identifier.
    pub id: String,

    /// Human-readable skill name.
    pub name: String,

    /// Description of what the skill does.
    pub description: String,

    /// Categorization tags.
    pub tags: Vec<String>,

    /// Example prompts/inputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<String>>,

    /// MIME types this skill accepts as input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_modes: Option<Vec<String>>,

    /// MIME types this skill produces as output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_modes: Option<Vec<String>>,

    /// Security requirements for this skill.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_requirements: Option<Vec<SecurityRequirement>>,

    /// Security (Python SDK shorthand).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<Vec<HashMap<String, Vec<String>>>>,
}

/// Information about the agent's provider/organization.
///
/// Proto ref: `message AgentProvider`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProvider {
    /// Organization name.
    pub organization: String,

    /// Organization URL.
    pub url: String,
}

/// JWS signature for an agent card (RFC 7515).
///
/// Proto ref: `message AgentCardSignature`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCardSignature {
    /// Base64url-encoded JWS protected header.
    pub protected: String,

    /// Base64url-encoded JWS signature.
    pub signature: String,

    /// Optional unprotected header.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<serde_json::Value>,
}

// ============================================================================
// Security Schemes
// ============================================================================

/// A security scheme definition.
///
/// Python SDK ref: `SecurityScheme` is a discriminated union using `type` field.
/// Each inner type has `type: Literal['apiKey'] | 'http' | 'oauth2' | 'openIdConnect' | 'mutualTLS'`.
///
/// JSON: `{"type": "apiKey", "in": "header", "name": "X-API-Key"}`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SecurityScheme {
    /// API key authentication.
    #[serde(rename = "apiKey")]
    ApiKey {
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Where the API key is sent (header, query, cookie).
        /// Python SDK: `in_: In` with JSON alias `"in"`.
        #[serde(rename = "in")]
        location: ApiKeyLocation,
        /// Name of the API key parameter.
        name: String,
    },
    /// HTTP authentication (Bearer, Basic, etc.).
    #[serde(rename = "http")]
    Http {
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Authentication scheme name (e.g. "bearer").
        scheme: String,
        /// Format of the bearer token.
        #[serde(skip_serializing_if = "Option::is_none", rename = "bearerFormat")]
        bearer_format: Option<String>,
    },
    /// OAuth 2.0 authentication.
    #[serde(rename = "oauth2")]
    OAuth2 {
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// OAuth 2.0 flow configuration.
        flows: OAuthFlows,
        /// URL for OAuth 2.0 metadata discovery.
        #[serde(skip_serializing_if = "Option::is_none", rename = "oauth2MetadataUrl")]
        oauth2_metadata_url: Option<String>,
    },
    /// OpenID Connect authentication.
    #[serde(rename = "openIdConnect")]
    OpenIdConnect {
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// OpenID Connect discovery URL.
        #[serde(rename = "openIdConnectUrl")]
        open_id_connect_url: String,
    },
    /// Mutual TLS authentication.
    #[serde(rename = "mutualTLS")]
    MutualTls {
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
}

/// OAuth 2.0 flow configurations.
///
/// Python SDK ref: `OAuthFlows` — has authorization_code, client_credentials,
/// implicit, password. No device_code.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct OAuthFlows {
    /// Authorization code flow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<AuthorizationCodeOAuthFlow>,

    /// Client credentials flow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_credentials: Option<ClientCredentialsOAuthFlow>,

    /// Implicit flow (deprecated in OAuth 2.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit: Option<ImplicitOAuthFlow>,

    /// Resource Owner Password flow (deprecated in OAuth 2.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<PasswordOAuthFlow>,
}

/// Authorization code OAuth flow.
///
/// Python SDK ref: `AuthorizationCodeOAuthFlow` — no pkce_required field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationCodeOAuthFlow {
    /// Authorization endpoint URL.
    pub authorization_url: String,

    /// Token endpoint URL.
    pub token_url: String,

    /// Token refresh endpoint URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,

    /// Available scopes (scope name -> description).
    pub scopes: HashMap<String, String>,
}

/// Client credentials OAuth flow.
///
/// Proto ref: `message ClientCredentialsOAuthFlow`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCredentialsOAuthFlow {
    /// Token endpoint URL.
    pub token_url: String,

    /// Token refresh endpoint URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,

    /// Available scopes (scope name -> description).
    pub scopes: HashMap<String, String>,
}

/// Implicit OAuth flow (deprecated in OAuth 2.1).
///
/// Proto ref: `message ImplicitOAuthFlow`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplicitOAuthFlow {
    /// Authorization endpoint URL.
    pub authorization_url: String,

    /// Token refresh endpoint URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,

    /// Available scopes.
    pub scopes: HashMap<String, String>,
}

/// Resource Owner Password OAuth flow (deprecated in OAuth 2.1).
///
/// Proto ref: `message PasswordOAuthFlow`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordOAuthFlow {
    /// Token endpoint URL.
    pub token_url: String,

    /// Token refresh endpoint URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,

    /// Available scopes.
    pub scopes: HashMap<String, String>,
}

/// A security requirement — maps scheme names to required scopes.
///
/// Python SDK: `list[dict[str, list[str]]]` -> JSON: `[{"oauth": ["read", "write"]}]`
///
/// Each entry is a HashMap from scheme name to list of required scopes.
pub type SecurityRequirement = HashMap<String, Vec<String>>;

// ============================================================================
// Push Notifications
// ============================================================================

/// Configuration for push notification delivery.
///
/// Proto ref: `message PushNotificationConfig`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNotificationConfig {
    /// Optional identifier for this config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// URL to deliver notifications to.
    pub url: String,

    /// Optional verification token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,

    /// Authentication configuration for the push endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<PushNotificationAuthenticationInfo>,
}

/// Authentication information for push notification delivery.
///
/// Python SDK ref: `PushNotificationAuthenticationInfo`
/// `schemes: list[str]` (PLURAL), `credentials: str | None`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNotificationAuthenticationInfo {
    /// List of supported authentication schemes (e.g. ["Bearer", "Basic"]).
    /// Python SDK: `schemes: list[str]` (PLURAL).
    pub schemes: Vec<String>,

    /// Optional credentials required by the push notification endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<String>,
}

/// Push notification config bound to a specific task.
///
/// Python SDK ref: `TaskPushNotificationConfig`
/// Proto ref: `message TaskPushNotificationConfig`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPushNotificationConfig {
    /// The id of this config.
    /// Proto: `string id = 1 [REQUIRED]`
    /// NOTE: Python SDK omits this field entirely from TaskPushNotificationConfig.
    /// Made optional for cross-SDK interoperability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Task this config applies to.
    /// Proto: `string task_id = 3 [REQUIRED]`
    pub task_id: String,

    /// The push notification configuration details.
    /// Proto: `PushNotificationConfig push_notification_config = 2 [REQUIRED]`
    pub push_notification_config: PushNotificationConfig,

    /// Optional tenant.
    /// Proto: `string tenant = 4`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

// ============================================================================
// JSON-RPC Foundation
// ============================================================================

/// A JSON-RPC 2.0 request/notification ID.
///
/// Can be a string, number, or null (for notifications).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcId {
    /// String identifier.
    String(String),
    /// Numeric identifier.
    Number(i64),
    /// Null (notification — no response expected).
    Null,
}

impl fmt::Display for JsonRpcId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonRpcId::String(s) => write!(f, "{}", s),
            JsonRpcId::Number(n) => write!(f, "{}", n),
            JsonRpcId::Null => write!(f, "null"),
        }
    }
}

/// A JSON-RPC 2.0 request.
///
/// Used for both requests (with `id`) and notifications (without `id`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcRequest {
    /// Protocol version — always "2.0".
    pub jsonrpc: String,

    /// Request identifier. Absent for notifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<JsonRpcId>,

    /// Method name.
    pub method: String,

    /// Method parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// A JSON-RPC 2.0 response.
///
/// Exactly one of `result` or `error` will be present.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcResponse {
    /// Protocol version — always "2.0".
    pub jsonrpc: String,

    /// Request identifier this response corresponds to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<JsonRpcId>,

    /// Successful result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    /// Error result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a successful JSON-RPC response.
    pub fn success(id: Option<JsonRpcId>, result: serde_json::Value) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error JSON-RPC response.
    pub fn error(id: Option<JsonRpcId>, error: JsonRpcError) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    /// Create a JSON-RPC error response from an [`A2AError`](crate::error::A2AError).
    ///
    /// Automatically maps the error code and message using the
    /// `From<A2AError> for JsonRpcError` conversion.
    pub fn from_a2a_error(id: Option<JsonRpcId>, err: crate::error::A2AError) -> Self {
        let rpc_err: JsonRpcError = err.into();
        Self::error(id, rpc_err)
    }
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcError {
    /// Error code.
    pub code: i64,

    /// Human-readable error message.
    pub message: String,

    /// Optional structured error data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// ============================================================================
// Request / Response Parameter Types
// ============================================================================

/// Parameters for `message/send` and `message/stream`.
///
/// Proto ref: `message SendMessageRequest`
/// Python SDK ref: `MessageSendParams`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageParams {
    /// The message to send.
    pub message: Message,

    /// Optional send configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration: Option<SendMessageConfiguration>,

    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

/// Configuration for a `message/send` request.
///
/// Proto ref: `message SendMessageConfiguration`
/// Python SDK ref: `MessageSendConfiguration`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct SendMessageConfiguration {
    /// MIME types the client can accept as output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_output_modes: Option<Vec<String>>,

    /// Push notification configuration for this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_notification_config: Option<PushNotificationConfig>,

    /// Maximum number of history messages to include in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_length: Option<i32>,

    /// Whether the request should block until the task completes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocking: Option<bool>,
}

/// Parameters for `tasks/get`.
///
/// Proto ref: `message GetTaskRequest`
/// Python SDK ref: `TaskQueryParams`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskParams {
    /// Task ID to retrieve.
    pub id: String,

    /// Maximum number of history messages to include.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_length: Option<i32>,

    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

/// Parameters for `tasks/list`.
///
/// Proto ref: `message ListTasksRequest`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksParams {
    /// Filter by context ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,

    /// Filter by task state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TaskState>,

    /// Maximum number of tasks to return per page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,

    /// Token for paginating through results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,

    /// Maximum number of history messages to include per task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_length: Option<i32>,

    /// Filter by status timestamp (only tasks updated after this time).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_timestamp_after: Option<String>,

    /// Whether to include artifacts in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_artifacts: Option<bool>,

    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

/// Response for `tasks/list`.
///
/// Proto ref: `message ListTasksResponse`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksResponse {
    /// Tasks matching the query.
    pub tasks: Vec<Task>,

    /// Token for retrieving the next page.
    pub next_page_token: String,

    /// Number of tasks in this page.
    pub page_size: i32,

    /// Total number of matching tasks.
    pub total_size: i32,
}

/// Parameters for `tasks/cancel`.
///
/// Proto ref: `message CancelTaskRequest`
/// Python SDK ref: `TaskIdParams`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelTaskParams {
    /// ID of the task to cancel.
    pub id: String,

    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

/// Parameters for `tasks/subscribe` and `tasks/resubscribe`.
///
/// Proto ref: `message SubscribeToTaskRequest`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeToTaskParams {
    /// ID of the task to subscribe to.
    pub id: String,

    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

/// Task ID parameter type used for simple task lookups.
///
/// Python SDK ref: `TaskIdParams`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskIdParams {
    /// The task ID.
    pub id: String,

    /// Arbitrary metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// Push Notification Request/Response Types
// ============================================================================

/// Parameters for creating a push notification config for a task.
///
/// Proto ref: `message CreateTaskPushNotificationConfigRequest`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskPushNotificationConfigParams {
    /// The parent task resource id.
    /// Proto: `string task_id = 1 [REQUIRED]`
    pub task_id: String,

    /// The ID for the new config.
    /// Proto: `string config_id = 2 [REQUIRED]`
    pub config_id: String,

    /// The push notification configuration to create.
    /// Proto: `PushNotificationConfig config = 5 [REQUIRED]`
    pub config: PushNotificationConfig,

    /// Optional tenant, provided as a path parameter.
    /// Proto: `string tenant = 4`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

/// Backward-compat alias for `CreateTaskPushNotificationConfigParams`.
pub type SetTaskPushNotificationConfigParams = CreateTaskPushNotificationConfigParams;

/// Parameters for getting a push notification config for a task.
///
/// Proto ref: `message GetTaskPushNotificationConfigRequest`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskPushNotificationConfigParams {
    /// The unique identifier (e.g. UUID) of the task.
    /// Matches Python SDK: `id: str`
    pub id: String,

    /// The ID of the push notification configuration to retrieve.
    /// Matches Python SDK: `push_notification_config_id: str | None`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_notification_config_id: Option<String>,

    /// Optional metadata associated with the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Parameters for listing push notification configs for a task.
///
/// Proto ref: `message ListTaskPushNotificationConfigRequest`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTaskPushNotificationConfigParams {
    /// The unique identifier (e.g. UUID) of the task.
    /// Matches Python SDK: `id: str`
    pub id: String,

    /// Optional metadata associated with the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Response for listing push notification configs.
///
/// Proto ref: `message ListTaskPushNotificationConfigResponse`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTaskPushNotificationConfigResponse {
    /// The list of push notification configurations.
    /// Proto: `repeated TaskPushNotificationConfig configs = 1`
    pub configs: Vec<TaskPushNotificationConfig>,

    /// A token for retrieving the next page. Omitted if no more pages.
    /// Proto: `string next_page_token = 2`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

/// Parameters for deleting a push notification config for a task.
///
/// Proto ref: `message DeleteTaskPushNotificationConfigRequest`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTaskPushNotificationConfigParams {
    /// The unique identifier (e.g. UUID) of the task.
    /// Matches Python SDK: `id: str`
    pub id: String,

    /// The ID of the push notification configuration to delete.
    /// Matches Python SDK: `push_notification_config_id: str`
    pub push_notification_config_id: String,

    /// Optional metadata associated with the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Parameters for getting the extended agent card.
///
/// Proto ref: `message GetExtendedAgentCardRequest`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetExtendedAgentCardParams {
    /// Optional tenant.
    /// Proto: `string tenant = 1`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

// ============================================================================
// SendMessageResponse (oneof: Task | Message)
// ============================================================================

/// Response payload for `message/send`.
///
/// Python SDK ref: `SendMessageSuccessResponse.result` is `Task | Message`.
///
/// Each inner type has a `kind` discriminator:
/// - `"task"` -> Task
/// - `"message"` -> Message
///
/// Serializes FLAT (no wrapper keys).
#[derive(Debug, Clone)]
pub enum SendMessageResponse {
    /// A task was created/updated.
    Task(Task),

    /// A direct message response.
    Message(Message),
}

impl Serialize for SendMessageResponse {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            SendMessageResponse::Task(inner) => inner.serialize(serializer),
            SendMessageResponse::Message(inner) => inner.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for SendMessageResponse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let kind = value
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| serde::de::Error::custom("missing 'kind' field"))?;

        match kind {
            "task" => {
                let task: Task = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(SendMessageResponse::Task(task))
            }
            "message" => {
                let msg: Message =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(SendMessageResponse::Message(msg))
            }
            other => Err(serde::de::Error::custom(format!(
                "unknown kind '{}' — expected one of: task, message",
                other
            ))),
        }
    }
}

// ============================================================================
// Convenience Constructors
// ============================================================================

impl Part {
    /// Create a text part.
    ///
    /// Produces JSON: `{"kind": "text", "text": "..."}`
    pub fn text(text: impl Into<String>) -> Self {
        Part::Text {
            text: text.into(),
            metadata: None,
        }
    }

    /// Create a file part from base64-encoded bytes.
    ///
    /// Produces JSON: `{"kind": "file", "file": {"bytes": "...", "mimeType": "...", "name": "..."}}`
    pub fn file_from_bytes(
        bytes: impl Into<String>,
        name: Option<String>,
        mime_type: Option<String>,
    ) -> Self {
        Part::File {
            file: FileContent::Bytes(FileWithBytes {
                bytes: bytes.into(),
                mime_type,
                name,
            }),
            metadata: None,
        }
    }

    /// Create a file part from a URI reference.
    ///
    /// Produces JSON: `{"kind": "file", "file": {"uri": "...", "mimeType": "...", "name": "..."}}`
    pub fn file_from_uri(
        uri: impl Into<String>,
        name: Option<String>,
        mime_type: Option<String>,
    ) -> Self {
        Part::File {
            file: FileContent::Uri(FileWithUri {
                uri: uri.into(),
                mime_type,
                name,
            }),
            metadata: None,
        }
    }

    /// Create a structured data part.
    ///
    /// Produces JSON: `{"kind": "data", "data": {...}}`
    pub fn data(data: serde_json::Value) -> Self {
        Part::Data {
            data,
            metadata: None,
        }
    }
}

impl Message {
    /// Create a new user message with text content.
    pub fn user(message_id: impl Into<String>, text: impl Into<String>) -> Self {
        Message {
            message_id: message_id.into(),
            role: Role::User,
            kind: kind_message(),
            parts: vec![Part::text(text)],
            context_id: None,
            task_id: None,
            metadata: None,
            extensions: None,
            reference_task_ids: None,
        }
    }

    /// Create a new agent message with text content.
    pub fn agent(message_id: impl Into<String>, text: impl Into<String>) -> Self {
        Message {
            message_id: message_id.into(),
            role: Role::Agent,
            kind: kind_message(),
            parts: vec![Part::text(text)],
            context_id: None,
            task_id: None,
            metadata: None,
            extensions: None,
            reference_task_ids: None,
        }
    }
}

impl TaskStatus {
    /// Create a new TaskStatus with the given state and no message.
    pub fn new(state: TaskState) -> Self {
        TaskStatus {
            state,
            message: None,
            timestamp: None,
        }
    }

    /// Create a new TaskStatus with the given state and an ISO-8601 timestamp.
    pub fn with_timestamp(state: TaskState, timestamp: impl Into<String>) -> Self {
        TaskStatus {
            state,
            message: None,
            timestamp: Some(timestamp.into()),
        }
    }
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC 2.0 request.
    pub fn new(
        id: impl Into<JsonRpcId>,
        method: impl Into<String>,
        params: Option<serde_json::Value>,
    ) -> Self {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(id.into()),
            method: method.into(),
            params,
        }
    }

    /// Create a JSON-RPC 2.0 notification (no id, no response expected).
    pub fn notification(method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: method.into(),
            params,
        }
    }
}

impl From<String> for JsonRpcId {
    fn from(s: String) -> Self {
        JsonRpcId::String(s)
    }
}

impl From<&str> for JsonRpcId {
    fn from(s: &str) -> Self {
        JsonRpcId::String(s.to_string())
    }
}

impl From<i64> for JsonRpcId {
    fn from(n: i64) -> Self {
        JsonRpcId::Number(n)
    }
}

impl From<i32> for JsonRpcId {
    fn from(n: i32) -> Self {
        JsonRpcId::Number(n as i64)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---- ProtoJSON serialization tests for TaskState ----

    #[test]
    fn task_state_serialization() {
        assert_eq!(
            serde_json::to_string(&TaskState::Submitted).unwrap(),
            r#""submitted""#
        );
        assert_eq!(
            serde_json::to_string(&TaskState::Working).unwrap(),
            r#""working""#
        );
        assert_eq!(
            serde_json::to_string(&TaskState::Completed).unwrap(),
            r#""completed""#
        );
        assert_eq!(
            serde_json::to_string(&TaskState::Failed).unwrap(),
            r#""failed""#
        );
        assert_eq!(
            serde_json::to_string(&TaskState::Canceled).unwrap(),
            r#""canceled""#
        );
        assert_eq!(
            serde_json::to_string(&TaskState::InputRequired).unwrap(),
            r#""input-required""#
        );
        assert_eq!(
            serde_json::to_string(&TaskState::Rejected).unwrap(),
            r#""rejected""#
        );
        assert_eq!(
            serde_json::to_string(&TaskState::AuthRequired).unwrap(),
            r#""auth-required""#
        );
        assert_eq!(
            serde_json::to_string(&TaskState::Unknown).unwrap(),
            r#""unknown""#
        );
    }

    #[test]
    fn task_state_deserialization() {
        let state: TaskState = serde_json::from_str(r#""input-required""#).unwrap();
        assert_eq!(state, TaskState::InputRequired);

        let state: TaskState = serde_json::from_str(r#""working""#).unwrap();
        assert_eq!(state, TaskState::Working);

        let state: TaskState = serde_json::from_str(r#""unknown""#).unwrap();
        assert_eq!(state, TaskState::Unknown);
    }

    #[test]
    fn task_state_display() {
        assert_eq!(TaskState::Submitted.to_string(), "submitted");
        assert_eq!(TaskState::InputRequired.to_string(), "input-required");
        assert_eq!(TaskState::AuthRequired.to_string(), "auth-required");
        assert_eq!(TaskState::Unknown.to_string(), "unknown");
    }

    // ---- ProtoJSON serialization tests for Role ----

    #[test]
    fn role_serialization() {
        assert_eq!(serde_json::to_string(&Role::User).unwrap(), r#""user""#);
        assert_eq!(serde_json::to_string(&Role::Agent).unwrap(), r#""agent""#);
        assert_eq!(
            serde_json::to_string(&Role::Unspecified).unwrap(),
            r#""unspecified""#
        );
    }

    #[test]
    fn role_deserialization() {
        let role: Role = serde_json::from_str(r#""user""#).unwrap();
        assert_eq!(role, Role::User);

        let role: Role = serde_json::from_str(r#""agent""#).unwrap();
        assert_eq!(role, Role::Agent);

        let role: Role = serde_json::from_str(r#""unspecified""#).unwrap();
        assert_eq!(role, Role::Unspecified);
    }

    #[test]
    fn role_display() {
        assert_eq!(Role::User.to_string(), "user");
        assert_eq!(Role::Agent.to_string(), "agent");
        assert_eq!(Role::Unspecified.to_string(), "unspecified");
    }

    // ---- Part serialization tests (Python SDK — "kind" discriminated union) ----

    #[test]
    fn text_part_serialization() {
        let part = Part::text("Hello");
        let json = serde_json::to_value(&part).unwrap();
        // Python SDK: {"kind": "text", "text": "Hello"}
        assert_eq!(json["kind"], "text");
        assert_eq!(json["text"], "Hello");
    }

    #[test]
    fn text_part_roundtrip() {
        let part = Part::text("hello world");
        let json = serde_json::to_value(&part).unwrap();
        // Has "kind" field — Python SDK tagged union
        assert_eq!(json["kind"], "text");
        assert_eq!(json["text"], "hello world");

        let decoded: Part = serde_json::from_value(json).unwrap();
        match decoded {
            Part::Text { text, .. } => assert_eq!(text, "hello world"),
            _ => panic!("expected Text part"),
        }
    }

    #[test]
    fn file_uri_part_serialization() {
        let part = Part::file_from_uri(
            "https://example.com/file.pdf",
            None,
            Some("application/pdf".to_string()),
        );
        let json = serde_json::to_value(&part).unwrap();
        // Python SDK: {"kind": "file", "file": {"uri": "...", "mimeType": "..."}}
        assert_eq!(json["kind"], "file");
        assert!(json.get("file").is_some());
        assert_eq!(json["file"]["uri"], "https://example.com/file.pdf");
        assert_eq!(json["file"]["mimeType"], "application/pdf");
    }

    #[test]
    fn file_uri_part_with_name() {
        let part = Part::File {
            file: FileContent::Uri(FileWithUri {
                uri: "https://example.com/image.png".to_string(),
                name: Some("diagram.png".to_string()),
                mime_type: Some("image/png".to_string()),
            }),
            metadata: None,
        };
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["kind"], "file");
        assert_eq!(json["file"]["uri"], "https://example.com/image.png");
        assert_eq!(json["file"]["name"], "diagram.png");
        assert_eq!(json["file"]["mimeType"], "image/png");
    }

    #[test]
    fn file_uri_part_roundtrip() {
        let part = Part::file_from_uri(
            "https://example.com/file.pdf",
            None,
            Some("application/pdf".to_string()),
        );
        let json = serde_json::to_value(&part).unwrap();
        let decoded: Part = serde_json::from_value(json).unwrap();
        match decoded {
            Part::File {
                file: FileContent::Uri(f),
                ..
            } => {
                assert_eq!(f.uri, "https://example.com/file.pdf");
                assert_eq!(f.mime_type, Some("application/pdf".to_string()));
            }
            _ => panic!("expected File(Uri) part"),
        }
    }

    #[test]
    fn file_bytes_part_serialization() {
        let part = Part::file_from_bytes(
            "SGVsbG8=",
            Some("hello.txt".to_string()),
            Some("text/plain".to_string()),
        );
        let json = serde_json::to_value(&part).unwrap();
        // Python SDK: {"kind": "file", "file": {"bytes": "...", "name": "...", "mimeType": "..."}}
        assert_eq!(json["kind"], "file");
        assert_eq!(json["file"]["bytes"], "SGVsbG8=");
        assert_eq!(json["file"]["name"], "hello.txt");
        assert_eq!(json["file"]["mimeType"], "text/plain");
    }

    #[test]
    fn file_bytes_part_roundtrip() {
        let part = Part::file_from_bytes(
            "SGVsbG8=",
            Some("hello.txt".to_string()),
            Some("text/plain".to_string()),
        );
        let json = serde_json::to_value(&part).unwrap();
        let decoded: Part = serde_json::from_value(json).unwrap();
        match decoded {
            Part::File {
                file: FileContent::Bytes(f),
                ..
            } => {
                assert_eq!(f.bytes, "SGVsbG8=");
                assert_eq!(f.name, Some("hello.txt".to_string()));
                assert_eq!(f.mime_type, Some("text/plain".to_string()));
            }
            _ => panic!("expected File(Bytes) part"),
        }
    }

    #[test]
    fn data_part_serialization() {
        let part = Part::data(json!({"key": "value"}));
        let json_val = serde_json::to_value(&part).unwrap();
        // Python SDK: {"kind": "data", "data": {"key": "value"}}
        assert_eq!(json_val["kind"], "data");
        assert_eq!(json_val["data"]["key"], "value");
    }

    #[test]
    fn data_part_roundtrip() {
        let part = Part::data(json!({"key": "value"}));
        let json_val = serde_json::to_value(&part).unwrap();
        let decoded: Part = serde_json::from_value(json_val).unwrap();
        match decoded {
            Part::Data { data, .. } => assert_eq!(data["key"], "value"),
            _ => panic!("expected Data part"),
        }
    }

    #[test]
    fn text_part_with_metadata() {
        let part = Part::Text {
            text: "hello".to_string(),
            metadata: Some(json!({"source": "test"})),
        };
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["kind"], "text");
        assert_eq!(json["text"], "hello");
        assert_eq!(json["metadata"]["source"], "test");
    }

    #[test]
    fn text_part_has_kind_field() {
        // Python SDK: Part JSON MUST contain a "kind" discriminator
        let part = Part::text("test");
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["kind"], "text", "Part JSON must contain 'kind' field");
    }

    #[test]
    fn message_serialization() {
        let msg = Message::user("m1", "Hello, agent!");
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["messageId"], "m1");
        assert_eq!(json["role"], "user");
        // Python SDK: Message has "kind": "message"
        assert_eq!(json["kind"], "message");
        assert_eq!(json["parts"][0]["text"], "Hello, agent!");
    }

    #[test]
    fn task_serialization() {
        let task = Task {
            id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_task(),
            status: TaskStatus::new(TaskState::Working),
            artifacts: None,
            history: None,
            metadata: None,
        };
        let json = serde_json::to_value(&task).unwrap();
        assert_eq!(json["id"], "t1");
        assert_eq!(json["contextId"], "ctx1");
        assert_eq!(json["status"]["state"], "working");
        // Python SDK: Task has "kind": "task"
        assert_eq!(json["kind"], "task");
        // None fields should be omitted
        assert!(json.get("artifacts").is_none());
        assert!(json.get("history").is_none());
    }

    #[test]
    fn task_status_update_event_final_field() {
        // Python SDK: final is required bool
        let event = TaskStatusUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_status_update(),
            status: TaskStatus::new(TaskState::Completed),
            r#final: true,
            metadata: None,
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["final"], true);
        assert_eq!(json["kind"], "status-update");

        // When final is false, it should still be present
        let event_false = TaskStatusUpdateEvent {
            task_id: "t2".to_string(),
            context_id: "ctx2".to_string(),
            kind: kind_status_update(),
            status: TaskStatus::new(TaskState::Working),
            r#final: false,
            metadata: None,
        };
        let json = serde_json::to_value(&event_false).unwrap();
        assert_eq!(json["final"], false);
    }

    #[test]
    fn json_rpc_request() {
        let req = JsonRpcRequest::new(1i64, "message/send", Some(json!({"message": {}})));
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 1);
        assert_eq!(json["method"], "message/send");
    }

    #[test]
    fn json_rpc_response_success() {
        let resp = JsonRpcResponse::success(Some(JsonRpcId::Number(1)), json!({"id": "t1"}));
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 1);
        assert!(json["result"].is_object());
        assert!(json.get("error").is_none());
    }

    #[test]
    fn json_rpc_response_error() {
        let err = JsonRpcError {
            code: -32001,
            message: "Task not found".to_string(),
            data: None,
        };
        let resp = JsonRpcResponse::error(Some(JsonRpcId::Number(1)), err);
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["error"]["code"], -32001);
        assert!(json.get("result").is_none());
    }

    #[test]
    fn json_rpc_id_variants() {
        let id_str: JsonRpcId = "abc".into();
        assert_eq!(serde_json::to_string(&id_str).unwrap(), "\"abc\"");

        let id_num: JsonRpcId = 42i64.into();
        assert_eq!(serde_json::to_string(&id_num).unwrap(), "42");

        let id_null = JsonRpcId::Null;
        assert_eq!(serde_json::to_string(&id_null).unwrap(), "null");
    }

    #[test]
    fn security_scheme_api_key_roundtrip() {
        let scheme = SecurityScheme::ApiKey {
            description: None,
            location: ApiKeyLocation::Header,
            name: "X-API-Key".to_string(),
        };
        let json = serde_json::to_value(&scheme).unwrap();
        // Python SDK: {"type": "apiKey", "in": "header", "name": "X-API-Key"}
        assert_eq!(json["type"], "apiKey");
        assert_eq!(json["in"], "header");
        assert_eq!(json["name"], "X-API-Key");

        let decoded: SecurityScheme = serde_json::from_value(json).unwrap();
        match decoded {
            SecurityScheme::ApiKey { name, location, .. } => {
                assert_eq!(name, "X-API-Key");
                assert_eq!(location, ApiKeyLocation::Header);
            }
            _ => panic!("expected ApiKey"),
        }
    }

    #[test]
    fn security_scheme_oauth2_roundtrip() {
        let scheme = SecurityScheme::OAuth2 {
            description: Some("OAuth2 auth".to_string()),
            flows: OAuthFlows {
                authorization_code: Some(AuthorizationCodeOAuthFlow {
                    authorization_url: "https://auth.example.com/authorize".to_string(),
                    token_url: "https://auth.example.com/token".to_string(),
                    refresh_url: None,
                    scopes: HashMap::from([("read".to_string(), "Read access".to_string())]),
                }),
                client_credentials: None,
                implicit: None,
                password: None,
            },
            oauth2_metadata_url: None,
        };
        let json = serde_json::to_value(&scheme).unwrap();
        assert_eq!(json["type"], "oauth2");
        assert!(json["flows"]["authorizationCode"]["authorizationUrl"].is_string());
    }

    #[test]
    fn security_scheme_mutual_tls() {
        let scheme = SecurityScheme::MutualTls {
            description: Some("mTLS".to_string()),
        };
        let json = serde_json::to_value(&scheme).unwrap();
        assert_eq!(json["type"], "mutualTLS");
        assert_eq!(json["description"], "mTLS");
    }

    #[test]
    fn agent_card_minimal() {
        let card = AgentCard {
            name: "Test Agent".to_string(),
            description: "A test agent".to_string(),
            version: "1.0.0".to_string(),
            url: "http://localhost:8080/a2a".to_string(),
            supported_interfaces: vec![AgentInterface {
                url: "http://localhost:8080/a2a".to_string(),
                transport: "JSONRPC".to_string(),
                tenant: None,
                protocol_version: Some("0.3".to_string()),
            }],
            provider: None,
            documentation_url: None,
            capabilities: AgentCapabilities {
                streaming: Some(true),
                push_notifications: None,
                extensions: None,
                state_transition_history: None,
            },
            security_schemes: None,
            security_requirements: vec![],
            default_input_modes: vec!["text/plain".to_string()],
            default_output_modes: vec!["text/plain".to_string()],
            skills: vec![AgentSkill {
                id: "code".to_string(),
                name: "Code Generation".to_string(),
                description: "Generates code".to_string(),
                tags: vec!["coding".to_string()],
                examples: None,
                input_modes: None,
                output_modes: None,
                security_requirements: None,
                security: None,
            }],
            signatures: None,
            icon_url: None,
            additional_interfaces: None,
            preferred_transport: None,
            protocol_version: None,
            supports_authenticated_extended_card: None,
            security: None,
        };
        let json = serde_json::to_value(&card).unwrap();
        assert_eq!(json["name"], "Test Agent");
        assert_eq!(json["url"], "http://localhost:8080/a2a");
        // Python SDK: uses "transport" not "protocolBinding"
        assert_eq!(json["supportedInterfaces"][0]["transport"], "JSONRPC");
        assert_eq!(json["capabilities"]["streaming"], true);
        assert_eq!(json["skills"][0]["id"], "code");
    }

    #[test]
    fn artifact_serialization() {
        let artifact = Artifact {
            artifact_id: "a1".to_string(),
            name: Some("output.rs".to_string()),
            description: None,
            parts: vec![Part::text("fn main() {}")],
            metadata: None,
            extensions: None,
        };
        let json = serde_json::to_value(&artifact).unwrap();
        assert_eq!(json["artifactId"], "a1");
        assert_eq!(json["name"], "output.rs");
        assert_eq!(json["parts"][0]["text"], "fn main() {}");
    }

    #[test]
    fn send_message_params() {
        let params = SendMessageParams {
            message: Message::user("m1", "Hello"),
            configuration: Some(SendMessageConfiguration {
                accepted_output_modes: Some(vec!["text/plain".to_string()]),
                push_notification_config: None,
                history_length: Some(10),
                blocking: Some(true),
            }),
            metadata: None,
            tenant: None,
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["message"]["messageId"], "m1");
        assert_eq!(json["configuration"]["blocking"], true);
        assert_eq!(json["configuration"]["historyLength"], 10);
    }

    #[test]
    fn list_tasks_response() {
        let resp = ListTasksResponse {
            tasks: vec![],
            next_page_token: "".to_string(),
            page_size: 10,
            total_size: 0,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["pageSize"], 10);
        assert_eq!(json["totalSize"], 0);
    }

    #[test]
    fn push_notification_config_roundtrip() {
        let config = PushNotificationConfig {
            id: Some("pnc-1".to_string()),
            url: "https://example.com/webhook".to_string(),
            token: Some("secret-token".to_string()),
            authentication: Some(PushNotificationAuthenticationInfo {
                schemes: vec!["Bearer".to_string()],
                credentials: Some("my-cred".to_string()),
            }),
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["url"], "https://example.com/webhook");
        // Python SDK: plural "schemes" as list
        assert_eq!(json["authentication"]["schemes"], json!(["Bearer"]));

        let decoded: PushNotificationConfig = serde_json::from_value(json).unwrap();
        assert_eq!(decoded.url, "https://example.com/webhook");
    }

    #[test]
    fn a2a_task_is_task_alias() {
        let task: A2ATask = Task {
            id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_task(),
            status: TaskStatus::new(TaskState::Submitted),
            artifacts: None,
            history: None,
            metadata: None,
        };
        // A2ATask and Task are the same type.
        let _: Task = task;
    }

    #[test]
    fn stream_response_status_update_pattern_match() {
        let event = TaskStatusUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_status_update(),
            status: TaskStatus::new(TaskState::Working),
            r#final: false,
            metadata: None,
        };
        let sr = StreamResponse::StatusUpdate(event);

        match &sr {
            StreamResponse::StatusUpdate(update) => {
                assert_eq!(update.task_id, "t1");
                assert_eq!(update.r#final, false);
            }
            _ => panic!("expected StatusUpdate"),
        }
    }

    #[test]
    fn stream_response_artifact_update_pattern_match() {
        let event = TaskArtifactUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_artifact_update(),
            artifact: Artifact {
                artifact_id: "a1".to_string(),
                name: None,
                description: None,
                parts: vec![Part::text("content")],
                metadata: None,
                extensions: None,
            },
            append: None,
            last_chunk: None,
            metadata: None,
        };
        let sr = StreamResponse::ArtifactUpdate(event);

        match &sr {
            StreamResponse::ArtifactUpdate(update) => {
                assert_eq!(update.artifact.artifact_id, "a1");
            }
            _ => panic!("expected ArtifactUpdate"),
        }
    }

    #[test]
    fn stream_response_task_pattern_match() {
        let task = Task {
            id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_task(),
            status: TaskStatus::new(TaskState::Completed),
            artifacts: None,
            history: None,
            metadata: None,
        };
        let sr = StreamResponse::Task(task);

        match &sr {
            StreamResponse::Task(t) => {
                assert_eq!(t.id, "t1");
            }
            _ => panic!("expected Task"),
        }
    }

    #[test]
    fn stream_response_message_pattern_match() {
        let msg = Message::agent("m1", "Hello!");
        let sr = StreamResponse::Message(msg);

        match &sr {
            StreamResponse::Message(m) => {
                assert_eq!(m.message_id, "m1");
            }
            _ => panic!("expected Message"),
        }
    }

    // ---- Python SDK kind-based serialization tests ----

    #[test]
    fn stream_response_task_serialization() {
        let task = Task {
            id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_task(),
            status: TaskStatus::new(TaskState::Working),
            artifacts: None,
            history: None,
            metadata: None,
        };
        let sr = StreamResponse::Task(task);
        let json = serde_json::to_value(&sr).unwrap();

        // Python SDK: flat with "kind": "task"
        assert_eq!(json["kind"], "task");
        assert_eq!(json["id"], "t1");
        assert_eq!(json["contextId"], "ctx1");
    }

    #[test]
    fn stream_response_task_roundtrip() {
        let task = Task {
            id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_task(),
            status: TaskStatus::new(TaskState::Working),
            artifacts: None,
            history: None,
            metadata: None,
        };
        let sr = StreamResponse::Task(task);
        let json = serde_json::to_value(&sr).unwrap();

        let decoded: StreamResponse = serde_json::from_value(json).unwrap();
        match decoded {
            StreamResponse::Task(t) => {
                assert_eq!(t.id, "t1");
            }
            _ => panic!("expected Task"),
        }
    }

    #[test]
    fn stream_response_message_serialization() {
        let msg = Message::user("m1", "Hello!");
        let sr = StreamResponse::Message(msg);
        let json = serde_json::to_value(&sr).unwrap();

        // Python SDK: flat with "kind": "message"
        assert_eq!(json["kind"], "message");
        assert_eq!(json["messageId"], "m1");
    }

    #[test]
    fn stream_response_message_roundtrip() {
        let msg = Message::user("m1", "Hello!");
        let sr = StreamResponse::Message(msg);
        let json = serde_json::to_value(&sr).unwrap();

        let decoded: StreamResponse = serde_json::from_value(json).unwrap();
        match decoded {
            StreamResponse::Message(m) => {
                assert_eq!(m.message_id, "m1");
            }
            _ => panic!("expected Message"),
        }
    }

    #[test]
    fn stream_response_status_update_serialization() {
        let event = TaskStatusUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_status_update(),
            status: TaskStatus::new(TaskState::Completed),
            r#final: true,
            metadata: None,
        };
        let sr = StreamResponse::StatusUpdate(event);
        let json = serde_json::to_value(&sr).unwrap();

        // Python SDK: flat with "kind": "status-update"
        assert_eq!(json["kind"], "status-update");
        assert_eq!(json["taskId"], "t1");
        assert_eq!(json["final"], true);
    }

    #[test]
    fn stream_response_status_update_roundtrip() {
        let event = TaskStatusUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_status_update(),
            status: TaskStatus::new(TaskState::Completed),
            r#final: true,
            metadata: None,
        };
        let sr = StreamResponse::StatusUpdate(event);
        let json = serde_json::to_value(&sr).unwrap();

        let decoded: StreamResponse = serde_json::from_value(json).unwrap();
        match decoded {
            StreamResponse::StatusUpdate(e) => {
                assert_eq!(e.task_id, "t1");
                assert_eq!(e.r#final, true);
            }
            _ => panic!("expected StatusUpdate"),
        }
    }

    #[test]
    fn stream_response_artifact_update_serialization() {
        let event = TaskArtifactUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_artifact_update(),
            artifact: Artifact {
                artifact_id: "a1".to_string(),
                name: Some("test.txt".to_string()),
                description: None,
                parts: vec![Part::text("content")],
                metadata: None,
                extensions: None,
            },
            append: Some(false),
            last_chunk: Some(true),
            metadata: None,
        };
        let sr = StreamResponse::ArtifactUpdate(event);
        let json = serde_json::to_value(&sr).unwrap();

        // Python SDK: flat with "kind": "artifact-update"
        assert_eq!(json["kind"], "artifact-update");
        assert_eq!(json["taskId"], "t1");
        assert_eq!(json["lastChunk"], true);
        assert_eq!(json["artifact"]["artifactId"], "a1");
    }

    #[test]
    fn stream_response_artifact_update_roundtrip() {
        let event = TaskArtifactUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_artifact_update(),
            artifact: Artifact {
                artifact_id: "a1".to_string(),
                name: Some("test.txt".to_string()),
                description: None,
                parts: vec![Part::text("content")],
                metadata: None,
                extensions: None,
            },
            append: Some(false),
            last_chunk: Some(true),
            metadata: None,
        };
        let sr = StreamResponse::ArtifactUpdate(event);
        let json = serde_json::to_value(&sr).unwrap();

        let decoded: StreamResponse = serde_json::from_value(json).unwrap();
        match decoded {
            StreamResponse::ArtifactUpdate(e) => {
                assert_eq!(e.task_id, "t1");
                assert_eq!(e.artifact.artifact_id, "a1");
            }
            _ => panic!("expected ArtifactUpdate"),
        }
    }

    #[test]
    fn send_message_response_task_serialization() {
        let task = Task {
            id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_task(),
            status: TaskStatus::new(TaskState::Submitted),
            artifacts: None,
            history: None,
            metadata: None,
        };
        let resp = SendMessageResponse::Task(task);
        let json = serde_json::to_value(&resp).unwrap();

        // Python SDK: flat with "kind": "task"
        assert_eq!(json["kind"], "task");
        assert_eq!(json["id"], "t1");
    }

    #[test]
    fn send_message_response_task_roundtrip() {
        let task = Task {
            id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_task(),
            status: TaskStatus::new(TaskState::Submitted),
            artifacts: None,
            history: None,
            metadata: None,
        };
        let resp = SendMessageResponse::Task(task);
        let json = serde_json::to_value(&resp).unwrap();

        let decoded: SendMessageResponse = serde_json::from_value(json).unwrap();
        match decoded {
            SendMessageResponse::Task(t) => assert_eq!(t.id, "t1"),
            _ => panic!("expected Task"),
        }
    }

    #[test]
    fn send_message_response_message_serialization() {
        let msg = Message::agent("m1", "Response text");
        let resp = SendMessageResponse::Message(msg);
        let json = serde_json::to_value(&resp).unwrap();

        // Python SDK: flat with "kind": "message"
        assert_eq!(json["kind"], "message");
        assert_eq!(json["messageId"], "m1");
    }

    #[test]
    fn send_message_response_message_roundtrip() {
        let msg = Message::agent("m1", "Response text");
        let resp = SendMessageResponse::Message(msg);
        let json = serde_json::to_value(&resp).unwrap();

        let decoded: SendMessageResponse = serde_json::from_value(json).unwrap();
        match decoded {
            SendMessageResponse::Message(m) => assert_eq!(m.message_id, "m1"),
            _ => panic!("expected Message"),
        }
    }

    #[test]
    fn stream_response_has_kind_in_output() {
        // Verify the Python SDK requirement: kind field present in output JSON
        let task = Task {
            id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_task(),
            status: TaskStatus::new(TaskState::Working),
            artifacts: None,
            history: None,
            metadata: None,
        };
        let json_str = serde_json::to_string(&StreamResponse::Task(task)).unwrap();
        assert!(
            json_str.contains("\"kind\":\"task\""),
            "JSON must contain kind: {}",
            json_str
        );

        let msg = Message::agent("m1", "Hi");
        let json_str = serde_json::to_string(&StreamResponse::Message(msg)).unwrap();
        assert!(
            json_str.contains("\"kind\":\"message\""),
            "JSON must contain kind: {}",
            json_str
        );

        let event = TaskStatusUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx1".to_string(),
            kind: kind_status_update(),
            status: TaskStatus::new(TaskState::Working),
            r#final: false,
            metadata: None,
        };
        let json_str = serde_json::to_string(&StreamResponse::StatusUpdate(event)).unwrap();
        assert!(
            json_str.contains("\"kind\":\"status-update\""),
            "JSON must contain kind: {}",
            json_str
        );
    }

    #[test]
    fn task_deserialize_without_kind() {
        // Task can be deserialized from JSON without a "kind" field (default applied)
        let json = json!({
            "id": "t1",
            "contextId": "ctx1",
            "status": { "state": "working" }
        });
        let task: Task = serde_json::from_value(json).unwrap();
        assert_eq!(task.id, "t1");
        assert_eq!(task.kind, "task");
    }

    #[test]
    fn message_deserialize_without_kind() {
        // Message can be deserialized from JSON without a "kind" field (default applied)
        let json = json!({
            "messageId": "m1",
            "role": "user",
            "parts": [{"kind": "text", "text": "hello"}]
        });
        let msg: Message = serde_json::from_value(json).unwrap();
        assert_eq!(msg.message_id, "m1");
        assert_eq!(msg.kind, "message");
    }

    // ====================================================================
    // Python SDK compliance tests
    // ====================================================================

    // --- SecurityScheme uses "type" tag ---

    #[test]
    fn security_scheme_api_key_python_format() {
        let scheme = SecurityScheme::ApiKey {
            description: Some("Test".to_string()),
            location: ApiKeyLocation::Header,
            name: "X-Key".to_string(),
        };
        let json = serde_json::to_value(&scheme).unwrap();

        // Python: {"type": "apiKey", "in": "header", "name": "X-Key"}
        assert_eq!(json["type"], "apiKey");
        assert_eq!(json["in"], "header");
        assert_eq!(json["name"], "X-Key");

        let decoded: SecurityScheme = serde_json::from_value(json).unwrap();
        match &decoded {
            SecurityScheme::ApiKey { name, location, .. } => {
                assert_eq!(*name, "X-Key");
                assert_eq!(*location, ApiKeyLocation::Header);
            }
            _ => panic!("Expected ApiKey variant"),
        }
    }

    #[test]
    fn security_scheme_http_python_format() {
        let scheme = SecurityScheme::Http {
            description: None,
            scheme: "bearer".to_string(),
            bearer_format: Some("JWT".to_string()),
        };
        let json = serde_json::to_value(&scheme).unwrap();

        assert_eq!(json["type"], "http");
        assert_eq!(json["scheme"], "bearer");
        assert_eq!(json["bearerFormat"], "JWT");

        let decoded: SecurityScheme = serde_json::from_value(json).unwrap();
        match &decoded {
            SecurityScheme::Http {
                scheme,
                bearer_format,
                ..
            } => {
                assert_eq!(*scheme, "bearer");
                assert_eq!(*bearer_format, Some("JWT".to_string()));
            }
            _ => panic!("Expected Http variant"),
        }
    }

    #[test]
    fn security_scheme_oauth2_python_format() {
        let scheme = SecurityScheme::OAuth2 {
            description: None,
            flows: OAuthFlows::default(),
            oauth2_metadata_url: Some("https://example.com/.well-known/oauth".to_string()),
        };
        let json = serde_json::to_value(&scheme).unwrap();

        assert_eq!(json["type"], "oauth2");
        assert_eq!(
            json["oauth2MetadataUrl"],
            "https://example.com/.well-known/oauth"
        );

        let decoded: SecurityScheme = serde_json::from_value(json).unwrap();
        match &decoded {
            SecurityScheme::OAuth2 {
                oauth2_metadata_url,
                ..
            } => {
                assert!(oauth2_metadata_url.is_some());
            }
            _ => panic!("Expected OAuth2 variant"),
        }
    }

    #[test]
    fn security_scheme_openid_python_format() {
        let scheme = SecurityScheme::OpenIdConnect {
            description: None,
            open_id_connect_url: "https://example.com/.well-known/openid-configuration".to_string(),
        };
        let json = serde_json::to_value(&scheme).unwrap();

        assert_eq!(json["type"], "openIdConnect");
        assert!(json["openIdConnectUrl"].is_string());

        let decoded: SecurityScheme = serde_json::from_value(json).unwrap();
        match &decoded {
            SecurityScheme::OpenIdConnect {
                open_id_connect_url,
                ..
            } => {
                assert!(open_id_connect_url.contains("openid-configuration"));
            }
            _ => panic!("Expected OpenIdConnect variant"),
        }
    }

    #[test]
    fn security_scheme_mtls_python_format() {
        let scheme = SecurityScheme::MutualTls {
            description: Some("Mutual TLS".to_string()),
        };
        let json = serde_json::to_value(&scheme).unwrap();

        assert_eq!(json["type"], "mutualTLS");
        assert_eq!(json["description"], "Mutual TLS");

        let decoded: SecurityScheme = serde_json::from_value(json).unwrap();
        match &decoded {
            SecurityScheme::MutualTls { description } => {
                assert_eq!(*description, Some("Mutual TLS".to_string()));
            }
            _ => panic!("Expected MutualTls variant"),
        }
    }

    // --- API key uses "in" in JSON (matching Python SDK) ---

    #[test]
    fn api_key_security_scheme_in_field() {
        let scheme = SecurityScheme::ApiKey {
            description: None,
            location: ApiKeyLocation::Query,
            name: "api_key".to_string(),
        };
        let json = serde_json::to_value(&scheme).unwrap();

        // Python SDK: uses "in" for location
        assert_eq!(json["in"], "query");
        assert_eq!(json["name"], "api_key");
        assert_eq!(json["type"], "apiKey");
    }

    // --- AgentInterface uses "transport" ---

    #[test]
    fn agent_interface_transport_field() {
        let iface = AgentInterface {
            url: "https://example.com/a2a".to_string(),
            transport: "JSONRPC".to_string(),
            tenant: None,
            protocol_version: Some("0.3".to_string()),
        };
        let json = serde_json::to_value(&iface).unwrap();

        // Python SDK: uses "transport", not "protocolBinding"
        assert_eq!(json["transport"], "JSONRPC");
        assert!(
            json.get("protocolBinding").is_none(),
            "Must not use 'protocolBinding' field name"
        );
        // Tenant is None so should be absent
        assert!(json.get("tenant").is_none());

        let decoded: AgentInterface = serde_json::from_value(json).unwrap();
        assert_eq!(decoded.transport, "JSONRPC");
    }

    #[test]
    fn agent_interface_with_tenant() {
        let iface = AgentInterface {
            url: "https://example.com/a2a".to_string(),
            transport: "GRPC".to_string(),
            tenant: Some("my-tenant".to_string()),
            protocol_version: Some("0.3".to_string()),
        };
        let json = serde_json::to_value(&iface).unwrap();

        assert_eq!(json["transport"], "GRPC");
        assert_eq!(json["tenant"], "my-tenant");

        let decoded: AgentInterface = serde_json::from_value(json).unwrap();
        assert_eq!(decoded.tenant, Some("my-tenant".to_string()));
    }

    // --- PushNotificationAuthenticationInfo uses plural "schemes" ---

    #[test]
    fn push_notification_auth_plural_schemes() {
        let auth = PushNotificationAuthenticationInfo {
            schemes: vec!["Bearer".to_string()],
            credentials: Some("my-token".to_string()),
        };
        let json = serde_json::to_value(&auth).unwrap();

        // Python SDK: plural "schemes" as list
        assert_eq!(json["schemes"], json!(["Bearer"]));
        assert!(
            json.get("scheme").is_none(),
            "Must not use singular 'scheme'"
        );
        assert_eq!(json["credentials"], "my-token");

        let decoded: PushNotificationAuthenticationInfo = serde_json::from_value(json).unwrap();
        assert_eq!(decoded.schemes, vec!["Bearer"]);
        assert_eq!(decoded.credentials, Some("my-token".to_string()));
    }

    #[test]
    fn push_notification_auth_no_credentials() {
        let auth = PushNotificationAuthenticationInfo {
            schemes: vec!["Basic".to_string()],
            credentials: None,
        };
        let json = serde_json::to_value(&auth).unwrap();

        assert_eq!(json["schemes"], json!(["Basic"]));
        assert!(
            json.get("credentials").is_none(),
            "None credentials should be omitted"
        );
    }

    // --- SecurityRequirement is flat HashMap ---

    #[test]
    fn security_requirement_python_format() {
        let req: SecurityRequirement = HashMap::from([(
            "oauth".to_string(),
            vec!["read".to_string(), "write".to_string()],
        )]);
        let json = serde_json::to_value(&req).unwrap();

        // Python SDK: {"oauth": ["read", "write"]}
        assert_eq!(json["oauth"], json!(["read", "write"]));

        let decoded: SecurityRequirement = serde_json::from_value(json).unwrap();
        assert_eq!(decoded["oauth"], vec!["read", "write"]);
    }

    #[test]
    fn security_requirement_multiple_schemes() {
        let req: SecurityRequirement = HashMap::from([
            ("oauth".to_string(), vec!["read".to_string()]),
            ("apiKey".to_string(), vec![]),
        ]);
        let json = serde_json::to_value(&req).unwrap();

        assert_eq!(json["oauth"], json!(["read"]));
        assert_eq!(json["apiKey"], json!([]));

        let decoded: SecurityRequirement = serde_json::from_value(json).unwrap();
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded["apiKey"].len(), 0);
    }
}
