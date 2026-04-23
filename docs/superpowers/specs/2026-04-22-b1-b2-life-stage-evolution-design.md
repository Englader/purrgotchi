# B1 + B2 — Life-stage sprite evolution & level-up celebration

_Design doc, v3.4.3. Phase 2 opener (master plan items B1 + B2 bundled)._
_Date: 2026-04-22_

---

## 1. Overview

Purrgotchi's kitten currently renders via a single-dimension sprite lookup `(rarity, mood, frame)`. v3.4.3 introduces a **life stage** as a fourth dimension and a **celebration overlay** that briefly overrides sprite + speech after level-ups.

Life stage is derived from `level`:

| Stage | Level range |
|---|---|
| kitten | 1–9 |
| young cat | 10–49 |
| adult | 50+ |

With the existing formula `level = 1 + total_tool_calls / 50`, young-cat unlocks at 450 tool calls and adulthood at 2450. Pacing is intentionally slow — adulthood should feel earned across weeks or months of real usage.

B1 delivers the stage-aware sprite matrix. B2 delivers the celebration moment. They ship together because the stage transition (level 10 and level 50) is the biggest celebration in B2; splitting them would either ship B1 with no fanfare or ship B2 without the climactic moments.

**Out of scope for this release:** name color by rarity (B3), breed-specific sprites, LLM-generated celebration speech (Phase 3).

---

## 2. Data model

Two fields added to `State` in `rust/src/state.rs`. Both use `#[serde(default)]` so v3.4.2 state files deserialize unchanged — no schema version bump, no migration, no rehatch.

```rust
pub struct State {
    // ... existing fields
    #[serde(default)]
    pub levelup_renders_remaining: u8,
    #[serde(default)]
    pub celebration_kind: CelebrationKind,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum CelebrationKind {
    #[default]
    None,
    LevelUp,
    EvolvingYoung,   // kitten → young cat
    EvolvingAdult,   // young cat → adult
}
```

Life stage is **derived**, not stored:

```rust
pub fn life_stage(level: u64) -> &'static str {
    match level {
        1..=9 => "kitten",
        10..=49 => "young",
        _ => "adult",
    }
}
```

**Overlap rule:** if a level-up fires while a celebration is already active, the new event overwrites `celebration_kind` and resets `levelup_renders_remaining`. Stage transitions overwrite regular level-ups; the reverse overwrites but keeps kind=LevelUp.

---

## 3. Sprite taxonomy

### File naming

Extend the existing flat `sprites/` convention from `{rarity}_{mood}_{frame}.txt` to `{stage}_{rarity}_{mood}_{frame}.txt`. Directory stays flat.

### File matrix

| Stage | Rarities | Mood/frame combos per cell | Files |
|---|---|---|---|
| kitten | common, uncommon, rare, epic, legendary | 11 | 55 |
| young | common, uncommon, rare, epic, legendary | 11 | 55 |
| adult | common, uncommon, rare, epic, legendary | 11 | 55 |

The 11 mood/frame combos per cell: `calm_0`, `calm_1`, `happy_0`, `happy_1`, `alert_0`, `alert_1`, `panic_0`, `panic_1`, `hungry_0`, `sleep_0`, `sleep_1` — identical to Phase 1's per-rarity set.

**Mapping existing 55 sprites:** the v3.4.2 sprite set becomes the **young cat** stage. Kitten and adult sprites are newly authored. No existing art is thrown away.

**Rename pass:** all 55 existing files get renamed from `{rarity}_{mood}_{frame}.txt` to `young_{rarity}_{mood}_{frame}.txt`. For common-tier files that currently have no rarity prefix (`calm_0.txt` etc.), they become `young_common_{mood}_{frame}.txt`.

### Celebration sprites (tier-agnostic)

| Mood pool | Files |
|---|---|
| `levelup` | `levelup_0.txt`, `levelup_1.txt` |
| `evolving` | `evolving_0.txt`, `evolving_1.txt` |

Celebrations override the whole mood matrix while active, so only one sprite-per-frame is needed — no rarity or stage variants.

### Total new sprite files: **114**
- 110 stage sprites (55 kitten + 55 adult)
- 4 celebration sprites

### Lookup function

Signature changes in `rust/src/sprites.rs`:

```rust
pub fn get_sprite_lines(stage: &str, rarity: &str, mood: &str, frame: u8) -> Vec<String>;
pub fn get_celebration_sprite(kind: CelebrationKind, frame: u8) -> Vec<String>;
```

Fallback chain for `get_sprite_lines`:

1. Exact `(stage, rarity, mood, frame)`
2. Same `(stage, rarity, mood)`, frame 0
3. Same `(stage, rarity)`, `mood = "calm"`, frame 0
4. Same `stage`, `rarity = "common"`, same mood, frame 0
5. `young_common_calm_0.txt` (guaranteed-exists ultimate fallback)

`get_celebration_sprite` returns the requested frame, falling back to frame 0 if missing.

---

## 4. Level-up detection

In `rust/src/hooks.rs`, current logic is:

