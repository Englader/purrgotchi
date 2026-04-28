//! Provider dispatcher.
//!
//! Reads `PURR_LLM_PROVIDER` (required) and `PURR_LLM_MODEL` (optional) and
//! returns a `Box<dyn LlmProvider>` for the matching impl. The split between
//! `current_provider()` (env-reading) and `select_provider()` (pure args)
//! keeps tests free of process-global env-var mutation.

use std::env;

use super::{LlmProvider, anthropic, gemini, openai};

/// Configuration failures. All non-fatal at the call site.
#[derive(Debug)]
pub enum ConfigError {
    /// `PURR_LLM_PROVIDER` is unset or empty.
    ProviderNotSet,
    /// `PURR_LLM_PROVIDER` was set to a value we don't recognize.
    UnknownProvider(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::ProviderNotSet => write!(f, "PURR_LLM_PROVIDER not set"),
            ConfigError::UnknownProvider(s) => {
                write!(f, "PURR_LLM_PROVIDER={s:?} (expected anthropic|gemini|openai)")
            }
        }
    }
}

/// Read env vars and dispatch to the matching provider impl.
pub fn current_provider() -> Result<Box<dyn LlmProvider>, ConfigError> {
    let provider = env::var("PURR_LLM_PROVIDER").ok();
    let model = env::var("PURR_LLM_MODEL").ok().filter(|s| !s.is_empty());
    select_provider(provider.as_deref(), model)
}

/// Pure dispatcher — same logic as `current_provider` but takes its inputs
/// explicitly so tests can exercise it without mutating process env vars.
pub(crate) fn select_provider(
    provider: Option<&str>,
    model: Option<String>,
) -> Result<Box<dyn LlmProvider>, ConfigError> {
    let raw = provider.filter(|s| !s.is_empty()).ok_or(ConfigError::ProviderNotSet)?;
    match raw.to_lowercase().as_str() {
        "anthropic" => Ok(Box::new(anthropic::Anthropic::new(model))),
        "gemini" => Ok(Box::new(gemini::Gemini::new(model))),
        "openai" => Ok(Box::new(openai::OpenAi::new(model))),
        _ => Err(ConfigError::UnknownProvider(raw.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_anthropic_lowercase() {
        let p = select_provider(Some("anthropic"), None).unwrap();
        assert_eq!(p.name(), "anthropic");
    }

    #[test]
    fn select_gemini_lowercase() {
        let p = select_provider(Some("gemini"), None).unwrap();
        assert_eq!(p.name(), "gemini");
    }

    #[test]
    fn select_openai_lowercase() {
        let p = select_provider(Some("openai"), None).unwrap();
        assert_eq!(p.name(), "openai");
    }

    #[test]
    fn select_is_case_insensitive() {
        assert_eq!(select_provider(Some("ANTHROPIC"), None).unwrap().name(), "anthropic");
        assert_eq!(select_provider(Some("Gemini"), None).unwrap().name(), "gemini");
        assert_eq!(select_provider(Some("OpenAI"), None).unwrap().name(), "openai");
    }

    #[test]
    fn select_provider_not_set_when_none() {
        assert!(matches!(
            select_provider(None, None),
            Err(ConfigError::ProviderNotSet)
        ));
    }

    #[test]
    fn select_provider_not_set_when_empty() {
        assert!(matches!(
            select_provider(Some(""), None),
            Err(ConfigError::ProviderNotSet)
        ));
    }

    #[test]
    fn select_unknown_provider_carries_name() {
        match select_provider(Some("ollama"), None) {
            Err(ConfigError::UnknownProvider(s)) => assert_eq!(s, "ollama"),
            Err(other) => panic!("expected UnknownProvider, got {other:?}"),
            Ok(_) => panic!("expected UnknownProvider, got Ok"),
        }
    }

    #[test]
    fn config_error_display_provider_not_set() {
        let s = format!("{}", ConfigError::ProviderNotSet);
        assert!(s.contains("PURR_LLM_PROVIDER"));
    }

    #[test]
    fn config_error_display_unknown_includes_value() {
        let s = format!("{}", ConfigError::UnknownProvider("foo".into()));
        assert!(s.contains("foo"));
        assert!(s.contains("anthropic"));
    }

    // Model-passthrough tests verify the dispatcher hands the override to the
    // selected provider. We can't peek at the inner struct's `model` through
    // the trait, but `name()` confirms the right type was constructed; the
    // model field itself is unit-tested in the provider modules.
    #[test]
    fn select_with_model_override_returns_correct_provider() {
        let p = select_provider(Some("anthropic"), Some("claude-3-7-sonnet".into())).unwrap();
        assert_eq!(p.name(), "anthropic");
    }
}
