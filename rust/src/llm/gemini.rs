//! Gemini provider — Google Generative Language API.
//!
//! Implements `LlmProvider` against
//! `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`.
//! Reads `GEMINI_API_KEY` from env. Default model is `gemini-2.5-flash`;
//! caller can override via `Gemini::new(Some(model))`.
//!
//! Auth uses the `x-goog-api-key` header (cleaner than the `?key=` query param
//! variant — keeps the secret out of URL-logging surfaces).

use std::env;
use std::time::Duration;

use super::{LlmProvider, LlmProviderError, LlmRequest, LlmResponse};

const GEMINI_API_URL_TEMPLATE: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent";
pub const GEMINI_DEFAULT_MODEL: &str = "gemini-2.5-flash";
const CONNECT_TIMEOUT_SECS: u64 = 10;
const READ_TIMEOUT_SECS: u64 = 10;
const ERROR_BODY_TRUNCATE: usize = 512;

pub struct Gemini {
    model: String,
}

impl Gemini {
    pub fn new(model: Option<String>) -> Self {
        Self { model: model.unwrap_or_else(|| GEMINI_DEFAULT_MODEL.to_string()) }
    }
}

/// Reads `GEMINI_API_KEY`. Returns `None` for missing or empty.
pub fn api_key() -> Option<String> {
    match env::var("GEMINI_API_KEY") {
        Ok(s) if !s.is_empty() => Some(s),
        _ => None,
    }
}

/// Build the request URL with the model name interpolated.
fn url_for_model(model: &str) -> String {
    GEMINI_API_URL_TEMPLATE.replace("{model}", model)
}

/// Build the request body from a normalized `LlmRequest`.
fn build_body(req: &LlmRequest) -> serde_json::Value {
    serde_json::json!({
        "systemInstruction": {
            "parts": [{"text": req.system}]
        },
        "contents": [{
            "role": "user",
            "parts": [{"text": req.user}]
        }],
        "generationConfig": {
            "maxOutputTokens": req.max_tokens
        }
    })
}

/// Parse the Gemini response JSON. Pure; no I/O.
fn parse_response(body: &str) -> Result<(String, u32, u32), LlmProviderError> {
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| LlmProviderError::MalformedResponse(format!("json: {e}")))?;
    if v.get("error").is_some() {
        let msg = v.pointer("/error/message").and_then(|m| m.as_str()).unwrap_or("unknown");
        return Err(LlmProviderError::MalformedResponse(format!("api error: {msg}")));
    }
    let text = v.pointer("/candidates/0/content/parts/0/text")
        .and_then(|t| t.as_str())
        .ok_or_else(|| {
            LlmProviderError::MalformedResponse("missing /candidates/0/content/parts/0/text".into())
        })?
        .to_string();
    let input = v.pointer("/usageMetadata/promptTokenCount")
        .and_then(|t| t.as_u64()).unwrap_or(0) as u32;
    let output = v.pointer("/usageMetadata/candidatesTokenCount")
        .and_then(|t| t.as_u64()).unwrap_or(0) as u32;
    Ok((text, input, output))
}

fn truncate_body(s: String) -> String {
    if s.len() <= ERROR_BODY_TRUNCATE { s } else { format!("{}…", &s[..ERROR_BODY_TRUNCATE]) }
}

impl LlmProvider for Gemini {
    fn name(&self) -> &'static str { "gemini" }

    fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmProviderError> {
        let key = api_key().ok_or(LlmProviderError::AuthMissing)?;
        let url = url_for_model(&self.model);
        let body = build_body(&req);

        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            .timeout_read(Duration::from_secs(READ_TIMEOUT_SECS))
            .build();

        let t0 = std::time::Instant::now();
        let resp = agent.post(&url)
            .set("x-goog-api-key", &key)
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
            "candidates":[{"content":{"parts":[{"text":"meow there."}],"role":"model"},"finishReason":"STOP"}],
            "usageMetadata":{"promptTokenCount":42,"candidatesTokenCount":7,"totalTokenCount":49}
        }"#;
        let (text, in_tok, out_tok) = parse_response(body).unwrap();
        assert_eq!(text, "meow there.");
        assert_eq!(in_tok, 42);
        assert_eq!(out_tok, 7);
    }

    #[test]
    fn parse_error_envelope_is_error() {
        let body = r#"{"error":{"code":400,"message":"API key invalid","status":"INVALID_ARGUMENT"}}"#;
        match parse_response(body) {
            Err(LlmProviderError::MalformedResponse(msg)) => assert!(msg.contains("API key invalid")),
            other => panic!("expected MalformedResponse, got {other:?}"),
        }
    }

    #[test]
    fn parse_missing_text_is_error() {
        let body = r#"{"candidates":[{"content":{"parts":[]}}]}"#;
        assert!(matches!(parse_response(body), Err(LlmProviderError::MalformedResponse(_))));
    }

    #[test]
    fn parse_empty_candidates_is_error() {
        let body = r#"{"candidates":[]}"#;
        assert!(matches!(parse_response(body), Err(LlmProviderError::MalformedResponse(_))));
    }

    #[test]
    fn parse_truncated_json_is_error() {
        let body = r#"{"candidates":[{"content":"#;
        assert!(matches!(parse_response(body), Err(LlmProviderError::MalformedResponse(_))));
    }

    #[test]
    fn parse_missing_usage_defaults_to_zero() {
        let body = r#"{"candidates":[{"content":{"parts":[{"text":"hi"}]}}]}"#;
        let (text, in_tok, out_tok) = parse_response(body).unwrap();
        assert_eq!(text, "hi");
        assert_eq!(in_tok, 0);
        assert_eq!(out_tok, 0);
    }

    #[test]
    fn api_key_returns_none_for_empty() {
        if env::var("GEMINI_API_KEY").is_ok() {
            return;
        }
        assert!(api_key().is_none());
    }

    #[test]
    fn new_uses_default_model_when_none() {
        let g = Gemini::new(None);
        assert_eq!(g.model, GEMINI_DEFAULT_MODEL);
    }

    #[test]
    fn new_uses_override_model_when_some() {
        let g = Gemini::new(Some("gemini-1.5-pro".into()));
        assert_eq!(g.model, "gemini-1.5-pro");
    }

    #[test]
    fn name_returns_gemini() {
        assert_eq!(Gemini::new(None).name(), "gemini");
    }

    #[test]
    fn url_for_model_interpolates() {
        let url = url_for_model("gemini-2.5-flash");
        assert!(url.starts_with("https://generativelanguage.googleapis.com/v1beta/models/"));
        assert!(url.contains("gemini-2.5-flash:generateContent"));
        assert!(!url.contains("{model}"));
    }

    #[test]
    fn build_body_has_required_shape() {
        let req = LlmRequest {
            system: "You are a cat.".into(),
            user: "Say hi.".into(),
            max_tokens: 50,
        };
        let body = build_body(&req);
        assert_eq!(body.pointer("/systemInstruction/parts/0/text").and_then(|v| v.as_str()), Some("You are a cat."));
        assert_eq!(body.pointer("/contents/0/role").and_then(|v| v.as_str()), Some("user"));
        assert_eq!(body.pointer("/contents/0/parts/0/text").and_then(|v| v.as_str()), Some("Say hi."));
        assert_eq!(body.pointer("/generationConfig/maxOutputTokens").and_then(|v| v.as_u64()), Some(50));
    }

    #[test]
    #[ignore] // run with: LLM_INTEGRATION=1 cargo test -- --ignored llm_integration_gemini
    fn llm_integration_gemini_round_trip() {
        if env::var("LLM_INTEGRATION").is_err() {
            return;
        }
        let provider = Gemini::new(None);
        let resp = provider.complete(LlmRequest {
            system: "You are a one-word answerer.".into(),
            user: "Reply with exactly the word: meow".into(),
            max_tokens: 10,
        }).expect("integration call should succeed");
        assert!(!resp.text.is_empty(), "got empty text");
        assert!(resp.input_tokens > 0);
        // candidatesTokenCount may be 0 if model emits a single-token reply with stop;
        // we don't assert > 0 to keep the test stable.
        assert!(resp.latency_ms > 0);
        assert_eq!(resp.model, GEMINI_DEFAULT_MODEL);
    }
}
