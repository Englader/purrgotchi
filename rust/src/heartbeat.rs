//! Band heartbeat: the mod writes `band-<session>.alive` (decimal epoch ms) about once a
//! second while the cat is really on screen. The statusline drops its own cat only while
//! that beat is fresh, so a crashed or collapsed band hands the cat back within seconds.

use std::fs;
use std::path::{Path, PathBuf};

/// A beat older than this (ms) no longer counts.
const MAX_AGE_MS: u64 = 3000;
/// Clock skew tolerated for a beat stamped in the future (ms).
const MAX_SKEW_MS: u64 = 1000;

/// `[A-Za-z0-9-]{1,64}`: keeps the id safe to embed in a file name.
pub(crate) fn valid_session_id(id: &str) -> bool {
    (1..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

pub(crate) fn heartbeat_path(dir: &Path, session_id: &str) -> Option<PathBuf> {
    valid_session_id(session_id).then(|| dir.join(format!("band-{session_id}.alive")))
}

fn fresh(path: &Path, now_ms: u64) -> bool {
    let Some(hb) = fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
    else {
        return false;
    };
    hb <= now_ms.saturating_add(MAX_SKEW_MS) && now_ms.saturating_sub(hb) <= MAX_AGE_MS
}

/// Is a band live for this session? Falls back to the global `band.alive`, which the mod
/// writes only when it could not learn a session id.
pub(crate) fn band_alive_in(dir: &Path, session_id: &str, now_ms: u64) -> bool {
    heartbeat_path(dir, session_id).is_some_and(|p| fresh(&p, now_ms))
        || fresh(&dir.join("band.alive"), now_ms)
}

// Wired into the statusline in Task 3.4.
#[allow(dead_code)]
pub(crate) fn band_alive(session_id: &str) -> bool {
    band_alive_in(
        &crate::paths::state_dir(),
        session_id,
        crate::state::now_micros() / 1000,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_800_000_000_000;

    fn dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "purr-hb-{}-{}",
            std::process::id(),
            crate::state::now_micros()
        ));
        let d = d.join(format!("{:?}", std::thread::current().id()).replace(['(', ')'], ""));
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn beat(d: &Path, name: &str, body: &str) {
        fs::write(d.join(name), body).unwrap();
    }

    #[test]
    fn fresh_within_3s() {
        let d = dir();
        beat(&d, "band-s1.alive", &(NOW - 3000).to_string());
        assert!(band_alive_in(&d, "s1", NOW));
        beat(&d, "band-s1.alive", &(NOW - 3001).to_string());
        assert!(!band_alive_in(&d, "s1", NOW));
    }

    #[test]
    fn future_skew_tolerated_1s() {
        let d = dir();
        beat(&d, "band-s1.alive", &(NOW + 1000).to_string());
        assert!(band_alive_in(&d, "s1", NOW));
        beat(&d, "band-s1.alive", &(NOW + 1001).to_string());
        assert!(!band_alive_in(&d, "s1", NOW));
    }

    #[test]
    fn per_session_beats_global() {
        let d = dir();
        beat(&d, "band-other.alive", &NOW.to_string());
        assert!(!band_alive_in(&d, "s1", NOW));
        assert!(band_alive_in(&d, "other", NOW));
    }

    #[test]
    fn global_fallback() {
        let d = dir();
        beat(&d, "band.alive", &format!(" {NOW}\n"));
        assert!(band_alive_in(&d, "s1", NOW));
        // An invalid id can still see the global beat, never a path.
        assert!(band_alive_in(&d, "../x", NOW));
    }

    #[test]
    fn bad_ids_rejected() {
        let d = dir();
        assert!(!valid_session_id("../x"));
        assert!(!valid_session_id(""));
        assert!(!valid_session_id(&"a".repeat(65)));
        assert!(valid_session_id(&"a".repeat(64)));
        assert!(valid_session_id("ab-12-CD"));
        assert!(heartbeat_path(&d, "../x").is_none());
        assert_eq!(heartbeat_path(&d, "s1"), Some(d.join("band-s1.alive")));
    }

    #[test]
    fn garbage_content_false() {
        let d = dir();
        beat(&d, "band-s1.alive", "not a number");
        assert!(!band_alive_in(&d, "s1", NOW));
        beat(&d, "band-s1.alive", "");
        assert!(!band_alive_in(&d, "s1", NOW));
    }
}
