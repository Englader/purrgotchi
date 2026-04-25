use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Payload {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub model: ModelField,
    #[serde(default)]
    pub workspace: WorkspaceField,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub context_window: ContextWindow,
    #[serde(default)]
    pub rate_limits: RateLimits,
    #[serde(default)]
    pub cost: CostField,
}

#[derive(Deserialize, Default)]
pub struct ModelField {
    #[serde(default)]
    pub display_name: String,
}

#[derive(Deserialize, Default)]
pub struct WorkspaceField {
    #[serde(default)]
    pub current_dir: String,
}

#[derive(Deserialize, Default)]
pub struct ContextWindow {
    #[serde(default)]
    pub used_percentage: f64,
    #[serde(default)]
    pub current_usage: CurrentUsage,
}

#[derive(Deserialize, Default)]
pub struct CurrentUsage {
    #[serde(default)]
    pub input_tokens: i64,
    #[serde(default)]
    pub output_tokens: i64,
    #[serde(default)]
    pub cache_creation_input_tokens: i64,
    #[serde(default)]
    pub cache_read_input_tokens: i64,
}

#[derive(Deserialize, Default)]
pub struct RateLimits {
    #[serde(default)]
    pub five_hour: RateWindow,
    #[serde(default)]
    pub seven_day: RateWindow,
}

#[derive(Deserialize, Default)]
pub struct RateWindow {
    #[serde(default)]
    pub used_percentage: f64,
    #[serde(default)]
    pub resets_at: i64,
}

#[derive(Deserialize, Default)]
pub struct CostField {
    #[serde(default)]
    pub total_cost_usd: f64,
    #[serde(default)]
    pub total_lines_added: i64,
    #[serde(default)]
    pub total_lines_removed: i64,
    #[serde(default)]
    pub total_duration_ms: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
