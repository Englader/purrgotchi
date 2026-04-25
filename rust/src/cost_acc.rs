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

/// Pure: how much new cost to credit, given the previous and current cumulative
/// values for one session. Clamps negatives to zero so we never debit.
pub fn compute_credit(prev: f64, cur: f64) -> f64 {
    (cur - prev).max(0.0)
}

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

/// Pure: drop entries whose `last_seen_ts` is older than `now - max_age_secs`.
/// Saturating subtraction keeps the boundary safe when `now < max_age_secs`.
pub fn gc_old_entries(sessions: &mut CostSessions, now: u64, max_age_secs: u64) {
    let cutoff = now.saturating_sub(max_age_secs);
    sessions.retain(|_, entry| entry.last_seen_ts >= cutoff);
}

const MAX_AGE_SECS: u64 = 30 * 86_400;

/// Pure orchestrator: skip on empty id; otherwise compute credit, update state,
/// upsert sidecar entry, GC stale entries. Returns the credit applied.
pub fn apply(
    sessions: &mut CostSessions,
    state: &mut crate::state::State,
    session_id: &str,
    cur: f64,
    now: u64,
    max_age_secs: u64,
) -> f64 {
    if session_id.is_empty() {
        return 0.0;
    }
    let prev = sessions.get(session_id).map(|e| e.last_cost).unwrap_or(0.0);
    let credit = compute_credit(prev, cur);
    state.lifetime_cost_usd += credit;
    sessions.insert(
        session_id.to_string(),
        SessionEntry { last_cost: cur, last_seen_ts: now },
    );
    gc_old_entries(sessions, now, max_age_secs);
    credit
}

