//! Bond stat: dynamic 0..=100 affection, gated by a cooldown and decayed over
//! wall-clock time. Pure logic — no I/O, no state mutation outside `try_pet`'s
//! mutable-reference API.

pub const BOND_START: u8 = 50;
#[allow(dead_code)] // TODO(Task 3): remove once try_pet uses this
pub const BOND_PET_BUMP: u8 = 15;
#[allow(dead_code)] // TODO(Task 3): remove once try_pet uses this
pub const BOND_PET_COOLDOWN_SEC: u64 = 4 * 3600;
pub const BOND_DECAY_PER_DAY: u32 = 5;
#[allow(dead_code)] // TODO(Task 4): remove once mood logic uses this
pub const BOND_HAPPY_MOOD_WINDOW_SEC: i64 = 120;

#[allow(dead_code)] // TODO(Task 3/7/8): remove once try_pet, card, statusline wire this up
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
