# v3.4.5 Bond + Pet (C5 + C1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship v3.4.5 — a dynamic `bond` stat (0..=100) that grows via the new `purrgotchi pet` command and decays over wall-clock time, plus HUD + card display changes to surface it.

**Architecture:** One new pure-logic module (`bond.rs`) holds the tuning constants and the `effective_bond` / `try_pet` helpers. State schema grows two additive `#[serde(default)]` fields. Mood computation gains one parameter (`last_pet_delta_sec`) and a top-priority "happy" rule. Statusline rearranges the H1 rail (rarity stars inline, PEAK shifts to column-3 center) and swaps RARITY → BOND in H6 col 3. Card inserts a BOND row between the stat block and personality with its own divider.

**Tech Stack:** Rust 2024 edition, serde/serde_json, `#[cfg(test)]` inline unit tests. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-04-23-c5-c1-bond-and-pet-design.md`

---

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `rust/src/bond.rs` | **Create** | Constants, `PetResult`, `effective_bond`, `try_pet`, unit tests |
| `rust/src/state.rs` | Modify | Add `bond: u8` + `last_pet_at: u64` serde-default fields; update `State::base`; update existing tests |
| `rust/src/mood.rs` | Modify | Add `last_pet_delta_sec` parameter; happy override rule |
| `rust/src/hooks.rs` | Modify | Add `pub fn pet()` with output formatting |
| `rust/src/main.rs` | Modify | Module decl + `"pet"` match arm + usage line |
| `rust/src/statusline.rs` | Modify | Compute `effective_bond`, pass `last_pet_delta_sec` to mood; H6 col 3 renders BOND; H1 rail gets stars + PEAK shifts to col-3 center |
| `rust/src/card.rs` | Modify | Insert BOND row + divider between stat block and personality |
| `rust/Cargo.toml` | Modify | Bump version to `3.4.5` |
| `CHANGELOG.md` | Modify | v3.4.5 entry |

---

## Task 0: Feature branch

**Files:** none

- [ ] **Step 1: Create and check out branch**

Run: `git checkout -b c5-c1-bond-and-pet`

Expected output: `Switched to a new branch 'c5-c1-bond-and-pet'`

- [ ] **Step 2: Confirm clean tree**

Run: `git status`
Expected: `nothing to commit, working tree clean` (or only untracked local files).

---

## Task 1: State schema — add `bond` and `last_pet_at` fields

**Files:**
- Modify: `rust/src/state.rs`

- [ ] **Step 1: Write the failing test for serde default of missing bond fields**

Add this test inside the `mod tests` block at the end of `rust/src/state.rs`, just before the closing `}`:

```rust
    #[test]
    fn v3_4_4_state_without_bond_fields_defaults_to_fifty_and_zero() {
        // A v3.4.4-shaped state.json has no bond or last_pet_at fields.
        // serde(default) must make bond=50 (BOND_START), last_pet_at=0.
        let json = r#"{
            "version": 2,
            "name": "Thalia",
            "birth_time": 1000,
            "last_fed_at": 1000,
            "total_tool_calls": 5,
            "coins": 5,
            "level": 1,
            "lifetime_cost_usd": 0.1,
            "breed": "Tabby",
            "rarity": "common",
            "shiny": false,
            "stats": [42, 80, 10, 55, 30],
            "personality": "A streetwise cat."
        }"#;
        let s = parse_state(json.as_bytes()).expect("v3.4.4 shape should load");
        assert_eq!(s.bond, 50);
        assert_eq!(s.last_pet_at, 0);
    }

    #[test]
    fn state_base_initializes_bond_at_fifty() {
        let s = State::base(12345);
        assert_eq!(s.bond, 50);
        assert_eq!(s.last_pet_at, 0);
    }
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cd rust && cargo test --bin purrgotchi state::`
Expected: both new tests fail to compile with `no field 'bond' on type '&State'` / `no field 'last_pet_at' on type '&State'`.

- [ ] **Step 3: Add the fields and a default function to `State`**

Edit `rust/src/state.rs`. Inside the `State` struct, after the `celebration_kind` field and before the closing `}`, add:

```rust
    // bond (v3.4.5, additive — serde(default) for back-compat)
    #[serde(default = "default_bond")]
    pub bond: u8,
    #[serde(default)]
    pub last_pet_at: u64,
```

Add this free function immediately after the `State` struct definition (before `impl State`):

```rust
fn default_bond() -> u8 { 50 }
```

Update `State::base` — inside the returned struct literal, after `celebration_kind: CelebrationKind::None,` add:

```rust
            bond: 50,
            last_pet_at: 0,
