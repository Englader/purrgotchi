//! Lifetime cost accumulator.
//!
//! `statusline::run` calls `accumulate_from_payload(session_id, cur)` on every
//! render. We track per-session monotonic deltas in a sidecar file
//! (`paths::cost_sessions_file()`) and credit each delta to
//! `state.lifetime_cost_usd`. See
//! `docs/superpowers/specs/2026-04-25-v3.4.7-cost-accumulator-design.md`.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub struct SessionEntry {
    pub last_cost: f64,
    pub last_seen_ts: u64,
}

pub type CostSessions = HashMap<String, SessionEntry>;

/// Load sidecar from `path`. Missing file or corrupt JSON yields an empty map.
pub fn load_at(path: &Path) -> CostSessions {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(_) => return CostSessions::new(),
    };
    serde_json::from_slice::<CostSessions>(&bytes).unwrap_or_default()
}

/// Save sidecar to `path` atomically (write temp + rename). Best-effort —
/// caller decides what to do on failure.
pub fn save_at(path: &Path, sessions: &CostSessions) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(sessions)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_entry_serde_round_trip() {
        let mut map: CostSessions = std::collections::HashMap::new();
        map.insert(
            "uuid-1".to_string(),
            SessionEntry { last_cost: 1.23, last_seen_ts: 1_700_000_000 },
        );
        let json = serde_json::to_string(&map).unwrap();
        let back: CostSessions = serde_json::from_str(&json).unwrap();
        let e = back.get("uuid-1").unwrap();
        assert!((e.last_cost - 1.23).abs() < 1e-9);
        assert_eq!(e.last_seen_ts, 1_700_000_000);
    }

    #[test]
    fn load_returns_empty_when_file_missing() {
        let p = std::env::temp_dir().join("purrgotchi-test-missing-cost-sessions.json");
        let _ = std::fs::remove_file(&p);
        let map = load_at(&p);
        assert!(map.is_empty());
    }

    #[test]
    fn load_returns_empty_when_file_corrupt() {
        let p = std::env::temp_dir().join("purrgotchi-test-corrupt-cost-sessions.json");
        std::fs::write(&p, "this is not json").unwrap();
        let map = load_at(&p);
        assert!(map.is_empty());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn save_then_load_round_trip() {
        let p = std::env::temp_dir().join("purrgotchi-test-rt-cost-sessions.json");
        let _ = std::fs::remove_file(&p);
        let mut m: CostSessions = std::collections::HashMap::new();
        m.insert("u".to_string(), SessionEntry { last_cost: 4.2, last_seen_ts: 99 });
        save_at(&p, &m).unwrap();
        let loaded = load_at(&p);
        assert_eq!(loaded.len(), 1);
        let e = loaded.get("u").unwrap();
        assert!((e.last_cost - 4.2).abs() < 1e-9);
        assert_eq!(e.last_seen_ts, 99);
        let _ = std::fs::remove_file(&p);
    }
}
