//! Agent card discovery and resolution.
//!
//! Implements the well-known URI convention for discovering A2A agent cards.
//! An agent card describes the agent's capabilities, supported interfaces,
//! skills, and the endpoint URL for JSON-RPC communication.

use crate::error::{A2AError, A2AResult};
use crate::types::AgentCard;

/// Default path for the agent card well-known endpoint (A2A v0.3+).
const DEFAULT_AGENT_CARD_PATH: &str = "/.well-known/agent-card.json";

/// Previous well-known path (pre-v0.3 compat).
const PREV_AGENT_CARD_PATH: &str = "/.well-known/agent.json";

/// Resolves [`AgentCard`]s from agent base URLs.
///
/// Fetches the agent card from the well-known endpoint
/// (`{base_url}/.well-known/agent.json`) and deserializes it into an
/// [`AgentCard`].
///
/// # Example
///
/// ```no_run
/// use a2a_rs::client::CardResolver;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let resolver = CardResolver::new();
/// let card = resolver.resolve("http://localhost:7420").await?;
/// println!("Agent: {} v{}", card.name, card.version);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct CardResolver {
    client: reqwest::Client,
    /// Override the default agent card path. If `None`, uses
    /// `/.well-known/agent.json`.
    card_path: Option<String>,
}

impl CardResolver {
    /// Create a new resolver with default settings.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            card_path: None,
        }
    }

    /// Create a new resolver with an existing `reqwest::Client`.
    pub fn with_client(client: reqwest::Client) -> Self {
        Self {
            client,
            card_path: None,
        }
    }

    /// Override the agent card path (instead of `/.well-known/agent.json`).
    pub fn with_card_path(mut self, path: impl Into<String>) -> Self {
        self.card_path = Some(path.into());
        self
    }

    /// Fetch and parse the agent card from the given base URL.
    ///
    /// Constructs the full URL as `{base_url}{card_path}` and performs an
    /// HTTP GET request. The response is parsed as JSON into an [`AgentCard`].
    ///
    /// When using the default card path, this method first tries the new
    /// `/.well-known/agent-card.json` path. If that returns a 404, it falls
    /// back to the previous `/.well-known/agent.json` path for backwards
    /// compatibility.
    ///
    /// # Errors
    ///
    /// Returns [`A2AError::Transport`] on connection failures, [`A2AError::Http`]
    /// on non-2xx responses, and [`A2AError::InvalidJson`] on parse failures.
    pub async fn resolve(&self, base_url: &str) -> A2AResult<AgentCard> {
        let base = base_url.trim_end_matches('/');

        if self.card_path.is_some() {
            // Custom path — try it directly, no fallback.
            let path = self.card_path.as_deref().unwrap();
            return self.fetch_card(base, path).await;
        }

        // Try the new well-known path first.
        match self.fetch_card(base, DEFAULT_AGENT_CARD_PATH).await {
            Ok(card) => Ok(card),
            Err(A2AError::Http { status: 404, .. }) => {
                // Fall back to the previous well-known path.
                tracing::debug!(
                    "agent card not found at {}{}, trying fallback path {}",
                    base,
                    DEFAULT_AGENT_CARD_PATH,
                    PREV_AGENT_CARD_PATH,
                );
                self.fetch_card(base, PREV_AGENT_CARD_PATH).await
            }
            Err(e) => Err(e),
        }
    }

    /// Fetch and parse an agent card from a specific path relative to a base URL.
    async fn fetch_card(&self, base: &str, path: &str) -> A2AResult<AgentCard> {
        // Ensure path starts with '/'.
        let path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };

        let url = format!("{base}{path}");

        tracing::debug!("resolving agent card from {}", url);

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    A2AError::Transport(format!("failed to connect to agent at {url}: {e}"))
                } else if e.is_timeout() {
                    A2AError::Timeout(format!("timed out fetching agent card from {url}: {e}"))
                } else {
                    A2AError::Transport(format!("failed to fetch agent card from {url}: {e}"))
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(A2AError::Http {
                status: status.as_u16(),
                body,
            });
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| A2AError::Transport(format!("failed to read agent card response: {e}")))?;

        let card: AgentCard = serde_json::from_slice(&bytes)
            .map_err(|e| A2AError::InvalidJson(format!("failed to parse agent card: {e}")))?;

        tracing::debug!("resolved agent card: {} v{}", card.name, card.version);

        Ok(card)
    }

    /// Extract the A2A endpoint URL from an agent card.
    ///
    /// Looks for the first `SupportedInterface` with `transport` of
    /// `"JSONRPC"` (case-insensitive) and returns its URL.
    ///
    /// Returns `None` if no JSON-RPC interface is found.
    pub fn get_a2a_url(card: &AgentCard) -> Option<String> {
        card.supported_interfaces
            .iter()
            .find(|iface| iface.transport.eq_ignore_ascii_case("JSONRPC"))
            .map(|iface| iface.url.clone())
    }
}

impl Default for CardResolver {
    fn default() -> Self {
        Self::new()
    }
}
