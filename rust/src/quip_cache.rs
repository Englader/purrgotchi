//! Persistent cache for the L1 LLM-quip overlay.
//!
//! File: `~/.claude/purrgotchi/llm-quip.json`. Written by the detached
//! `purrgotchi react` worker, consumed by the statusline once per render until
//! `renders_remaining` hits zero. Kept separate from `state.json` so the async
//! worker doesn't race the synchronous `feed`/`pet`/`play`/`treat` writers.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;

use serde::{Deserialize, Serialize};

use crate::paths;

const QUIP_RENDERS: u8 = 6;
const QUIP_STALE_SECS: u64 = 5 * 60;
/// Minimum seconds between render-count decrements. With statusLine
/// refreshInterval=1 the statusline renders every second; ungated, a quip's
/// 6 renders would burn in 6 seconds instead of spanning real activity.
const QUIP_MIN_SECS_PER_RENDER: u64 = 5;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Quip {
    pub quip: String,
    pub generated_at: u64,
    pub renders_remaining: u8,
    /// Wall-clock of the last decrement (v3.7.0, additive — serde(default)
    /// for back-compat with quips written by older binaries).
    #[serde(default)]
    pub last_render_at: u64,
}

/// Atomically write a fresh quip with `renders_remaining = QUIP_RENDERS`.
/// Used by the `react` worker only.
pub fn write(text: &str) -> io::Result<()> {
    let q = Quip {
        quip: text.to_string(),
        generated_at: crate::state::now_secs(),
        renders_remaining: QUIP_RENDERS,
        last_render_at: 0,
    };
    // Serialize against a statusline consume() mid-rewrite so the fresh quip
    // can't be clobbered by a stale decremented copy.
    let _lock = crate::state::acquire_lock();
    write_quip(&q)
}

