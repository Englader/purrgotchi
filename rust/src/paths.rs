use std::env;
use std::path::PathBuf;

pub fn home_dir() -> PathBuf {
    if let Ok(h) = env::var("HOME") {
        return PathBuf::from(h);
    }
    if let Ok(h) = env::var("USERPROFILE") {
        return PathBuf::from(h);
    }
    PathBuf::from(".")
}

pub fn state_dir() -> PathBuf {
    home_dir().join(".claude").join("purrgotchi")
}

pub fn state_file() -> PathBuf {
    state_dir().join("state.json")
}

pub fn turn_anchor_file() -> PathBuf {
    state_dir().join("turn-start-cost.txt")
}

pub fn cost_sessions_file() -> PathBuf {
    state_dir().join("cost-sessions.json")
}
