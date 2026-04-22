use std::fs;

use crate::paths;
use crate::state;

pub fn feed() {
    let _ = feed_inner();
}

fn feed_inner() -> Option<()> {
    let mut s = state::load().unwrap_or_else(|| {
        let fresh = crate::hatch::generate(&crate::hatch::HatchOpts::default());
        let _ = state::save_atomic(&fresh);
        fresh
    });
    s.total_tool_calls = s.total_tool_calls.saturating_add(1);
    s.coins = s.coins.saturating_add(1);
    s.last_fed_at = state::now_secs();
    s.level = 1 + s.total_tool_calls / 50;
    state::save_atomic(&s).ok()?;
    Some(())
}

pub fn turn() {
    let _ = fs::remove_file(paths::turn_anchor_file());
}

pub fn rename(new_name: &str) -> i32 {
    if new_name.is_empty() {
        eprintln!("rename: name cannot be empty");
        return 1;
    }
    let mut s = state::load().unwrap_or_else(|| {
        let fresh = crate::hatch::generate(&crate::hatch::HatchOpts::default());
        let _ = state::save_atomic(&fresh);
        fresh
    });
    s.name = new_name.to_string();
    match state::save_atomic(&s) {
        Ok(()) => {
            println!("kitten renamed to: {new_name}");
            0
        }
        Err(e) => {
            eprintln!("rename: failed to write state.json: {e}");
            1
        }
    }
}
