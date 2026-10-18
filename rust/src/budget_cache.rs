//! Native writer for the budget-pacing session cache.
//!
//! `~/.claude/hooks/budget/budget.py` learns plan usage from a per-session JSON file.
//! Spawning a Python wrapper on every refresh is too slow on Windows, so the statusline
//! writes it directly. Opt-in: nothing is created unless `~/.claude/hooks/budget/` exists.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

/// Write the cache for this render. Never panics, never prints; all errors are swallowed.
pub fn write_cache(raw_input: &str) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let dir = crate::paths::home_dir().join(".claude").join("hooks").join("budget");
    write_cache_in(&dir, raw_input, now);
}

/// Pure core of [`write_cache`]: `budget_dir` is `<home>/.claude/hooks/budget`.
/// `rate_limits` and `context_window` are copied raw (not via `Payload`, which would
/// drop or zero fields budget.py reads, e.g. the original `resets_at` number).
pub(crate) fn write_cache_in(budget_dir: &Path, raw_input: &str, now: f64) {
    if !budget_dir.is_dir() {
        return;
    }
    let Ok(input) = serde_json::from_str::<Value>(raw_input) else { return };
    let sid = input.get("session_id").and_then(Value::as_str).unwrap_or("");
    if sid.is_empty() {
        return;
    }
    // Same filter as budget.py: keep [A-Za-z0-9_-] only.
    let mut safe: String = sid
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if safe.is_empty() {
        safe = "unknown".to_string();
    }
    let body = json!({
        "ts": now,
        "rate_limits": input.get("rate_limits").cloned().unwrap_or(Value::Null),
        "context_window": input.get("context_window").cloned().unwrap_or(Value::Null),
    });
    let Ok(text) = serde_json::to_string(&body) else { return };

    let cache_dir = budget_dir.join("cache");
    if fs::create_dir_all(&cache_dir).is_err() {
        return;
    }
    let target = cache_dir.join(format!("{safe}.json"));
    let tmp = cache_dir.join(format!("{safe}.json.tmp{}", std::process::id()));
    // Single attempt, no retries: this is the once-a-second hot path.
    if fs::write(&tmp, text).is_err() || fs::rename(&tmp, &target).is_err() {
        let _ = fs::remove_file(&tmp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Fresh, unique scratch dir (not created); tests build `<root>/budget` inside it.
    fn root(tag: &str) -> PathBuf {
        let d = std::env::temp_dir()
            .join(format!("purrgotchi-test-budget-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    fn with_budget_dir(tag: &str) -> PathBuf {
        let b = root(tag).join("budget");
        fs::create_dir_all(&b).unwrap();
        b
    }

    const FULL: &str = r#"{"session_id":"abc-123_X","rate_limits":{"five_hour":{"used_percentage":33,"resets_at":1776805200},"seven_day":{"used_percentage":20,"resets_at":1777014000}},"context_window":{"total_input_tokens":14301,"context_window_size":1000000,"current_usage":{"input_tokens":1,"output_tokens":882,"cache_creation_input_tokens":412,"cache_read_input_tokens":343223},"used_percentage":34}}"#;

    fn read(b: &Path, name: &str) -> Value {
        serde_json::from_str(&fs::read_to_string(b.join("cache").join(name)).unwrap()).unwrap()
    }

    #[test]
    fn sid_is_filtered_to_safe_chars() {
        let b = with_budget_dir("filter");
        write_cache_in(&b, r#"{"session_id":"a/b..c d_e-f9"}"#, 1.0);
        assert!(b.join("cache").join("abcd_e-f9.json").exists());
        let _ = fs::remove_dir_all(b.parent().unwrap());
    }

    #[test]
    fn all_unsafe_sid_falls_back_to_unknown() {
        let b = with_budget_dir("unknown");
        write_cache_in(&b, r#"{"session_id":"../.."}"#, 1.0);
        assert!(b.join("cache").join("unknown.json").exists());
        let _ = fs::remove_dir_all(b.parent().unwrap());
    }

    #[test]
    fn empty_or_missing_sid_writes_nothing() {
        let b = with_budget_dir("nosid");
        write_cache_in(&b, r#"{"session_id":""}"#, 1.0);
        write_cache_in(&b, r#"{"rate_limits":null}"#, 1.0);
        assert!(!b.join("cache").exists());
        let _ = fs::remove_dir_all(b.parent().unwrap());
    }

    #[test]
    fn missing_budget_dir_creates_nothing() {
        let r = root("nodir");
        let b = r.join("budget");
        write_cache_in(&b, FULL, 1.0);
        assert!(!r.exists());
    }

    #[test]
    fn creates_cache_subdir_when_budget_dir_exists() {
        let b = with_budget_dir("mkcache");
        assert!(!b.join("cache").exists());
        write_cache_in(&b, FULL, 1.0);
        assert!(b.join("cache").join("abc-123_X.json").is_file());
        let _ = fs::remove_dir_all(b.parent().unwrap());
    }

    #[test]
    fn raw_values_are_preserved() {
        let b = with_budget_dir("raw");
        write_cache_in(&b, FULL, 1.0);
        let v = read(&b, "abc-123_X.json");
        assert_eq!(v["rate_limits"]["five_hour"]["resets_at"].as_i64(), Some(1776805200));
        assert_eq!(v["rate_limits"]["seven_day"]["used_percentage"].as_i64(), Some(20));
        assert_eq!(v["context_window"]["current_usage"]["cache_read_input_tokens"].as_i64(), Some(343223));
        assert_eq!(v["context_window"]["context_window_size"].as_i64(), Some(1000000));
        let input: Value = serde_json::from_str(FULL).unwrap();
        assert_eq!(v["rate_limits"], input["rate_limits"]);
        assert_eq!(v["context_window"], input["context_window"]);
        let _ = fs::remove_dir_all(b.parent().unwrap());
    }

    #[test]
    fn absent_or_null_sections_become_null() {
        let b = with_budget_dir("null");
        write_cache_in(&b, r#"{"session_id":"s1"}"#, 1.0);
        write_cache_in(&b, r#"{"session_id":"s2","rate_limits":null,"context_window":null}"#, 1.0);
        for n in ["s1.json", "s2.json"] {
            let v = read(&b, n);
            assert!(v["rate_limits"].is_null(), "{n}");
            assert!(v["context_window"].is_null(), "{n}");
        }
        let _ = fs::remove_dir_all(b.parent().unwrap());
    }

    #[test]
    fn ts_is_a_float_near_now() {
        let b = with_budget_dir("ts");
        write_cache_in(&b, FULL, 1776800000.25);
        let v = read(&b, "abc-123_X.json");
        assert!(v["ts"].is_f64());
        assert!((v["ts"].as_f64().unwrap() - 1776800000.25).abs() < 1e-6);
        let _ = fs::remove_dir_all(b.parent().unwrap());
    }

    #[test]
    fn garbage_input_writes_nothing() {
        let b = with_budget_dir("garbage");
        for g in ["", "not json", "{", "[1,2]", "null", "42"] {
            write_cache_in(&b, g, 1.0);
        }
        assert!(!b.join("cache").exists());
        let _ = fs::remove_dir_all(b.parent().unwrap());
    }

    #[test]
    fn overwrites_existing_and_leaves_no_tmp() {
        let b = with_budget_dir("overwrite");
        write_cache_in(&b, FULL, 1.0);
        write_cache_in(&b, r#"{"session_id":"abc-123_X"}"#, 2.0);
        let v = read(&b, "abc-123_X.json");
        assert_eq!(v["ts"].as_f64(), Some(2.0));
        assert!(v["rate_limits"].is_null());
        let names: Vec<String> = fs::read_dir(b.join("cache"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["abc-123_X.json".to_string()]);
        let _ = fs::remove_dir_all(b.parent().unwrap());
    }
}
