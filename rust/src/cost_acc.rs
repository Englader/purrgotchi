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

/// Save sidecar to `path` atomically (write PID-qualified temp + rename).
///
/// The tmp filename includes the process ID (`.cost-sessions.{pid}.tmp`) so
/// two parallel Claude Code windows never clobber each other's tmp file.
/// Mirrors the pattern used by `state::save_atomic`.  Best-effort — caller
/// decides what to do on failure.
pub fn save_at(path: &Path, sessions: &CostSessions) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let pid = std::process::id();
    let tmp = match path.parent() {
        Some(dir) => dir.join(format!(".cost-sessions.{pid}.tmp")),
        None => std::path::PathBuf::from(format!(".cost-sessions.{pid}.tmp")),
    };
    let bytes = serde_json::to_vec_pretty(sessions).map_err(io::Error::other)?;
    fs::write(&tmp, bytes)?;
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Pure: drop entries whose `last_seen_ts` is older than `now - max_age_secs`.
/// Saturating subtraction keeps the boundary safe when `now < max_age_secs`.
pub fn gc_old_entries(sessions: &mut CostSessions, now: u64, max_age_secs: u64) {
    let cutoff = now.saturating_sub(max_age_secs);
    sessions.retain(|_, entry| entry.last_seen_ts >= cutoff);
}

const MAX_AGE_SECS: u64 = 30 * 86_400;

/// Return value from [`apply`]: the credit amount and whether the sidecar map
/// was actually mutated (so the caller can skip disk I/O when nothing changed).
pub struct ApplyOutcome {
    pub credit: f64,
    pub sidecar_dirty: bool,
}