```

- [ ] **Step 4: Run tests to verify pass**

Run: `cd rust && cargo test --bin purrgotchi state::`
Expected: all state tests pass, including the two new ones.

- [ ] **Step 5: Run full test suite to confirm no regressions**

Run: `cd rust && cargo test`
Expected: all pre-existing tests still pass.

- [ ] **Step 6: Commit**

```bash
git add rust/src/state.rs
git commit -m "state: add bond + last_pet_at fields with serde defaults"
```

---

## Task 2: `bond.rs` module — constants, `PetResult`, `effective_bond`

**Files:**
- Create: `rust/src/bond.rs`
- Modify: `rust/src/main.rs` (module declaration only)

- [ ] **Step 1: Declare the new module in main.rs**

Edit `rust/src/main.rs`. In the alphabetical module list at the top, add `mod bond;` between `mod card;` and `mod colors;`:

```rust
mod bond;
mod card;
mod colors;
```

- [ ] **Step 2: Write the failing tests**

Create `rust/src/bond.rs` with this initial content (tests first, no implementation yet):

```rust
//! Bond stat: dynamic 0..=100 affection, gated by a cooldown and decayed over
//! wall-clock time. Pure logic — no I/O, no state mutation outside `try_pet`'s
//! mutable-reference API.

use crate::state::State;

pub const BOND_START: u8 = 50;
pub const BOND_PET_BUMP: u8 = 15;
pub const BOND_PET_COOLDOWN_SEC: u64 = 4 * 3600;
pub const BOND_DECAY_PER_DAY: u32 = 5;
pub const BOND_HAPPY_MOOD_WINDOW_SEC: i64 = 120;

