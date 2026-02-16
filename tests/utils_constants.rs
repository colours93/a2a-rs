//! Tests for utils::constants module
//! Ported from reference/a2a-python/tests/utils/test_constants.py

use a2a_rs::utils::constants;

#[test]
fn test_agent_card_constants() {
    // Test that agent card constants have expected values
    assert_eq!(
        constants::AGENT_CARD_WELL_KNOWN_PATH,
        "/.well-known/agent-card.json"
    );
    assert_eq!(
        constants::PREV_AGENT_CARD_WELL_KNOWN_PATH,
        "/.well-known/agent.json"
    );
    assert_eq!(
        constants::EXTENDED_AGENT_CARD_PATH,
        "/agent/authenticatedExtendedCard"
    );
}

#[test]
fn test_default_rpc_url() {
    // Test default RPC URL constant
    assert_eq!(constants::DEFAULT_RPC_URL, "/");
}
