//! Provider-agnostic LLM surface.
//!
//! Defines the `LlmProvider` trait and normalized request/response/error types.
//! Concrete provider impls live in sibling modules (e.g. `anthropic`, and in
//! T3/T4: `gemini`, `openai`). Every error variant is non-fatal at the call
//! site; callers log via `debug::log` and fall back to static behaviour.
//!
//! The `HaikuRequest`/`HaikuResponse`/`LlmError`/`call_haiku()` shim below
//! preserves the v3.5.0 API for existing call sites in `react.rs` and
//! `hatch.rs`. T6/T7 will rewrite those call sites to use the dispatcher
//! (T5) directly, after which the shim is removed.

pub mod anthropic;
pub mod gemini;
pub mod openai;
pub mod select;

pub use anthropic::api_key;

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

/// A pluggable LLM backend. Object-safe so call sites can hold `Box<dyn LlmProvider>`.
pub trait LlmProvider {
    /// Stable lowercase identifier: `"anthropic"`, `"gemini"`, `"openai"`.
    fn name(&self) -> &'static str;
    /// Round-trip a normalized request. Implementations measure latency themselves.
    fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmProviderError>;
}

// --- Legacy v3.5.0 shim (removed in T7) ---

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

/// Backward-compat shim that calls Anthropic via the new trait, then converts
/// the result back into the legacy `HaikuResponse`/`LlmError` shapes used by
/// `react.rs` and `hatch.rs`. Deleted in T7 once both call sites switch to the
/// dispatcher.
pub fn call_haiku(req: HaikuRequest) -> Result<HaikuResponse, LlmError> {
    let provider = anthropic::Anthropic::new(None);
    let llm_req = LlmRequest {
        system: req.system,
        user: req.user,
        max_tokens: req.max_tokens,
    };
    match provider.complete(llm_req) {
        Ok(r) => Ok(HaikuResponse {
            text: r.text,
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            latency_ms: r.latency_ms as u64,
        }),
        Err(e) => Err(match e {
            LlmProviderError::AuthMissing => LlmError::MissingKey,
            LlmProviderError::Http(code, _body) => LlmError::Http(code),
            LlmProviderError::Network(s) => LlmError::Network(s),
            LlmProviderError::Timeout => LlmError::Timeout,
            LlmProviderError::MalformedResponse(s) => LlmError::MalformedResponse(s),
        }),
    }
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

    // --- T2 shim conversion tests ---

    #[test]
    fn legacy_error_variants_all_format() {
        let _ = format!("{:?}", LlmError::MissingKey);
        let _ = format!("{:?}", LlmError::Http(500));
        let _ = format!("{:?}", LlmError::Network("dns".into()));
        let _ = format!("{:?}", LlmError::Timeout);
        let _ = format!("{:?}", LlmError::MalformedResponse("bad".into()));
    }

    #[test]
    fn legacy_haiku_request_response_round_trip() {
        let req = HaikuRequest { system: "s".into(), user: "u".into(), max_tokens: 5 };
        assert_eq!(req.max_tokens, 5);
        let resp = HaikuResponse {
            text: "hi".into(),
            input_tokens: 1,
            output_tokens: 2,
            latency_ms: 99,
        };
        assert_eq!(resp.text, "hi");
        assert_eq!(resp.latency_ms, 99);
    }
}
