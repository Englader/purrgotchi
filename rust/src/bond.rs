//! Bond stat: dynamic 0..=100 affection, gated by a cooldown and decayed over
//! wall-clock time. Pure logic — no I/O, no state mutation outside `try_pet`'s
//! mutable-reference API.

use crate::state::State;

pub const BOND_START: u8 = 50;
pub const BOND_PET_BUMP: u8 = 15;
pub const BOND_PET_COOLDOWN_SEC: u64 = 4 * 3600;
pub const BOND_DECAY_PER_DAY: u32 = 5;
pub const BOND_HAPPY_MOOD_WINDOW_SEC: i64 = 120;

// v3.4.6 — play tuning
pub const PLAY_COST: u64 = 50;
pub const PLAY_BOND_BUMP: u8 = 20;
pub const PLAY_COOLDOWN_SEC: u64 = 6 * 3600;
pub const PLAY_HAPPY_MOOD_WINDOW_SEC: i64 = 180;

// v3.4.6 — treat tuning
pub const TREAT_COST: u64 = 25;
pub const TREAT_BOND_BUMP: u8 = 5;
pub const TREAT_COOLDOWN_SEC: u64 = 4 * 3600;
pub const TREAT_HAPPY_MOOD_WINDOW_SEC: i64 = 120;
pub const TREAT_HUNGER_THRESHOLD_SEC: u64 = 3600;

pub fn effective_bond(bond: u8, last_pet_at: u64, now: u64) -> u8 {
    if last_pet_at == 0 {
        return bond;
    }
    let secs = now.saturating_sub(last_pet_at);
    let decay_points = (secs as u128)
        .saturating_mul(BOND_DECAY_PER_DAY as u128)
        / 86_400;
    let decay = decay_points.min(u8::MAX as u128) as u8;
    bond.saturating_sub(decay)
}

/// Returns the most recent of (`last_pet_at`, `last_played_at`, `last_treated_at`).
/// Zero means "never interacted at all" — used as the decay anchor.
pub fn last_interaction_at(s: &State) -> u64 {
    s.last_pet_at
        .max(s.last_played_at)
        .max(s.last_treated_at)
}

#[derive(Debug, PartialEq)]
pub enum PetResult {
    Applied { old: u8, new: u8, capped: bool },
    Cooldown { wait_secs: u64 },
}

