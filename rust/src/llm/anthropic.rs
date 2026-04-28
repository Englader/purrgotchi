//! Anthropic Messages API provider.
//!
//! Implements `LlmProvider` against `https://api.anthropic.com/v1/messages`.
//! Reads `ANTHROPIC_API_KEY` from env. Default model is `claude-haiku-4-5`;
//! caller can override via `Anthropic::new(Some(model))`.

use std::env;

use super::{LlmProvider, LlmProviderError, LlmRequest, LlmResponse};
use super::transport;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
pub const ANTHROPIC_DEFAULT_MODEL: &str = "claude-haiku-4-5";

pub struct Anthropic {
    model: String,
}

impl Anthropic {
    /// Construct with an optional model override. `None` → default.
    pub fn new(model: Option<String>) -> Self {
        Self { model: model.unwrap_or_else(|| ANTHROPIC_DEFAULT_MODEL.to_string()) }
    }
}

/// Reads `ANTHROPIC_API_KEY` from env. Returns `None` for missing or empty.
pub fn api_key() -> Option<String> {
    match env::var("ANTHROPIC_API_KEY") {
        Ok(s) if !s.is_empty() => Some(s),
        _ => None,
    }
}

/// Parse the Anthropic Messages API response JSON. Pure; no I/O.
fn parse_response(body: &str) -> Result<(String, u32, u32), LlmProviderError> {
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| LlmProviderError::MalformedResponse(format!("json: {e}")))?;
    if v.get("type").and_then(|t| t.as_str()) == Some("error") {
        let msg = v.pointer("/error/message").and_then(|m| m.as_str()).unwrap_or("unknown");
        return Err(LlmProviderError::MalformedResponse(format!("api error: {msg}")));
    }
    let text = v.pointer("/content/0/text")
        .and_then(|t| t.as_str())
        .ok_or_else(|| LlmProviderError::MalformedResponse("missing /content/0/text".into()))?
        .to_string();
    let input = v.pointer("/usage/input_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as u32;
    let output = v.pointer("/usage/output_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as u32;
    Ok((text, input, output))
}

impl LlmProvider for Anthropic {
    fn name(&self) -> &'static str { "anthropic" }

    fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmProviderError> {
        let key = api_key().ok_or(LlmProviderError::AuthMissing)?;
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": req.max_tokens,
            "system": req.system,
            "messages": [{"role": "user", "content": req.user}]
        });
        let agent = transport::build_agent();

        let t0 = std::time::Instant::now();
        let resp = agent.post(ANTHROPIC_API_URL)
            .set("x-api-key", &key)
            .set("anthropic-version", ANTHROPIC_VERSION)
            .set("content-type", "application/json")
            .send_json(body);
        let latency_ms = t0.elapsed().as_millis();

        match resp {
            Ok(r) => {
                let body = r.into_string().map_err(|e| LlmProviderError::Network(e.to_string()))?;
                let (text, input_tokens, output_tokens) = parse_response(&body)?;
                Ok(LlmResponse {
                    text,
                    input_tokens,
                    output_tokens,
                    model: self.model.clone(),
                    latency_ms,
                })
            }
            Err(e) => Err(transport::map_ureq_error(e)),
        }
    }
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
        let (text, in_tok, out_tok) = parse_response(body).unwrap();
        assert_eq!(text, "mrow.");
        assert_eq!(in_tok, 1234);
        assert_eq!(out_tok, 5);
    }

    #[test]
    fn parse_error_envelope_is_error() {
        let body = r#"{"type":"error","error":{"type":"invalid_request_error","message":"bad key"}}"#;
        match parse_response(body) {
            Err(LlmProviderError::MalformedResponse(msg)) => assert!(msg.contains("bad key")),
            other => panic!("expected MalformedResponse, got {other:?}"),
        }
    }

    #[test]
    fn parse_missing_content_is_error() {
        let body = r#"{"id":"msg_01","type":"message","content":[]}"#;
        assert!(matches!(parse_response(body), Err(LlmProviderError::MalformedResponse(_))));
    }

    #[test]
    fn parse_truncated_json_is_error() {
        let body = r#"{"content":[{"text":"hi"#;
        assert!(matches!(parse_response(body), Err(LlmProviderError::MalformedResponse(_))));
    }

    #[test]
    fn parse_missing_usage_defaults_to_zero() {
        let body = r#"{"content":[{"type":"text","text":"hi"}]}"#;
        let (text, in_tok, out_tok) = parse_response(body).unwrap();
        assert_eq!(text, "hi");
        assert_eq!(in_tok, 0);
        assert_eq!(out_tok, 0);
    }

    #[test]
    fn api_key_returns_none_for_empty() {
        if env::var("ANTHROPIC_API_KEY").is_ok() {
            return; // skip if developer has it set
        }
        assert!(api_key().is_none());
    }

    #[test]
    fn new_uses_default_model_when_none() {
        let a = Anthropic::new(None);
        assert_eq!(a.model, ANTHROPIC_DEFAULT_MODEL);
    }

    #[test]
    fn new_uses_override_model_when_some() {
        let a = Anthropic::new(Some("claude-3-7-sonnet-20250219".into()));
        assert_eq!(a.model, "claude-3-7-sonnet-20250219");
    }

    #[test]
    fn name_returns_anthropic() {
        assert_eq!(Anthropic::new(None).name(), "anthropic");
    }

    #[test]
    #[ignore] // run with: LLM_INTEGRATION=1 cargo test -- --ignored llm_integration
    fn llm_integration_anthropic_round_trip() {
        if env::var("LLM_INTEGRATION").is_err() {
            return;
        }
        let provider = Anthropic::new(None);
        let resp = provider.complete(LlmRequest {
            system: "You are a one-word answerer.".into(),
            user: "Reply with exactly the word: meow".into(),
            max_tokens: 10,
        }).expect("integration call should succeed");
        assert!(!resp.text.is_empty(), "got empty text");
        assert!(resp.input_tokens > 0);
        assert!(resp.output_tokens > 0);
        assert!(resp.latency_ms > 0);
        assert_eq!(resp.model, ANTHROPIC_DEFAULT_MODEL);
    }
}
