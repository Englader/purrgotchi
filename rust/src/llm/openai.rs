//! OpenAI provider — Chat Completions API.
//!
//! Implements `LlmProvider` against `https://api.openai.com/v1/chat/completions`.
//! Reads `OPENAI_API_KEY` from env. Default model is `gpt-4o-mini`; caller can
//! override via `OpenAi::new(Some(model))`.
//!
//! Auth uses `Authorization: Bearer <key>`. Note: this provider passes
//! `max_tokens` (the classic Chat Completions field). Newer reasoning models
//! (o1/o3) require `max_completion_tokens` instead — those models will reject
//! requests from this provider until the abstraction grows a per-model knob.

use std::env;
use std::time::Duration;

use super::{LlmProvider, LlmProviderError, LlmRequest, LlmResponse};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";
pub const OPENAI_DEFAULT_MODEL: &str = "gpt-4o-mini";
const CONNECT_TIMEOUT_SECS: u64 = 10;
const READ_TIMEOUT_SECS: u64 = 10;
const ERROR_BODY_TRUNCATE: usize = 512;

pub struct OpenAi {
    model: String,
}

impl OpenAi {
    pub fn new(model: Option<String>) -> Self {
        Self { model: model.unwrap_or_else(|| OPENAI_DEFAULT_MODEL.to_string()) }
    }
}

/// Reads `OPENAI_API_KEY`. Returns `None` for missing or empty.
pub fn api_key() -> Option<String> {
    match env::var("OPENAI_API_KEY") {
        Ok(s) if !s.is_empty() => Some(s),
        _ => None,
    }
}

/// Build the request body from a normalized `LlmRequest`.
fn build_body(model: &str, req: &LlmRequest) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "max_tokens": req.max_tokens,
        "messages": [
            {"role": "system", "content": req.system},
            {"role": "user", "content": req.user}
        ]
    })
}

/// Parse the Chat Completions response JSON. Pure; no I/O.
fn parse_response(body: &str) -> Result<(String, u32, u32), LlmProviderError> {
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| LlmProviderError::MalformedResponse(format!("json: {e}")))?;
    if v.get("error").is_some() {
        let msg = v.pointer("/error/message").and_then(|m| m.as_str()).unwrap_or("unknown");
        return Err(LlmProviderError::MalformedResponse(format!("api error: {msg}")));
    }
    let text = v.pointer("/choices/0/message/content")
        .and_then(|t| t.as_str())
        .ok_or_else(|| {
            LlmProviderError::MalformedResponse("missing /choices/0/message/content".into())
        })?
        .to_string();
    let input = v.pointer("/usage/prompt_tokens")
        .and_then(|t| t.as_u64()).unwrap_or(0) as u32;
    let output = v.pointer("/usage/completion_tokens")
        .and_then(|t| t.as_u64()).unwrap_or(0) as u32;
    Ok((text, input, output))
}

fn truncate_body(s: String) -> String {
    if s.len() <= ERROR_BODY_TRUNCATE { s } else { format!("{}…", &s[..ERROR_BODY_TRUNCATE]) }
}

impl LlmProvider for OpenAi {
    fn name(&self) -> &'static str { "openai" }

    fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmProviderError> {
        let key = api_key().ok_or(LlmProviderError::AuthMissing)?;
        let body = build_body(&self.model, &req);

        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            .timeout_read(Duration::from_secs(READ_TIMEOUT_SECS))
            .build();

        let t0 = std::time::Instant::now();
        let resp = agent.post(OPENAI_API_URL)
            .set("Authorization", &format!("Bearer {key}"))
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
            Err(ureq::Error::Status(code, response)) => {
                let body = response.into_string().unwrap_or_default();
                Err(LlmProviderError::Http(code, truncate_body(body)))
            }
            Err(ureq::Error::Transport(t)) => {
                let s = t.to_string();
                if s.contains("timed out") || s.contains("timeout") {
                    Err(LlmProviderError::Timeout)
                } else {
                    Err(LlmProviderError::Network(s))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_response() {
        let body = r#"{
            "id":"chatcmpl-1",
            "choices":[{"index":0,"message":{"role":"assistant","content":"meow."},"finish_reason":"stop"}],
            "usage":{"prompt_tokens":12,"completion_tokens":3,"total_tokens":15}
        }"#;
        let (text, in_tok, out_tok) = parse_response(body).unwrap();
        assert_eq!(text, "meow.");
        assert_eq!(in_tok, 12);
        assert_eq!(out_tok, 3);
    }

    #[test]
    fn parse_error_envelope_is_error() {
        let body = r#"{"error":{"message":"Incorrect API key","type":"invalid_request_error","code":"invalid_api_key"}}"#;
        match parse_response(body) {
            Err(LlmProviderError::MalformedResponse(msg)) => assert!(msg.contains("Incorrect API key")),
            other => panic!("expected MalformedResponse, got {other:?}"),
        }
    }

    #[test]
    fn parse_missing_content_is_error() {
        let body = r#"{"choices":[{"message":{}}]}"#;
        assert!(matches!(parse_response(body), Err(LlmProviderError::MalformedResponse(_))));
    }

    #[test]
    fn parse_empty_choices_is_error() {
        let body = r#"{"choices":[]}"#;
        assert!(matches!(parse_response(body), Err(LlmProviderError::MalformedResponse(_))));
    }

    #[test]
    fn parse_truncated_json_is_error() {
        let body = r#"{"choices":[{"message":"#;
        assert!(matches!(parse_response(body), Err(LlmProviderError::MalformedResponse(_))));
    }

    #[test]
    fn parse_missing_usage_defaults_to_zero() {
        let body = r#"{"choices":[{"message":{"content":"hi"}}]}"#;
        let (text, in_tok, out_tok) = parse_response(body).unwrap();
        assert_eq!(text, "hi");
        assert_eq!(in_tok, 0);
        assert_eq!(out_tok, 0);
    }

    #[test]
    fn api_key_returns_none_for_empty() {
        if env::var("OPENAI_API_KEY").is_ok() {
            return;
        }
        assert!(api_key().is_none());
    }

    #[test]
    fn new_uses_default_model_when_none() {
        let o = OpenAi::new(None);
        assert_eq!(o.model, OPENAI_DEFAULT_MODEL);
    }

    #[test]
    fn new_uses_override_model_when_some() {
        let o = OpenAi::new(Some("gpt-4o".into()));
        assert_eq!(o.model, "gpt-4o");
    }

    #[test]
    fn name_returns_openai() {
        assert_eq!(OpenAi::new(None).name(), "openai");
    }

    #[test]
    fn build_body_has_required_shape() {
        let req = LlmRequest {
            system: "You are a cat.".into(),
            user: "Say hi.".into(),
            max_tokens: 50,
        };
        let body = build_body("gpt-4o-mini", &req);
        assert_eq!(body.pointer("/model").and_then(|v| v.as_str()), Some("gpt-4o-mini"));
        assert_eq!(body.pointer("/max_tokens").and_then(|v| v.as_u64()), Some(50));
        assert_eq!(body.pointer("/messages/0/role").and_then(|v| v.as_str()), Some("system"));
        assert_eq!(body.pointer("/messages/0/content").and_then(|v| v.as_str()), Some("You are a cat."));
        assert_eq!(body.pointer("/messages/1/role").and_then(|v| v.as_str()), Some("user"));
        assert_eq!(body.pointer("/messages/1/content").and_then(|v| v.as_str()), Some("Say hi."));
    }

    #[test]
    #[ignore] // run with: LLM_INTEGRATION=1 cargo test -- --ignored llm_integration_openai
    fn llm_integration_openai_round_trip() {
        if env::var("LLM_INTEGRATION").is_err() {
            return;
        }
        let provider = OpenAi::new(None);
        let resp = provider.complete(LlmRequest {
            system: "You are a one-word answerer.".into(),
            user: "Reply with exactly the word: meow".into(),
            max_tokens: 10,
        }).expect("integration call should succeed");
        assert!(!resp.text.is_empty(), "got empty text");
        assert!(resp.input_tokens > 0);
        assert!(resp.latency_ms > 0);
        assert_eq!(resp.model, OPENAI_DEFAULT_MODEL);
    }
}
