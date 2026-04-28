//! Provider-agnostic LLM surface.
//!
//! Defines the `LlmProvider` trait and normalized request/response/error types.
//! Concrete provider impls live in sibling modules: `anthropic`, `gemini`,
//! `openai`. The `select` module reads `PURR_LLM_PROVIDER` at runtime and
//! dispatches. Every error variant is non-fatal at the call site; callers log
//! via `debug::log` and fall back to static behaviour.

pub mod anthropic;
pub mod gemini;
pub mod openai;
pub mod select;

// --- Provider-agnostic surface (T1) ---

/// Normalized request shape submitted to any LLM provider.
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub system: String,
    pub user: String,
    pub max_tokens: u32,
}

/// Normalized response shape returned by any LLM provider.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub text: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub model: String,
    pub latency_ms: u128,
}

/// Failure modes for an LLM call. All non-fatal at the call site.
#[derive(Debug)]
pub enum LlmProviderError {
    /// The provider's API key env var is missing or empty.
    AuthMissing,
    /// HTTP error response: status code + truncated body for diagnostics.
    Http(u16, String),
    /// Network/transport failure (connection refused, DNS, etc.).
    Network(String),
    /// Request timed out.
    Timeout,
    /// Response body did not match the expected schema.
    MalformedResponse(String),
}

impl std::fmt::Display for LlmProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmProviderError::AuthMissing => write!(f, "auth missing"),
            LlmProviderError::Http(status, body) => write!(f, "HTTP {status}: {body}"),
            LlmProviderError::Network(msg) => write!(f, "network: {msg}"),
            LlmProviderError::Timeout => write!(f, "timeout"),
            LlmProviderError::MalformedResponse(msg) => write!(f, "malformed response: {msg}"),
        }
    }
}

/// A pluggable LLM backend. Object-safe so call sites can hold `Box<dyn LlmProvider>`.
pub trait LlmProvider {
    /// Stable lowercase identifier: `"anthropic"`, `"gemini"`, `"openai"`.
    fn name(&self) -> &'static str;
    /// Round-trip a normalized request. Implementations measure latency themselves.
    fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmProviderError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- T1 trait + types tests ---

    #[test]
    fn llm_request_debug_round_trips_fields() {
        let req = LlmRequest { system: "sys".into(), user: "usr".into(), max_tokens: 42 };
        let s = format!("{req:?}");
        assert!(s.contains("system: \"sys\""));
        assert!(s.contains("user: \"usr\""));
        assert!(s.contains("max_tokens: 42"));
    }

    #[test]
    fn llm_response_debug_round_trips_fields() {
        let r = LlmResponse {
            text: "mrow".into(),
            input_tokens: 7,
            output_tokens: 3,
            model: "test-model".into(),
            latency_ms: 123,
        };
        let s = format!("{r:?}");
        assert!(s.contains("text: \"mrow\""));
        assert!(s.contains("input_tokens: 7"));
        assert!(s.contains("output_tokens: 3"));
        assert!(s.contains("model: \"test-model\""));
        assert!(s.contains("latency_ms: 123"));
    }

    #[test]
    fn llm_provider_error_http_carries_status_and_body() {
        let e = LlmProviderError::Http(429, "rate limit".into());
        let s = format!("{e:?}");
        assert!(s.contains("429"));
        assert!(s.contains("rate limit"));
    }

    #[test]
    fn llm_provider_error_variants_all_format() {
        let _ = format!("{:?}", LlmProviderError::AuthMissing);
        let _ = format!("{:?}", LlmProviderError::Http(500, "oops".into()));
        let _ = format!("{:?}", LlmProviderError::Network("dns".into()));
        let _ = format!("{:?}", LlmProviderError::Timeout);
        let _ = format!("{:?}", LlmProviderError::MalformedResponse("bad".into()));
    }

    #[test]
    fn llm_provider_is_object_safe() {
        struct Stub;
        impl LlmProvider for Stub {
            fn name(&self) -> &'static str { "stub" }
            fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmProviderError> {
                Ok(LlmResponse {
                    text: format!("echo:{}", req.user),
                    input_tokens: 0,
                    output_tokens: 0,
                    model: "stub".into(),
                    latency_ms: 0,
                })
            }
        }
        let p: Box<dyn LlmProvider> = Box::new(Stub);
        assert_eq!(p.name(), "stub");
        let resp = p.complete(LlmRequest {
            system: "s".into(),
            user: "u".into(),
            max_tokens: 1,
        }).unwrap();
        assert_eq!(resp.text, "echo:u");
    }

    // --- Display impl tests (v3.6.2) ---

    #[test]
    fn display_auth_missing_formats_human_readable() {
        let s = format!("{}", LlmProviderError::AuthMissing);
        assert_eq!(s, "auth missing");
    }

    #[test]
    fn display_http_includes_status_and_body() {
        let s = format!("{}", LlmProviderError::Http(429, "rate limit exceeded".into()));
        assert_eq!(s, "HTTP 429: rate limit exceeded");
    }

    #[test]
    fn display_network_includes_message() {
        let s = format!("{}", LlmProviderError::Network("dns lookup failed".into()));
        assert_eq!(s, "network: dns lookup failed");
    }

    #[test]
    fn display_timeout_formats_human_readable() {
        let s = format!("{}", LlmProviderError::Timeout);
        assert_eq!(s, "timeout");
    }

    #[test]
    fn display_malformed_includes_message() {
        let s = format!("{}", LlmProviderError::MalformedResponse("missing parts".into()));
        assert_eq!(s, "malformed response: missing parts");
    }

}
