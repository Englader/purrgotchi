// Embedded sprites — all files under ../../sprites/ relative to this source.
// One flat SPRITES table indexed by (stage, rarity, mood, frame), and a separate
// CELEBRATIONS table for tier-agnostic level-up/evolution overlays.

use crate::state::CelebrationKind;

macro_rules! sprite {
    ($path:literal) => { include_str!(concat!("../../sprites/", $path)) };
}

type SpriteRow = (&'static str, &'static str, &'static str, u8, &'static str);

// 165 entries: 3 stages × 5 rarities × 11 mood/frame combos.
// Order: stage → rarity → mood → frame. Kept grouped by (stage, rarity) for readability.
static SPRITES: &[SpriteRow] = &[
    // ---- KITTEN ----
    ("kitten", "common", "calm",   0, sprite!("kitten_common_calm_0.txt")),
    ("kitten", "common", "calm",   1, sprite!("kitten_common_calm_1.txt")),
    ("kitten", "common", "happy",  0, sprite!("kitten_common_happy_0.txt")),
    ("kitten", "common", "happy",  1, sprite!("kitten_common_happy_1.txt")),
    ("kitten", "common", "alert",  0, sprite!("kitten_common_alert_0.txt")),
    ("kitten", "common", "alert",  1, sprite!("kitten_common_alert_1.txt")),
    ("kitten", "common", "panic",  0, sprite!("kitten_common_panic_0.txt")),
    ("kitten", "common", "panic",  1, sprite!("kitten_common_panic_1.txt")),
    ("kitten", "common", "hungry", 0, sprite!("kitten_common_hungry_0.txt")),
    ("kitten", "common", "sleep",  0, sprite!("kitten_common_sleep_0.txt")),
    ("kitten", "common", "sleep",  1, sprite!("kitten_common_sleep_1.txt")),
    ("kitten", "uncommon", "calm",   0, sprite!("kitten_uncommon_calm_0.txt")),
    ("kitten", "uncommon", "calm",   1, sprite!("kitten_uncommon_calm_1.txt")),
    ("kitten", "uncommon", "happy",  0, sprite!("kitten_uncommon_happy_0.txt")),
    ("kitten", "uncommon", "happy",  1, sprite!("kitten_uncommon_happy_1.txt")),
    ("kitten", "uncommon", "alert",  0, sprite!("kitten_uncommon_alert_0.txt")),
    ("kitten", "uncommon", "alert",  1, sprite!("kitten_uncommon_alert_1.txt")),
    ("kitten", "uncommon", "panic",  0, sprite!("kitten_uncommon_panic_0.txt")),
    ("kitten", "uncommon", "panic",  1, sprite!("kitten_uncommon_panic_1.txt")),
    ("kitten", "uncommon", "hungry", 0, sprite!("kitten_uncommon_hungry_0.txt")),
    ("kitten", "uncommon", "sleep",  0, sprite!("kitten_uncommon_sleep_0.txt")),
    ("kitten", "uncommon", "sleep",  1, sprite!("kitten_uncommon_sleep_1.txt")),
    ("kitten", "rare", "calm",   0, sprite!("kitten_rare_calm_0.txt")),
    ("kitten", "rare", "calm",   1, sprite!("kitten_rare_calm_1.txt")),
    ("kitten", "rare", "happy",  0, sprite!("kitten_rare_happy_0.txt")),
    ("kitten", "rare", "happy",  1, sprite!("kitten_rare_happy_1.txt")),
    ("kitten", "rare", "alert",  0, sprite!("kitten_rare_alert_0.txt")),
    ("kitten", "rare", "alert",  1, sprite!("kitten_rare_alert_1.txt")),
    ("kitten", "rare", "panic",  0, sprite!("kitten_rare_panic_0.txt")),
    ("kitten", "rare", "panic",  1, sprite!("kitten_rare_panic_1.txt")),
    ("kitten", "rare", "hungry", 0, sprite!("kitten_rare_hungry_0.txt")),
    ("kitten", "rare", "sleep",  0, sprite!("kitten_rare_sleep_0.txt")),
    ("kitten", "rare", "sleep",  1, sprite!("kitten_rare_sleep_1.txt")),
    ("kitten", "epic", "calm",   0, sprite!("kitten_epic_calm_0.txt")),
    ("kitten", "epic", "calm",   1, sprite!("kitten_epic_calm_1.txt")),
    ("kitten", "epic", "happy",  0, sprite!("kitten_epic_happy_0.txt")),
    ("kitten", "epic", "happy",  1, sprite!("kitten_epic_happy_1.txt")),
    ("kitten", "epic", "alert",  0, sprite!("kitten_epic_alert_0.txt")),
    ("kitten", "epic", "alert",  1, sprite!("kitten_epic_alert_1.txt")),
    ("kitten", "epic", "panic",  0, sprite!("kitten_epic_panic_0.txt")),
    ("kitten", "epic", "panic",  1, sprite!("kitten_epic_panic_1.txt")),
    ("kitten", "epic", "hungry", 0, sprite!("kitten_epic_hungry_0.txt")),
    ("kitten", "epic", "sleep",  0, sprite!("kitten_epic_sleep_0.txt")),
    ("kitten", "epic", "sleep",  1, sprite!("kitten_epic_sleep_1.txt")),
    ("kitten", "legendary", "calm",   0, sprite!("kitten_legendary_calm_0.txt")),
    ("kitten", "legendary", "calm",   1, sprite!("kitten_legendary_calm_1.txt")),
    ("kitten", "legendary", "happy",  0, sprite!("kitten_legendary_happy_0.txt")),
    ("kitten", "legendary", "happy",  1, sprite!("kitten_legendary_happy_1.txt")),
    ("kitten", "legendary", "alert",  0, sprite!("kitten_legendary_alert_0.txt")),
    ("kitten", "legendary", "alert",  1, sprite!("kitten_legendary_alert_1.txt")),
    ("kitten", "legendary", "panic",  0, sprite!("kitten_legendary_panic_0.txt")),
    ("kitten", "legendary", "panic",  1, sprite!("kitten_legendary_panic_1.txt")),
    ("kitten", "legendary", "hungry", 0, sprite!("kitten_legendary_hungry_0.txt")),
    ("kitten", "legendary", "sleep",  0, sprite!("kitten_legendary_sleep_0.txt")),
    ("kitten", "legendary", "sleep",  1, sprite!("kitten_legendary_sleep_1.txt")),

    // ---- YOUNG (renamed from v3.4.2 sprites) ----
    ("young", "common", "calm",   0, sprite!("young_common_calm_0.txt")),
    ("young", "common", "calm",   1, sprite!("young_common_calm_1.txt")),
    ("young", "common", "happy",  0, sprite!("young_common_happy_0.txt")),
    ("young", "common", "happy",  1, sprite!("young_common_happy_1.txt")),
    ("young", "common", "alert",  0, sprite!("young_common_alert_0.txt")),
    ("young", "common", "alert",  1, sprite!("young_common_alert_1.txt")),
    ("young", "common", "panic",  0, sprite!("young_common_panic_0.txt")),
    ("young", "common", "panic",  1, sprite!("young_common_panic_1.txt")),
    ("young", "common", "hungry", 0, sprite!("young_common_hungry_0.txt")),
    ("young", "common", "sleep",  0, sprite!("young_common_sleep_0.txt")),
    ("young", "common", "sleep",  1, sprite!("young_common_sleep_1.txt")),
    ("young", "uncommon", "calm",   0, sprite!("young_uncommon_calm_0.txt")),
    ("young", "uncommon", "calm",   1, sprite!("young_uncommon_calm_1.txt")),
    ("young", "uncommon", "happy",  0, sprite!("young_uncommon_happy_0.txt")),
    ("young", "uncommon", "happy",  1, sprite!("young_uncommon_happy_1.txt")),
    ("young", "uncommon", "alert",  0, sprite!("young_uncommon_alert_0.txt")),
    ("young", "uncommon", "alert",  1, sprite!("young_uncommon_alert_1.txt")),
    ("young", "uncommon", "panic",  0, sprite!("young_uncommon_panic_0.txt")),
    ("young", "uncommon", "panic",  1, sprite!("young_uncommon_panic_1.txt")),
    ("young", "uncommon", "hungry", 0, sprite!("young_uncommon_hungry_0.txt")),
    ("young", "uncommon", "sleep",  0, sprite!("young_uncommon_sleep_0.txt")),
    ("young", "uncommon", "sleep",  1, sprite!("young_uncommon_sleep_1.txt")),
    ("young", "rare", "calm",   0, sprite!("young_rare_calm_0.txt")),
    ("young", "rare", "calm",   1, sprite!("young_rare_calm_1.txt")),
    ("young", "rare", "happy",  0, sprite!("young_rare_happy_0.txt")),
    ("young", "rare", "happy",  1, sprite!("young_rare_happy_1.txt")),
    ("young", "rare", "alert",  0, sprite!("young_rare_alert_0.txt")),
    ("young", "rare", "alert",  1, sprite!("young_rare_alert_1.txt")),
    ("young", "rare", "panic",  0, sprite!("young_rare_panic_0.txt")),
    ("young", "rare", "panic",  1, sprite!("young_rare_panic_1.txt")),
    ("young", "rare", "hungry", 0, sprite!("young_rare_hungry_0.txt")),
    ("young", "rare", "sleep",  0, sprite!("young_rare_sleep_0.txt")),
    ("young", "rare", "sleep",  1, sprite!("young_rare_sleep_1.txt")),
    ("young", "epic", "calm",   0, sprite!("young_epic_calm_0.txt")),
    ("young", "epic", "calm",   1, sprite!("young_epic_calm_1.txt")),
    ("young", "epic", "happy",  0, sprite!("young_epic_happy_0.txt")),
    ("young", "epic", "happy",  1, sprite!("young_epic_happy_1.txt")),
    ("young", "epic", "alert",  0, sprite!("young_epic_alert_0.txt")),
    ("young", "epic", "alert",  1, sprite!("young_epic_alert_1.txt")),
    ("young", "epic", "panic",  0, sprite!("young_epic_panic_0.txt")),
    ("young", "epic", "panic",  1, sprite!("young_epic_panic_1.txt")),
    ("young", "epic", "hungry", 0, sprite!("young_epic_hungry_0.txt")),
    ("young", "epic", "sleep",  0, sprite!("young_epic_sleep_0.txt")),
    ("young", "epic", "sleep",  1, sprite!("young_epic_sleep_1.txt")),
    ("young", "legendary", "calm",   0, sprite!("young_legendary_calm_0.txt")),
    ("young", "legendary", "calm",   1, sprite!("young_legendary_calm_1.txt")),
    ("young", "legendary", "happy",  0, sprite!("young_legendary_happy_0.txt")),
    ("young", "legendary", "happy",  1, sprite!("young_legendary_happy_1.txt")),
    ("young", "legendary", "alert",  0, sprite!("young_legendary_alert_0.txt")),
    ("young", "legendary", "alert",  1, sprite!("young_legendary_alert_1.txt")),
    ("young", "legendary", "panic",  0, sprite!("young_legendary_panic_0.txt")),
    ("young", "legendary", "panic",  1, sprite!("young_legendary_panic_1.txt")),
    ("young", "legendary", "hungry", 0, sprite!("young_legendary_hungry_0.txt")),
    ("young", "legendary", "sleep",  0, sprite!("young_legendary_sleep_0.txt")),
    ("young", "legendary", "sleep",  1, sprite!("young_legendary_sleep_1.txt")),

    // ---- ADULT ----
    ("adult", "common", "calm",   0, sprite!("adult_common_calm_0.txt")),
    ("adult", "common", "calm",   1, sprite!("adult_common_calm_1.txt")),
    ("adult", "common", "happy",  0, sprite!("adult_common_happy_0.txt")),
    ("adult", "common", "happy",  1, sprite!("adult_common_happy_1.txt")),
    ("adult", "common", "alert",  0, sprite!("adult_common_alert_0.txt")),
    ("adult", "common", "alert",  1, sprite!("adult_common_alert_1.txt")),
    ("adult", "common", "panic",  0, sprite!("adult_common_panic_0.txt")),
    ("adult", "common", "panic",  1, sprite!("adult_common_panic_1.txt")),
    ("adult", "common", "hungry", 0, sprite!("adult_common_hungry_0.txt")),
    ("adult", "common", "sleep",  0, sprite!("adult_common_sleep_0.txt")),
    ("adult", "common", "sleep",  1, sprite!("adult_common_sleep_1.txt")),
    ("adult", "uncommon", "calm",   0, sprite!("adult_uncommon_calm_0.txt")),
    ("adult", "uncommon", "calm",   1, sprite!("adult_uncommon_calm_1.txt")),
    ("adult", "uncommon", "happy",  0, sprite!("adult_uncommon_happy_0.txt")),
    ("adult", "uncommon", "happy",  1, sprite!("adult_uncommon_happy_1.txt")),
    ("adult", "uncommon", "alert",  0, sprite!("adult_uncommon_alert_0.txt")),
    ("adult", "uncommon", "alert",  1, sprite!("adult_uncommon_alert_1.txt")),
    ("adult", "uncommon", "panic",  0, sprite!("adult_uncommon_panic_0.txt")),
    ("adult", "uncommon", "panic",  1, sprite!("adult_uncommon_panic_1.txt")),
    ("adult", "uncommon", "hungry", 0, sprite!("adult_uncommon_hungry_0.txt")),
    ("adult", "uncommon", "sleep",  0, sprite!("adult_uncommon_sleep_0.txt")),
    ("adult", "uncommon", "sleep",  1, sprite!("adult_uncommon_sleep_1.txt")),
    ("adult", "rare", "calm",   0, sprite!("adult_rare_calm_0.txt")),
    ("adult", "rare", "calm",   1, sprite!("adult_rare_calm_1.txt")),
    ("adult", "rare", "happy",  0, sprite!("adult_rare_happy_0.txt")),
    ("adult", "rare", "happy",  1, sprite!("adult_rare_happy_1.txt")),
    ("adult", "rare", "alert",  0, sprite!("adult_rare_alert_0.txt")),
    ("adult", "rare", "alert",  1, sprite!("adult_rare_alert_1.txt")),
    ("adult", "rare", "panic",  0, sprite!("adult_rare_panic_0.txt")),
    ("adult", "rare", "panic",  1, sprite!("adult_rare_panic_1.txt")),
    ("adult", "rare", "hungry", 0, sprite!("adult_rare_hungry_0.txt")),
    ("adult", "rare", "sleep",  0, sprite!("adult_rare_sleep_0.txt")),
    ("adult", "rare", "sleep",  1, sprite!("adult_rare_sleep_1.txt")),
    ("adult", "epic", "calm",   0, sprite!("adult_epic_calm_0.txt")),
    ("adult", "epic", "calm",   1, sprite!("adult_epic_calm_1.txt")),
    ("adult", "epic", "happy",  0, sprite!("adult_epic_happy_0.txt")),
    ("adult", "epic", "happy",  1, sprite!("adult_epic_happy_1.txt")),
    ("adult", "epic", "alert",  0, sprite!("adult_epic_alert_0.txt")),
    ("adult", "epic", "alert",  1, sprite!("adult_epic_alert_1.txt")),
    ("adult", "epic", "panic",  0, sprite!("adult_epic_panic_0.txt")),
    ("adult", "epic", "panic",  1, sprite!("adult_epic_panic_1.txt")),
    ("adult", "epic", "hungry", 0, sprite!("adult_epic_hungry_0.txt")),
    ("adult", "epic", "sleep",  0, sprite!("adult_epic_sleep_0.txt")),
    ("adult", "epic", "sleep",  1, sprite!("adult_epic_sleep_1.txt")),
    ("adult", "legendary", "calm",   0, sprite!("adult_legendary_calm_0.txt")),
    ("adult", "legendary", "calm",   1, sprite!("adult_legendary_calm_1.txt")),
    ("adult", "legendary", "happy",  0, sprite!("adult_legendary_happy_0.txt")),
    ("adult", "legendary", "happy",  1, sprite!("adult_legendary_happy_1.txt")),
    ("adult", "legendary", "alert",  0, sprite!("adult_legendary_alert_0.txt")),
    ("adult", "legendary", "alert",  1, sprite!("adult_legendary_alert_1.txt")),
    ("adult", "legendary", "panic",  0, sprite!("adult_legendary_panic_0.txt")),
    ("adult", "legendary", "panic",  1, sprite!("adult_legendary_panic_1.txt")),
    ("adult", "legendary", "hungry", 0, sprite!("adult_legendary_hungry_0.txt")),
    ("adult", "legendary", "sleep",  0, sprite!("adult_legendary_sleep_0.txt")),
    ("adult", "legendary", "sleep",  1, sprite!("adult_legendary_sleep_1.txt")),
];

