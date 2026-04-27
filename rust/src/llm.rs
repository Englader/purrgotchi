//! Shared HTTP + API surface for talking to the Anthropic Messages API.
//!
//! Used by `react.rs` (per-turn quips) and `hatch.rs` (one-shot personality
//! blurb). Every error variant is non-fatal at the call site; callers log via
//! `debug::log` and fall back to static behaviour.

use std::env;
use std::time::Duration;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const HAIKU_MODEL: &str = "claude-haiku-4-5";
const CONNECT_TIMEOUT_SECS: u64 = 10;
const READ_TIMEOUT_SECS: u64 = 10;

#[derive(Debug)]
pub struct HaikuRequest {
    pub system: String,
    pub user: String,
    pub max_tokens: u32,
}

#[derive(Debug)]
pub struct HaikuResponse {
    pub text: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub latency_ms: u64,
}

#[derive(Debug)]
pub enum LlmError {
    MissingKey,
    Network(String),
    Http(u16),
    MalformedResponse(String),
    Timeout,
}

/// Reads `ANTHROPIC_API_KEY` from env. Returns `None` for missing or empty.
pub fn api_key() -> Option<String> {
    match env::var("ANTHROPIC_API_KEY") {
        Ok(s) if !s.is_empty() => Some(s),
        _ => None,
    }
}

/// Parse the Anthropic Messages API response JSON into a `HaikuResponse`.
/// Pure; no I/O. The `latency_ms` field is set by the caller after the round-trip.
pub fn parse_haiku_response(body: &str) -> Result<(String, u32, u32), LlmError> {
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| LlmError::MalformedResponse(format!("json: {e}")))?;
    // Error envelope: {"type":"error","error":{"type":"...","message":"..."}}
    if v.get("type").and_then(|t| t.as_str()) == Some("error") {
        let msg = v.pointer("/error/message").and_then(|m| m.as_str()).unwrap_or("unknown");
        return Err(LlmError::MalformedResponse(format!("api error: {msg}")));
    }
    let text = v.pointer("/content/0/text")
        .and_then(|t| t.as_str())
        .ok_or_else(|| LlmError::MalformedResponse("missing /content/0/text".into()))?
        .to_string();
    let input = v.pointer("/usage/input_tokens")
        .and_then(|t| t.as_u64()).unwrap_or(0) as u32;
    let output = v.pointer("/usage/output_tokens")
        .and_then(|t| t.as_u64()).unwrap_or(0) as u32;
    Ok((text, input, output))
}

#[allow(dead_code)] // wire call lands in T2b
pub fn call_haiku(_req: HaikuRequest) -> Result<HaikuResponse, LlmError> {
    Err(LlmError::Network("not yet implemented".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_response() {
        let body = r#"{
            "id":"msg_01",
            "type":"message",
            "content":[{"type":"text","text":"mrow."}],
            "usage":{"input_tokens":1234,"output_tokens":5}
        }"#;
        let (text, in_tok, out_tok) = parse_haiku_response(body).unwrap();
        assert_eq!(text, "mrow.");
        assert_eq!(in_tok, 1234);
        assert_eq!(out_tok, 5);
    }

    #[test]
    fn parse_error_envelope_is_error() {
        let body = r#"{"type":"error","error":{"type":"invalid_request_error","message":"bad key"}}"#;
        match parse_haiku_response(body) {
            Err(LlmError::MalformedResponse(msg)) => assert!(msg.contains("bad key")),
            other => panic!("expected MalformedResponse, got {other:?}"),
        }
    }

    #[test]
    fn parse_missing_content_is_error() {
        let body = r#"{"id":"msg_01","type":"message","content":[]}"#;
        assert!(matches!(parse_haiku_response(body), Err(LlmError::MalformedResponse(_))));
    }

    #[test]
    fn parse_truncated_json_is_error() {
        let body = r#"{"content":[{"text":"hi"#;
        assert!(matches!(parse_haiku_response(body), Err(LlmError::MalformedResponse(_))));
    }

    #[test]
    fn parse_missing_usage_defaults_to_zero() {
        let body = r#"{"content":[{"type":"text","text":"hi"}]}"#;
        let (text, in_tok, out_tok) = parse_haiku_response(body).unwrap();
        assert_eq!(text, "hi");
        assert_eq!(in_tok, 0);
        assert_eq!(out_tok, 0);
    }

    #[test]
    fn api_key_returns_none_for_empty() {
        // Note: this test relies on env var not being set during `cargo test`.
        // If a developer has ANTHROPIC_API_KEY set globally, this test will skip.
        if env::var("ANTHROPIC_API_KEY").is_ok() {
            return; // skip
        }
        assert!(api_key().is_none());
    }
}

// Silence the unused `Duration` import until T2b lands the wire call.
#[allow(dead_code)]
fn _hold_imports(_: Duration) {}
