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
    // identity (v2)
    pub breed: String,
    pub rarity: String,
    pub shiny: bool,
    pub stats: [u8; 5],       // [CURIOSITY, MISCHIEF, LAZINESS, AFFECTION, WISDOM]
    pub personality: String,  // blurb with {name} already substituted
}

impl State {
    pub fn base(now: u64) -> Self {
        State {
            version: 2,
            name: "Purrgotchi".to_string(),
            birth_time: now,
            last_fed_at: now,
            total_tool_calls: 0,
            coins: 0,
            level: 1,
            lifetime_cost_usd: 0.0,
            breed: String::new(),
            rarity: String::new(),
            shiny: false,
            stats: [0u8; 5],
            personality: String::new(),
        }
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn parse_state(bytes: &[u8]) -> Option<State> {
    let s: State = serde_json::from_slice(bytes).ok()?;
    if s.version != 2 { return None; }
    Some(s)
}

pub fn load() -> Option<State> {
    let bytes = fs::read(paths::state_file()).ok()?;
    parse_state(&bytes)
}

pub fn load_or_base() -> State {
    load().unwrap_or_else(|| State::base(now_secs()))
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
    // Atomicity comes from tempfile + rename below; fsync would add Windows
    // latency on every feed hook without changing crash behaviour.
    let mut f = fs::File::create(path)?;
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    f.write_all(json.as_bytes())?;
    f.write_all(b"\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_state_returns_none() {
        let v1_json = r#"{
            "version": 1,
            "name": "Thalia",
            "birth_time": 1000,
            "last_fed_at": 1000,
            "total_tool_calls": 5,
            "coins": 5,
            "level": 1,
            "lifetime_cost_usd": 0.1
        }"#;
        assert!(parse_state(v1_json.as_bytes()).is_none());
    }

    #[test]
    fn v2_state_loads_correctly() {
        let v2_json = r#"{
            "version": 2,
            "name": "Thalia",
            "birth_time": 1000,
            "last_fed_at": 1000,
            "total_tool_calls": 5,
            "coins": 5,
            "level": 1,
            "lifetime_cost_usd": 0.1,
            "breed": "Tabby",
            "rarity": "common",
            "shiny": false,
            "stats": [42, 80, 10, 55, 30],
            "personality": "A streetwise cat."
        }"#;
        let s = parse_state(v2_json.as_bytes()).expect("v2 JSON should load");
        assert_eq!(s.breed, "Tabby");
        assert_eq!(s.stats[1], 80);
    }
}
