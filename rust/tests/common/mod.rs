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
