# Life-Stage Sprite Evolution & Level-Up Celebration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship Purrgotchi v3.4.3 — adds a `life_stage` dimension to sprite selection (kitten/young/adult by level) and a 3–6-render celebration overlay that fires on level-up, with bigger ceremony on stage transitions (levels 10 and 50).

**Architecture:** A pure `life_stage(level)` helper derives stage from the existing `level` field (no new stored state for staging). Two new `State` fields (`levelup_renders_remaining: u8`, `celebration_kind: CelebrationKind`) drive the overlay via `#[serde(default)]` — no schema version bump. Level-up detection hooks into the existing `feed_inner` tool-call path. The sprite module refactors from 5 per-rarity lookup functions to a single flat `SPRITES` slice indexed by `(stage, rarity, mood, frame)`, plus a separate `CELEBRATIONS` slice for the 4 tier-agnostic celebration sprites.

**Tech Stack:** Rust 2024, `serde`, `serde_json`, `include_str!` for compile-time-embedded ASCII sprites.

**Spec:** `docs/superpowers/specs/2026-04-22-b1-b2-life-stage-evolution-design.md`

---

## File Structure

**Created (source):**
- None — all new code lives in existing modules.

**Created (sprite assets):** 114 new `.txt` files in `sprites/`:
- 55 `kitten_{rarity}_{mood}_{frame}.txt`
- 55 `adult_{rarity}_{mood}_{frame}.txt`
- 2 `levelup_{0,1}.txt`
- 2 `evolving_{0,1}.txt`

**Renamed:** 55 existing sprite files get `young_` prefix (and `common_` infix for the 11 currently-prefixless files):
- `calm_0.txt` → `young_common_calm_0.txt` (× 11 common files)
- `uncommon_calm_0.txt` → `young_uncommon_calm_0.txt` (× 11 uncommon)
- `rare_*` → `young_rare_*` (× 11)
- `epic_*` → `young_epic_*` (× 11)
- `legendary_*` → `young_legendary_*` (× 11)

**Modified:**
- `rust/src/state.rs` — add `CelebrationKind` enum, two `State` fields, `life_stage()` helper, serde back-compat test.
- `rust/src/sprites.rs` — full replacement: flat `SPRITES` slice, new `get_sprite_lines(stage, rarity, mood, frame)` signature, new `get_celebration_sprite(kind, frame)`.
- `rust/src/hooks.rs` — level-up detection in `feed_inner`, extracted to a pure `detect_celebration()` for unit tests.
- `rust/src/speech.rs` — add `LEVELUP`, `EVOLVING_YOUNG`, `EVOLVING_ADULT` pools and `compute_celebration_speech()`.
- `rust/src/statusline.rs` — branch sprite/speech selection on `celebration_kind`, decrement counter + save state when active.
- `rust/src/card.rs` — add "Stage: <label>" row.
- `rust/Cargo.toml` — version bump 3.4.2 → 3.4.3.
- `CHANGELOG.md` — v3.4.3 entry.

---

## Task Ordering Rationale

1. **Data model first (Tasks 1–3):** pure Rust additions, unit-testable, each compiles and ships green. No sprite files touched yet.
2. **Placeholder sprite files (Task 4):** create 114 empty `.txt` files so `include_str!` compiles after the rename.
3. **Sprite module refactor + rename (Task 5):** does the 55-file rename and replaces `sprites.rs`. One commit because either half alone breaks the build.
4. **Celebration lookup + speech (Tasks 6–7):** small additive units, each testable.
5. **Rendering wire-up (Task 8):** `statusline.rs` branches on celebration, saves state.
6. **Card stage line (Task 9):** cosmetic one-liner.
7. **Sprite authoring (Task 10):** the big manual art pass. Replaces placeholder content in 114 files.
8. **Version + CHANGELOG (Task 11):** release prep.
9. **Tag + push (Task 12):** user action, not automated.

---

## Task 1: Add `CelebrationKind` enum + serde back-compat fields

**Files:**
- Modify: `rust/src/state.rs:11-27` (struct), `rust/src/state.rs:30-46` (base), `rust/src/state.rs:96-136` (tests)

- [ ] **Step 1: Write the failing test**

Append inside the existing `#[cfg(test)] mod tests` block in `rust/src/state.rs`:

```rust
    #[test]
    fn v2_state_without_celebration_fields_defaults_clean() {
        // A v3.4.2-shaped state.json has no levelup_renders_remaining or
        // celebration_kind fields. serde(default) must make them zero/None.
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
        let s = parse_state(json.as_bytes()).expect("v3.4.2 shape should load");
        assert_eq!(s.levelup_renders_remaining, 0);
        assert_eq!(s.celebration_kind, CelebrationKind::None);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path rust/Cargo.toml v2_state_without_celebration_fields_defaults_clean`

Expected: FAIL — compile error ("cannot find type `CelebrationKind`" or "no field `levelup_renders_remaining`").

- [ ] **Step 3: Add the enum and fields**

In `rust/src/state.rs`, replace lines 11–27 (the `State` struct) with:

```rust
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CelebrationKind {
    #[default]
    None,
    LevelUp,
    EvolvingYoung,
    EvolvingAdult,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct State {
    pub version: u32,
    pub name: String,
    pub birth_time: u64,
    pub last_fed_at: u64,
    pub total_tool_calls: u64,
    pub coins: u64,
    pub level: u64,
    pub lifetime_cost_usd: f64,
    // identity (v2)
    pub breed: String,
    pub rarity: String,
    pub shiny: bool,
    pub stats: [u8; 5],       // [CURIOSITY, MISCHIEF, LAZINESS, AFFECTION, WISDOM]
    pub personality: String,  // blurb with {name} already substituted
    // celebration overlay (v3.4.3, additive — serde(default) for back-compat)
    #[serde(default)]
    pub levelup_renders_remaining: u8,
    #[serde(default)]
    pub celebration_kind: CelebrationKind,
}
```

And update `State::base` to initialize the new fields. Replace lines 30–46 with:

```rust
impl State {
    pub fn base(now: u64) -> Self {
        State {
            version: 2,
            name: "Purrgotchi".to_string(),
            birth_time: now,
            last_fed_at: now,
            total_tool_calls: 0,
            coins: 0,
            level: 1,
            lifetime_cost_usd: 0.0,
            breed: String::new(),
            rarity: String::new(),
            shiny: false,
            stats: [0u8; 5],
            personality: String::new(),
            levelup_renders_remaining: 0,
            celebration_kind: CelebrationKind::None,
        }
    }
}
```

- [ ] **Step 4: Run tests to verify green**

Run: `cargo test --manifest-path rust/Cargo.toml`

Expected: all existing tests + new `v2_state_without_celebration_fields_defaults_clean` pass.

- [ ] **Step 5: Commit**

```bash
git add rust/src/state.rs
git commit -m "state: add CelebrationKind and celebration fields (serde default)"
```

---

## Task 2: Add `life_stage()` helper

**Files:**
- Modify: `rust/src/state.rs` (append pure function + tests)

- [ ] **Step 1: Write failing tests**

Append inside `mod tests` in `rust/src/state.rs`:

```rust
    #[test]
    fn life_stage_kitten_boundaries() {
        assert_eq!(life_stage(1), "kitten");
        assert_eq!(life_stage(9), "kitten");
    }

    #[test]
    fn life_stage_young_boundaries() {
        assert_eq!(life_stage(10), "young");
        assert_eq!(life_stage(49), "young");
    }

    #[test]
    fn life_stage_adult_boundaries() {
        assert_eq!(life_stage(50), "adult");
        assert_eq!(life_stage(9_999), "adult");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path rust/Cargo.toml life_stage_`

Expected: FAIL — "cannot find function `life_stage`".

- [ ] **Step 3: Add the function**

Append to `rust/src/state.rs` (outside the `impl`, near `now_secs`):

```rust
pub fn life_stage(level: u64) -> &'static str {
    match level {
        1..=9 => "kitten",
        10..=49 => "young",
        _ => "adult",
    }
}
```

- [ ] **Step 4: Run tests to verify green**

Run: `cargo test --manifest-path rust/Cargo.toml life_stage_`