/// Pure orchestrator: skip on empty id; otherwise compute credit, update state,
/// upsert sidecar entry, GC stale entries. Returns [`ApplyOutcome`].
///
/// **Steady-state no-op optimisation:** when `credit == 0` *and* `cur` equals
/// the stored `last_cost` *and* no entries are old enough to be GC'd, we skip
/// the upsert and the GC pass entirely and return `sidecar_dirty = false`.
/// This is intentionally OK — GC only matters at the moment GC runs, and the
/// entry will get a fresh `last_seen_ts` on the next render where credit DOES
/// move.  Skipping the upsert prevents every no-op render from touching the
/// sidecar file on disk.
pub fn apply(
    sessions: &mut CostSessions,
    state: &mut crate::state::State,
    session_id: &str,
    cur: f64,
    now: u64,
    max_age_secs: u64,
) -> ApplyOutcome {
    if session_id.is_empty() {
        return ApplyOutcome { credit: 0.0, sidecar_dirty: false };
    }
    let prev_entry = sessions.get(session_id).cloned();
    let prev_cost = prev_entry.as_ref().map(|e| e.last_cost).unwrap_or(0.0);
    let credit = compute_credit(prev_cost, cur);
    state.lifetime_cost_usd += credit;

    // cur_differs_from_prev is true for brand-new sessions, cost growth, and
    // cost drops (cur < prev).  It is only false on a true steady-state render
    // where the session already exists and cur hasn't moved at all.
    let cur_differs_from_prev = match &prev_entry {
        Some(e) => (e.last_cost - cur).abs() > f64::EPSILON,
        None => true,
    };
    let need_upsert = credit > 0.0 || cur_differs_from_prev;

    let gc_will_change = sessions
        .iter()
        .any(|(_, e)| e.last_seen_ts < now.saturating_sub(max_age_secs));

    let sidecar_dirty = need_upsert || gc_will_change;

    if !sidecar_dirty {
        // Steady-state no-op: same session, same cost, no entries to GC.
        return ApplyOutcome { credit, sidecar_dirty };
    }

    sessions.insert(
        session_id.to_string(),
        SessionEntry { last_cost: cur, last_seen_ts: now },
    );
    gc_old_entries(sessions, now, max_age_secs);

    ApplyOutcome { credit, sidecar_dirty }
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
pub fn accumulate_from_payload(session_id: &str, cur: f64) -> f64 {
    if session_id.is_empty() {
        return 0.0;
    }
    let now = crate::state::now_secs();
    let path = crate::paths::cost_sessions_file();
    // v3.7.0: hold the global state lock across the whole read-modify-write.
    // Without it, two concurrent Claude Code windows last-writer-win each
    // other's sidecar entry (double-crediting lifetime cost), and our
    // whole-struct state save could clobber a concurrent feed hook's
    // increments. A missed lock just skips this render; the next one retries
    // the same delta.
    let Some(_lock) = crate::state::acquire_lock() else { return 0.0 };
    let mut sessions = load_at(&path);

    let mut state = match crate::state::load() {
        Some(s) => s,
        None => return 0.0, // not yet hatched — nothing to accumulate against
    };

    let outcome = apply(&mut sessions, &mut state, session_id, cur, now, MAX_AGE_SECS);

    if !outcome.sidecar_dirty && outcome.credit == 0.0 {
        return 0.0; // steady-state no-op render — zero disk writes
    }

    // Sidecar first.
    if outcome.sidecar_dirty && save_at(&path, &sessions).is_err() {
        return 0.0;
    }
    // Only credit the lifetime field on disk if we actually moved the needle.
    if outcome.credit > 0.0 && crate::state::save_atomic(&state).is_err() {
        return 0.0;
    }
    outcome.credit
}

/// `purrgotchi cost --session <id> --total <usd> [--json]` — the statusline's
/// lifetime-cost tally as a command, for the mod's `session.measure` hook.
pub fn run_cmd(session_id: &str, total: f64, json: bool) -> i32 {
    // Check first: accumulate_from_payload would take the lock (creating the
    // lock file) before discovering there is no kitten.
    if let Err(fail) = crate::state::load_for_read() {
        crate::json::state_read_error(fail, json);
        return 1;
    }
    let credit = accumulate_from_payload(session_id, total);
    let lifetime = crate::state::load().map(|s| s.lifetime_cost_usd).unwrap_or(0.0);
    if json {
        crate::json::print(&crate::json::CostJson { v: crate::json::V, ok: true, credit, lifetime_cost_usd: lifetime });
    } else {
        println!("credited ${credit:.4} (lifetime ${lifetime:.2})");
    }
    0
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
        let outcome = apply(&mut sessions, &mut state, "sess-A", 1.50, 1_000, 30 * 86_400);
        assert!((outcome.credit - 1.50).abs() < 1e-9);
        assert!(outcome.sidecar_dirty, "brand-new session must be dirty");
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
        let outcome2 = apply(&mut sessions, &mut state, "sess-A", 2.10, 1_010, 30 * 86_400);
        assert!((outcome2.credit - 0.60).abs() < 1e-9);
        assert!((state.lifetime_cost_usd - 2.10).abs() < 1e-9);
    }

    #[test]
    fn apply_clamps_when_cur_drops_below_prev() {
        let mut sessions: CostSessions = std::collections::HashMap::new();
        let mut state = fresh_state();
        apply(&mut sessions, &mut state, "sess-A", 5.00, 1_000, 30 * 86_400);
        let outcome = apply(&mut sessions, &mut state, "sess-A", 4.00, 1_010, 30 * 86_400);
        assert_eq!(outcome.credit, 0.0);
        assert!((state.lifetime_cost_usd - 5.00).abs() < 1e-9);
        // Sidecar still advances to the new (lower) cur so we don't re-credit
        // on future renders if cur recovers. cur differs from prev → dirty.
        assert!(outcome.sidecar_dirty, "cur drop should still be dirty to advance last_cost");
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
        let outcome = apply(&mut sessions, &mut state, "", 9.99, 1_000, 30 * 86_400);
        assert_eq!(outcome.credit, 0.0);
        assert!(!outcome.sidecar_dirty, "empty session_id should not be dirty");
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

    // ── New tests for ApplyOutcome dirty flag ─────────────────────────────────

    #[test]
    fn apply_returns_dirty_true_when_credit_grew() {
        let mut sessions: CostSessions = std::collections::HashMap::new();
        let mut state = fresh_state();
        // Seed entry.
        apply(&mut sessions, &mut state, "sess-A", 1.00, 1_000, 30 * 86_400);
        // cur grows → credit > 0 → dirty.
        let outcome = apply(&mut sessions, &mut state, "sess-A", 1.50, 1_010, 30 * 86_400);
        assert!((outcome.credit - 0.50).abs() < 1e-9);
        assert!(outcome.sidecar_dirty);
    }

    #[test]
    fn apply_returns_dirty_true_when_new_session_id() {
        let mut sessions: CostSessions = std::collections::HashMap::new();
        let mut state = fresh_state();
        // A completely fresh session_id with cur=0 is still a new entry.
        let outcome = apply(&mut sessions, &mut state, "brand-new", 0.00, 1_000, 30 * 86_400);
        assert!(outcome.sidecar_dirty, "inserting a new entry must be dirty");
    }

    #[test]
    fn apply_returns_dirty_false_for_steady_state_no_op() {
        let mut sessions: CostSessions = std::collections::HashMap::new();
        let mut state = fresh_state();
        // First call seeds the entry.
        let r1 = apply(&mut sessions, &mut state, "sess-A", 1.50, 1_000, 30 * 86_400);
        assert!(r1.sidecar_dirty);
        // Second call with the SAME cur → steady-state no-op.
        let r2 = apply(&mut sessions, &mut state, "sess-A", 1.50, 1_010, 30 * 86_400);
        assert_eq!(r2.credit, 0.0);
        assert!(!r2.sidecar_dirty, "no credit, no GC, same cur → not dirty");
    }

    #[test]
    fn apply_returns_dirty_true_when_gc_removes_stale_entry() {
        let mut sessions: CostSessions = std::collections::HashMap::new();
        // Plant a stale entry (ts = 0 → very old).
        sessions.insert("stale".into(), SessionEntry { last_cost: 7.0, last_seen_ts: 0 });
        // Plant a current entry for sess-A (ts well within max_age).
        sessions.insert(
            "sess-A".into(),
            SessionEntry { last_cost: 1.50, last_seen_ts: 30 * 86_400 + 100 },
        );
        let mut state = fresh_state();
        // No credit movement (same cur as stored), but stale entry needs GC.
        let outcome = apply(
            &mut sessions,
            &mut state,
            "sess-A",
            1.50,
            30 * 86_400 + 200,
            30 * 86_400,
        );
        assert_eq!(outcome.credit, 0.0);
        assert!(outcome.sidecar_dirty, "GC removal alone should mark dirty");
        assert!(!sessions.contains_key("stale"), "stale entry should be GC'd");
    }
}