// Celebration sprites — tier-agnostic. Both EvolvingYoung and EvolvingAdult
// use the same `evolving_*` art; the distinction lives in the speech pool.
static CELEBRATIONS: &[(CelebrationKind, u8, &str)] = &[
    (CelebrationKind::LevelUp,        0, sprite!("levelup_0.txt")),
    (CelebrationKind::LevelUp,        1, sprite!("levelup_1.txt")),
    (CelebrationKind::EvolvingYoung,  0, sprite!("evolving_0.txt")),
    (CelebrationKind::EvolvingYoung,  1, sprite!("evolving_1.txt")),
    (CelebrationKind::EvolvingAdult,  0, sprite!("evolving_0.txt")),
    (CelebrationKind::EvolvingAdult,  1, sprite!("evolving_1.txt")),
];

fn lookup_sprite(stage: &str, rarity: &str, mood: &str, frame: u8) -> Option<&'static str> {
    SPRITES.iter()
        .find(|(s, r, m, f, _)| *s == stage && *r == rarity && *m == mood && *f == frame)
        .map(|(_, _, _, _, art)| *art)
}

fn lookup_celebration(kind: CelebrationKind, frame: u8) -> Option<&'static str> {
    CELEBRATIONS.iter()
        .find(|(k, f, _)| *k == kind && *f == frame)
        .map(|(_, _, art)| *art)
}

