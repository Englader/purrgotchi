use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct State {
    pub version: u32,
    pub name: String,
    pub birth_time: u64,
    pub last_fed_at: u64,
    pub total_tool_calls: u64,
    pub coins: u64,
    pub level: u64,
    pub lifetime_cost_usd: f64,
}

impl State {
    pub fn fresh(now: u64) -> Self {
        State {
            version: 1,
            name: "Purrgotchi".to_string(),
            birth_time: now,
            last_fed_at: now,
            total_tool_calls: 0,
            coins: 0,
            level: 1,
            lifetime_cost_usd: 0.0,
        }
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn load() -> Option<State> {
    let bytes = fs::read(paths::state_file()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn load_or_fresh() -> State {
    load().unwrap_or_else(|| State::fresh(now_secs()))
}

pub fn save_atomic(state: &State) -> io::Result<()> {
    let dir = paths::state_dir();
    fs::create_dir_all(&dir)?;

    let pid = process::id();
    let tmp = dir.join(format!(".state.{pid}.tmp"));
    let final_path = paths::state_file();

    write_json(&tmp, state)?;
    match fs::rename(&tmp, &final_path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

fn write_json(path: &Path, state: &State) -> io::Result<()> {
    let mut f = fs::File::create(path)?;
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    f.write_all(json.as_bytes())?;
    f.write_all(b"\n")?;
    f.sync_all()?;
    Ok(())
}
