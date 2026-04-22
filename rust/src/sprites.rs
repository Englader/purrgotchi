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

// Uncommon tier
const UNCOMMON_CALM_0:   &str = sprite!("uncommon_calm_0.txt");
const UNCOMMON_CALM_1:   &str = sprite!("uncommon_calm_1.txt");
const UNCOMMON_HAPPY_0:  &str = sprite!("uncommon_happy_0.txt");
const UNCOMMON_HAPPY_1:  &str = sprite!("uncommon_happy_1.txt");
const UNCOMMON_ALERT_0:  &str = sprite!("uncommon_alert_0.txt");
const UNCOMMON_ALERT_1:  &str = sprite!("uncommon_alert_1.txt");
const UNCOMMON_PANIC_0:  &str = sprite!("uncommon_panic_0.txt");
const UNCOMMON_PANIC_1:  &str = sprite!("uncommon_panic_1.txt");
const UNCOMMON_HUNGRY_0: &str = sprite!("uncommon_hungry_0.txt");
const UNCOMMON_SLEEP_0:  &str = sprite!("uncommon_sleep_0.txt");
const UNCOMMON_SLEEP_1:  &str = sprite!("uncommon_sleep_1.txt");

// Rare tier
const RARE_CALM_0:   &str = sprite!("rare_calm_0.txt");
const RARE_CALM_1:   &str = sprite!("rare_calm_1.txt");
const RARE_HAPPY_0:  &str = sprite!("rare_happy_0.txt");
const RARE_HAPPY_1:  &str = sprite!("rare_happy_1.txt");
const RARE_ALERT_0:  &str = sprite!("rare_alert_0.txt");
const RARE_ALERT_1:  &str = sprite!("rare_alert_1.txt");
const RARE_PANIC_0:  &str = sprite!("rare_panic_0.txt");
const RARE_PANIC_1:  &str = sprite!("rare_panic_1.txt");
const RARE_HUNGRY_0: &str = sprite!("rare_hungry_0.txt");
const RARE_SLEEP_0:  &str = sprite!("rare_sleep_0.txt");
const RARE_SLEEP_1:  &str = sprite!("rare_sleep_1.txt");

// Epic tier
const EPIC_CALM_0:   &str = sprite!("epic_calm_0.txt");
const EPIC_CALM_1:   &str = sprite!("epic_calm_1.txt");
const EPIC_HAPPY_0:  &str = sprite!("epic_happy_0.txt");
const EPIC_HAPPY_1:  &str = sprite!("epic_happy_1.txt");
const EPIC_ALERT_0:  &str = sprite!("epic_alert_0.txt");
const EPIC_ALERT_1:  &str = sprite!("epic_alert_1.txt");
const EPIC_PANIC_0:  &str = sprite!("epic_panic_0.txt");
const EPIC_PANIC_1:  &str = sprite!("epic_panic_1.txt");
const EPIC_HUNGRY_0: &str = sprite!("epic_hungry_0.txt");
const EPIC_SLEEP_0:  &str = sprite!("epic_sleep_0.txt");
const EPIC_SLEEP_1:  &str = sprite!("epic_sleep_1.txt");

// Legendary tier
const LEGENDARY_CALM_0:   &str = sprite!("legendary_calm_0.txt");
const LEGENDARY_CALM_1:   &str = sprite!("legendary_calm_1.txt");
const LEGENDARY_HAPPY_0:  &str = sprite!("legendary_happy_0.txt");
const LEGENDARY_HAPPY_1:  &str = sprite!("legendary_happy_1.txt");
const LEGENDARY_ALERT_0:  &str = sprite!("legendary_alert_0.txt");
const LEGENDARY_ALERT_1:  &str = sprite!("legendary_alert_1.txt");
const LEGENDARY_PANIC_0:  &str = sprite!("legendary_panic_0.txt");
const LEGENDARY_PANIC_1:  &str = sprite!("legendary_panic_1.txt");
const LEGENDARY_HUNGRY_0: &str = sprite!("legendary_hungry_0.txt");
const LEGENDARY_SLEEP_0:  &str = sprite!("legendary_sleep_0.txt");
const LEGENDARY_SLEEP_1:  &str = sprite!("legendary_sleep_1.txt");

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

