//! Constants for well-known URIs used throughout the A2A Rust SDK.

/// The well-known path for the agent card (v0.3+ of A2A spec)
pub const AGENT_CARD_WELL_KNOWN_PATH: &str = "/.well-known/agent-card.json";

/// The previous well-known path for the agent card (deprecated, but still supported)
pub const PREV_AGENT_CARD_WELL_KNOWN_PATH: &str = "/.well-known/agent.json";

/// The path for the authenticated extended agent card
pub const EXTENDED_AGENT_CARD_PATH: &str = "/agent/authenticatedExtendedCard";

/// The default RPC URL path
pub const DEFAULT_RPC_URL: &str = "/";
