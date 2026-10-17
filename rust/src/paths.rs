use std::env;
use std::path::PathBuf;

pub fn home_dir() -> PathBuf {
    // Windows: USERPROFILE first — HOME may carry a POSIX-style path (MSYS2/
    // Cygwin) that Windows APIs read as drive-relative (C:\home\...), which
    // would silently split runtime state into a second kitten.
    #[cfg(windows)]
    const ORDER: [&str; 2] = ["USERPROFILE", "HOME"];
    #[cfg(not(windows))]
    const ORDER: [&str; 2] = ["HOME", "USERPROFILE"];
    for var in ORDER {
        if let Ok(h) = env::var(var)
            && !h.is_empty()
        {
            return PathBuf::from(h);
        }
    }
    PathBuf::from(".")
}

pub fn state_dir() -> PathBuf {
    state_dir_from(env::var_os("PURR_STATE_DIR"), home_dir())
}

/// Pure twin of [`state_dir`]. A non-empty, absolute `PURR_STATE_DIR` replaces the whole
/// `~/.claude/purrgotchi` path (an advanced/testing knob, documented in the README), so
/// tests never have to fake HOME.
pub(crate) fn state_dir_from(override_dir: Option<std::ffi::OsString>, home: PathBuf) -> PathBuf {
    match override_dir {
        // Relative paths would resolve against whatever project Claude Code
        // happens to have as cwd — ignore them rather than scatter state.
        Some(d) if !d.is_empty() && std::path::Path::new(&d).is_absolute() => PathBuf::from(d),
        _ => home.join(".claude").join("purrgotchi"),
    }
}

pub fn state_file() -> PathBuf {
    state_dir().join("state.json")
}

/// Legacy global anchor — kept only as the empty-session-id fallback and so
/// `hooks::turn` can clean up files left by pre-v3.7.0 binaries.
pub fn turn_anchor_file() -> PathBuf {
    state_dir().join("turn-start-cost.txt")
}

/// Per-session turn-cost anchor. Two concurrent Claude Code windows carry
/// different session ids and different cumulative costs; a shared anchor made
/// each window corrupt the other's per-turn Cost display.
pub fn turn_anchor_file_for(session_id: &str) -> PathBuf {
    if session_id.is_empty() {
        return turn_anchor_file();
    }
    let h = crate::identity::fnv1a(session_id);
    state_dir().join(format!("turn-start-cost-{h:016x}.txt"))
}

/// Cross-process lock file serializing all state mutations.
pub fn lock_file() -> PathBuf {
    state_dir().join(".purr.lock")
}

/// Trusted .env location. The binary deliberately does not read .env from the
/// inherited cwd (any open project could inject HOME or API keys).
pub fn env_file() -> PathBuf {
    state_dir().join(".env")
}

/// Dev/test backdoors that only the process environment may set. The state-dir
/// `.env` is user-writable config for API keys — it must never be able to
/// freeze the clock (PURR_NOW) or redirect the state dir (PURR_STATE_DIR).
const PROCESS_ENV_ONLY: [&str; 2] = ["PURR_NOW", "PURR_STATE_DIR"];

/// Load `.env` from the trusted state dir into the process environment.
/// Same semantics as `dotenvy::from_path` (existing process vars win, a missing
/// file is a no-op, parsing stops at the first malformed line) except that the
/// [`PROCESS_ENV_ONLY`] keys are skipped.
pub fn load_trusted_env() {
    let Ok(iter) = dotenvy::from_path_iter(env_file()) else { return };
    for item in iter {
        let Ok((key, value)) = item else { break };
        if PROCESS_ENV_ONLY.contains(&key.as_str()) || env::var_os(&key).is_some() {
            continue;
        }
        // SAFETY: called first thing in main(), before any thread is spawned.
        unsafe { env::set_var(&key, &value) };
    }
}

pub fn cost_sessions_file() -> PathBuf {
    state_dir().join("cost-sessions.json")
}

pub fn quip_cache_file() -> PathBuf {
    state_dir().join("llm-quip.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn state_dir_defaults_under_home() {
        assert_eq!(
            state_dir_from(None, PathBuf::from("/h")),
            PathBuf::from("/h").join(".claude").join("purrgotchi")
        );
    }

    #[test]
    fn state_dir_override_wins() {
        assert_eq!(
            state_dir_from(Some(OsString::from("/tmp/purr-x")), PathBuf::from("/h")),
            PathBuf::from("/tmp/purr-x")
        );
    }

    #[test]
    fn state_dir_relative_override_is_ignored() {
        for rel in ["purr-x", "./purr-x", "../purr-x"] {
            assert_eq!(
                state_dir_from(Some(OsString::from(rel)), PathBuf::from("/h")),
                PathBuf::from("/h").join(".claude").join("purrgotchi"),
                "{rel}"
            );
        }
    }

    #[test]
    fn state_dir_empty_override_is_ignored() {
        assert_eq!(
            state_dir_from(Some(OsString::new()), PathBuf::from("/h")),
            PathBuf::from("/h").join(".claude").join("purrgotchi")
        );
    }
}