fn write_quip(q: &Quip) -> io::Result<()> {
    let dir = paths::state_dir();
    fs::create_dir_all(&dir)?;
    let pid = process::id();
    let tmp = dir.join(format!(".llm-quip.{pid}.tmp"));
    let final_path = paths::quip_cache_file();
    let json = serde_json::to_string_pretty(q)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let write_result = (|| -> io::Result<()> {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(json.as_bytes())?;
        f.write_all(b"\n")
    })();
    if let Err(e) = write_result {
        // Without this, a failed write strands a PID-named tmp forever.
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    match fs::rename(&tmp, &final_path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Read the quip file, returning `None` on missing or corrupt.
fn read_inner(path: &Path) -> Option<Quip> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Statusline-side: read the active quip, decrement (at most once per
/// QUIP_MIN_SECS_PER_RENDER), atomic-write. Returns the quip text when the
/// quip is fresh and has renders left, else None. Deletes the file when it's
/// stale or exhausted.
pub fn consume() -> Option<String> {
    let path = paths::quip_cache_file();
    // Racing the react worker's write() (or another window's consume) without
    // this lock could delete a just-written fresh quip or resurrect a stale
    // one. If the lock is busy, serve the text read-only without mutating.
    let lock = crate::state::acquire_lock();
    let mut q = read_inner(&path)?;
    let now = crate::state::now_secs();
    let stale = now.saturating_sub(q.generated_at) > QUIP_STALE_SECS;
    if stale || q.renders_remaining == 0 {
        if lock.is_some() {
            let _ = fs::remove_file(&path);
        }
        return None;
    }
    let text = q.quip.clone();
    if lock.is_none() {
        return Some(text);
    }
    if now.saturating_sub(q.last_render_at) < QUIP_MIN_SECS_PER_RENDER {
        // Gated: keep showing the quip without spending a render.
        return Some(text);
    }
    q.renders_remaining -= 1;
    q.last_render_at = now;
    if q.renders_remaining == 0 {
        let _ = fs::remove_file(&path);
    } else {
        let _ = write_quip(&q);
    }
    Some(text)
}

/// Read-only twin of [`consume`] for `frame` (no lock, no decrement, no
/// delete). Same freshness rules.
pub fn peek() -> Option<String> {
    peek_at(&paths::quip_cache_file(), crate::state::now_secs())
}

pub(crate) fn peek_at(path: &Path, now: u64) -> Option<String> {
    let q = read_inner(path)?;
    let stale = now.saturating_sub(q.generated_at) > QUIP_STALE_SECS;
    if stale || q.renders_remaining == 0 {
        return None;
    }
    Some(q.quip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quip_round_trips_through_serde() {
        let q = Quip { quip: "mrow.".into(), generated_at: 1000, renders_remaining: 6, last_render_at: 0 };
        let json = serde_json::to_string(&q).unwrap();
        let back: Quip = serde_json::from_str(&json).unwrap();
        assert_eq!(q, back);
    }

    #[test]
    fn quip_from_pre_v3_7_binary_defaults_last_render_at_to_zero() {
        // Older binaries wrote quips without the last_render_at field.
        let json = r#"{"quip":"mrow.","generated_at":1000,"renders_remaining":6}"#;
        let q: Quip = serde_json::from_str(json).unwrap();
        assert_eq!(q.last_render_at, 0);
    }

    #[test]
    fn quip_default_renders_remaining_is_six() {
        // Locks the constant against accidental change since the spec says 6 renders.
        assert_eq!(QUIP_RENDERS, 6);
    }

    #[test]
    fn stale_threshold_is_five_minutes() {
        // Locks the constant against accidental change since the spec says 5 min.
        assert_eq!(QUIP_STALE_SECS, 300);
    }

    // Filesystem-touching tests would race other tests sharing the same
    // `~/.claude/purrgotchi/llm-quip.json` path. We exercise the consume logic
    // in isolation through a pure helper:
    fn step(q: &mut Option<Quip>, now: u64) -> Option<String> {
        let cur = q.as_ref()?;
        let stale = now.saturating_sub(cur.generated_at) > QUIP_STALE_SECS;
        if stale || cur.renders_remaining == 0 {
            *q = None;
            return None;
        }
        let text = cur.quip.clone();
        if now.saturating_sub(cur.last_render_at) < QUIP_MIN_SECS_PER_RENDER {
            return Some(text);
        }
        let mut updated = cur.clone();
        updated.renders_remaining -= 1;
        updated.last_render_at = now;
        if updated.renders_remaining == 0 {
            *q = None;
        } else {
            *q = Some(updated);
        }
        Some(text)
    }

    fn fresh(quip: &str, renders: u8) -> Option<Quip> {
        Some(Quip {
            quip: quip.into(),
            generated_at: 1000,
            renders_remaining: renders,
            last_render_at: 0,
        })
    }

    #[test]
    fn consume_returns_text_and_decrements() {
        let mut q = fresh("hi", 6);
        let got = step(&mut q, 1001).unwrap();
        assert_eq!(got, "hi");
        assert_eq!(q.unwrap().renders_remaining, 5);
    }

    #[test]
    fn consume_deletes_when_renders_remaining_hits_zero() {
        let mut q = fresh("bye", 1);
        let got = step(&mut q, 1001).unwrap();
        assert_eq!(got, "bye");
        assert!(q.is_none());
    }

    #[test]
    fn consume_drops_stale_quip_without_serving() {
        let mut q = fresh("old", 6);
        let got = step(&mut q, 1000 + QUIP_STALE_SECS + 1);
        assert!(got.is_none());
        assert!(q.is_none());
    }

    #[test]
    fn consume_returns_none_for_empty() {
        let mut q: Option<Quip> = None;
        assert!(step(&mut q, 1000).is_none());
    }

    #[test]
    fn consume_drops_quip_at_renders_remaining_zero_input() {
        let mut q = fresh("zero", 0);
        let got = step(&mut q, 1001);
        assert!(got.is_none());
        assert!(q.is_none());
    }

    #[test]
    fn consume_within_gate_serves_without_decrementing() {
        // Simulates refreshInterval=1: renders at t+1, t+2, ... must keep
        // serving the quip but only spend one render per 5s window.
        let mut q = fresh("gated", 6);
        assert!(step(&mut q, 1001).is_some()); // decrements (last_render_at 0 -> 1001)
        assert_eq!(q.as_ref().unwrap().renders_remaining, 5);
        assert!(step(&mut q, 1002).is_some()); // gated — no decrement
        assert!(step(&mut q, 1005).is_some()); // gated — no decrement
        assert_eq!(q.as_ref().unwrap().renders_remaining, 5);
        assert!(step(&mut q, 1006).is_some()); // 1006-1001 >= 5 — decrements
        assert_eq!(q.as_ref().unwrap().renders_remaining, 4);
    }

    #[test]
    fn min_seconds_per_render_is_five() {
        // Locks the pacing constant: 6 renders x 5s = a quip lives >= 30s
        // under a 1s refresh interval.
        assert_eq!(QUIP_MIN_SECS_PER_RENDER, 5);
    }

    #[test]
    fn peek_returns_fresh_quip_without_touching_the_file() {
        let p = std::env::temp_dir().join(format!("purrgotchi-test-peek-{}.json", std::process::id()));
        let q = Quip { quip: "mrow.".into(), generated_at: 1000, renders_remaining: 3, last_render_at: 0 };
        std::fs::write(&p, serde_json::to_string(&q).unwrap()).unwrap();
        let before = std::fs::read(&p).unwrap();
        assert_eq!(peek_at(&p, 1010), Some("mrow.".to_string()));
        assert_eq!(std::fs::read(&p).unwrap(), before);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn peek_ignores_stale_exhausted_and_missing() {
        let p = std::env::temp_dir().join(format!("purrgotchi-test-peek-stale-{}.json", std::process::id()));
        let stale = Quip { quip: "x".into(), generated_at: 1000, renders_remaining: 3, last_render_at: 0 };
        std::fs::write(&p, serde_json::to_string(&stale).unwrap()).unwrap();
        assert_eq!(peek_at(&p, 1000 + QUIP_STALE_SECS + 1), None);
        let spent = Quip { renders_remaining: 0, ..stale };
        std::fs::write(&p, serde_json::to_string(&spent).unwrap()).unwrap();
        assert_eq!(peek_at(&p, 1001), None);
        assert!(p.exists(), "peek never deletes");
        let _ = std::fs::remove_file(&p);
        assert_eq!(peek_at(&p, 1001), None);
    }
}
