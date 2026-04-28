//! Shared transport helpers for the three LLM provider impls.
//!
//! Keeps timeout policy, agent construction, error mapping, and body
//! truncation in one place so the per-provider modules stay focused on
//! request shape and response parsing.

use std::time::Duration;

use crate::llm::LlmProviderError;

const CONNECT_TIMEOUT_SECS: u64 = 10;
const READ_TIMEOUT_SECS: u64 = 10;
const ERROR_BODY_MAX_CHARS: usize = 500;

/// Standard ureq agent for any LLM provider call. Identical across all
/// three impls; centralized so timeout / future proxy tweaks land once.
pub(super) fn build_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .timeout_read(Duration::from_secs(READ_TIMEOUT_SECS))
        .build()
}

/// Map a ureq error into our normalized provider error.
///   Status(code, resp)  → Http(code, truncated body)
///   Transport(timeout)  → Timeout
///   Transport(other)    → Network(msg)
pub(super) fn map_ureq_error(e: ureq::Error) -> LlmProviderError {
    match e {
        ureq::Error::Status(code, response) => {
            let body = response.into_string().unwrap_or_default();
            LlmProviderError::Http(code, truncate_body(body))
        }
        ureq::Error::Transport(t) => map_transport_message(&t.to_string()),
    }
}

/// Pure helper for the Transport arm: classify the message string into
/// Timeout vs Network. Split out so unit tests can exercise the
/// classification without constructing a `ureq::Error::Transport`.
pub(super) fn map_transport_message(s: &str) -> LlmProviderError {
    if s.contains("timed out") || s.contains("timeout") {
        LlmProviderError::Timeout
    } else {
        LlmProviderError::Network(s.to_string())
    }
}

pub(super) fn truncate_body(s: String) -> String {
    if s.chars().count() <= ERROR_BODY_MAX_CHARS {
        s
    } else {
        s.chars().take(ERROR_BODY_MAX_CHARS).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_transport_message_recognizes_timed_out() {
        let e = map_transport_message("connection timed out after 10s");
        match e {
            LlmProviderError::Timeout => {}
            other => panic!("expected Timeout, got {other:?}"),
        }
    }

    #[test]
    fn map_transport_message_recognizes_timeout_word() {
        let e = map_transport_message("timeout occurred");
        match e {
            LlmProviderError::Timeout => {}
            other => panic!("expected Timeout, got {other:?}"),
        }
    }

    #[test]
    fn map_transport_message_other_message_becomes_network() {
        let e = map_transport_message("dns lookup failed");
        match e {
            LlmProviderError::Network(s) => assert_eq!(s, "dns lookup failed"),
            other => panic!("expected Network, got {other:?}"),
        }
    }

    #[test]
    fn truncate_body_short_string_returned_unchanged() {
        assert_eq!(truncate_body("short".to_string()), "short");
    }

    #[test]
    fn truncate_body_caps_at_500_chars() {
        let long = "a".repeat(600);
        let got = truncate_body(long);
        assert_eq!(got.chars().count(), 500);
    }

    #[test]
    fn build_agent_is_constructable() {
        // Smoke: just confirm the agent constructs without panic.
        let _agent = build_agent();
    }
}
