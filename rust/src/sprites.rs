// Embedded sprites — relative to src/sprites.rs, so ../../sprites/ from the
// workspace lands in the bash project's sprites/ dir. One source of truth.

macro_rules! sprite {
    ($path:literal) => { include_str!(concat!("../../sprites/", $path)) };
}

const CALM_0: &str = sprite!("calm_0.txt");
const CALM_1: &str = sprite!("calm_1.txt");
const HAPPY_0: &str = sprite!("happy_0.txt");
const HAPPY_1: &str = sprite!("happy_1.txt");
const ALERT_0: &str = sprite!("alert_0.txt");
const ALERT_1: &str = sprite!("alert_1.txt");
const PANIC_0: &str = sprite!("panic_0.txt");
const PANIC_1: &str = sprite!("panic_1.txt");
const HUNGRY_0: &str = sprite!("hungry_0.txt");
const SLEEP_0: &str = sprite!("sleep_0.txt");
const SLEEP_1: &str = sprite!("sleep_1.txt");

fn lookup_common(mood: &str, frame: u8) -> Option<&'static str> {
    match (mood, frame) {
        ("calm", 0) => Some(CALM_0),
        ("calm", 1) => Some(CALM_1),
        ("happy", 0) => Some(HAPPY_0),
        ("happy", 1) => Some(HAPPY_1),
        ("alert", 0) => Some(ALERT_0),
        ("alert", 1) => Some(ALERT_1),
        ("panic", 0) => Some(PANIC_0),
        ("panic", 1) => Some(PANIC_1),
        ("hungry", 0) => Some(HUNGRY_0),
        ("sleep", 0) => Some(SLEEP_0),
        ("sleep", 1) => Some(SLEEP_1),
        _ => None,
    }
}

// Stubs — Task 7 will replace these with real sprite constants per rarity tier.
#[allow(dead_code)]
fn lookup_uncommon(_mood: &str, _frame: u8) -> Option<&'static str> { None }
#[allow(dead_code)]
fn lookup_rare(_mood: &str, _frame: u8) -> Option<&'static str> { None }
#[allow(dead_code)]
fn lookup_epic(_mood: &str, _frame: u8) -> Option<&'static str> { None }
#[allow(dead_code)]
fn lookup_legendary(_mood: &str, _frame: u8) -> Option<&'static str> { None }

fn lookup_rarity(rarity: &str, mood: &str, frame: u8) -> Option<&'static str> {
    match rarity {
        "uncommon" => lookup_uncommon(mood, frame),
        "rare" => lookup_rare(mood, frame),
        "epic" => lookup_epic(mood, frame),
        "legendary" => lookup_legendary(mood, frame),
        _ => lookup_common(mood, frame),
    }
}

pub fn get_sprite_lines(rarity: &str, mood: &str, frame: u8) -> Vec<String> {
    let raw = lookup_rarity(rarity, mood, frame)
        .or_else(|| lookup_rarity(rarity, mood, 0))      // same rarity, frame 0
        .or_else(|| lookup_common(mood, frame))           // fall back to common tier
        .or_else(|| lookup_common("calm", 0))             // ultimate fallback
        .unwrap_or("");

    let mut lines: Vec<String> = raw
        .split('\n')
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();

    if lines.last().map(|s| s.is_empty()).unwrap_or(false) {
        lines.pop();
    }

    while lines.len() < 6 {
        lines.push(String::new());
    }
    lines
}