Expected: all three PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/src/state.rs
git commit -m "state: add life_stage(level) — kitten/young/adult thresholds"
```

---

## Task 3: Level-up celebration detection (`detect_celebration`)

**Files:**
- Modify: `rust/src/hooks.rs` (add pure helper + update `feed_inner`), tests live in `hooks.rs`

- [ ] **Step 1: Write failing tests**

Append a `#[cfg(test)] mod tests` block at the bottom of `rust/src/hooks.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
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
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path rust/Cargo.toml -p purrgotchi hooks::tests`

Expected: FAIL — `detect_celebration` not found.

- [ ] **Step 3: Implement `detect_celebration` and wire it into `feed_inner`**

Replace the entire contents of `rust/src/hooks.rs` with:

```rust
use std::fs;

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
```

- [ ] **Step 4: Run tests to verify green**

Run: `cargo test --manifest-path rust/Cargo.toml`

Expected: all existing tests + 7 new `detect_celebration` tests pass.

- [ ] **Step 5: Commit**

```bash
git add rust/src/hooks.rs
git commit -m "hooks: detect level-up celebrations and stage transitions"
```

---

## Task 4: Create the 114 placeholder sprite files

**Files:**
- Create: 114 new files in `sprites/` (empty placeholders — real art in Task 10)

Rationale: the `sprites.rs` refactor in Task 5 needs these files to exist so `include_str!` compiles. Placeholder content is fine; Task 10 replaces it.

- [ ] **Step 1: Generate all placeholder files**

Run from the repo root:

```bash
bash -c '
cd sprites
rarities=(common uncommon rare epic legendary)
# 11 mood-frame combos per (stage, rarity) cell
combos=(calm_0 calm_1 happy_0 happy_1 alert_0 alert_1 panic_0 panic_1 hungry_0 sleep_0 sleep_1)

for stage in kitten adult; do
  for rarity in "${rarities[@]}"; do
    for combo in "${combos[@]}"; do
      f="${stage}_${rarity}_${combo}.txt"
      if [ ! -f "$f" ]; then
        # 6-line placeholder so pad_sprite does not collapse the column
        printf "[%s %s %s]\n\n\n\n\n\n" "$stage" "$rarity" "$combo" > "$f"
      fi
    done
  done
done

for combo in levelup_0 levelup_1 evolving_0 evolving_1; do
  f="${combo}.txt"
  if [ ! -f "$f" ]; then
    printf "[%s]\n\n\n\n\n\n" "$combo" > "$f"
  fi
done
'
```

- [ ] **Step 2: Verify count**

Run: `ls sprites/kitten_*.txt sprites/adult_*.txt sprites/levelup_*.txt sprites/evolving_*.txt | wc -l`

Expected: `114`.

- [ ] **Step 3: Commit**

```bash
git add sprites/kitten_*.txt sprites/adult_*.txt sprites/levelup_*.txt sprites/evolving_*.txt
git commit -m "sprites: add 114 placeholder files for kitten/adult stages + celebrations"
```

---

## Task 5: Rename existing sprites and rewrite `sprites.rs`

**Files:**
- Rename: 55 files in `sprites/` get `young_` prefix (and `common_` infix for prefixless ones)
- Modify: `rust/src/sprites.rs` (full replacement)
- Modify: `rust/src/statusline.rs:108` (call-site — add stage argument)

This task MUST land as one commit — the rename and the `sprites.rs` rewrite depend on each other for the build to pass.

- [ ] **Step 1: Rename common-tier files (add `young_common_` prefix)**

Run:

```bash
bash -c '
cd sprites
for f in calm_0.txt calm_1.txt happy_0.txt happy_1.txt alert_0.txt alert_1.txt panic_0.txt panic_1.txt hungry_0.txt sleep_0.txt sleep_1.txt; do
  git mv "$f" "young_common_$f"
done
'
```

- [ ] **Step 2: Rename uncommon/rare/epic/legendary files (add `young_` prefix)**

```bash
bash -c '
cd sprites
for rarity in uncommon rare epic legendary; do
  for suffix in calm_0 calm_1 happy_0 happy_1 alert_0 alert_1 panic_0 panic_1 hungry_0 sleep_0 sleep_1; do
    git mv "${rarity}_${suffix}.txt" "young_${rarity}_${suffix}.txt"
  done