pub fn try_pet(state: &mut State, now: u64) -> PetResult {
    if state.last_pet_at != 0 {
        let elapsed = now.saturating_sub(state.last_pet_at);
        if elapsed < BOND_PET_COOLDOWN_SEC {
            return PetResult::Cooldown {
                wait_secs: BOND_PET_COOLDOWN_SEC - elapsed,
            };
        }
    }
    // Decay anchors on the most recent interaction of any kind, not just pet.
    let anchor = last_interaction_at(state);
    let effective = effective_bond(state.bond, anchor, now);
    let capped = effective == 100;
    let new = effective.saturating_add(BOND_PET_BUMP).min(100);
    state.bond = new;
    state.last_pet_at = now;
    PetResult::Applied { old: effective, new, capped }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::State;

    #[test]
    fn effective_bond_never_petted_returns_raw() {
        // last_pet_at == 0 means "never petted" — no decay applied.
        assert_eq!(effective_bond(50, 0, 1_000_000), 50);
        assert_eq!(effective_bond(100, 0, 999_999_999), 100);
    }

    #[test]
    fn effective_bond_fresh_pet_unchanged() {
        // now == last_pet_at: zero elapsed, zero decay.
        assert_eq!(effective_bond(80, 1000, 1000), 80);
    }

    #[test]
    fn effective_bond_one_day_loses_five() {
        // 86400 seconds == 5 decay points at BOND_DECAY_PER_DAY=5.
        assert_eq!(effective_bond(80, 0 + 1, 1 + 86_400), 75);
    }

    #[test]
    fn effective_bond_half_day_loses_two_floor() {
        // 43200 s * 5 / 86400 = 2 (integer division).
        assert_eq!(effective_bond(80, 1, 1 + 43_200), 78);
    }

    #[test]
    fn effective_bond_floors_at_zero_never_wraps() {
        // 100 days of neglect; still 0, never negative/wrapped.
        let hundred_days = 100 * 86_400;
        assert_eq!(effective_bond(50, 1, 1 + hundred_days), 0);
    }

    #[test]
    fn effective_bond_saturating_on_extreme_now() {
        // Pathological: now far less than last_pet_at (clock went backwards).
        // saturating_sub should keep elapsed at 0, no decay.
        assert_eq!(effective_bond(50, 1_000_000, 0), 50);
    }

    fn mk_state(bond: u8, last_pet_at: u64) -> State {
        let mut s = State::base(1000);
        s.bond = bond;
        s.last_pet_at = last_pet_at;
        s
    }

    #[test]
    fn try_pet_first_ever_applies_bump_from_raw_bond() {
        let mut s = mk_state(50, 0); // never petted
        let now = 10_000;
        let r = try_pet(&mut s, now);
        match r {
            PetResult::Applied { old, new, capped } => {
                assert_eq!(old, 50);
                assert_eq!(new, 65);
                assert!(!capped);
            }
            _ => panic!("expected Applied, got {r:?}"),
        }
        assert_eq!(s.bond, 65);
        assert_eq!(s.last_pet_at, now);
    }

    #[test]
    fn try_pet_applies_bump_to_effective_not_raw() {
        // Raw bond=80, but one day of decay → effective=75. Pet bumps to 90.
        let mut s = mk_state(80, 1);
        let now = 1 + 86_400;
        let r = try_pet(&mut s, now);
        match r {
            PetResult::Applied { old, new, capped } => {
                assert_eq!(old, 75, "should bump from effective not raw");
                assert_eq!(new, 90);
                assert!(!capped);
            }
            _ => panic!("expected Applied, got {r:?}"),
        }
    }

    #[test]
    fn try_pet_bumping_to_hundred_leaves_capped_false() {
        let mut s = mk_state(95, 1);
        // Outside cooldown: now >= last_pet_at + BOND_PET_COOLDOWN_SEC.
        let now = 1 + BOND_PET_COOLDOWN_SEC;
        let r = try_pet(&mut s, now);
        match r {
            PetResult::Applied { old: _, new, capped } => {
                assert_eq!(new, 100);
                assert!(!capped, "saturation during pet should not set capped");
            }
            _ => panic!("expected Applied, got {r:?}"),
        }
        assert_eq!(s.bond, 100);
    }

    #[test]
    fn try_pet_reports_capped_true_when_effective_was_already_hundred() {
        let mut s = mk_state(100, 1);
        let now = 1 + BOND_PET_COOLDOWN_SEC;
        let r = try_pet(&mut s, now);
        match r {
            PetResult::Applied { old, new, capped } => {
                assert_eq!(old, 100);
                assert_eq!(new, 100);
                assert!(capped, "capped flag must be true when already at max");
            }
            _ => panic!("expected Applied, got {r:?}"),
        }
    }

    #[test]
    fn try_pet_within_cooldown_rejects_with_remaining_wait() {
        let mut s = mk_state(50, 1000);
        let now = 1000 + 3600; // 1h after last pet, cooldown is 4h
        let r = try_pet(&mut s, now);
        match r {
            PetResult::Cooldown { wait_secs } => {
                assert_eq!(wait_secs, BOND_PET_COOLDOWN_SEC - 3600);
            }
            _ => panic!("expected Cooldown, got {r:?}"),
        }
        // State unchanged on cooldown.
        assert_eq!(s.bond, 50);
        assert_eq!(s.last_pet_at, 1000);
    }

    #[test]
    fn try_pet_exactly_at_cooldown_boundary_is_allowed() {
        let mut s = mk_state(50, 1000);
        let now = 1000 + BOND_PET_COOLDOWN_SEC;
        let r = try_pet(&mut s, now);
        assert!(matches!(r, PetResult::Applied { .. }));
    }

    #[test]
    fn last_interaction_at_returns_max_of_three() {
        let mut s = State::base(1000);
        s.last_pet_at = 100;
        s.last_played_at = 500;
        s.last_treated_at = 300;
        assert_eq!(last_interaction_at(&s), 500);
    }

    #[test]
    fn last_interaction_at_zero_when_never_interacted() {
        let s = State::base(1000);
        assert_eq!(last_interaction_at(&s), 0);
    }

    #[test]
    fn last_interaction_at_ignores_zero_sentinels() {
        let mut s = State::base(1000);
        s.last_pet_at = 0;
        s.last_played_at = 200;
        s.last_treated_at = 0;
        assert_eq!(last_interaction_at(&s), 200);
    }

    #[test]
    fn try_pet_decays_from_last_played_when_pet_never_happened() {
        // Played yesterday, never petted. Decay should anchor to play, not
        // to `last_pet_at == 0` (which would wrongly skip decay).
        let mut s = State::base(1000);
        s.bond = 80;
        s.last_pet_at = 0;
        s.last_played_at = 1;
        s.last_treated_at = 0;
        // Now is exactly 1 day after last_played_at, so 5 points decay.
        let now = 1 + 86_400;
        let r = try_pet(&mut s, now);
        match r {
            PetResult::Applied { old, new, capped: _ } => {
                // Effective = 80 - 5 decay = 75, then + BOND_PET_BUMP(15) = 90.
                assert_eq!(old, 75, "should bump from effective anchored on last_played_at");
                assert_eq!(new, 90);
            }
            _ => panic!("expected Applied, got {r:?}"),
        }
    }
}
