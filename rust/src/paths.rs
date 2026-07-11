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
    home_dir().join(".claude").join("purrgotchi")
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

pub fn cost_sessions_file() -> PathBuf {
    state_dir().join("cost-sessions.json")
}

pub fn quip_cache_file() -> PathBuf {
    state_dir().join("llm-quip.json")
}
