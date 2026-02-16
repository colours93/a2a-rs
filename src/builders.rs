//! Builder patterns for ergonomic construction of A2A types.

use crate::types::*;
use std::collections::HashMap;

/// Builder for constructing [`AgentCard`] with sensible defaults.
///
/// # Example
///
/// ```
/// use a2a_rs::builders::AgentCardBuilder;
///
/// let card = AgentCardBuilder::new("My Agent", "An example agent", "1.0.0")
///     .with_jsonrpc_interface("http://localhost:8080/a2a")
///     .with_skill("chat", "Chat", "Conversational AI", vec!["conversation".to_string()])
///     .with_streaming(true)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct AgentCardBuilder {
    name: String,
    description: String,
    version: String,
    supported_interfaces: Vec<AgentInterface>,
    provider: Option<AgentProvider>,
    documentation_url: Option<String>,
    capabilities: AgentCapabilities,
    security_schemes: Option<HashMap<String, SecurityScheme>>,
    security_requirements: Vec<SecurityRequirement>,
    default_input_modes: Vec<String>,
    default_output_modes: Vec<String>,
    skills: Vec<AgentSkill>,
    signatures: Option<Vec<AgentCardSignature>>,
    icon_url: Option<String>,
    additional_interfaces: Option<Vec<AgentInterface>>,
    preferred_transport: Option<String>,
    protocol_version: Option<String>,
    url: String,
    supports_authenticated_extended_card: Option<bool>,
    security: Option<Vec<HashMap<String, Vec<String>>>>,
}

impl AgentCardBuilder {
    /// Create a new builder with required fields.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable agent name
    /// * `description` - Description of agent capabilities
    /// * `version` - Version string (e.g., "1.0.0")
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            version: version.into(),
            supported_interfaces: Vec::new(),
            provider: None,
            documentation_url: None,
            capabilities: AgentCapabilities {
                streaming: None,
                push_notifications: None,
                extensions: None,
                state_transition_history: None,
            },
            security_schemes: None,
            security_requirements: Vec::new(),
            default_input_modes: vec!["text/plain".to_string()],
            default_output_modes: vec!["text/plain".to_string()],
            skills: Vec::new(),
            signatures: None,
            icon_url: None,
            additional_interfaces: None,
            preferred_transport: None,
            protocol_version: Some("0.3".to_string()),
            url: String::new(),
            supports_authenticated_extended_card: None,
            security: None,
        }
    }

    /// Add a JSON-RPC interface at the given URL.
    pub fn with_jsonrpc_interface(mut self, url: impl Into<String>) -> Self {
        let url_str = url.into();
        self.supported_interfaces.push(AgentInterface {
            url: url_str.clone(),
            transport: "JSONRPC".to_string(),
            tenant: None,
            protocol_version: Some("0.3".to_string()),
        });
        if self.url.is_empty() {
            self.url = url_str;
        }
        self
    }

    /// Add a custom interface.
    pub fn with_interface(mut self, interface: AgentInterface) -> Self {
        self.supported_interfaces.push(interface);
        self
    }

    /// Set the provider information.
    pub fn with_provider(
        mut self,
        organization: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        self.provider = Some(AgentProvider {
            organization: organization.into(),
            url: url.into(),
        });
        self
    }

    /// Set the documentation URL.
    pub fn with_documentation_url(mut self, url: impl Into<String>) -> Self {
        self.documentation_url = Some(url.into());
        self
    }

    /// Enable or disable streaming support.
    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.capabilities.streaming = Some(enabled);
        self
    }

    /// Enable or disable push notifications support.
    pub fn with_push_notifications(mut self, enabled: bool) -> Self {
        self.capabilities.push_notifications = Some(enabled);
        self
    }

    /// Add a protocol extension.
    pub fn with_extension(
        mut self,
        uri: impl Into<String>,
        description: Option<String>,
        required: bool,
    ) -> Self {
        self.capabilities
            .extensions
            .get_or_insert_with(Vec::new)
            .push(AgentExtension {
                uri: uri.into(),
                description,
                required: Some(required),
                params: None,
            });
        self
    }

    /// Add a skill to the agent card.
    pub fn with_skill(
        mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        tags: Vec<String>,
    ) -> Self {
        self.skills.push(AgentSkill {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            tags,
            examples: None,
            input_modes: None,
            output_modes: None,
            security_requirements: None,
            security: None,
        });
        self
    }

    /// Add a skill with examples.
    pub fn with_skill_examples(
        mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        tags: Vec<String>,
        examples: Vec<String>,
    ) -> Self {
        self.skills.push(AgentSkill {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            tags,
            examples: Some(examples),
            input_modes: None,
            output_modes: None,
            security_requirements: None,
            security: None,
        });
        self
    }

    /// Set the default input MIME types.
    pub fn with_input_modes(mut self, modes: Vec<String>) -> Self {
        self.default_input_modes = modes;
        self
    }

    /// Set the default output MIME types.
    pub fn with_output_modes(mut self, modes: Vec<String>) -> Self {
        self.default_output_modes = modes;
        self
    }

    /// Set the icon URL.
    pub fn with_icon_url(mut self, url: impl Into<String>) -> Self {
        self.icon_url = Some(url.into());
        self
    }

    /// Set the preferred transport protocol.
    pub fn with_preferred_transport(mut self, transport: impl Into<String>) -> Self {
        self.preferred_transport = Some(transport.into());
        self
    }

    /// Build the [`AgentCard`].
    pub fn build(self) -> AgentCard {
        AgentCard {
            name: self.name,
            description: self.description,
            version: self.version,
            supported_interfaces: self.supported_interfaces,
            provider: self.provider,
            documentation_url: self.documentation_url,
            capabilities: self.capabilities,
            security_schemes: self.security_schemes,
            security_requirements: self.security_requirements,
            default_input_modes: self.default_input_modes,
            default_output_modes: self.default_output_modes,
            skills: self.skills,
            signatures: self.signatures,
            icon_url: self.icon_url,
            additional_interfaces: self.additional_interfaces,
            preferred_transport: self.preferred_transport,
            protocol_version: self.protocol_version,
            url: self.url,
            supports_authenticated_extended_card: self.supports_authenticated_extended_card,
            security: self.security,
        }
    }
}

