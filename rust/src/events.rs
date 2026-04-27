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

/// Detects a tool-call-count milestone crossing.
///
/// Returns `Some((kind, new_watermark))` when the increment crosses a milestone
/// threshold; otherwise `None`. The watermark gate prevents re-firing the same
/// milestone if `total_tool_calls` ever moves backward or stays put.
///
/// Under the current +1 invariant (`total_tool_calls += 1` per `feed()`), at
/// most one milestone can fire per call. The elif chain ensures that a
/// pathological multi-threshold jump still produces exactly one event per call.
pub fn detect_milestone(
    prev: u64,
    new: u64,
    watermark: u64,
) -> Option<(CelebrationKind, u64)> {
    if prev < 100 && new >= 100 && watermark < 100 {
        Some((CelebrationKind::MilestoneCenturion, 100))
    } else if prev < 500 && new >= 500 && watermark < 500 {
        Some((CelebrationKind::MilestoneVeteran, 500))
    } else if prev < 1000 && new >= 1000 && watermark < 1000 {
        Some((CelebrationKind::MilestoneMaster, 1000))
    } else if new >= 2000 && new % 1000 == 0 && new > watermark {
        Some((CelebrationKind::MilestoneLegendary, new))
    } else {
        None
    }
}

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

    // T3: detect_milestone tests
    use crate::state::CelebrationKind;

    #[test]
    fn milestone_centurion_fires_at_100() {
        assert_eq!(super::detect_milestone(99, 100, 0), Some((CelebrationKind::MilestoneCenturion, 100)));
    }

    #[test]
    fn milestone_centurion_does_not_re_fire() {
        assert_eq!(super::detect_milestone(100, 101, 100), None);
    }

    #[test]
    fn milestone_veteran_fires_at_500() {
        assert_eq!(super::detect_milestone(499, 500, 100), Some((CelebrationKind::MilestoneVeteran, 500)));
    }

    #[test]
    fn milestone_master_fires_at_1000() {
        assert_eq!(super::detect_milestone(999, 1000, 500), Some((CelebrationKind::MilestoneMaster, 1000)));
    }

    #[test]
    fn milestone_legendary_fires_at_2000() {
        assert_eq!(super::detect_milestone(1999, 2000, 1000), Some((CelebrationKind::MilestoneLegendary, 2000)));
    }

    #[test]
    fn milestone_legendary_fires_at_each_kilo_multiple() {
        assert_eq!(super::detect_milestone(2999, 3000, 2000), Some((CelebrationKind::MilestoneLegendary, 3000)));
        assert_eq!(super::detect_milestone(4999, 5000, 4000), Some((CelebrationKind::MilestoneLegendary, 5000)));
        assert_eq!(super::detect_milestone(49_999, 50_000, 49_000), Some((CelebrationKind::MilestoneLegendary, 50_000)));
    }

    #[test]
    fn milestone_no_fire_between_kilo_multiples() {
        assert_eq!(super::detect_milestone(2500, 2501, 2000), None);
        assert_eq!(super::detect_milestone(3499, 3500, 3000), None); // 3500 not a kilo-multiple
    }

    #[test]
    fn milestone_no_fire_below_centurion() {
        assert_eq!(super::detect_milestone(0, 1, 0), None);
        assert_eq!(super::detect_milestone(98, 99, 0), None);
    }

    #[test]
    fn milestone_defense_in_depth_jump_fires_centurion_first() {
        // Pathological: total jumps 50 → 1500 in one call (impossible under the
        // current +1 invariant, but if it ever happens watermark gating prevents
        // firing two milestones in one call. Centurion wins because it's checked first.)
        assert_eq!(super::detect_milestone(50, 1500, 0), Some((CelebrationKind::MilestoneCenturion, 100)));
    }
}

// Silences the unused-const warnings until the integration tasks consume them.
#[allow(dead_code)]
fn _scaffold_holds(_: CelebrationKind) {}