done
'
```

- [ ] **Step 3: Verify rename count**

Run: `ls sprites/young_*.txt | wc -l`

Expected: `55`.

- [ ] **Step 4: Replace `rust/src/sprites.rs` with the stage-aware version**

Write the full new `rust/src/sprites.rs`:

```rust
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
static CELEBRATIONS: &[(CelebrationKind, u8, &'static str)] = &[
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
```

- [ ] **Step 5: Update `statusline.rs:108` to pass the stage**

In `rust/src/statusline.rs`, change line 108 from:

```rust
    let sprite_lines = get_sprite_lines(&rarity, mood, frame);
```

to:

```rust
    let stage = crate::state::life_stage(level);
    let sprite_lines = get_sprite_lines(stage, &rarity, mood, frame);
```

- [ ] **Step 6: Run full test suite + build**

Run: `cargo test --manifest-path rust/Cargo.toml`
Then: `cargo build --manifest-path rust/Cargo.toml --release`

Expected: all tests pass; release build succeeds.

- [ ] **Step 7: Commit**

```bash
git add sprites/ rust/src/sprites.rs rust/src/statusline.rs
git commit -m "sprites: add stage dimension, rename current set to young_*, flat lookup table"
```

---

## Task 6: Celebration speech pools

**Files:**
- Modify: `rust/src/speech.rs` (append pools + dispatch function + tests)

- [ ] **Step 1: Write failing tests**

Append to `rust/src/speech.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CelebrationKind;

    #[test]
    fn levelup_returns_from_levelup_pool() {
        let phrase = compute_celebration_speech(CelebrationKind::LevelUp, 0);
        assert!(LEVELUP.contains(&phrase));
    }

    #[test]
    fn evolving_young_returns_from_young_pool() {
        let phrase = compute_celebration_speech(CelebrationKind::EvolvingYoung, 0);
        assert!(EVOLVING_YOUNG.contains(&phrase));
    }

    #[test]
    fn evolving_adult_returns_from_adult_pool() {
        let phrase = compute_celebration_speech(CelebrationKind::EvolvingAdult, 0);
        assert!(EVOLVING_ADULT.contains(&phrase));
    }

    #[test]
    fn none_returns_fallback_mrow() {
        assert_eq!(compute_celebration_speech(CelebrationKind::None, 0), "mrow");
    }

    #[test]
    fn celebration_speech_rotates_over_time() {
        // 30s bucket rotation — same bucket gives same phrase
        let a = compute_celebration_speech(CelebrationKind::LevelUp, 0);
        let b = compute_celebration_speech(CelebrationKind::LevelUp, 15);
        assert_eq!(a, b);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path rust/Cargo.toml speech::tests::`

Expected: FAIL — `LEVELUP`, `EVOLVING_YOUNG`, `EVOLVING_ADULT`, and `compute_celebration_speech` undefined.

- [ ] **Step 3: Add pools and dispatch**

Insert after the existing `SLEEP` constant (around line 47) in `rust/src/speech.rs`:

```rust
const LEVELUP: &[&str] = &[
    "mrow! level up!",
    "+1 purr",
    "feeling stronger",
    "*stretches proudly*",
    "level +1 nya~",
];

const EVOLVING_YOUNG: &[&str] = &[
    "not a kitten anymore...",
    "mrrreow! i've grown!",
    "look at me now!",
    "bigger paws!",
    "young cat arc ✦",
];

const EVOLVING_ADULT: &[&str] = &[
    "all grown up",
    "dignified now, obviously",
    "an adult cat regards you",
    "fully realized meow",
    "adult arc unlocked",
];
```

And append after `compute_speech`:

```rust
pub fn compute_celebration_speech(
    kind: crate::state::CelebrationKind,
    now_secs: i64,
) -> &'static str {
    use crate::state::CelebrationKind;
    let pool: &[&str] = match kind {
        CelebrationKind::LevelUp       => LEVELUP,
        CelebrationKind::EvolvingYoung => EVOLVING_YOUNG,
        CelebrationKind::EvolvingAdult => EVOLVING_ADULT,
        CelebrationKind::None          => return "mrow",
    };
    let idx = ((now_secs.max(0) / 30) as usize) % pool.len();
    pool[idx]
}
```

- [ ] **Step 4: Run tests to verify green**

Run: `cargo test --manifest-path rust/Cargo.toml speech::tests::`

Expected: all 5 new tests pass.

- [ ] **Step 5: Commit**

```bash
git add rust/src/speech.rs
git commit -m "speech: add levelup and evolving pools + dispatch"
```

---

## Task 7: Wire celebration overlay into statusline rendering

**Files:**
- Modify: `rust/src/statusline.rs` (branch sprite+speech on celebration_kind, decrement counter + save)

- [ ] **Step 1: Read the current statusline.rs state-load + sprite/speech section**

The current flow (lines 56–108):

```rust
    let st = crate::hatch::load_or_hatch();
    let (coins, level, last_fed_at, kitten_name, rarity, shiny) =
        (st.coins, st.level, st.last_fed_at, st.name.clone(), st.rarity.clone(), st.shiny);
    // ...
    let speech = compute_speech(mood, now);
    // ...
    let stage = crate::state::life_stage(level);
    let sprite_lines = get_sprite_lines(stage, &rarity, mood, frame);
```

We need to: load `st` mutably, branch on `st.celebration_kind`, and if active, override sprite + speech AND decrement + save.

- [ ] **Step 2: Update the imports at the top of statusline.rs**

In `rust/src/statusline.rs`, change the existing imports:

```rust
use crate::speech::compute_speech;
use crate::sprites::get_sprite_lines;
```

to:

```rust
use crate::speech::{compute_celebration_speech, compute_speech};
use crate::sprites::{get_celebration_sprite, get_sprite_lines};
use crate::state::{self, CelebrationKind};
```

(The existing unused-qualified `crate::state::life_stage` call from Task 5 now resolves through this `state` import too — you can simplify `crate::state::life_stage(level)` to `state::life_stage(level)` but it's optional.)

- [ ] **Step 3: Replace the state-load block and sprite/speech selection**

Replace lines 56–58 (current `let st = ...`):

```rust
    let st = crate::hatch::load_or_hatch();
    let (coins, level, last_fed_at, kitten_name, rarity, shiny) =
        (st.coins, st.level, st.last_fed_at, st.name.clone(), st.rarity.clone(), st.shiny);
```

with:

```rust
    let mut st = crate::hatch::load_or_hatch();
    let (coins, level, last_fed_at, kitten_name, rarity, shiny) =
        (st.coins, st.level, st.last_fed_at, st.name.clone(), st.rarity.clone(), st.shiny);
    let celebration_kind = st.celebration_kind;
```

Then replace line 76 (the `compute_speech` call):

```rust
    let speech = compute_speech(mood, now);
```

with:

```rust
    let speech = if celebration_kind != CelebrationKind::None {
        compute_celebration_speech(celebration_kind, now)
    } else {
        compute_speech(mood, now)
    };
```

Then replace the stage+sprite lookup (the two lines introduced in Task 5):

```rust
    let stage = crate::state::life_stage(level);
    let sprite_lines = get_sprite_lines(stage, &rarity, mood, frame);
```

with:

```rust
    let sprite_lines = if celebration_kind != CelebrationKind::None {
        get_celebration_sprite(celebration_kind, frame)
    } else {
        let stage = state::life_stage(level);
        get_sprite_lines(stage, &rarity, mood, frame)
    };
```

- [ ] **Step 4: Add the counter decrement + save at the end of `run()`**

In `rust/src/statusline.rs`, find the end of the `run()` function (right before the `if debug::enabled()` block near line 281). Insert **before** the debug block:

```rust
    // Celebration counter: decrement per render, clear kind when done.
    // Writes state once per active-celebration render (3-6 writes total per event).
    if st.levelup_renders_remaining > 0 {
        st.levelup_renders_remaining -= 1;
        if st.levelup_renders_remaining == 0 {
            st.celebration_kind = CelebrationKind::None;
        }
        let _ = state::save_atomic(&st);
    }
```

- [ ] **Step 5: Run tests + release build**

Run: `cargo test --manifest-path rust/Cargo.toml`
Then: `cargo build --manifest-path rust/Cargo.toml --release`

Expected: all tests green, release build succeeds.

- [ ] **Step 6: Manual smoke test**

Run the following to simulate a kitten→young transition and verify the celebration renders:

```bash
# Temp HOME isolation
export PURR_HOME_BACKUP="$HOME"
export HOME="/tmp/purr-smoke-$$"
mkdir -p "$HOME"

# Fresh hatch, then manually mutate state to be just below level 10
rust/target/release/purrgotchi.exe hatch || true
# Edit state.json: set total_tool_calls=449 so next feed pushes level from 9 -> 10
sed -i 's/"total_tool_calls": [0-9]*/"total_tool_calls": 449/' "$HOME/.claude/purrgotchi/state.json"
sed -i 's/"level": [0-9]*/"level": 9/' "$HOME/.claude/purrgotchi/state.json"

# Trigger a feed (simulates a tool call): pipe empty JSON to statusline — no, use feed hook path
rust/target/release/purrgotchi.exe feed  # or whichever subcommand invokes feed_inner

# Check state shows celebration active
cat "$HOME/.claude/purrgotchi/state.json" | grep -E "(levelup|celebration)"

# Run statusline 6 times — confirm counter decrements to 0
for i in 1 2 3 4 5 6 7; do
  echo '{}' | rust/target/release/purrgotchi.exe statusline >/dev/null
done
cat "$HOME/.claude/purrgotchi/state.json" | grep -E "(levelup|celebration)"
# Expected: levelup_renders_remaining = 0, celebration_kind = "None"

# Cleanup
rm -rf "$HOME"
export HOME="$PURR_HOME_BACKUP"
unset PURR_HOME_BACKUP
```

Expected: after hatch, the `feed` invocation sets `levelup_renders_remaining=6` and `celebration_kind="EvolvingYoung"`. After 6 statusline renders, both return to 0/None.

(Exact subcommand names depend on your `main.rs` argv routing — use whichever subcommands already exist for feed and statusline.)

- [ ] **Step 7: Commit**

```bash
git add rust/src/statusline.rs
git commit -m "statusline: wire celebration overlay — override sprite/speech, decrement counter"
```

---

## Task 8: Add "Stage" line to the card

**Files:**
- Modify: `rust/src/card.rs` (add stage row after shiny/born row, before divider)

- [ ] **Step 1: Write failing test**

Append at the bottom of `rust/src/card.rs` (create `#[cfg(test)] mod tests` if none exists):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(level: u64) -> State {
        let mut s = State::base(1000);
        s.name = "Luna".to_string();
        s.breed = "Tabby".to_string();
        s.rarity = "common".to_string();
        s.level = level;
        s.stats = [50, 50, 50, 50, 50];
        s.personality = "A chill cat.".to_string();
        s
    }

    #[test]
    fn card_shows_kitten_stage_at_low_level() {
        let out = render(&make_state(3));
        assert!(out.contains("Stage: kitten"), "expected kitten stage in card, got:\n{out}");
    }

    #[test]
    fn card_shows_young_cat_stage_at_middle_level() {
        let out = render(&make_state(25));
        assert!(out.contains("Stage: young cat"), "expected young cat stage in card, got:\n{out}");
    }

    #[test]
    fn card_shows_adult_cat_stage_at_high_level() {
        let out = render(&make_state(100));
        assert!(out.contains("Stage: adult cat"), "expected adult cat stage in card, got:\n{out}");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path rust/Cargo.toml card::tests::card_shows`

Expected: FAIL — no "Stage:" string in output.

- [ ] **Step 3: Add a `stage_label` helper and a new row in `render`**

In `rust/src/card.rs`, at the top add:

```rust
use crate::state::life_stage;
```

Then add a helper above `pub fn run`:

```rust
fn stage_label(level: u64) -> &'static str {
    match life_stage(level) {
        "kitten" => "kitten",
        "young"  => "young cat",
        _        => "adult cat",
    }
}
```

Then inside `pub fn render(s: &State) -> String` — after the shiny/born row block (lines 42–49 in the current file), **before** the divider at line 52 — insert:

```rust
    // Stage row — sits between the identity rows and the sprite divider.
    let stage_row: String = format!("Stage: {}", stage_label(s.level))
        .chars()
        .take(inner_content)
        .collect();
    writeln!(out, "║  {:<width$}║", stage_row, width = inner_content).unwrap();
```

- [ ] **Step 4: Run tests to verify green**

Run: `cargo test --manifest-path rust/Cargo.toml`

Expected: all existing + 3 new card tests pass.

- [ ] **Step 5: Commit**

```bash
git add rust/src/card.rs
git commit -m "card: show life stage (kitten / young cat / adult cat)"
```

---

## Task 9: Sprite authoring — replace 114 placeholders with real ASCII art

**Files:**
- Modify: 114 sprite files in `sprites/` (kitten_*, adult_*, levelup_*, evolving_*)

This task is the largest by human-hours. It's a single commit because the user wants to approve all sprites at once (confirmed during brainstorming). No tests — the `cargo test` suite only confirms files exist and load, not that the art looks good. Visual review is by the user.

- [ ] **Step 1: Author 55 kitten sprites**

For each `(rarity, mood, frame)` combination, replace the placeholder content of `sprites/kitten_{rarity}_{mood}_{frame}.txt` with 6 lines of ASCII kitten art. The 11 mood/frame combos per rarity are identical to the existing young-cat set:

```
calm_0, calm_1,
happy_0, happy_1,
alert_0, alert_1,
panic_0, panic_1,
hungry_0,
sleep_0, sleep_1
```

Each file must be **exactly 6 non-empty lines** (or lines padded with spaces) so the sprite column in the statusline doesn't collapse. Look at `sprites/young_common_calm_0.txt` (the renamed original `calm_0.txt`) for the expected format and dimensions (~16 chars wide, 6 lines tall).

Art direction: kittens are smaller, rounder, cuter. Common kittens look generic; rare/epic/legendary kittens hint at their rarity (e.g., legendary kitten has cosmic/magical traits visible even at kitten size).

- [ ] **Step 2: Author 55 adult sprites**

Same process for `sprites/adult_{rarity}_{mood}_{frame}.txt`. Adult cats are larger, more dignified, with visible markers of their rarity (bigger mane, magical aura, etc.).

- [ ] **Step 3: Author 4 celebration sprites**

- `sprites/levelup_0.txt` and `sprites/levelup_1.txt`: celebratory poses with sparkles/stars around a neutral-rarity cat silhouette. Tier-agnostic — appears for all kittens regardless of rarity when a level-up happens.
- `sprites/evolving_0.txt` and `sprites/evolving_1.txt`: dramatic transformation poses (glowing outline, rays of light, etc.). Appears for the kitten→young and young→adult transitions.

All 4 files: exactly 6 lines, ~16 chars wide.

- [ ] **Step 4: Build + test to confirm nothing broke**

Run: `cargo test --manifest-path rust/Cargo.toml`
Then: `cargo build --manifest-path rust/Cargo.toml --release`

Expected: all green. (Art doesn't affect test results, only file presence does.)

- [ ] **Step 5: Visual smoke test**

```bash
export PURR_HOME_BACKUP="$HOME"
export HOME="/tmp/purr-smoke-$$"
mkdir -p "$HOME"

# Hatch as a kitten (fresh)
rust/target/release/purrgotchi.exe hatch
rust/target/release/purrgotchi.exe card   # visually confirm kitten stage + sprite

# Force young cat
sed -i 's/"level": [0-9]*/"level": 25/' "$HOME/.claude/purrgotchi/state.json"
rust/target/release/purrgotchi.exe card   # young cat stage + sprite

# Force adult
sed -i 's/"level": [0-9]*/"level": 80/' "$HOME/.claude/purrgotchi/state.json"
rust/target/release/purrgotchi.exe card   # adult cat stage + sprite

# Cleanup
rm -rf "$HOME"
export HOME="$PURR_HOME_BACKUP"
unset PURR_HOME_BACKUP
```

Expected: three visually distinct cards corresponding to the three life stages.

- [ ] **Step 6: Commit**

```bash
git add sprites/
git commit -m "sprites: author kitten, adult, and celebration art (114 files)"
```

---

## Task 10: Version bump + CHANGELOG entry

**Files:**
- Modify: `rust/Cargo.toml:3` (version)
- Modify: `CHANGELOG.md:8-33` (add v3.4.3 section above v3.4.2)

- [ ] **Step 1: Bump version**

In `rust/Cargo.toml`, change line 3 from:

```toml
version = "3.4.2"
```

to:

```toml
version = "3.4.3"
```

- [ ] **Step 2: Rebuild to update Cargo.lock**

Run: `cargo build --manifest-path rust/Cargo.toml --release`

Expected: succeeds; `rust/Cargo.lock` updates.

- [ ] **Step 3: Add CHANGELOG entry**

In `CHANGELOG.md`, insert immediately after the `## [Unreleased]` block (i.e., before the `## [3.4.2] - 2026-04-22` heading):

```markdown
## [3.4.3] - 2026-04-22

### Added

- **Life-stage sprite evolution**. Kittens now grow through three life stages driven by `level`: `kitten` (levels 1–9), `young cat` (10–49), and `adult cat` (50+). Every rarity tier has its own sprite set for each stage — 165 sprites in total, of which 114 are new for this release. The existing v3.4.2 art becomes the young-cat stage; kittens are smaller and rounder, adult cats are larger and more dignified.
- **Level-up celebration overlay**. When `level` increments, the statusline overrides sprite and speech with a celebratory `levelup_*` sprite + phrase from the new `LEVELUP` speech pool for the next 3 renders. Stage transitions (level 9→10 and 49→50) use a distinct `evolving_*` sprite and a stage-specific phrase pool (`EVOLVING_YOUNG` / `EVOLVING_ADULT`) for 6 renders — a bigger moment for the bigger milestone.
- **`Stage: <label>` row on the card**. `purrgotchi card` now shows the life stage alongside breed and rarity.

### Changed

- **Sprite module refactored** to a flat `(stage, rarity, mood, frame)` table with a single lookup function, replacing the 5 per-rarity match functions. Fallback chain still defends against missing cells; ultimate fallback is `young_common_calm_0`.
- **Existing 55 sprite files renamed** with a `young_` prefix (and `common_` infix for the 11 previously-prefixless common-tier files) — e.g., `calm_0.txt` → `young_common_calm_0.txt`. No art changed; just filenames.

### Backwards compatibility

- **No state schema bump**. The two new fields on `State` (`levelup_renders_remaining`, `celebration_kind`) use `#[serde(default)]`, so v3.4.2 `state.json` files deserialize cleanly with zero-valued defaults. No rehatch, no progress loss.
```

- [ ] **Step 4: Commit**

```bash
git add rust/Cargo.toml rust/Cargo.lock CHANGELOG.md
git commit -m "v3.4.3: life-stage sprite evolution + level-up celebration"
```

---

## Task 11: Final verification and tag

**Files:** no file changes — verification + git actions.

- [ ] **Step 1: Run the full pipeline one last time**

```bash
cargo test --manifest-path rust/Cargo.toml
cargo clippy --manifest-path rust/Cargo.toml --release -- -D warnings
cargo build --manifest-path rust/Cargo.toml --release
```

Expected: all tests pass, zero clippy warnings, release binary builds.

- [ ] **Step 2: Confirm binary version**

Run: `rust/target/release/purrgotchi.exe --version` (or however the binary reports version — if there's no version flag, just note that the build came from a 3.4.3 Cargo.toml).

Expected: `3.4.3`.

- [ ] **Step 3: Create annotated tag**

```bash
git tag -a v3.4.3 -m "v3.4.3 — life-stage sprite evolution + level-up celebration (B1+B2)"
```

- [ ] **Step 4: Push to remote** (user confirms first per preferences)

```bash
git push origin main
git push origin v3.4.3
```

- [ ] **Step 5: Update the master plan progress log**

In `docs/superpowers/plans/kitten-mechanics-master-plan.md`, under the Phase 2 entry, add:

```markdown
  - [x] B1 + B2 bundle (v3.4.3) — life-stage sprite evolution + level-up celebration
```

Replacing the `B/C/D items TBD per iteration` line or adding above it.

Commit:

```bash
git add docs/superpowers/plans/kitten-mechanics-master-plan.md
git commit -m "master plan: mark B1+B2 (v3.4.3) complete"
git push origin main
```

---

## Self-Review Checklist

1. **Spec coverage:** every section of the spec maps to one or more tasks — Section 2 (data model) → Task 1; life_stage helper → Task 2; Section 4 (detection) → Task 3; Section 3 (sprite taxonomy) → Tasks 4+5; Section 6 (speech) → Task 6; Section 5 (rendering) → Task 7; Section 7 (card) → Task 8; sprite authoring → Task 9; deliverables (CHANGELOG/version) → Task 10; tag → Task 11. No gaps.
2. **Placeholder scan:** no TBD/TODO/"add appropriate error handling"/etc. Every code block is complete.
3. **Type consistency:** `CelebrationKind` enum variants (`None`, `LevelUp`, `EvolvingYoung`, `EvolvingAdult`) used identically across Tasks 1, 3, 5, 6, 7, 8. Field names (`levelup_renders_remaining`, `celebration_kind`) consistent. Function signatures (`detect_celebration`, `life_stage`, `get_sprite_lines`, `get_celebration_sprite`, `compute_celebration_speech`) match between definition and call sites.

---

## Notes for the executor

- **Testing note on `statusline.rs`:** no unit tests target `statusline::run` because it depends on stdin/stdout and filesystem side effects. Its behavior is verified by the manual smoke test in Task 7 and by the 36 existing tests proving the underlying pieces still work.
- **"Save on every render during celebration" is deliberate** — see spec section 5. 3–6 extra writes per celebration is acceptable overhead for deterministic render-count behavior.
- **Placeholder sprites must produce 6 lines** or the sprite column in the statusline will collapse. The placeholder command in Task 4 writes 6 lines via `printf "...\n\n\n\n\n\n"`.
- **Cargo manifest path:** all `cargo` commands in this plan use `--manifest-path rust/Cargo.toml` because the binary crate lives at `rust/`, not the repo root.
- **Windows build:** binary extension is `.exe`. Smoke test scripts in the plan assume a Bash-compatible shell (Git Bash / WSL on Windows).
