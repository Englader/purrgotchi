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

const LOG_ROTATE_BYTES: u64 = 512 * 1024;

fn log_inner(line: &str) -> std::io::Result<()> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let dir = paths::state_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join("debug.log");
    // Rotate at 512 KiB — with statusLine refreshInterval=1 an enabled debug
    // log gains at least a line per second and would otherwise grow unbounded.
    if fs::metadata(&path).map(|m| m.len() > LOG_ROTATE_BYTES).unwrap_or(false) {
        let old = dir.join("debug.log.1");
        let _ = fs::remove_file(&old);
        let _ = fs::rename(&path, &old);
    }
    // Intentionally unbuffered: react::run's watchdog calls debug::log
    // immediately before std::process::exit(0). Close-on-Drop is a
    // synchronous flush only because there is no BufWriter here.
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(
        f,
        "{}.{:06} pid={} {line}",
        now.as_secs(),
        now.subsec_micros(),
        std::process::id()
    )
}