```rust
s.level = 1 + s.total_tool_calls / 50;
```

Replace with:

```rust
let prev_level = s.level;
s.level = 1 + s.total_tool_calls / 50;
if s.level > prev_level {
    let prev_stage = life_stage(prev_level);
    let new_stage = life_stage(s.level);
    s.celebration_kind = match (prev_stage, new_stage) {
        ("kitten", "young") => CelebrationKind::EvolvingYoung,
        ("young", "adult")  => CelebrationKind::EvolvingAdult,
        _                   => CelebrationKind::LevelUp,
    };
    s.levelup_renders_remaining = match s.celebration_kind {
        CelebrationKind::LevelUp => 3,
        _                        => 6,
    };
}
```

Note: `s.level > prev_level` (not `!=`) defends against the pathological case where `total_tool_calls` could ever decrease.

---

## 5. Rendering

In `rust/src/render.rs` (or `statusline.rs`, wherever sprite/speech selection happens today):

```rust
let sprite_lines = if s.celebration_kind != CelebrationKind::None {
    get_celebration_sprite(s.celebration_kind, frame)
} else {
    let stage = life_stage(s.level);
    get_sprite_lines(stage, &s.rarity, &mood, frame)
};

let speech = if s.celebration_kind != CelebrationKind::None {
    compute_celebration_speech(s.celebration_kind, now_secs)
} else {
    compute_speech(&mood, now_secs)
};
```

**Counter decrement after render:**

```rust
if s.levelup_renders_remaining > 0 {
    s.levelup_renders_remaining -= 1;
    if s.levelup_renders_remaining == 0 {
        s.celebration_kind = CelebrationKind::None;
    }
    save_atomic(&s);  // 3-6 writes per celebration is acceptable
}
```

Disk write per decrement is deliberate. A celebration produces 3 or 6 extra atomic JSON writes, invisible overhead. The alternative (wall-clock TTL) changes the UX: a backgrounded terminal would miss the celebration entirely. Render-counter guarantees the promise "you see it 3 times."

---

## 6. Speech

Add three new static pools to `rust/src/speech.rs` plus a dispatch function:

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

pub fn compute_celebration_speech(kind: CelebrationKind, now_secs: i64) -> &'static str {
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

Phrases may be polished during implementation; the structure is what this design locks.

---

## 7. Card display

In `rust/src/card.rs`, add a "Stage" line near the breed/rarity row. Small cosmetic addition; the existing box-drawing layout is preserved.

Example:

```
Luna the Shiny Chimera
Legendary ★★★★★  ·  Stage: young cat
```

Stage label strings: `"kitten"`, `"young cat"`, `"adult cat"` (note the space in "young cat" — the stage identifier in sprite lookup is `"young"`, but the display string is `"young cat"`).

---

## 8. Testing

### Unit tests (target: ~8 new, keeping all 36 existing green)

- `life_stage` boundary tests at levels 9, 10, 49, 50.
- `get_sprite_lines` fallback — `(kitten, legendary, panic, 1)` resolves; `(adult, nonsense, nonsense, 99)` returns the ultimate fallback without panic.
- Level-up detection — synthetic `total_tool_calls` increments across 50, 500, 2500 thresholds; assert `celebration_kind` is `LevelUp`, `EvolvingYoung`, `EvolvingAdult` with counter 3/6/6.
- Decrement — after N renders, counter reaches 0 and kind resets to `None`.
- Overlap — regular level-up during active `LevelUp` resets counter to 3; stage transition during `LevelUp` overwrites kind.
- Serde backwards-compat — deserialize a v3.4.2-shaped state.json, assert the two new fields default to zero/None.

### Manual smoke tests

- Temp-HOME hatch; use dev backdoor or direct state edit to set `total_tool_calls = 449`; trigger a tool call; confirm `EvolvingYoung` celebration renders for 6 ticks.
- Same at `total_tool_calls = 2449` for `EvolvingAdult`.
- Mid-range level-up (e.g., `total_tool_calls = 749 → 750`) shows plain `LevelUp` for 3 ticks.

---

## 9. Deliverables

| Category | Scope |
|---|---|
| New sprites | 114 files (55 kitten + 55 adult + 4 celebration) |
| Renamed sprites | 55 files gain `young_` prefix |
| Modified source | `state.rs`, `hooks.rs`, `render.rs` (or `statusline.rs`), `sprites.rs`, `speech.rs`, `card.rs` |
| New tests | ~8 unit tests |
| CHANGELOG | v3.4.3 entry |
| Cargo.toml | 3.4.2 → 3.4.3 |
| Tag | v3.4.3 |

### Implementation sequencing note

Sprite authoring is the dominant human-time cost. The implementation plan should separate Rust wiring (data model, detection, rendering, speech, tests) from sprite authoring so the wiring can be verified against placeholder sprites first, then the real art drops in. All sprites will be authored and approved in a single batch (user approves all at once, not in stage-by-stage gates).

---

## 10. Open questions

None blocking. The master plan's "Phase 2 — bond decay rate" question is for a later item (C5) and doesn't affect this release.
