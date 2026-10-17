//! Shared harness for integration tests. Every test runs the real binary in a
//! sandbox: HOME and PURR_STATE_DIR both point into a fresh temp dir and the
//! child's environment is cleared, so the developer's own PURR_* / TZ /
//! COLUMNS / API keys can never leak into results. No extra dependencies.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct Sandbox {
    pub root: PathBuf,
    pub home: PathBuf,
    pub state: PathBuf,
}

impl Sandbox {
    pub fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("purr-it-{tag}-{}-{n}", std::process::id()));
        // read_git_branch walks up from the payload cwd. A sandbox inside the
        // repo would find the repo's own .git and leak the real branch name.
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
        assert!(!root.starts_with(&repo), "sandbox {root:?} is inside the repo");
        let _ = std::fs::remove_dir_all(&root);
        let home = root.join("home");
        let state = root.join("state");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&state).unwrap();
        Sandbox { root, home, state }
    }

    pub fn cmd(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_purrgotchi"));
        c.args(args)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("PURR_STATE_DIR", &self.state);
        c
    }

    /// Spawn with `stdin` piped in plus extra env vars; returns the Output.
    pub fn run(&self, args: &[&str], stdin: &str, env: &[(&str, &str)]) -> Output {
        let mut c = self.cmd(args);
        for (k, v) in env {
            c.env(k, v);
        }
        c.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = c.spawn().expect("spawn purrgotchi");
        // Commands that never read stdin (feed) may exit first → EPIPE. Ignore.
        let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
        child.wait_with_output().expect("wait purrgotchi")
    }

    pub fn write_state(&self, v: &serde_json::Value) {
        std::fs::write(self.state.join("state.json"), serde_json::to_string_pretty(v).unwrap()).unwrap();
    }

    pub fn read_state(&self) -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(self.state.join("state.json")).unwrap()).unwrap()
    }

    /// Every regular file directly under the state dir → bytes (the state dir
    /// is flat). Used for "this command wrote nothing" assertions.
    pub fn snapshot(&self) -> BTreeMap<String, Vec<u8>> {
        let mut m = BTreeMap::new();
        for e in std::fs::read_dir(&self.state).unwrap().flatten() {
            if e.path().is_file() {
                m.insert(e.file_name().to_string_lossy().into_owned(), std::fs::read(e.path()).unwrap());
            }
        }
        m
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

use serde_json::{json, Value};

/// 2027-01-15 08:00:00 UTC. NOW % 2 == 0 (frame 0), NOW % 6 == 0 (legendary hue RED).
pub const NOW: u64 = 1_800_000_000;
pub const DAY: u64 = 86_400;

/// Common, kitten (L5), calm (fed 120 s ago), no celebration, no scene due.
pub fn base_state() -> Value {
    json!({
        "version": 2, "name": "Thalia", "birth_time": NOW - 30 * DAY, "last_fed_at": NOW - 120,
        "total_tool_calls": 230, "coins": 340, "level": 5, "lifetime_cost_usd": 3.42,
        "breed": "Maine Coon", "rarity": "common", "shiny": false, "stats": [60, 40, 20, 70, 50],
        "personality": "{name} watches the cursor.",
        "levelup_renders_remaining": 0, "celebration_kind": "None",
        "bond": 50, "last_pet_at": 0, "last_played_at": 0, "last_treated_at": 0,
        "streak_count": 0, "last_milestone_celebrated": 200, "last_turn_at": NOW - 600,
        "last_celebration_tick_at": 0,
        "scene_kind": "None", "scene_started_at": 0, "scene_duration_secs": 0, "scene_seed": 0,
        "scene_next_at": NOW + 600, "scene_last_kind": "None"
    })
}

/// Full statusline stdin payload. `session_id` is "" on purpose: it skips the
/// lifetime-cost sidecar and pins the turn-cost anchor to the fixed legacy path.
pub fn base_payload(cwd: &str) -> Value {
    json!({
        "session_id": "",
        "model": { "display_name": "Opus 5.5" },
        "workspace": { "current_dir": cwd },
        "context_window": {
            "used_percentage": 20.0,
            "current_usage": { "input_tokens": 80000, "output_tokens": 5000,
                               "cache_creation_input_tokens": 1000, "cache_read_input_tokens": 1000 }
        },
        "rate_limits": {
            "five_hour": { "used_percentage": 37.0, "resets_at": NOW + 7200 },
            "seven_day": { "used_percentage": 12.0, "resets_at": NOW + 4 * DAY + 15 * 3600 }
        },
        "cost": { "total_cost_usd": 3.42, "total_lines_added": 218, "total_lines_removed": 41,
                  "total_duration_ms": 2_700_000 }
    })
}

/// Recursive object merge: keys in `patch` overwrite / descend into `base`.
pub fn merge(base: &mut Value, patch: Value) {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, v) in p {
                merge(b.entry(k).or_insert(Value::Null), v);
            }
        }
        (b, p) => *b = p,
    }
}

/// Drop every `ESC [ … <letter>` sequence.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\x1b' && it.peek() == Some(&'[') {
            it.next();
            for d in it.by_ref() {
                if d.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Compare `actual` to `rust/tests/golden/<rel>`. `UPDATE_GOLDEN=1` (re)writes
/// the file — allowed in Task 2 ONLY; afterwards a diff is always a bug.
pub fn assert_golden(rel: &str, actual: &[u8]) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("golden").join(rel);
    if std::env::var("UPDATE_GOLDEN").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read(&path).unwrap_or_else(|_| panic!("missing golden {path:?} — record it in Task 2"));
    if expected != actual {
        let e = String::from_utf8_lossy(&expected);
        let a = String::from_utf8_lossy(actual);
        for (i, (el, al)) in e.lines().zip(a.lines()).enumerate() {
            if el != al {
                panic!("golden {rel} differs at line {i}:\n  expected {el:?}\n  actual   {al:?}");
            }
        }
        panic!("golden {rel} differs in line count/trailing bytes: expected {} bytes, got {}", expected.len(), actual.len());
    }
}