pub fn effective_bond(_bond: u8, _last_pet_at: u64, _now: u64) -> u8 {
    unimplemented!()
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
```

- [ ] **Step 3: Run tests to verify failure**

Run: `cd rust && cargo test --bin purrgotchi bond::`
Expected: tests fail (or panic) because `effective_bond` is `unimplemented!()`.

- [ ] **Step 4: Implement `effective_bond`**

Replace the `unimplemented!()` body in `rust/src/bond.rs`:

```rust
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
```

- [ ] **Step 5: Run tests to verify pass**

Run: `cd rust && cargo test --bin purrgotchi bond::`
Expected: all 6 bond tests pass.

- [ ] **Step 6: Run full test suite**

Run: `cd rust && cargo test`
Expected: everything passes.

- [ ] **Step 7: Commit**

```bash
git add rust/src/bond.rs rust/src/main.rs
git commit -m "bond: add module with constants and effective_bond() pure helper"
```

---

## Task 3: `bond.rs` — `PetResult` enum and `try_pet`

**Files:**
- Modify: `rust/src/bond.rs`

- [ ] **Step 1: Write failing tests for `try_pet`**

Append these tests to the `mod tests` block in `rust/src/bond.rs`:

```rust
    use crate::state::State;

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
    fn try_pet_saturates_at_hundred_with_capped_flag() {
        let mut s = mk_state(95, 1);
        // Outside cooldown: now >= last_pet_at + BOND_PET_COOLDOWN_SEC.
        let now = 1 + BOND_PET_COOLDOWN_SEC;
        let r = try_pet(&mut s, now);
        match r {
            PetResult::Applied { old: _, new, capped: _ } => {
                assert_eq!(new, 100);
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
```

Also add a `#[derive(Debug, PartialEq)]` line to the `PetResult` enum (still to be declared in Step 2) so the `panic!("... {r:?}")` messages compile.

- [ ] **Step 2: Declare `PetResult` and a stub `try_pet`**

Add this immediately below the `effective_bond` function in `rust/src/bond.rs`:

```rust
#[derive(Debug, PartialEq)]
pub enum PetResult {
    Applied { old: u8, new: u8, capped: bool },
    Cooldown { wait_secs: u64 },
}

pub fn try_pet(_state: &mut State, _now: u64) -> PetResult {
    unimplemented!()
}
```

- [ ] **Step 3: Run tests to verify failure**

Run: `cd rust && cargo test --bin purrgotchi bond::`
Expected: new tests fail at `unimplemented!()`; compile succeeds.

- [ ] **Step 4: Implement `try_pet`**

Replace the stub body:

```rust
pub fn try_pet(state: &mut State, now: u64) -> PetResult {
    if state.last_pet_at != 0 {
        let elapsed = now.saturating_sub(state.last_pet_at);
        if elapsed < BOND_PET_COOLDOWN_SEC {
            return PetResult::Cooldown {
                wait_secs: BOND_PET_COOLDOWN_SEC - elapsed,
            };
        }
    }
    let effective = effective_bond(state.bond, state.last_pet_at, now);
    let capped = effective == 100;
    let new = effective.saturating_add(BOND_PET_BUMP).min(100);
    state.bond = new;
    state.last_pet_at = now;
    PetResult::Applied { old: effective, new, capped }
}
```

- [ ] **Step 5: Run tests to verify pass**

Run: `cd rust && cargo test --bin purrgotchi bond::`
Expected: all 12 bond tests pass.

- [ ] **Step 6: Run full suite + clippy**

Run: `cd rust && cargo test && cargo clippy -- -D warnings`
Expected: all tests pass, clippy clean.

- [ ] **Step 7: Commit**

```bash
git add rust/src/bond.rs
git commit -m "bond: add PetResult and try_pet with cooldown + saturation"
```

---

## Task 4: Mood override — `happy` after a pet

**Files:**
- Modify: `rust/src/mood.rs`

- [ ] **Step 1: Write failing tests**

Replace the entire contents of `rust/src/mood.rs` with this — keeping the existing `compute_mood` signature intact initially is NOT possible because we're changing its arity. We'll add the new parameter and update callers next task.

First, append a `#[cfg(test)] mod tests` block to `rust/src/mood.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::bond::BOND_HAPPY_MOOD_WINDOW_SEC;

    #[test]
    fn mood_happy_when_recently_petted() {
        // Within the window, mood is happy regardless of feed time.
        assert_eq!(compute_mood(5.0, 0, 0), "happy");
        assert_eq!(compute_mood(5.0, 0, BOND_HAPPY_MOOD_WINDOW_SEC - 1), "happy");
    }

    #[test]
    fn mood_falls_through_after_pet_window_elapses() {
        // last_pet_delta >= window → happy does not apply; existing rules win.
        // At delta=0s fed, this should be happy-by-feed (< 30s), but the new
        // rule is about pet, not feed. Pass large feed delta to avoid the feed
        // "happy" rule and land on "calm".
        assert_eq!(
            compute_mood(5.0, 200, BOND_HAPPY_MOOD_WINDOW_SEC),
            "calm"
        );
    }

    #[test]
    fn mood_negative_last_pet_delta_means_never_petted_no_happy() {
        // Convention: negative last_pet_delta means "never petted" (caller
        // passes -1 or similar sentinel). Must NOT trigger happy override.
        assert_eq!(compute_mood(5.0, 200, -1), "calm");
    }

    #[test]
    fn mood_panic_beats_happy_even_when_recently_petted() {
        assert_eq!(compute_mood(95.0, 0, 0), "panic");
    }

    #[test]
    fn mood_alert_beats_happy_even_when_recently_petted() {
        assert_eq!(compute_mood(50.0, 0, 0), "alert");
    }
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cd rust && cargo test --bin purrgotchi mood::`
Expected: compile error — `compute_mood` only takes 2 arguments, tests pass 3.

- [ ] **Step 3: Update `compute_mood` signature and body**

Replace the entire `compute_mood` function in `rust/src/mood.rs` with:

```rust
pub fn compute_mood(
    ctx_pct: f64,
    last_fed_delta_sec: i64,
    last_pet_delta_sec: i64,
) -> &'static str {
    let ctx_int = ctx_pct.round() as i64;
    if ctx_int > 80 {
        "panic"
    } else if ctx_int > 40 {
        "alert"
    } else if last_pet_delta_sec >= 0
        && last_pet_delta_sec < crate::bond::BOND_HAPPY_MOOD_WINDOW_SEC
    {
        "happy"
    } else if last_fed_delta_sec > 21600 {
        "sleep"
    } else if last_fed_delta_sec > 3600 {
        "hungry"
    } else if last_fed_delta_sec < 30 {
        "happy"
    } else {
        "calm"
    }
}
```

Priority order now:
1. `panic` (ctx > 80%) — emergency overrides everything.
2. `alert` (ctx > 40%) — same reason.
3. `happy` (pet within 120s) — new rule.
4. `sleep` / `hungry` / feed-based `happy` / `calm` — unchanged.

- [ ] **Step 4: Run mood tests**

Run: `cd rust && cargo test --bin purrgotchi mood::`
Expected: all 5 mood tests pass.

- [ ] **Step 5: Update the single statusline caller of `compute_mood`**

The signature change breaks `statusline.rs`. Edit `rust/src/statusline.rs`:

Find the line:
```rust
    let mood = compute_mood(payload.context_window.used_percentage, last_fed_delta);
```

Replace with:
```rust
    let last_pet_delta: i64 = if st.last_pet_at > 0 {
        now - st.last_pet_at as i64
    } else {
        -1
    };
    let mood = compute_mood(
        payload.context_window.used_percentage,
        last_fed_delta,
        last_pet_delta,
    );
```

(`now` and `st` are already bound earlier in `run()`. The `-1` sentinel prevents the happy override from triggering on a never-petted kitten.)

- [ ] **Step 6: Run full suite + clippy**

Run: `cd rust && cargo test && cargo clippy -- -D warnings`
Expected: all tests pass (including the 4 pre-existing statusline tests), clippy clean.

- [ ] **Step 7: Commit**

```bash
git add rust/src/mood.rs rust/src/statusline.rs
git commit -m "mood: add happy override within BOND_HAPPY_MOOD_WINDOW_SEC of last pet"
```

---

## Task 5: `hooks::pet` — the command behavior

**Files:**
- Modify: `rust/src/hooks.rs`

- [ ] **Step 1: Write tests for the pet output formatter**

We'll extract the output-string formatting into a pure helper so it's testable. Append this test block — actually, add it inside the existing `#[cfg(test)] mod tests` block in `rust/src/hooks.rs`, above the closing `}` of that block:

```rust
    use crate::bond::PetResult;

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
        // 2h 47m == 2*3600 + 47*60 = 9_420s
        let msg = format_pet_message(
            "Thalia",
            &PetResult::Cooldown { wait_secs: 9_420 },
        );
        assert!(msg.contains("Thalia"));
        assert!(msg.contains("2h"));
        assert!(msg.contains("47m"));
        assert!(msg.contains("next pet"));
    }
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cd rust && cargo test --bin purrgotchi hooks::`
Expected: compile error — `format_pet_message` undefined.

- [ ] **Step 3: Implement `format_pet_message` and `pet`**

Edit `rust/src/hooks.rs`. At the top of the file, under the existing `use` lines, add:

```rust
use crate::bond::{self, PetResult};
use crate::colors::{C_BOLD, C_PINK, C_RESET, C_YELLOW};
```

(Keep the pre-existing `use crate::paths;` and `use crate::state::...` lines.)

Below the existing `pub fn rename(...)` function (but above `#[cfg(test)] mod tests`), add:

```rust
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
```

Notes:
- `\u{2665}` = `♥`, `\u{2192}` = `→`. Using escapes keeps the source file pure ASCII.
- On the capped branch, `old` in "was {old}" is the effective bond *before* the pet. If the user was already at 100, `old` and `new` both equal 100 — message reads "Bond: 100 (max, was 100)". Acceptable; alternative of dropping the `was` branch is fine but wordier.

- [ ] **Step 4: Run tests to verify pass**

Run: `cd rust && cargo test --bin purrgotchi hooks::`
Expected: all hooks tests pass, including the 3 new format tests.

- [ ] **Step 5: Run full suite + clippy**

Run: `cd rust && cargo test && cargo clippy -- -D warnings`
Expected: green.

- [ ] **Step 6: Commit**

```bash
git add rust/src/hooks.rs
git commit -m "hooks: add pet() command with formatted output"
```

---

## Task 6: Wire up `purrgotchi pet` in main.rs

**Files:**
- Modify: `rust/src/main.rs`

- [ ] **Step 1: Add the match arm**

Edit `rust/src/main.rs`. In the `match cmd { ... }` block, add a new arm between the existing `"card"` and `"hatch"` arms:

```rust
        "pet" => {
            hooks::pet();
            process::exit(0);
        }
```

- [ ] **Step 2: Update the usage line**

In the same file, find:

```rust
            eprintln!("usage: purrgotchi <statusline|feed|turn|rename|hatch|card>");
```

Replace with:

```rust
            eprintln!("usage: purrgotchi <statusline|feed|turn|rename|hatch|card|pet>");
```

- [ ] **Step 3: Manually verify the command runs**

Run: `cd rust && cargo build && target/debug/purrgotchi pet`
Expected: a line printed that contains the kitten's name and "Bond:". On a fresh install, this will first auto-hatch, then pet.

Run again immediately: `target/debug/purrgotchi pet`
Expected: "is already purring at you. (next pet in 3h 59m)" (or similar — within the 4h cooldown).

- [ ] **Step 4: Confirm full test suite still passes**

Run: `cd rust && cargo test && cargo clippy -- -D warnings`
Expected: green.

- [ ] **Step 5: Commit**

```bash
git add rust/src/main.rs
git commit -m "main: wire purrgotchi pet subcommand"
```

---

## Task 7: Card — add BOND section between stat block and personality

**Files:**
- Modify: `rust/src/card.rs`

- [ ] **Step 1: Write the failing tests**

Append these tests inside the existing `mod tests` block in `rust/src/card.rs`, just before the closing `}` of that block:

```rust
    #[test]
    fn card_contains_bond_label() {
        let output = render(&test_state());
        assert!(output.contains("BOND"), "card missing BOND label:\n{output}");
    }

    #[test]
    fn card_shows_bond_value() {
        let mut s = test_state();
        s.bond = 73;
        s.last_pet_at = 0; // never petted → effective == raw
        let output = render(&s);
        assert!(output.contains("73"), "card missing bond value 73:\n{output}");
    }

    #[test]
    fn card_bond_has_heart_glyph() {
        let output = render(&test_state());
        assert!(output.contains('\u{2665}'), "card missing ♥ glyph:\n{output}");
    }

    #[test]
    fn card_bond_row_reflects_decay() {
        // Raw bond=80, petted 2 days ago → effective = 80 - 10 = 70.
        // (now_secs() is live wall-clock; use a last_pet_at that's "now - 2d".)
        let mut s = test_state();
        s.bond = 80;
        s.last_pet_at = crate::state::now_secs() - 2 * 86_400;
        let output = render(&s);
        // Value could be 70 exactly or 69/71 depending on sub-second timing.
        // Accept any of 69/70/71.
        let contains_any = ["70", "69", "71"].iter().any(|v| output.contains(v));
        assert!(contains_any, "card bond value should reflect decay, got:\n{output}");
    }
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cd rust && cargo test --bin purrgotchi card::`
Expected: new tests fail (card doesn't contain "BOND" yet).

- [ ] **Step 3: Add the BOND row and divider to `render`**

Edit `rust/src/card.rs`. Find this block (around line 108–111, right after the stats loop):

```rust
    // Divider
    writeln!(out, "╠{}╣", "═".repeat(CARD_WIDTH)).unwrap();

    // Personality (word-wrapped, quoted), indented by 2 spaces
```

Replace with:

```rust
    // Divider
    writeln!(out, "╠{}╣", "═".repeat(CARD_WIDTH)).unwrap();

    // Bond row (dynamic stat, visually separated from the 5 identity stats above).
    let now: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let effective = crate::bond::effective_bond(s.bond, s.last_pet_at, now);
    let bond_bar = stat_bar(effective);
    let bond_content = format!(
        "  {:<11}{C_PINK}\u{2665}{C_RESET} {bond_bar}  {:>3}",
        "BOND", effective,
        C_PINK = crate::colors::C_PINK,
        C_RESET = crate::colors::C_RESET,
    );
    // Plain width (for the outer pad): 2 + 11 + 1 + 1 + 10 + 2 + 3 = 30
    let plain_bond_len = 2 + 11 + 1 + 1 + 10 + 2 + 3;
    let pad = inner.saturating_sub(plain_bond_len);
    let bond_line = format!("{bond_content}{}", " ".repeat(pad));
    writeln!(out, "║{bond_line}║").unwrap();

    // Divider
    writeln!(out, "╠{}╣", "═".repeat(CARD_WIDTH)).unwrap();

    // Personality (word-wrapped, quoted), indented by 2 spaces
```

Note the ANSI escapes embedded in `bond_content` — the `card_line_widths_consistent` test strips ANSI via `strip_ansi`, so these don't break it. But the pre-existing name-title and sprite rows use a different padding pattern (pad plain first, then splice color in), which would double-count char widths if we used the same pattern for BOND. The approach above computes plain width manually (`plain_bond_len`) to avoid that trap.

The `format!` variable-capture syntax `C_PINK = crate::colors::C_PINK` uses Rust's named argument feature — valid Rust but uncommon in this codebase. If that's awkward, use the more common form:

```rust
    let pink = crate::colors::C_PINK;
    let reset = crate::colors::C_RESET;
    let bond_content = format!(
        "  {:<11}{pink}\u{2665}{reset} {bond_bar}  {:>3}",
        "BOND", effective,
    );
```

Use whichever form matches the surrounding code style — pick the second (`let pink = ...`) if unsure.

- [ ] **Step 4: Run the card tests**

Run: `cd rust && cargo test --bin purrgotchi card::`
Expected: all card tests pass, including the 4 new bond tests AND the pre-existing `card_line_widths_consistent`.

If `card_line_widths_consistent` fails, it means the plain width calc is off. Check that `plain_bond_len` correctly counts `♥` as 1 char (it's a single `char` in Rust; if the test strips ANSI and counts `chars()`, the heart counts as 1).

- [ ] **Step 5: Run full suite + clippy**

Run: `cd rust && cargo test && cargo clippy -- -D warnings`
Expected: green.

- [ ] **Step 6: Commit**

```bash
git add rust/src/card.rs
git commit -m "card: add BOND row with its own divider between stats and personality"
```

---

## Task 8: Statusline H6 col 3 — replace RARITY with BOND

**Files:**
- Modify: `rust/src/statusline.rs`

- [ ] **Step 1: Read context**

Skim `rust/src/statusline.rs` lines 248–276 — the H6 block. The column-3 currently renders `RARITY: {stars}`. We're swapping it for BOND.

- [ ] **Step 2: Replace the H6 col 3 block**

Edit `rust/src/statusline.rs`. Find:

```rust
    let (c3p, c3a) = if rarity.is_empty() {
        pad_col("", "", W3)
    } else {
        let stars = identity::rarity_stars(&rarity, shiny);
        pad_col(
            &format!("RARITY: {stars}"),
            &format!("{C_DIM}RARITY:{C_RESET} {C_YELLOW}{stars}{C_RESET}"),
            W3,
        )
    };
```

Replace with:

```rust
    let effective_bond = crate::bond::effective_bond(st.bond, st.last_pet_at, now as u64);
    let bond_bar = crate::render::progress_bar(effective_bond as f64, 10);
    let (c3p, c3a) = pad_col(
        &format!("BOND: \u{2665} {bond_bar} {effective_bond}"),
        &format!(
            "{C_DIM}BOND:{C_RESET} {C_PINK}\u{2665}{C_RESET} {C_PINK}{bond_bar}{C_RESET} {C_BOLD}{effective_bond}{C_RESET}"
        ),
        W3,
    );
```

Note: `now` earlier in this function is `i64`; cast to `u64` for the bond call. Bond is never computed against a negative timestamp.

- [ ] **Step 3: Build to verify the change compiles**

Run: `cd rust && cargo build`
Expected: success. If the `identity::rarity_stars` import is now unused, clippy will flag it in the next step — we'll still need `identity` for Task 9 (H1 rail stars), so leave the import alone.

- [ ] **Step 4: Run tests**

Run: `cd rust && cargo test`
Expected: all tests pass. The existing statusline rail-width tests still pass because H6 col 3 plain width (`BOND: ♥ ██████████ 100` = up to ~21 chars) fits within W3=28.

- [ ] **Step 5: Visual check**

Run: `cd rust && cargo run -- statusline < /dev/null 2>/dev/null | tail -10` (or manually via your real Claude Code session if easier).
Expected: H6 shows `BOND: ♥ {bar} {value}` instead of `RARITY: {stars}`.

- [ ] **Step 6: Commit**

```bash
git add rust/src/statusline.rs
git commit -m "statusline: swap H6 col 3 from RARITY stars to BOND bar"
```

---

## Task 9: Statusline H1 rail — inline rarity stars, shift PEAK to col-3 center

**Files:**
- Modify: `rust/src/statusline.rs`

- [ ] **Step 1: Read the current H1 rail assembly**

Lines ~144–157 of `rust/src/statusline.rs`. The current structure:

1. `title_plain` = `═╡ NETRUNNER // {name} ╞`
2. `peak_plain` / `peak_ansi` from `peak_label(local_hour())`
3. `w2_center = 1 + W1 + 3 + W2 / 2` — PEAK centered over H6 col 2
4. Compute `pre_len` and `post_len` rail spans
5. Assemble with `{pink}╔{reset}{title_ansi}{pink}{pre_rail}{reset}{peak_ansi}{pink}{post_rail}╗{reset}`

We're changing this to: title → `pre_rail` → **stars** → `mid_rail` → peak (centered over col 3) → `post_rail` → `╗`.

- [ ] **Step 2: Write tests for the updated rail geometry**

Edit the tests at the bottom of `rust/src/statusline.rs`. Replace the two existing tests `test_h1_rail_total_len_peak` and `test_h1_rail_total_len_off_peak` with a helper + parameterized tests that cover four combinations (peak × shiny) × (peak × non-shiny) × (off-peak × shiny) × (off-peak × non-shiny):

```rust
    fn build_h1_parts(name: &str, hour: u32, rarity: &str, shiny: bool)
        -> (usize, usize, usize, usize, usize, usize) {
        let title_plain = format!("═╡ NETRUNNER // {} ╞", name);
        let (peak_plain, _) = peak_label(hour);
        let stars = crate::identity::rarity_stars(rarity, shiny);
        let stars_plain = format!(" {stars} ");

        let title_len = title_plain.chars().count();
        let peak_len = peak_plain.chars().count();
        let stars_len = stars_plain.chars().count();

        let w3_center = 1 + W1 + 3 + W2 + 3 + W3 / 2;
        let peak_start = w3_center.saturating_sub(peak_len / 2);

        let gap = peak_start.saturating_sub(title_len);
        let pre_stars = gap.saturating_sub(stars_len) / 2;
        let mid_stars = gap.saturating_sub(stars_len + pre_stars);
        let post_peak = BOX_WIDTH.saturating_sub(title_len + gap + peak_len);

        (title_len, pre_stars, stars_len, mid_stars, peak_len, post_peak)
    }

    #[test]
    fn test_h1_rail_total_len_peak_common_non_shiny() {
        let (t, a, s, b, p, c) = build_h1_parts("Thalia", 17, "common", false);
        assert_eq!(t + a + s + b + p + c, BOX_WIDTH);
    }

    #[test]
    fn test_h1_rail_total_len_off_peak_common_non_shiny() {
        let (t, a, s, b, p, c) = build_h1_parts("Thalia", 8, "common", false);
        assert_eq!(t + a + s + b + p + c, BOX_WIDTH);
    }

    #[test]
    fn test_h1_rail_total_len_peak_legendary_shiny() {
        // shiny adds a ✦ prefix → stars_plain is 1 char longer.
        let (t, a, s, b, p, c) = build_h1_parts("Thalia", 17, "legendary", true);
        assert_eq!(t + a + s + b + p + c, BOX_WIDTH);
    }

    #[test]
    fn test_h1_rail_total_len_off_peak_legendary_shiny() {
        let (t, a, s, b, p, c) = build_h1_parts("Thalia", 8, "legendary", true);
        assert_eq!(t + a + s + b + p + c, BOX_WIDTH);
    }
```

(Delete `test_h1_rail_total_len_peak` and `test_h1_rail_total_len_off_peak`; the four new tests subsume them.)

- [ ] **Step 3: Run tests to verify failure**

Run: `cd rust && cargo test --bin purrgotchi statusline::`
Expected: the four new tests fail — they reference logic (`w3_center`, stars gap math) that the production code doesn't yet implement, but since the tests are self-contained (they compute their own math) they should actually **pass** as-is. That's acceptable — these tests codify the geometry we want; the production code change in Step 4 must produce the same widths.

If all four new tests pass here, great — move on. If any fail, the geometry isn't right; fix the math in both the test helper AND Step 4 until they agree.

- [ ] **Step 4: Update the H1 rail assembly in `run`**

Replace the H1 block in `rust/src/statusline.rs` — find:

```rust
    // H1 — top rail. Name stays pink to match the rail glyphs (rarity color
    // lives on the sprite; keeping the HUD name pink avoids a color-vibe clash
    // with the cyan NETRUNNER brand).
    let title_plain = format!("═╡ NETRUNNER // {kitten_name} ╞");
    let title_ansi = format!(
        "{C_PINK}═╡{C_RESET} {C_BOLD}{C_CYAN}NETRUNNER{C_RESET} {C_MAGENTA}//{C_RESET} {C_PINK}{kitten_name}{C_RESET} {C_PINK}╞{C_RESET}"
    );
    let (peak_plain, peak_ansi) = peak_label(local_hour());
    let title_len = title_plain.chars().count();
    let peak_len = peak_plain.chars().count();
    let w2_center = 1 + W1 + 3 + W2 / 2;
    let peak_start = w2_center.saturating_sub(peak_len / 2);
    let pre_len = peak_start.saturating_sub(title_len).max(1);
    let post_len = BOX_WIDTH.saturating_sub(title_len + pre_len + peak_len);
    let pre_rail = "═".repeat(pre_len);
    let post_rail = "═".repeat(post_len);
    let h1 = format!("{C_PINK}╔{C_RESET}{title_ansi}{C_PINK}{pre_rail}{C_RESET}{peak_ansi}{C_PINK}{post_rail}╗{C_RESET}");
```

Replace with:

```rust
    // H1 — top rail. Name stays pink (rail glyphs + name match). Rarity stars
    // ride inline in the rail between the name cap and the PEAK label. PEAK
    // label is now centered over H6 col 3 (above GIT / CTX / Cost / Total).
    let title_plain = format!("═╡ NETRUNNER // {kitten_name} ╞");
    let title_ansi = format!(
        "{C_PINK}═╡{C_RESET} {C_BOLD}{C_CYAN}NETRUNNER{C_RESET} {C_MAGENTA}//{C_RESET} {C_PINK}{kitten_name}{C_RESET} {C_PINK}╞{C_RESET}"
    );
    let (peak_plain, peak_ansi) = peak_label(local_hour());

    let stars = identity::rarity_stars(&rarity, shiny);
    let stars_color = crate::colors::rarity_color(&rarity, shiny, now);
    let stars_plain = format!(" {stars} ");
    let stars_ansi = format!(" {stars_color}{stars}{C_RESET} ");

    let title_len = title_plain.chars().count();
    let peak_len = peak_plain.chars().count();
    let stars_len = stars_plain.chars().count();

    let w3_center = 1 + W1 + 3 + W2 + 3 + W3 / 2;
    let peak_start = w3_center.saturating_sub(peak_len / 2);

    let gap = peak_start.saturating_sub(title_len);
    let pre_stars = gap.saturating_sub(stars_len) / 2;
    let mid_stars = gap.saturating_sub(stars_len + pre_stars);
    let post_peak = BOX_WIDTH.saturating_sub(title_len + gap + peak_len);

    let pre_rail = "═".repeat(pre_stars);
    let mid_rail = "═".repeat(mid_stars);
    let post_rail = "═".repeat(post_peak);
    let h1 = format!(
        "{C_PINK}╔{C_RESET}{title_ansi}\
         {C_PINK}{pre_rail}{C_RESET}{stars_ansi}\
         {C_PINK}{mid_rail}{C_RESET}{peak_ansi}\
         {C_PINK}{post_rail}╗{C_RESET}"
    );
```

Notes:
- The `identity` module is already imported at the top of the file.
- `now` is `i64` in this scope, which is what `rarity_color` expects.
- The `\` line continuations inside the `format!` are valid Rust string literal continuations; they're whitespace-collapsing so the output has no extra spaces.
- Rail spans can be zero-length (e.g. if `gap < stars_len`) — that renders as the empty string and the total still sums to `BOX_WIDTH`.

- [ ] **Step 5: Run tests**

Run: `cd rust && cargo test`
Expected: all tests pass — including the four parameterized H1 rail tests.

- [ ] **Step 6: Run clippy**

Run: `cd rust && cargo clippy -- -D warnings`
Expected: clean.

- [ ] **Step 7: Visual confirmation**

Run: `cd rust && cargo run -- statusline < /dev/null 2>/dev/null | head -1`
(Or view in your real Claude Code statusline.)
Expected: the H1 rail contains `═╡ NETRUNNER // Thalia ╞` on the left, ` ★★★★★ ` (or equivalent stars) somewhere in the middle rail, and `╡ PEAK 2PM-8PM ╞` (or off-peak variant) positioned above column 3. Total visible width (after stripping ANSI) is 94 chars.

- [ ] **Step 8: Commit**

```bash
git add rust/src/statusline.rs
git commit -m "statusline: inline rarity stars in H1 rail and shift PEAK to col-3 center"
```

---

## Task 10: Version bump + CHANGELOG + release binary

**Files:**
- Modify: `rust/Cargo.toml`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Bump version**

Edit `rust/Cargo.toml`. Change:

```toml
version = "3.4.4"
```

to:

```toml
version = "3.4.5"
```

- [ ] **Step 2: Add CHANGELOG entry**

Open `CHANGELOG.md`. Under the top heading (above the v3.4.4 entry), insert:

```markdown
## v3.4.5 — Bond stat & pet command (C5 + C1)

- New: dynamic `bond` stat (0–100). Grows via `purrgotchi pet` (+15, 4h cooldown), decays 5/day with inactivity. Starts at 50 on hatch.
- New: `purrgotchi pet` subcommand. Prints the bond delta (or cooldown-remaining message if within 4h of last pet).
- HUD: H6 col 3 now shows `BOND: ♥ {bar} {value}`. Rarity stars moved to the H1 rail (between the kitten name and the PEAK label), colored by rarity.
- HUD: PEAK/OFF PEAK label recentered over H6 column 3 (above GIT / CTX / Cost / Total).
- Card: new BOND row with its own divider between the 5 identity stats and the personality block.
- Mood: a pet within the last 120s puts the kitten in `happy` mood for the next statusline render (below panic/alert, above the feed-time rules).
- State schema: two additive `#[serde(default)]` fields (`bond`, `last_pet_at`). v3.4.4 state files load with `bond=50`, `last_pet_at=0` — no migration required.

```

(Match the heading style used by existing CHANGELOG entries — check `git log --oneline -- CHANGELOG.md` or glance at the file for the exact format.)

- [ ] **Step 3: Build the release binary**

Run: `cd rust && cargo build --release`
Expected: compiles clean, produces `rust/target/release/purrgotchi`.

- [ ] **Step 4: Smoke-test the release binary end-to-end**

```bash
# Fresh pet (should succeed, assuming > 4h since any prior pet).
./rust/target/release/purrgotchi pet

# Immediate second pet (should report cooldown).
./rust/target/release/purrgotchi pet

# Card should show a BOND row.
./rust/target/release/purrgotchi card | grep -i bond
```

Expected output patterns:
1. First call: `♥ Thalia purrs contentedly. Bond: X → Y (+15)`
2. Second call: `Thalia is already purring at you. (next pet in 3h 59m)` (or similar)
3. `grep -i bond`: prints the BOND row from the card.

- [ ] **Step 5: Commit**

```bash
git add rust/Cargo.toml CHANGELOG.md
git commit -m "v3.4.5: bond stat + pet command + HUD rearrangement"
```

- [ ] **Step 6: Verify all changes**

Run: `git log --oneline main..HEAD`
Expected: the 9 feature commits from Tasks 1–9 plus the version-bump commit from this task.

Run: `cd rust && cargo test && cargo clippy -- -D warnings`
Expected: green.

---

## Self-Review Results

**Spec coverage** (each requirement in the spec → task that implements it):
- Tuning constants → Task 2 (Step 2 declares them).
- State schema (bond + last_pet_at + serde defaults + base init) → Task 1.
- `effective_bond` pure function with lazy decay → Task 2.
- `PetResult` enum + `try_pet` (cooldown, saturation, applies to effective) → Task 3.
- Happy-mood override in `compute_mood` → Task 4.
- `hooks::pet` with formatted output (applied / capped / cooldown) → Task 5.
- `main.rs` wiring + usage line → Task 6.
- Card BOND row + divider → Task 7.
- HUD H6 col 3 → BOND bar → Task 8.
- HUD H1 rail → stars + PEAK shift to col-3 center → Task 9.
- Version bump + CHANGELOG → Task 10.

**Placeholder scan:** No "TBD", "TODO", "implement later", or "similar to task N" without inlined code. Every code step shows the full code to write.

**Type consistency:**
- `PetResult` fields (`old: u8`, `new: u8`, `capped: bool`, `wait_secs: u64`) consistent between declaration (Task 3) and matchers (Tasks 3, 5).
- `effective_bond(bond: u8, last_pet_at: u64, now: u64) -> u8` consistent across callers (Task 2 def, Task 7 card, Task 8 statusline).
- `compute_mood(f64, i64, i64) -> &'static str` — three-arg signature consistent between Task 4 definition and Task 4 Step 5 caller update.
- `BOND_HAPPY_MOOD_WINDOW_SEC: i64` — matches the `i64` type of `last_pet_delta_sec` in `compute_mood`.

No gaps found.

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-04-23-c5-c1-bond-and-pet.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

**Which approach?**
