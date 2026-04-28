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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Quip {
    pub quip: String,
    pub generated_at: u64,
    pub renders_remaining: u8,
}

/// Atomically write a fresh quip with `renders_remaining = QUIP_RENDERS`.
/// Used by the `react` worker only.
pub fn write(text: &str) -> io::Result<()> {
    let q = Quip {
        quip: text.to_string(),
        generated_at: crate::state::now_secs(),
        renders_remaining: QUIP_RENDERS,
    };
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
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(json.as_bytes())?;
        f.write_all(b"\n")?;
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

/// Statusline-side: read the active quip, decrement, atomic-write.
/// Returns the quip text when the quip is fresh and has renders left, else None.
/// Deletes the file when it's stale or exhausted.
pub fn consume() -> Option<String> {
    let path = paths::quip_cache_file();
    let mut q = read_inner(&path)?;
    let now = crate::state::now_secs();
    let stale = now.saturating_sub(q.generated_at) > QUIP_STALE_SECS;
    if stale || q.renders_remaining == 0 {
        let _ = fs::remove_file(&path);
        return None;
    }
    let text = q.quip.clone();
    q.renders_remaining -= 1;
    if q.renders_remaining == 0 {
        let _ = fs::remove_file(&path);
    } else {
        let _ = write_quip(&q);
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quip_round_trips_through_serde() {
        let q = Quip { quip: "mrow.".into(), generated_at: 1000, renders_remaining: 6 };
        let json = serde_json::to_string(&q).unwrap();
        let back: Quip = serde_json::from_str(&json).unwrap();
        assert_eq!(q, back);
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
        let mut updated = cur.clone();
        updated.renders_remaining -= 1;
        if updated.renders_remaining == 0 {
            *q = None;
        } else {
            *q = Some(updated);
        }
        Some(text)
    }

    #[test]
    fn consume_returns_text_and_decrements() {
        let mut q = Some(Quip { quip: "hi".into(), generated_at: 1000, renders_remaining: 6 });
        let got = step(&mut q, 1001).unwrap();
        assert_eq!(got, "hi");
        assert_eq!(q.unwrap().renders_remaining, 5);
    }

    #[test]
    fn consume_deletes_when_renders_remaining_hits_zero() {
        let mut q = Some(Quip { quip: "bye".into(), generated_at: 1000, renders_remaining: 1 });
        let got = step(&mut q, 1001).unwrap();
        assert_eq!(got, "bye");
        assert!(q.is_none());
    }

    #[test]
    fn consume_drops_stale_quip_without_serving() {
        let mut q = Some(Quip { quip: "old".into(), generated_at: 1000, renders_remaining: 6 });
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
        let mut q = Some(Quip { quip: "zero".into(), generated_at: 1000, renders_remaining: 0 });
        let got = step(&mut q, 1001);
        assert!(got.is_none());
        assert!(q.is_none());
    }
}