fn lookup_uncommon(mood: &str, frame: u8) -> Option<&'static str> {
    match (mood, frame) {
        ("calm",   0) => Some(UNCOMMON_CALM_0),
        ("calm",   1) => Some(UNCOMMON_CALM_1),
        ("happy",  0) => Some(UNCOMMON_HAPPY_0),
        ("happy",  1) => Some(UNCOMMON_HAPPY_1),
        ("alert",  0) => Some(UNCOMMON_ALERT_0),
        ("alert",  1) => Some(UNCOMMON_ALERT_1),
        ("panic",  0) => Some(UNCOMMON_PANIC_0),
        ("panic",  1) => Some(UNCOMMON_PANIC_1),
        ("hungry", 0) => Some(UNCOMMON_HUNGRY_0),
        ("sleep",  0) => Some(UNCOMMON_SLEEP_0),
        ("sleep",  1) => Some(UNCOMMON_SLEEP_1),
        _ => None,
    }
}

fn lookup_rare(mood: &str, frame: u8) -> Option<&'static str> {
    match (mood, frame) {
        ("calm",   0) => Some(RARE_CALM_0),
        ("calm",   1) => Some(RARE_CALM_1),
        ("happy",  0) => Some(RARE_HAPPY_0),
        ("happy",  1) => Some(RARE_HAPPY_1),
        ("alert",  0) => Some(RARE_ALERT_0),
        ("alert",  1) => Some(RARE_ALERT_1),
        ("panic",  0) => Some(RARE_PANIC_0),
        ("panic",  1) => Some(RARE_PANIC_1),
        ("hungry", 0) => Some(RARE_HUNGRY_0),
        ("sleep",  0) => Some(RARE_SLEEP_0),
        ("sleep",  1) => Some(RARE_SLEEP_1),
        _ => None,
    }
}

fn lookup_epic(mood: &str, frame: u8) -> Option<&'static str> {
    match (mood, frame) {
        ("calm",   0) => Some(EPIC_CALM_0),
        ("calm",   1) => Some(EPIC_CALM_1),
        ("happy",  0) => Some(EPIC_HAPPY_0),
        ("happy",  1) => Some(EPIC_HAPPY_1),
        ("alert",  0) => Some(EPIC_ALERT_0),
        ("alert",  1) => Some(EPIC_ALERT_1),
        ("panic",  0) => Some(EPIC_PANIC_0),
        ("panic",  1) => Some(EPIC_PANIC_1),
        ("hungry", 0) => Some(EPIC_HUNGRY_0),
        ("sleep",  0) => Some(EPIC_SLEEP_0),
        ("sleep",  1) => Some(EPIC_SLEEP_1),
        _ => None,
    }
}

fn lookup_legendary(mood: &str, frame: u8) -> Option<&'static str> {
    match (mood, frame) {
        ("calm",   0) => Some(LEGENDARY_CALM_0),
        ("calm",   1) => Some(LEGENDARY_CALM_1),
        ("happy",  0) => Some(LEGENDARY_HAPPY_0),
        ("happy",  1) => Some(LEGENDARY_HAPPY_1),
        ("alert",  0) => Some(LEGENDARY_ALERT_0),
        ("alert",  1) => Some(LEGENDARY_ALERT_1),
        ("panic",  0) => Some(LEGENDARY_PANIC_0),
        ("panic",  1) => Some(LEGENDARY_PANIC_1),
        ("hungry", 0) => Some(LEGENDARY_HUNGRY_0),
        ("sleep",  0) => Some(LEGENDARY_SLEEP_0),
        ("sleep",  1) => Some(LEGENDARY_SLEEP_1),
        _ => None,
    }
}

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