/// Builder for constructing [`crate::client::A2AClient`] with custom configuration.
///
/// # Example
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// use a2a_rs::builders::ClientBuilder;
/// use std::time::Duration;
///
/// let client = ClientBuilder::new("http://localhost:7420")
///     .with_timeout(Duration::from_secs(30))
///     .build()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "client")]
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    url: String,
    timeout: Option<std::time::Duration>,
    headers: HashMap<String, String>,
}

#[cfg(feature = "client")]
impl ClientBuilder {
    /// Create a new client builder for the given base URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            timeout: None,
            headers: HashMap::new(),
        }
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Add a custom HTTP header.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Add an Authorization header with a bearer token.
    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", token.into()),
        );
        self
    }

    /// Add an API key header.
    pub fn with_api_key(
        mut self,
        header_name: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        self.headers.insert(header_name.into(), api_key.into());
        self
    }

    /// Build the client by resolving the agent card and creating the transport.
    pub async fn build(self) -> crate::A2AResult<crate::client::A2AClient> {
        use crate::client::{A2AClient, CardResolver, JsonRpcTransport};

        // Resolve the agent card.
        let resolver = CardResolver::new();
        let card = resolver.resolve(&self.url).await?;

        // Extract the JSON-RPC endpoint.
        let endpoint_url = CardResolver::get_a2a_url(&card).ok_or_else(|| {
            crate::error::A2AError::Transport(format!(
                "agent card for '{}' has no JSONRPC interface",
                card.name
            ))
        })?;

        // Create a custom transport with the configuration.
        let mut transport = JsonRpcTransport::new(endpoint_url);

        // Apply timeout if specified.
        if let Some(timeout) = self.timeout {
            transport = transport.with_timeout(timeout);
        }

        // Apply headers.
        for (key, value) in self.headers {
            transport = transport.with_header(&key, &value);
        }

        Ok(A2AClient::with_transport(Box::new(transport)))
    }

    /// Build a client from a direct endpoint URL (skip agent card resolution).
    pub fn build_from_endpoint(self) -> crate::client::A2AClient {
        use crate::client::{A2AClient, JsonRpcTransport};

        let mut transport = JsonRpcTransport::new(&self.url);

        if let Some(timeout) = self.timeout {
            transport = transport.with_timeout(timeout);
        }

        for (key, value) in self.headers {
            transport = transport.with_header(&key, &value);
        }

        A2AClient::with_transport(Box::new(transport))
    }
}

