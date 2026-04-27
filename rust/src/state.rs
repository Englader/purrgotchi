use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CelebrationKind {
    #[default]
    None,
    LevelUp,
    EvolvingYoung,
    EvolvingAdult,
    // D-group (v3.4.8)
    Streak,
    MilestoneCenturion,
    MilestoneVeteran,
    MilestoneMaster,
    MilestoneLegendary,
    Greeting,
    RandomDrop,
}

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
    // celebration overlay (v3.4.3, additive — serde(default) for back-compat)
    #[serde(default)]
    pub levelup_renders_remaining: u8,
    #[serde(default)]
    pub celebration_kind: CelebrationKind,
    // bond (v3.4.5, additive — serde(default) for back-compat)
    #[serde(default = "default_bond")]
    pub bond: u8,
    #[serde(default)]
    pub last_pet_at: u64,
    // interactions (v3.4.6, additive — serde(default) for back-compat)
    #[serde(default)]
    pub last_played_at: u64,
    #[serde(default)]
    pub last_treated_at: u64,
    // D-group events (v3.4.8, additive — serde(default) for back-compat)
    #[serde(default)]
    pub streak_count: u8,
    #[serde(default)]
    pub last_milestone_celebrated: u64,
    #[serde(default)]
    pub last_turn_at: u64,
}

fn default_bond() -> u8 { crate::bond::BOND_START }

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
            levelup_renders_remaining: 0,
            celebration_kind: CelebrationKind::None,
            bond: crate::bond::BOND_START,
            last_pet_at: 0,
            last_played_at: 0,
            last_treated_at: 0,
            streak_count: 0,
            last_milestone_celebrated: 0,
            last_turn_at: 0,
        }
    }
}

pub fn life_stage(level: u64) -> &'static str {
    match level {
        1..=9 => "kitten",
        10..=49 => "young",
        _ => "adult",
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

    #[test]
    fn life_stage_kitten_boundaries() {
        assert_eq!(life_stage(1), "kitten");
        assert_eq!(life_stage(9), "kitten");
    }

    #[test]
    fn life_stage_young_boundaries() {
        assert_eq!(life_stage(10), "young");
        assert_eq!(life_stage(49), "young");
    }

    #[test]
    fn life_stage_adult_boundaries() {
        assert_eq!(life_stage(50), "adult");
        assert_eq!(life_stage(9_999), "adult");
    }

    #[test]
    fn v2_state_without_celebration_fields_defaults_clean() {
        // A v3.4.2-shaped state.json has no levelup_renders_remaining or
        // celebration_kind fields. serde(default) must make them zero/None.
        let json = r#"{
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
        let s = parse_state(json.as_bytes()).expect("v3.4.2 shape should load");
        assert_eq!(s.levelup_renders_remaining, 0);
        assert_eq!(s.celebration_kind, CelebrationKind::None);
    }

    #[test]
    fn v3_4_4_state_without_bond_fields_defaults_to_fifty_and_zero() {
        // A v3.4.4-shaped state.json has no bond or last_pet_at fields.
        // serde(default) must make bond=50 (BOND_START), last_pet_at=0.
        let json = r#"{
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
        let s = parse_state(json.as_bytes()).expect("v3.4.4 shape should load");
        assert_eq!(s.bond, 50);
        assert_eq!(s.last_pet_at, 0);
    }

    #[test]
    fn state_base_initializes_bond_at_fifty() {
        let s = State::base(12345);
        assert_eq!(s.bond, 50);
        assert_eq!(s.last_pet_at, 0);
    }

    #[test]
    fn v3_4_5_state_without_play_treat_fields_defaults_to_zero() {
        // A v3.4.5-shaped state.json has `bond` and `last_pet_at` but no
        // `last_played_at` or `last_treated_at`. serde(default) must make
        // both default to 0 ("never").
        let json = r#"{
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
            "personality": "A streetwise cat.",
            "bond": 72,
            "last_pet_at": 2000
        }"#;
        let s = parse_state(json.as_bytes()).expect("v3.4.5 shape should load");
        assert_eq!(s.bond, 72);
        assert_eq!(s.last_pet_at, 2000);
        assert_eq!(s.last_played_at, 0);
        assert_eq!(s.last_treated_at, 0);
    }

    #[test]
    fn state_base_initializes_play_treat_timestamps_to_zero() {
        let s = State::base(12345);
        assert_eq!(s.last_played_at, 0);
        assert_eq!(s.last_treated_at, 0);
    }

    #[test]
    fn v3_4_7_state_without_d_group_fields_defaults_to_zero() {
        let json = r#"{
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
            "personality": "A streetwise cat.",
            "bond": 72,
            "last_pet_at": 2000,
            "last_played_at": 0,
            "last_treated_at": 0
        }"#;
        let s = parse_state(json.as_bytes()).expect("v3.4.7 shape should load");
        assert_eq!(s.streak_count, 0);
        assert_eq!(s.last_milestone_celebrated, 0);
        assert_eq!(s.last_turn_at, 0);
    }

    #[test]
    fn state_base_initializes_d_group_fields_to_zero() {
        let s = State::base(12345);
        assert_eq!(s.streak_count, 0);
        assert_eq!(s.last_milestone_celebrated, 0);
        assert_eq!(s.last_turn_at, 0);
    }
}
