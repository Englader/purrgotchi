use std::env;
use std::fs;
use std::io::Write;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::paths;

static ENABLED: OnceLock<bool> = OnceLock::new();

pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| {
        if matches!(env::var("PURR_DEBUG").as_deref(), Ok(v) if !v.is_empty() && v != "0") {
            return true;
        }
        env::args().any(|a| a == "--debug")
    })
}

/// Append one timestamped line to ~/.claude/purrgotchi/debug.log. Silent on
/// any I/O failure — debug logging must never break the hot path.
pub fn log(line: &str) {
    if !enabled() {
        return;
    }
    let _ = log_inner(line);
}

fn log_inner(line: &str) -> std::io::Result<()> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let dir = paths::state_dir();
    fs::create_dir_all(&dir)?;
    // Intentionally unbuffered: react::run's watchdog calls debug::log
    // immediately before std::process::exit(0). Close-on-Drop is a
    // synchronous flush only because there is no BufWriter here.
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("debug.log"))?;
    writeln!(
        f,
        "{}.{:06} pid={} {line}",
        now.as_secs(),
        now.subsec_micros(),
        std::process::id()
    )
}
