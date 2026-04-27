//! Pure detection helpers for the v3.4.8 D-group events (streak, milestone,
//! birthday, daily greeting, random drop). No I/O, no mutable state. All
//! callers (hooks.rs, statusline.rs) compose these pure functions with the
//! existing CelebrationKind plumbing.

use crate::state::CelebrationKind;

pub const STREAK_TRIGGER: u8 = 5;
pub const GREETING_GAP_SECS: u64 = 6 * 3600;
pub const RANDOM_DROP_REWARD: u64 = 10;
pub const RANDOM_DROP_PROB_PERMILLE: u64 = 5; // 5/1000 = 0.5%

pub const STREAK_RENDERS: u8 = 3;
pub const MILESTONE_RENDERS: u8 = 6;
pub const GREETING_RENDERS: u8 = 3;
pub const RANDOM_DROP_RENDERS: u8 = 3;

// Implementations land in subsequent tasks (T2-T6).

/// Returns true iff the streak counter has just reached the trigger threshold.
/// Counter is reset to 0 by `turn()` and incremented by `feed()` before this check.
/// Exact-equality (==) prevents re-fire on counts above the trigger.
pub fn detect_streak(streak_count: u8) -> bool {
    streak_count == STREAK_TRIGGER
}

#[cfg(test)]
mod tests {
    #[test]
    fn streak_does_not_fire_below_threshold() {
        assert!(!super::detect_streak(0));
        assert!(!super::detect_streak(1));
        assert!(!super::detect_streak(4));
    }

    #[test]
    fn streak_fires_at_exactly_threshold() {
        assert!(super::detect_streak(5));
    }

    #[test]
    fn streak_does_not_re_fire_above_threshold() {
        // Once a streak fires, the counter only resets in turn().
        // Subsequent feed() calls must not re-fire at 6, 7, 8...
        assert!(!super::detect_streak(6));
        assert!(!super::detect_streak(7));
        assert!(!super::detect_streak(255));
    }
}

// Silences the unused-const warnings until the integration tasks consume them.
#[allow(dead_code)]
fn _scaffold_holds(_: CelebrationKind) {}
