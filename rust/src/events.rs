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

#[cfg(test)]
mod tests {
    // Per-function tests live in their respective tasks.
}

// Silences the unused-const warnings until the integration tasks consume them.
#[allow(dead_code)]
fn _scaffold_holds(_: CelebrationKind) {}
