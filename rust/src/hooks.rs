use std::fs;

use crate::bond::{self, PetResult};
use crate::colors::{C_BOLD, C_PINK, C_RESET, C_YELLOW};
use crate::paths;
use crate::state::{self, CelebrationKind, life_stage};

/// Format a waiting duration as "Xh YYm" (when hours > 0) or "YYm".
/// Used by all three command cooldown messages.
fn format_cooldown(wait_secs: u64) -> String {
    let h = wait_secs / 3600;
    let m = (wait_secs % 3600) / 60;
    if h > 0 { format!("{h}h {m:02}m") } else { format!("{m}m") }
}

/// Pure helper: given previous and new level, return the celebration to fire.
/// Returns `None` if the level didn't go up.
pub fn detect_celebration(prev_level: u64, new_level: u64) -> Option<(CelebrationKind, u8)> {
    if new_level <= prev_level {
        return None;
    }
    let prev_stage = life_stage(prev_level);
    let new_stage = life_stage(new_level);
    let kind = match (prev_stage, new_stage) {
        ("kitten", "young") => CelebrationKind::EvolvingYoung,
        ("young", "adult")  => CelebrationKind::EvolvingAdult,
        // Defensive: if someone jumps kitten -> adult in one go, treat as EvolvingAdult.
        ("kitten", "adult") => CelebrationKind::EvolvingAdult,
        _                   => CelebrationKind::LevelUp,
    };
    let renders = match kind {
        CelebrationKind::LevelUp => 3,
        _                        => 6,
    };
    Some((kind, renders))
}

pub fn feed() {
    let _ = feed_inner();
}