/// Builder for constructing an A2A axum server with fluent configuration.
///
/// # Example
///
/// ```rust,ignore
/// use a2a_rs::builders::ServerBuilder;
/// use a2a_rs::server::{AgentExecutor, InMemoryTaskStore};
/// use std::sync::Arc;
///
/// # async fn example(executor: Arc<dyn AgentExecutor>) {
/// let app = ServerBuilder::new(executor)
///     .with_agent_card(|builder| {
///         builder
///             .with_jsonrpc_interface("http://localhost:8080/a2a")
///             .with_skill("chat", "Chat", "Conversational AI", vec!["conversation"])
///             .with_streaming(true)
///     })
///     .with_task_store(Arc::new(InMemoryTaskStore::new()))
///     .with_cors(true)
///     .build();
///
/// // Serve with axum.
/// axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
///     .serve(app.into_make_service())
///     .await
///     .unwrap();
/// # }
/// ```
#[cfg(feature = "server")]
pub struct ServerBuilder {
    executor: std::sync::Arc<dyn crate::server::AgentExecutor>,
    task_store: Option<std::sync::Arc<dyn crate::server::TaskStore>>,
    agent_card: Option<AgentCard>,
    cors_enabled: bool,
}

#[cfg(feature = "server")]
impl ServerBuilder {
    /// Create a new server builder with the given agent executor.
    pub fn new(executor: std::sync::Arc<dyn crate::server::AgentExecutor>) -> Self {
        Self {
            executor,
            task_store: None,
            agent_card: None,
            cors_enabled: false,
        }
    }

    /// Set the task store implementation.
    pub fn with_task_store(mut self, store: std::sync::Arc<dyn crate::server::TaskStore>) -> Self {
        self.task_store = Some(store);
        self
    }

    /// Configure the agent card using a builder callback.
    pub fn with_agent_card<F>(mut self, f: F) -> Self
    where
        F: FnOnce(AgentCardBuilder) -> AgentCardBuilder,
    {
        let builder = AgentCardBuilder::new("A2A Agent", "An A2A-compatible agent", "1.0.0");
        let builder = f(builder);
        self.agent_card = Some(builder.build());
        self
    }

    /// Set the agent card directly.
    pub fn with_agent_card_direct(mut self, card: AgentCard) -> Self {
        self.agent_card = Some(card);
        self
    }

    /// Enable or disable CORS middleware.
    pub fn with_cors(mut self, enabled: bool) -> Self {
        self.cors_enabled = enabled;
        self
    }

    /// Build the axum router.
    pub fn build(self) -> axum::Router {
        use crate::server::{a2a_router, DefaultRequestHandler, InMemoryTaskStore};
        use std::sync::Arc;

        let store = self
            .task_store
            .unwrap_or_else(|| Arc::new(InMemoryTaskStore::new()));
        let handler = Arc::new(DefaultRequestHandler::new(self.executor, store));
        let card = self.agent_card.unwrap_or_else(|| {
            AgentCardBuilder::new("A2A Agent", "An A2A-compatible agent", "1.0.0").build()
        });

        let mut router = a2a_router(handler, card);

        if self.cors_enabled {
            use tower_http::cors::CorsLayer;
            router = router.layer(CorsLayer::permissive());
        }

        router
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_card_builder_basic() {
        let card = AgentCardBuilder::new("Test Agent", "A test", "1.0.0")
            .with_jsonrpc_interface("http://localhost:8080/a2a")
            .build();

        assert_eq!(card.name, "Test Agent");
        assert_eq!(card.description, "A test");
        assert_eq!(card.version, "1.0.0");
        assert_eq!(card.supported_interfaces.len(), 1);
        assert_eq!(card.supported_interfaces[0].transport, "JSONRPC");
    }

    #[test]
    fn agent_card_builder_with_skills() {
        let card = AgentCardBuilder::new("Test", "Test", "1.0.0")
            .with_skill(
                "chat",
                "Chat",
                "Chat skill",
                vec!["conversation".to_string()],
            )
            .with_skill(
                "code",
                "Code",
                "Code generation",
                vec!["coding".to_string()],
            )
            .build();

        assert_eq!(card.skills.len(), 2);
        assert_eq!(card.skills[0].id, "chat");
        assert_eq!(card.skills[1].id, "code");
    }

    #[test]
    fn agent_card_builder_with_capabilities() {
        let card = AgentCardBuilder::new("Test", "Test", "1.0.0")
            .with_streaming(true)
            .with_push_notifications(false)
            .build();

        assert_eq!(card.capabilities.streaming, Some(true));
        assert_eq!(card.capabilities.push_notifications, Some(false));
    }

    #[cfg(feature = "client")]
    #[test]
    fn client_builder_basic() {
        let builder = ClientBuilder::new("http://localhost:8080")
            .with_timeout(std::time::Duration::from_secs(30))
            .with_bearer_token("test-token");

        assert_eq!(builder.url, "http://localhost:8080");
        assert_eq!(builder.timeout, Some(std::time::Duration::from_secs(30)));
        assert_eq!(
            builder.headers.get("Authorization"),
            Some(&"Bearer test-token".to_string())
        );
    }
}