/// I/O entry point: the only function `statusline::run` should call.
///
/// Order of operations:
/// 1. Bail immediately on empty `session_id`.
/// 2. Load sidecar (best-effort — corrupt/missing yields empty map).
/// 3. Apply pure update (advances sidecar map, increments in-memory state).
/// 4. **Save sidecar first.** If this fails, we abort and the next render
///    retries the same delta — no double-count.
/// 5. **Then save state.** If this fails, we lose at most one render's delta
///    — never duplicate. See spec §2 write-order rationale.
pub fn accumulate_from_payload(session_id: &str, cur: f64) {
    if session_id.is_empty() {
        return;
    }
    let now = crate::state::now_secs();
    let path = crate::paths::cost_sessions_file();
    let mut sessions = load_at(&path);

    let mut state = match crate::state::load() {
        Some(s) => s,
        None => return, // not yet hatched — nothing to accumulate against
    };

    let credit = apply(&mut sessions, &mut state, session_id, cur, now, MAX_AGE_SECS);

    // Sidecar first.
    if save_at(&path, &sessions).is_err() {
        return;
    }
    // Only credit the lifetime field on disk if we actually moved the needle.
    if credit > 0.0 {
        let _ = crate::state::save_atomic(&state);
    }
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

    #[test]
    fn credit_is_cur_minus_prev_when_growing() {
        assert!((compute_credit(1.0, 1.5) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn credit_is_zero_when_cur_equals_prev() {
        assert_eq!(compute_credit(1.5, 1.5), 0.0);
    }

    #[test]
    fn credit_clamps_to_zero_when_cur_drops() {
        // Guards against any payload weirdness where cur briefly drops below prev.
        assert_eq!(compute_credit(2.0, 1.0), 0.0);
    }

    #[test]
    fn credit_for_brand_new_session_is_full_cur() {
        // Caller passes prev = 0.0 when the session_id is unknown.
        assert!((compute_credit(0.0, 0.42) - 0.42).abs() < 1e-9);
    }

    #[test]
    fn gc_drops_entries_older_than_max_age() {
        let now = 10_000_000;
        let max_age = 30 * 86_400; // 30 days in seconds
        let mut m: CostSessions = std::collections::HashMap::new();
        m.insert("fresh".into(),  SessionEntry { last_cost: 1.0, last_seen_ts: now });
        m.insert("recent".into(), SessionEntry { last_cost: 2.0, last_seen_ts: now - max_age + 60 });
        m.insert("stale".into(),  SessionEntry { last_cost: 3.0, last_seen_ts: now - max_age - 1 });
        m.insert("ancient".into(),SessionEntry { last_cost: 4.0, last_seen_ts: 100 });
        gc_old_entries(&mut m, now, max_age);
        assert!(m.contains_key("fresh"));
        assert!(m.contains_key("recent"));
        assert!(!m.contains_key("stale"));
        assert!(!m.contains_key("ancient"));
    }

    #[test]
    fn gc_keeps_entry_at_exactly_max_age_boundary() {
        // Boundary: last_seen_ts == now - max_age is kept (>= now - max_age).
        let now = 10_000_000;
        let max_age = 100;
        let mut m: CostSessions = std::collections::HashMap::new();
        m.insert("edge".into(), SessionEntry { last_cost: 1.0, last_seen_ts: now - max_age });
        gc_old_entries(&mut m, now, max_age);
        assert!(m.contains_key("edge"));
    }

    #[test]
    fn gc_handles_empty_map() {
        let mut m: CostSessions = std::collections::HashMap::new();
        gc_old_entries(&mut m, 10_000_000, 30 * 86_400);
        assert!(m.is_empty());
    }

    fn fresh_state() -> crate::state::State {
        // Minimal state for testing — only lifetime_cost_usd matters here.
        let mut s = crate::state::State::base(0);
        s.lifetime_cost_usd = 0.0;
        s
    }

    #[test]
    fn apply_credits_full_cur_for_brand_new_session_id() {
        let mut sessions: CostSessions = std::collections::HashMap::new();
        let mut state = fresh_state();
        let credit = apply(&mut sessions, &mut state, "sess-A", 1.50, 1_000, 30 * 86_400);
        assert!((credit - 1.50).abs() < 1e-9);
        assert!((state.lifetime_cost_usd - 1.50).abs() < 1e-9);
        let e = sessions.get("sess-A").unwrap();
        assert!((e.last_cost - 1.50).abs() < 1e-9);
        assert_eq!(e.last_seen_ts, 1_000);
    }

    #[test]
    fn apply_credits_only_delta_within_same_session() {
        let mut sessions: CostSessions = std::collections::HashMap::new();
        let mut state = fresh_state();
        apply(&mut sessions, &mut state, "sess-A", 1.50, 1_000, 30 * 86_400);
        let credit2 = apply(&mut sessions, &mut state, "sess-A", 2.10, 1_010, 30 * 86_400);
        assert!((credit2 - 0.60).abs() < 1e-9);
        assert!((state.lifetime_cost_usd - 2.10).abs() < 1e-9);
    }

    #[test]
    fn apply_clamps_when_cur_drops_below_prev() {
        let mut sessions: CostSessions = std::collections::HashMap::new();
        let mut state = fresh_state();
        apply(&mut sessions, &mut state, "sess-A", 5.00, 1_000, 30 * 86_400);
        let credit = apply(&mut sessions, &mut state, "sess-A", 4.00, 1_010, 30 * 86_400);
        assert_eq!(credit, 0.0);
        assert!((state.lifetime_cost_usd - 5.00).abs() < 1e-9);
        // Sidecar still advances to the new (lower) cur so we don't re-credit.
        let e = sessions.get("sess-A").unwrap();
        assert!((e.last_cost - 4.00).abs() < 1e-9);
    }

    #[test]
    fn apply_handles_parallel_sessions_independently() {
        let mut sessions: CostSessions = std::collections::HashMap::new();
        let mut state = fresh_state();
        apply(&mut sessions, &mut state, "sess-A", 1.00, 1_000, 30 * 86_400);
        apply(&mut sessions, &mut state, "sess-B", 0.00, 1_001, 30 * 86_400);
        apply(&mut sessions, &mut state, "sess-A", 1.50, 1_002, 30 * 86_400);
        apply(&mut sessions, &mut state, "sess-B", 0.25, 1_003, 30 * 86_400);
        // Total: 1.00 (A first) + 0.00 (B first) + 0.50 (A) + 0.25 (B) = 1.75
        assert!((state.lifetime_cost_usd - 1.75).abs() < 1e-9);
    }

    #[test]
    fn apply_skips_when_session_id_empty() {
        let mut sessions: CostSessions = std::collections::HashMap::new();
        let mut state = fresh_state();
        let credit = apply(&mut sessions, &mut state, "", 9.99, 1_000, 30 * 86_400);
        assert_eq!(credit, 0.0);
        assert_eq!(state.lifetime_cost_usd, 0.0);
        assert!(sessions.is_empty(), "no phantom \"\" entry should be created");
    }

    #[test]
    fn apply_runs_gc_on_each_call() {
        let mut sessions: CostSessions = std::collections::HashMap::new();
        sessions.insert(
            "stale".to_string(),
            SessionEntry { last_cost: 7.0, last_seen_ts: 0 },
        );
        let mut state = fresh_state();
        apply(&mut sessions, &mut state, "fresh", 1.0, 30 * 86_400 + 100, 30 * 86_400);
        assert!(!sessions.contains_key("stale"));
        assert!(sessions.contains_key("fresh"));
    }
}
