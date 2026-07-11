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
    } else if new >= 2000 && new.is_multiple_of(1000) && new > watermark {
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

/// Convert a Unix timestamp to (year, month, day) in UTC. We use UTC for
/// birthday matching so the celebration period is consistent regardless of
/// the user's timezone or DST transitions on their birth date — the kitten's
/// "day" is the calendar day in the timezone the box was hatched in, which
/// for this purpose is approximated by UTC. Day-long ambient mode tolerates
/// the ~few-hour offset.
///
/// Algorithm: Howard Hinnant's days_from_civil inverse, restricted to the
/// 1970..2100 range we actually care about.
pub fn secs_to_ymd(secs: u64) -> (i32, u8, u8) {
    let days = (secs / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

fn is_leap_year(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Returns true iff a daily-greeting one-shot should fire for this turn.
///
/// - Returns false if `last_turn_at == 0` (fresh hatch — first turn just
///   initializes the watermark, no welcome-back on a kitten that's never been
///   away).
/// - Returns false if `now < last_turn_at` (defensive against clock skew).
/// - Otherwise, returns true iff `now - last_turn_at >= GREETING_GAP_SECS`.
pub fn should_greet(last_turn_at: u64, now: u64) -> bool {
    if last_turn_at == 0 || now < last_turn_at {
        return false;
    }
    now.saturating_sub(last_turn_at) >= GREETING_GAP_SECS
}

/// Returns true iff `now` falls on the kitten's birthday anniversary.
///
/// Rules:
/// - Same year as birth_time → false (hatch day itself isn't a birthday).
/// - month/day match → true.
/// - Special case: kittens born Feb 29 celebrate Feb 28 in non-leap years.
pub fn is_birthday(birth_time: u64, now: u64) -> bool {
    let (by, bm, bd) = secs_to_ymd(birth_time);
    let (ny, nm, nd) = secs_to_ymd(now);
    if ny == by {
        return false;
    }
    if bm == nm && bd == nd {
        return true;
    }
    // Feb 29 birthday in a non-leap year → celebrate Feb 28.
    if bm == 2 && bd == 29 && nm == 2 && nd == 28 && !is_leap_year(ny) {
        return true;
    }
    false
}

/// Single-step LCG roll. Returns true iff the seed produces a "hit" within
/// the configured permille probability (`RANDOM_DROP_PROB_PERMILLE` / 1000).
///
/// Reuses the LCG primitive from `identity::Lcg` (already used at hatch),
/// keeping the dependency surface unchanged.
pub fn roll_random_drop_with_seed(seed: u64) -> bool {
    let mut rng = crate::identity::Lcg::new(seed);
    (rng.next() % 1000) < RANDOM_DROP_PROB_PERMILLE
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

    #[test]
    fn is_birthday_false_on_birth_day_itself() {
        // Same year as birth → not a birthday yet.
        let birth = ymd_to_secs(2026, 4, 25);
        let now = birth + 7200;  // 2 hours later, same day
        assert!(!super::is_birthday(birth, now));
    }

    #[test]
    fn is_birthday_true_one_year_later_same_md() {
        let birth = ymd_to_secs(2026, 4, 25);
        let now = ymd_to_secs(2027, 4, 25);
        assert!(super::is_birthday(birth, now));
    }

    #[test]
    fn is_birthday_false_different_month_day() {
        let birth = ymd_to_secs(2026, 4, 25);
        let now = ymd_to_secs(2027, 4, 26);
        assert!(!super::is_birthday(birth, now));

        let now = ymd_to_secs(2027, 5, 25);
        assert!(!super::is_birthday(birth, now));
    }

    #[test]
    fn is_birthday_feb29_kitten_celebrates_feb28_in_non_leap_year() {
        let birth = ymd_to_secs(2024, 2, 29);  // 2024 is a leap year
        // 2025 is not a leap year — Feb 29 doesn't exist; celebrate Feb 28.
        let now = ymd_to_secs(2025, 2, 28);
        assert!(super::is_birthday(birth, now));
        // Feb 27 should not match.
        let now = ymd_to_secs(2025, 2, 27);
        assert!(!super::is_birthday(birth, now));
    }

    #[test]
    fn is_birthday_feb29_kitten_celebrates_feb29_in_leap_year() {
        let birth = ymd_to_secs(2024, 2, 29);
        let now = ymd_to_secs(2028, 2, 29);  // 2028 is a leap year
        assert!(super::is_birthday(birth, now));
    }

    #[test]
    fn is_birthday_non_feb29_kitten_unaffected_by_leap_year_logic() {
        let birth = ymd_to_secs(2026, 3, 15);
        let now = ymd_to_secs(2027, 3, 15);
        assert!(super::is_birthday(birth, now));
        let now = ymd_to_secs(2027, 2, 28);
        assert!(!super::is_birthday(birth, now));
    }

    // T5: should_greet tests
    #[test]
    fn should_greet_false_on_first_turn_after_hatch() {
        // last_turn_at == 0 means "never seen a turn before". Initialize only.
        assert!(!super::should_greet(0, 1_000_000));
    }

    #[test]
    fn should_greet_false_for_short_gap() {
        let last = 1_000_000;
        let now = last + 5 * 3600 + 59 * 60;  // 5h 59m — under threshold
        assert!(!super::should_greet(last, now));
    }

    #[test]
    fn should_greet_true_at_exactly_six_hours() {
        let last = 1_000_000;
        let now = last + 6 * 3600;
        assert!(super::should_greet(last, now));
    }

    #[test]
    fn should_greet_true_for_long_gap() {
        let last = 1_000_000;
        let now = last + 86_400;  // 24h
        assert!(super::should_greet(last, now));
    }

    #[test]
    fn should_greet_false_when_now_is_before_last_turn_at() {
        // Defensive: clock skew or state restore — never panic, never greet on regression.
        assert!(!super::should_greet(2_000_000, 1_000_000));
    }

    // T6: roll_random_drop_with_seed tests
    #[test]
    fn random_drop_distribution_is_approximately_correct() {
        // Roll 10_000 deterministic seeds and count hits. Expected hit rate is
        // 0.5%, so ~50 hits out of 10_000. Allow a wide margin (15..=85) to keep
        // the test stable across LCG implementations while still catching e.g.
        // off-by-1000 bugs (5% would give ~500, 0.05% would give ~5).
        let hits: usize = (0..10_000u64)
            .filter(|i| super::roll_random_drop_with_seed(*i))
            .count();
        assert!(
            (15..=85).contains(&hits),
            "expected ~50 hits per 10k seeds, got {hits}",
        );
    }

    #[test]
    fn random_drop_is_deterministic_for_same_seed() {
        let a = super::roll_random_drop_with_seed(42);
        let b = super::roll_random_drop_with_seed(42);
        assert_eq!(a, b);
    }

    #[test]
    fn random_drop_is_not_constant_across_seeds() {
        // Smoke check: at least one of the first 1000 seeds hits, at least one misses.
        let mut hit = false;
        let mut miss = false;
        for s in 0..1000u64 {
            if super::roll_random_drop_with_seed(s) { hit = true; } else { miss = true; }
            if hit && miss { return; }
        }
        panic!("roll_random_drop_with_seed appears constant across 1000 seeds");
    }

    // Test helper: convert a UTC date (year, month, day) to a Unix timestamp
    // at noon UTC. Avoids midnight-edge timezone confusion in tests by
    // landing solidly in the middle of the day.
    fn ymd_to_secs(year: i32, month: u8, day: u8) -> u64 {
        // Hardcoded Julian-day arithmetic for the test range we care about
        // (1970..2100). Using the standard formula:
        let m = month as i32;
        let y = year;
        let (y, m) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
        let era = y.div_euclid(400);
        let yoe = (y - era * 400) as u32;
        let doy = (153 * (m - 3) as u32 + 2) / 5 + day as u32 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        let days_since_epoch = era as i64 * 146_097 + doe as i64 - 719_468;
        (days_since_epoch * 86_400 + 12 * 3600) as u64  // noon UTC
    }
}