fn feed_inner() -> Option<()> {
    let mut s = crate::hatch::load_or_hatch();
    s.total_tool_calls = s.total_tool_calls.saturating_add(1);
    s.coins = s.coins.saturating_add(1);
    s.last_fed_at = state::now_secs();

    let prev_level = s.level;
    s.level = 1 + s.total_tool_calls / 50;
    if let Some((kind, renders)) = detect_celebration(prev_level, s.level) {
        // Overwrite any in-progress celebration — the newer event wins.
        s.celebration_kind = kind;
        s.levelup_renders_remaining = renders;
    }

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
    let mut s = crate::hatch::load_or_hatch();
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

pub fn pet() {
    let mut s = crate::hatch::load_or_hatch();
    let now = state::now_secs();
    let result = bond::try_pet(&mut s, now);
    println!("{}", format_pet_message(&s.name, &result));
    if matches!(result, PetResult::Applied { .. }) {
        let _ = state::save_atomic(&s);
    }
}

fn format_pet_message(name: &str, result: &PetResult) -> String {
    match result {
        PetResult::Applied { old, new, capped: true } => {
            format!(
                "{C_PINK}\u{2665}{C_RESET} {name} purrs contentedly. \
                 Bond: {C_BOLD}{new}{C_RESET} (max, was {old})"
            )
        }
        PetResult::Applied { old, new, capped: false } => {
            let delta = *new as i32 - *old as i32;
            format!(
                "{C_PINK}\u{2665}{C_RESET} {name} purrs contentedly. \
                 Bond: {C_YELLOW}{old}{C_RESET} \u{2192} \
                 {C_BOLD}{C_YELLOW}{new}{C_RESET} ({C_YELLOW}+{delta}{C_RESET})"
            )
        }
        PetResult::Cooldown { wait_secs } => {
            let wait = format_cooldown(*wait_secs);
            format!("{name} is already purring at you. (next pet in {wait})")
        }
    }
}

pub fn play() {
    let mut s = crate::hatch::load_or_hatch();
    let now = state::now_secs();
    let result = crate::bond::try_play(&mut s, now);
    println!("{}", format_play_message(&s.name, &result, s.last_played_at));
    if matches!(result, crate::bond::PlayResult::Applied { .. }) {
        let _ = state::save_atomic(&s);
    }
}

fn format_play_message(
    name: &str,
    result: &crate::bond::PlayResult,
    last_played_at: u64,
) -> String {
    use crate::bond::PlayResult;
    match result {
        PlayResult::Applied { old, new: _, capped: true, cost } => {
            let base = play_applied_variant(name, last_played_at);
            format!(
                "{base} {C_BOLD}(max, was {old}){C_RESET} · {C_YELLOW}-{cost} coins{C_RESET}"
            )
        }
        PlayResult::Applied { old, new, capped: false, cost } => {
            let delta = *new as i32 - *old as i32;
            let line = play_applied_variant(name, last_played_at);
            format!(
                "{line} Bond: {C_YELLOW}{old}{C_RESET} \u{2192} \
                 {C_BOLD}{C_YELLOW}{new}{C_RESET} ({C_YELLOW}+{delta}{C_RESET}) · \
                 {C_YELLOW}-{cost} coins{C_RESET}"
            )
        }
        PlayResult::Cooldown { wait_secs } => {
            let wait = format_cooldown(*wait_secs);
            format!("{name} is still winded from the last play. (next play in {wait})")
        }
        PlayResult::InsufficientCoins { need, have } => {
            format!("not enough coins for play (need {need}, have {have})")
        }
    }
}

/// Selects one of 3 flavor variants deterministically by `last_played_at % 3`.
fn play_applied_variant(name: &str, last_played_at: u64) -> String {
    match last_played_at % 3 {
        0 => format!("\u{1F3BE} {name} pounces on the feather wand!"),
        1 => format!("\u{1F3BE} {name} chases the laser with frantic joy."),
        _ => format!("\u{1F3BE} {name} bats the toy around triumphantly."),
    }
}

pub fn treat() {
    let mut s = crate::hatch::load_or_hatch();
    let now = state::now_secs();
    let result = crate::bond::try_treat(&mut s, now);
    println!("{}", format_treat_message(&s.name, &result, s.last_treated_at));
    if matches!(result, crate::bond::TreatResult::Applied { .. }) {
        let _ = state::save_atomic(&s);
    }
}

fn format_treat_message(
    name: &str,
    result: &crate::bond::TreatResult,
    last_treated_at: u64,
) -> String {
    use crate::bond::TreatResult;
    match result {
        TreatResult::Applied { old, new: _, capped: true, cost, was_hungry } => {
            let base = treat_flavor_variant(name, last_treated_at, *was_hungry);
            format!(
                "{base} {C_BOLD}(max, was {old}){C_RESET} · {C_YELLOW}-{cost} coins{C_RESET}"
            )
        }
        TreatResult::Applied { old, new, capped: false, cost, was_hungry: true } => {
            let delta = *new as i32 - *old as i32;
            let line = treat_flavor_variant(name, last_treated_at, true);
            format!(
                "{line} Bond: {C_YELLOW}{old}{C_RESET} \u{2192} \
                 {C_BOLD}{C_YELLOW}{new}{C_RESET} ({C_YELLOW}+{delta}{C_RESET}) · \
                 {C_YELLOW}-{cost} coins{C_RESET}"
            )
        }
        TreatResult::Applied { old: _, new, capped: false, cost, was_hungry: false } => {
            let line = treat_flavor_variant(name, last_treated_at, false);
            format!(
                "{line} Bond: {C_YELLOW}{new}{C_RESET} (no change) · \
                 {C_YELLOW}-{cost} coins{C_RESET}"
            )
        }
        TreatResult::Cooldown { wait_secs } => {
            let wait = format_cooldown(*wait_secs);
            format!("{name} isn't ready for another treat yet. (next treat in {wait})")
        }
        TreatResult::InsufficientCoins { need, have } => {
            format!("not enough coins for a treat (need {need}, have {have})")
        }
    }
}

fn treat_hungry_variant(name: &str, last_treated_at: u64) -> String {
    match last_treated_at % 3 {
        0 => format!("\u{1F41F} {name} devours the treat."),
        1 => format!("\u{1F41F} {name} gobbles it up with a happy chirp."),
        _ => format!("\u{1F41F} {name} eats enthusiastically."),
    }
}

fn treat_not_hungry_variant(name: &str, last_treated_at: u64) -> String {
    match last_treated_at % 3 {
        0 => format!("\u{1F41F} {name} nibbles the treat but wasn't really hungry."),
        1 => format!("\u{1F41F} {name} sniffs it, eats it anyway."),
        _ => format!("\u{1F41F} {name} accepts politely but isn't hungry."),
    }
}

fn treat_flavor_variant(name: &str, last_treated_at: u64, was_hungry: bool) -> String {
    if was_hungry {
        treat_hungry_variant(name, last_treated_at)
    } else {
        treat_not_hungry_variant(name, last_treated_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bond::PetResult;
    use crate::state::CelebrationKind;

    #[test]
    fn no_change_when_level_same() {
        assert_eq!(detect_celebration(5, 5), None);
    }

    #[test]
    fn plain_levelup_within_kitten_stage() {
        let (kind, renders) = detect_celebration(3, 4).unwrap();
        assert_eq!(kind, CelebrationKind::LevelUp);
        assert_eq!(renders, 3);
    }

    #[test]
    fn plain_levelup_within_young_stage() {
        let (kind, renders) = detect_celebration(25, 26).unwrap();
        assert_eq!(kind, CelebrationKind::LevelUp);
        assert_eq!(renders, 3);
    }

    #[test]
    fn kitten_to_young_transition_gets_six_renders() {
        let (kind, renders) = detect_celebration(9, 10).unwrap();
        assert_eq!(kind, CelebrationKind::EvolvingYoung);
        assert_eq!(renders, 6);
    }

    #[test]
    fn young_to_adult_transition_gets_six_renders() {
        let (kind, renders) = detect_celebration(49, 50).unwrap();
        assert_eq!(kind, CelebrationKind::EvolvingAdult);
        assert_eq!(renders, 6);
    }

    #[test]
    fn skipping_multiple_levels_into_new_stage_still_transitions() {
        // defensive: if somehow total_tool_calls jumps, still recognize stage change
        let (kind, renders) = detect_celebration(8, 11).unwrap();
        assert_eq!(kind, CelebrationKind::EvolvingYoung);
        assert_eq!(renders, 6);
    }

    #[test]
    fn level_decrease_returns_none() {
        // pathological — never happens in feed_inner, but defends the invariant
        assert_eq!(detect_celebration(10, 9), None);
    }

    #[test]
    fn format_pet_message_applied_not_capped() {
        let msg = format_pet_message(
            "Thalia",
            &PetResult::Applied { old: 65, new: 80, capped: false },
        );
        assert!(msg.contains("Thalia"));
        assert!(msg.contains("65"));
        assert!(msg.contains("80"));
        assert!(msg.contains("+15"));
    }

    #[test]
    fn format_pet_message_applied_capped() {
        let msg = format_pet_message(
            "Thalia",
            &PetResult::Applied { old: 100, new: 100, capped: true },
        );
        assert!(msg.contains("100"));
        assert!(msg.contains("max"));
        // No arrow / delta when capped.
        assert!(!msg.contains("→"));
    }

    #[test]
    fn format_pet_message_cooldown() {
        // 2h 47m == 2*3600 + 47*60 = 10_020s
        let msg = format_pet_message(
            "Thalia",
            &PetResult::Cooldown { wait_secs: 10_020 },
        );
        assert!(msg.contains("Thalia"));
        assert!(msg.contains("2h"));
        assert!(msg.contains("47m"));
        assert!(msg.contains("next pet"));
    }

    use crate::bond::{PlayResult, PLAY_COST};

    #[test]
    fn format_play_message_applied_not_capped() {
        let msg = format_play_message(
            "Thalia",
            &PlayResult::Applied { old: 60, new: 80, capped: false, cost: PLAY_COST },
            /* last_played_at */ 0,
        );
        assert!(msg.contains("Thalia"));
        assert!(msg.contains("60"));
        assert!(msg.contains("80"));
        assert!(msg.contains("+20"));
        assert!(msg.contains("-50 coins"));
    }

    #[test]
    fn format_play_message_applied_capped() {
        let msg = format_play_message(
            "Thalia",
            &PlayResult::Applied { old: 100, new: 100, capped: true, cost: PLAY_COST },
            /* last_played_at */ 0,
        );
        assert!(msg.contains("100"));
        assert!(msg.contains("max"));
        assert!(!msg.contains("→"), "capped message has no delta arrow");
    }

    #[test]
    fn format_play_message_cooldown() {
        let msg = format_play_message(
            "Thalia",
            &PlayResult::Cooldown { wait_secs: 10_020 }, // 2h 47m
            0,
        );
        assert!(msg.contains("Thalia"));
        assert!(msg.contains("2h"));
        assert!(msg.contains("47m"));
        assert!(msg.contains("next play"));
    }

    #[test]
    fn format_play_message_insufficient_coins() {
        let msg = format_play_message(
            "Thalia",
            &PlayResult::InsufficientCoins { need: 50, have: 30 },
            0,
        );
        assert!(msg.contains("not enough coins"));
        assert!(msg.contains("50"));
        assert!(msg.contains("30"));
    }

    #[test]
    fn format_play_message_variant_rotation_is_deterministic() {
        // Three fixed last_played_at values hitting all three mod-3 slots.
        let r = PlayResult::Applied { old: 60, new: 80, capped: false, cost: PLAY_COST };
        let m0 = format_play_message("X", &r, 0);
        let m1 = format_play_message("X", &r, 1);
        let m2 = format_play_message("X", &r, 2);
        let m3 = format_play_message("X", &r, 3);
        assert_ne!(m0, m1, "rotation should produce different messages");
        assert_ne!(m1, m2, "rotation should produce different messages");
        assert_eq!(m0, m3, "rotation must cycle every 3");
    }

    use crate::bond::{TreatResult, TREAT_COST};

    #[test]
    fn format_treat_message_applied_hungry_not_capped() {
        let msg = format_treat_message(
            "Thalia",
            &TreatResult::Applied {
                old: 60,
                new: 65,
                capped: false,
                cost: TREAT_COST,
                was_hungry: true,
            },
            0,
        );
        assert!(msg.contains("Thalia"));
        assert!(msg.contains("60"));
        assert!(msg.contains("65"));
        assert!(msg.contains("+5"));
        assert!(msg.contains("-25 coins"));
    }

    #[test]
    fn format_treat_message_applied_not_hungry_bond_unchanged() {
        let msg = format_treat_message(
            "Thalia",
            &TreatResult::Applied {
                old: 60,
                new: 60,
                capped: false,
                cost: TREAT_COST,
                was_hungry: false,
            },
            0,
        );
        assert!(msg.contains("Thalia"));
        assert!(msg.contains("60"));
        assert!(msg.contains("no change"));
        assert!(msg.contains("-25 coins"));
        assert!(!msg.contains("→"), "not-hungry message has no delta arrow");
        assert!(!msg.contains("+5"));
    }

    #[test]
    fn format_treat_message_applied_capped() {
        let msg = format_treat_message(
            "Thalia",
            &TreatResult::Applied {
                old: 100,
                new: 100,
                capped: true,
                cost: TREAT_COST,
                was_hungry: true,
            },
            0,
        );
        assert!(msg.contains("100"));
        assert!(msg.contains("max"));
    }

    #[test]
    fn format_treat_message_cooldown() {
        let msg = format_treat_message(
            "Thalia",
            &TreatResult::Cooldown { wait_secs: 7_500 }, // 2h 05m
            0,
        );
        assert!(msg.contains("Thalia"));
        assert!(msg.contains("2h"));
        assert!(msg.contains("05m"));
        assert!(msg.contains("next treat"));
    }

    #[test]
    fn format_treat_message_insufficient_coins() {
        let msg = format_treat_message(
            "Thalia",
            &TreatResult::InsufficientCoins { need: 25, have: 12 },
            0,
        );
        assert!(msg.contains("not enough coins"));
        assert!(msg.contains("treat"));
        assert!(msg.contains("25"));
        assert!(msg.contains("12"));
    }

    #[test]
    fn format_treat_message_variant_rotation_is_deterministic() {
        let r = TreatResult::Applied {
            old: 60, new: 65, capped: false, cost: TREAT_COST, was_hungry: true,
        };
        let m0 = format_treat_message("X", &r, 0);
        let m1 = format_treat_message("X", &r, 1);
        let m2 = format_treat_message("X", &r, 2);
        let m3 = format_treat_message("X", &r, 3);
        assert_ne!(m0, m1);
        assert_ne!(m1, m2);
        assert_eq!(m0, m3);
    }

    #[test]
    fn format_treat_message_applied_capped_not_hungry_uses_not_hungry_copy() {
        let msg = format_treat_message(
            "Thalia",
            &TreatResult::Applied {
                old: 100, new: 100, capped: true, cost: TREAT_COST, was_hungry: false,
            },
            0,
        );
        assert!(msg.contains("max"));
        assert!(msg.contains("100"));
        // Must use the not-hungry flavor text, not the hungry copy
        assert!(!msg.contains("devours"));
        assert!(!msg.contains("gobbles"));
        assert!(!msg.contains("enthusiastically"));
    }
}
