use std::fs;

use crate::bond::{self, PetResult};
use crate::colors::{C_BOLD, C_PINK, C_RESET, C_YELLOW};
use crate::paths;
use crate::state::{self, CelebrationKind, life_stage};

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
            let h = wait_secs / 3600;
            let m = (wait_secs % 3600) / 60;
            let wait = if h > 0 {
                format!("{h}h {m:02}m")
            } else {
                format!("{m}m")
            };
            format!("{name} is already purring at you. (next pet in {wait})")
        }
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
}
