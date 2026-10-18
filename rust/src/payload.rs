use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// Claude Code sends `null` for fields it has no value for yet (before the first
/// API response, after /clear). Treat null like an absent field.
fn null_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}


#[derive(Deserialize, Default)]
pub struct Payload {
    #[serde(default, deserialize_with = "null_default")]
    pub session_id: String,
    #[serde(default, deserialize_with = "null_default")]
    pub transcript_path: String,
    #[serde(default, deserialize_with = "null_default")]
    pub model: ModelField,
    #[serde(default, deserialize_with = "null_default")]
    pub workspace: WorkspaceField,
    #[serde(default, deserialize_with = "null_default")]
    pub cwd: String,
    #[serde(default, deserialize_with = "null_default")]
    pub context_window: ContextWindow,
    #[serde(default, deserialize_with = "null_default")]
    pub rate_limits: RateLimits,
    #[serde(default, deserialize_with = "null_default")]
    pub cost: CostField,
}

#[derive(Deserialize, Default)]
pub struct ModelField {
    #[serde(default, deserialize_with = "null_default")]
    pub display_name: String,
}

#[derive(Deserialize, Default)]
pub struct WorkspaceField {
    #[serde(default, deserialize_with = "null_default")]
    pub current_dir: String,
}

#[derive(Deserialize, Default)]
pub struct ContextWindow {
    #[serde(default, deserialize_with = "null_default")]
    pub used_percentage: f64,
    #[serde(default, deserialize_with = "null_default")]
    pub current_usage: CurrentUsage,
}

#[derive(Deserialize, Default)]
pub struct CurrentUsage {
    #[serde(default, deserialize_with = "null_default")]
    pub input_tokens: i64,
    #[serde(default, deserialize_with = "null_default")]
    pub output_tokens: i64,
    #[serde(default, deserialize_with = "null_default")]
    pub cache_creation_input_tokens: i64,
    #[serde(default, deserialize_with = "null_default")]
    pub cache_read_input_tokens: i64,
}

#[derive(Deserialize, Default)]
pub struct RateLimits {
    #[serde(default, deserialize_with = "null_default")]
    pub five_hour: RateWindow,
    #[serde(default, deserialize_with = "null_default")]
    pub seven_day: RateWindow,
}

#[derive(Deserialize, Default)]
pub struct RateWindow {
    #[serde(default, deserialize_with = "null_default")]
    pub used_percentage: f64,
    #[serde(default, deserialize_with = "null_default")]
    pub resets_at: i64,
}

#[derive(Deserialize, Default)]
pub struct CostField {
    #[serde(default, deserialize_with = "null_default")]
    pub total_cost_usd: f64,
    #[serde(default, deserialize_with = "null_default")]
    pub total_lines_added: i64,
    #[serde(default, deserialize_with = "null_default")]
    pub total_lines_removed: i64,
    #[serde(default, deserialize_with = "null_default")]
    pub total_duration_ms: i64,
}

impl Payload {
    /// Parse a statusline/hook payload. Never fails: strict parsing first; if one
    /// field has the wrong type, salvage every field that is fine from the raw
    /// JSON so one odd field cannot zero the whole HUD or lose the session id.
    pub fn parse(input: &str) -> Payload {
        if input.trim().is_empty() {
            return Payload::default();
        }
        if let Ok(p) = serde_json::from_str::<Payload>(input) {
            return p;
        }
        match serde_json::from_str::<Value>(input) {
            Ok(v) => Payload::from_value(&v),
            Err(_) => Payload::default(),
        }
    }