pub fn get_sprite_lines(stage: &str, rarity: &str, mood: &str, frame: u8) -> Vec<String> {
    let raw = lookup_sprite(stage, rarity, mood, frame)
        .or_else(|| lookup_sprite(stage, rarity, mood, 0))          // same cell, frame 0
        .or_else(|| lookup_sprite(stage, rarity, "calm", 0))         // same cell, calm
        .or_else(|| lookup_sprite(stage, "common", mood, 0))         // same stage, common rarity
        .or_else(|| lookup_sprite("young", "common", "calm", 0))     // ultimate fallback
        .unwrap_or("");
    normalize_lines(raw)
}

pub fn get_celebration_sprite(kind: CelebrationKind, frame: u8) -> Vec<String> {
    let raw = lookup_celebration(kind, frame)
        .or_else(|| lookup_celebration(kind, 0))
        .or_else(|| lookup_sprite("young", "common", "calm", 0))
        .unwrap_or("");
    normalize_lines(raw)
}

fn normalize_lines(raw: &str) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_expected_count() {
        assert_eq!(SPRITES.len(), 165);
        assert_eq!(CELEBRATIONS.len(), 6);
    }

    #[test]
    fn lookup_exact_match_works() {
        assert!(lookup_sprite("young", "legendary", "calm", 0).is_some());
        assert!(lookup_sprite("kitten", "common", "happy", 1).is_some());
        assert!(lookup_sprite("adult", "epic", "panic", 1).is_some());
    }

    #[test]
    fn get_sprite_lines_always_returns_six_lines() {
        let lines = get_sprite_lines("young", "legendary", "calm", 0);
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn get_sprite_lines_falls_back_to_frame_zero() {
        // hungry only has frame 0 in every cell; frame 1 request must fall back.
        let a = get_sprite_lines("young", "common", "hungry", 0);
        let b = get_sprite_lines("young", "common", "hungry", 1);
        assert_eq!(a, b);
    }

    #[test]
    fn get_sprite_lines_unknown_inputs_hit_ultimate_fallback() {
        let lines = get_sprite_lines("nonsense_stage", "nonsense_rarity", "nonsense_mood", 99);
        assert_eq!(lines.len(), 6);
        // ultimate fallback is young/common/calm/0 — at least returns something non-panicking.
    }

    #[test]
    fn celebration_lookup_works_for_all_kinds() {
        assert!(lookup_celebration(CelebrationKind::LevelUp, 0).is_some());
        assert!(lookup_celebration(CelebrationKind::LevelUp, 1).is_some());
        assert!(lookup_celebration(CelebrationKind::EvolvingYoung, 0).is_some());
        assert!(lookup_celebration(CelebrationKind::EvolvingAdult, 1).is_some());
        assert!(lookup_celebration(CelebrationKind::None, 0).is_none());
    }

    #[test]
    fn celebration_none_falls_back_to_safe_sprite() {
        // CelebrationKind::None returns a non-panicking 6-line result.
        let lines = get_celebration_sprite(CelebrationKind::None, 0);
        assert_eq!(lines.len(), 6);
    }
}