    fn from_value(v: &Value) -> Payload {
        let s = |p: &str| v.pointer(p).and_then(Value::as_str).unwrap_or_default().to_string();
        let f = |p: &str| v.pointer(p).and_then(Value::as_f64).unwrap_or_default();
        let i = |p: &str| v.pointer(p).and_then(Value::as_i64).unwrap_or_default();
        let usage = "/context_window/current_usage";
        Payload {
            session_id: s("/session_id"),
            transcript_path: s("/transcript_path"),
            model: ModelField { display_name: s("/model/display_name") },
            workspace: WorkspaceField { current_dir: s("/workspace/current_dir") },
            cwd: s("/cwd"),
            context_window: ContextWindow {
                used_percentage: f("/context_window/used_percentage"),
                current_usage: CurrentUsage {
                    input_tokens: i(&format!("{usage}/input_tokens")),
                    output_tokens: i(&format!("{usage}/output_tokens")),
                    cache_creation_input_tokens: i(&format!("{usage}/cache_creation_input_tokens")),
                    cache_read_input_tokens: i(&format!("{usage}/cache_read_input_tokens")),
                },
            },
            rate_limits: RateLimits {
                five_hour: RateWindow {
                    used_percentage: f("/rate_limits/five_hour/used_percentage"),
                    resets_at: i("/rate_limits/five_hour/resets_at"),
                },
                seven_day: RateWindow {
                    used_percentage: f("/rate_limits/seven_day/used_percentage"),
                    resets_at: i("/rate_limits/seven_day/resets_at"),
                },
            },
            cost: CostField {
                total_cost_usd: f("/cost/total_cost_usd"),
                total_lines_added: i("/cost/total_lines_added"),
                total_lines_removed: i("/cost/total_lines_removed"),
                total_duration_ms: i("/cost/total_duration_ms"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NULL_CTX: &str = r#"{"session_id":"s1","model":{"display_name":"Opus"},
        "workspace":{"current_dir":"/w/d"},"cwd":"/w",
        "context_window":{"used_percentage":null,"current_usage":null},
        "cost":{"total_cost_usd":1.5}}"#;

    #[test]
    fn null_context_fields_keep_the_whole_payload() {
        let p: Payload = serde_json::from_str(NULL_CTX).unwrap();
        assert_eq!(p.session_id, "s1");
        assert_eq!(p.model.display_name, "Opus");
        assert_eq!(p.workspace.current_dir, "/w/d");
        assert_eq!(p.context_window.used_percentage, 0.0);
        assert_eq!(p.context_window.current_usage.input_tokens, 0);
        assert!((p.cost.total_cost_usd - 1.5).abs() < 1e-9);
    }

    #[test]
    fn every_field_tolerates_null() {
        let json = r#"{"session_id":null,"transcript_path":null,"model":null,"workspace":null,"cwd":null,
            "context_window":{"used_percentage":null,"current_usage":{"input_tokens":null,"output_tokens":null,
              "cache_creation_input_tokens":null,"cache_read_input_tokens":null}},
            "rate_limits":{"five_hour":{"used_percentage":null,"resets_at":null},"seven_day":null},
            "cost":{"total_cost_usd":null,"total_lines_added":null,"total_lines_removed":null,"total_duration_ms":null}}"#;
        let p: Payload = serde_json::from_str(json).unwrap();
        assert_eq!(p.session_id, "");
        let json = r#"{"session_id":"k","model":{"display_name":null},"workspace":{"current_dir":null},"rate_limits":null,"cost":null,"context_window":null}"#;
        assert_eq!(serde_json::from_str::<Payload>(json).unwrap().session_id, "k");
    }

    #[test]
    fn wrong_typed_field_keeps_session_via_value_fallback() {
        let json = r#"{"session_id":"s2","model":{"display_name":"Sonnet"},"workspace":{"current_dir":"/w/d"},
            "context_window":{"used_percentage":"lots","current_usage":{"input_tokens":7}},
            "cost":{"total_cost_usd":2.5}}"#;
        assert!(serde_json::from_str::<Payload>(json).is_err());
        let p = Payload::parse(json);
        assert_eq!(p.session_id, "s2");
        assert_eq!(p.model.display_name, "Sonnet");
        assert_eq!(p.workspace.current_dir, "/w/d");
        assert_eq!(p.context_window.used_percentage, 0.0);
        assert_eq!(p.context_window.current_usage.input_tokens, 7);
        assert!((p.cost.total_cost_usd - 2.5).abs() < 1e-9);
    }

    #[test]
    fn parse_survives_garbage_and_empty() {
        assert_eq!(Payload::parse("").session_id, "");
        assert_eq!(Payload::parse("not json").session_id, "");
        assert_eq!(Payload::parse("[1,2]").session_id, "");
    }

    #[test]
    fn session_id_deserializes_from_json() {
        let json = r#"{"session_id":"2de8ac34-3a21-4816-9b82-824f39c5bb99","cost":{"total_cost_usd":59.32}}"#;
        let p: Payload = serde_json::from_str(json).unwrap();
        assert_eq!(p.session_id, "2de8ac34-3a21-4816-9b82-824f39c5bb99");
        assert!((p.cost.total_cost_usd - 59.32).abs() < 1e-9);
    }

    #[test]
    fn session_id_defaults_to_empty_when_missing() {
        let json = r#"{"cost":{"total_cost_usd":1.0}}"#;
        let p: Payload = serde_json::from_str(json).unwrap();
        assert_eq!(p.session_id, "");
    }

    #[test]
    fn transcript_path_deserializes_from_json() {
        let json = r#"{"session_id":"abc","transcript_path":"C:/path/to/transcript.jsonl"}"#;
        let p: Payload = serde_json::from_str(json).unwrap();
        assert_eq!(p.transcript_path, "C:/path/to/transcript.jsonl");
    }

    #[test]
    fn transcript_path_defaults_to_empty_when_missing() {
        let json = r#"{"session_id":"abc"}"#;
        let p: Payload = serde_json::from_str(json).unwrap();
        assert_eq!(p.transcript_path, "");
    }
}
