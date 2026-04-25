# v3.4.8 — D-group Events Bundle

_Brainstormed: 2026-04-25 — successor to v3.4.7 (lifetime cost accumulator)_

## Goal

Bring the kitten alive with **five event-driven reactions** that fire on top of the existing mood/celebration system. The kitten should feel like a character that notices what you're doing — long sessions, milestones, anniversaries, returns from a break, and the occasional lucky drop.

This bundle implements all five Group D items from the master plan in a single release because they share the same one-shot celebration plumbing already established by B2 (`CelebrationKind` + `levelup_renders_remaining`).

## Why bundled

- All five fit the existing `CelebrationKind` enum + N-render decrement model (only D3 birthday is render-time, but it slots into the same statusline dispatch).
- Single state migration covers all three new fields (additive, `serde(default)`, no version bump).
- Per-item ship cadence would mean five round-trips through brainstorm/spec/plan/review for what's effectively one architectural pattern reused five ways.

## Non-goals

- LLM-generated speech for these events (deferred to Phase 3).
- Per-event sprite art — all new variants reuse the existing `LevelUp` celebration sprite. Bespoke birthday/milestone/streak sprites deferred.
- Cross-session streak tracking. Streaks are scoped to a single turn.
- Multi-kitten or per-project leaderboards.
- Configurable thresholds — milestone numbers, streak length, drop probability are all compile-time constants.

---

## 1. The five events

| ID | Trigger | Fires from | Visible result |
|---|---|---|---|
| **D1 Streak** | 5 tool calls within a single turn (no `UserPromptSubmit` between) | `feed()` | One-shot speech (3 renders) |
| **D2 Milestones** | `total_tool_calls` crosses 100, 500, 1000, or any kilo-multiple ≥ 2000 | `feed()` | One-shot speech (6 renders) |
| **D3 Birthday** | Today's local month/day == kitten's birth month/day (and not the birth year itself) | `statusline::run` | Day-long ambient mode (sprite + speech) |
| **D4 Greeting** | `turn()` invoked after a 6+ hour gap from previous turn | `turn()` | One-shot speech (3 renders) |
| **D5 Random Drop** | 0.5% RNG roll on every tool call | `feed()` | +10 coins **always**; one-shot speech (3 renders) if not eclipsed |

---

## 2. State schema additions

All additive, all `serde(default)`, no version bump. Three new fields:

```rust
#[serde(default)]
pub streak_count: u8,                  // D1
#[serde(default)]
pub last_milestone_celebrated: u64,    // D2 watermark
#[serde(default)]
pub last_turn_at: u64,                 // D4
```

`State::base(now)` initializes all three to `0`.

`CelebrationKind` enum gains seven new variants (existing `None`, `LevelUp`, `EvolvingYoung`, `EvolvingAdult` retained):

```rust
Streak,
MilestoneCenturion,    // 100
MilestoneVeteran,      // 500
MilestoneMaster,       // 1000
MilestoneLegendary,    // 2000+ (any kilo-multiple ≥ 2000)
Greeting,              // D4 daily greeting
RandomDrop,            // D5 found coins
```

**Birthday is intentionally NOT a CelebrationKind variant** — it's a render-time mode computed from `birth_time` and now, not a stored one-shot.

---

## 3. Feed-hook dispatch (`feed_inner`)

After the existing `total_tool_calls += 1` / `coins += 1` / level update, run detection in this order:

```
1. Levelup detection (existing — no change)
2. Milestone detection (uses prev = total - 1, new = total — leverages the +1 increment invariant):
   - if prev < 100  && new >= 100   → MilestoneCenturion + watermark = 100
   - elif prev < 500  && new >= 500   → MilestoneVeteran   + watermark = 500
   - elif prev < 1000 && new >= 1000  → MilestoneMaster    + watermark = 1000
   - elif new >= 2000 && new % 1000 == 0 && new > watermark
                                     → MilestoneLegendary + watermark = new
   - The watermark is defense-in-depth (in case `total_tool_calls` is ever
     bumped by more than +1 in a future change). Under the current +1 invariant,
     each call can cross at most one threshold.
3. Streak detection:
   - streak_count += 1
   - if streak_count == 5 → Streak detected
   - (counter only resets in turn(); does NOT re-fire at 6, 7, 8...)
4. Random drop:
   - roll = lcg(now_secs() ^ total_tool_calls) → bool (0.5% true)
   - if true: coins += 10 (always applied, even if eclipsed)
            : RandomDrop detected
```

**Priority** (highest wins, sets `celebration_kind`):

```
Milestone > LevelUp/Evolving > Streak > RandomDrop
```

**Eclipsed but not lost**: D5's `+10 coins` is applied unconditionally. If a higher-priority celebration fires the same call, the user sees that celebration's speech but their wallet still got the +10. Likewise, the underlying `level` / `last_milestone_celebrated` writes happen regardless of which speech wins.

**Render counts** (`levelup_renders_remaining`):
- Milestone: **6** (significant — matches Evolving)
- Levelup / Evolving: **3 / 6** (existing)
- Streak: **3**
- RandomDrop: **3**

---

## 4. Turn-hook dispatch (`turn`)

Currently `turn()` only deletes the turn anchor file. After this bundle it does real work:

```
1. Load state (load_or_hatch).
2. Read previous last_turn_at; compute gap = now - last_turn_at.
3. Reset streak_count to 0.
4. Greeting check:
   - if last_turn_at == 0 → INITIALIZE only, do NOT fire (avoids welcome-back on day-zero hatch)
   - elif gap >= 6 * 3600 → set celebration_kind = Greeting (renders = 3)
5. last_turn_at = now.
6. Save state.
7. Delete turn anchor file (existing behavior).
```

**Greeting × Birthday collision**: greeting fires its 3 renders first, then once `levelup_renders_remaining` decrements to 0 the day-long birthday mode takes over for the rest of the day. (Decided in brainstorm: option B + ii.)

---

## 5. Render-time dispatch (`statusline::run`)

Current logic:

```
if celebration_kind != None:
    sprite = celebration_sprite(kind)
    speech = celebration_speech(kind)
else:
    sprite = mood_sprite(mood)
    speech = mood_speech(mood)
```

New logic adds a birthday tier in the middle:

```
if celebration_kind != None:
    sprite = celebration_sprite(kind)         // expanded with new variants
    speech = celebration_speech(kind)
elif is_birthday(birth_time, now):
    sprite = celebration_sprite(LevelUp)      // reused
    speech = birthday_speech(now)             // BIRTHDAY pool, 30s rotation
else:
    sprite = mood_sprite(mood)
    speech = mood_speech(mood)
```

### `is_birthday(birth_time: u64, now: u64) -> bool`

Pure function. Returns `true` iff:

- `(month, day)` of `birth_time` == `(month, day)` of `now` in **local time**, AND
- year of `now` != year of `birth_time` (so hatch day itself isn't called a birthday)

**Leap-year edge case**: a kitten born Feb 29 celebrates on Feb 28 in non-leap years. Hard-coded special case inside `is_birthday`.

Implementation note: reuse the same `local_hour()` Win32 / Unix dispatch already present in `statusline.rs` to derive local YMD; do not call `now_secs()` and treat it as UTC.

### Sprite mapping for new celebration variants

All new variants — `Streak`, `Greeting`, `RandomDrop`, all four `Milestone*` — **reuse the existing `LevelUp` sprite**. Same rationale as v3.4.6 reusing `happy` mood: avoid the ~44-file sprite explosion across rarity tiers (4 mood-frame combos × 11 sprite files × 4 tiers). Speech bubble does the differentiation.

---

## 6. Speech pools

New constants in `speech.rs`. **Target 5 phrases per pool** for consistency with existing pools and the 30s rotation cadence.

```rust
const STREAK: &[&str] = &[
    "5 in a row! purr-fect!",
    "combo unlocked",
    "tool chain go brrr",
    "*tail flick* nice run",
    "we're cooking",
];

const MILESTONE_CENTURION: &[&str] = &[ /* ~5 "100 calls!" themed */ ];
const MILESTONE_VETERAN:   &[&str] = &[ /* ~5 "500 calls!" themed */ ];
const MILESTONE_MASTER:    &[&str] = &[ /* ~5 "1000 calls!" themed */ ];
const MILESTONE_LEGENDARY: &[&str] = &[ /* ~5 generic long-haul phrases — used for any kilo-multiple ≥ 2000 */ ];

const BIRTHDAY: &[&str] = &[
    "happy purr-thday!",
    "🎂 paw-ty time",
    "year older, year wiser",
    "*birthday meow*",
    "another lap around the sun",
];

const GREETING: &[&str] = &[
    "welcome back!",
    "you came back!",
    "missed you",
    "*stretches awake*",
    "mrow! about time",
];

const RANDOM_DROP: &[&str] = &[
    "found a coin!",
    "what's this? +10!",
    "treasure!",
    "lucky paw moment",
    "*pounces on shiny*",
];
```

Exact wording is final-pass authoring during implementation; counts and names are locked.

`compute_celebration_speech` extends its `match` with the seven new variants. `compute_birthday_speech(now_secs)` is a new pure function using the same `(now/30) % len` rotation.

---

## 7. Random drop RNG

D5 needs a non-deterministic roll on each `feed()`. Reuse the LCG primitive used at hatch (in `identity.rs`, or a small private helper if no public `lcg_step` is exposed — keep it internal to `events.rs` if so), seeded differently per call:

```rust
fn roll_random_drop_with_seed(seed: u64) -> bool {
    let r = lcg(seed);        // single LCG step
    (r % 1000) < 5            // 0.5% true
}
```

Caller passes `seed = now_secs() ^ total_tool_calls` to avoid same-second double-rolls returning the same boolean across two adjacent tool calls. **For tests**, expose a `roll_random_drop_with_seed(seed)` taking the seed explicitly, so unit tests can assert specific seeds produce specific outcomes.

---

## 8. Module placement

Following the v3.4.5 / v3.4.6 pattern of consolidating related logic in a single module per feature:

- **New module `events.rs`**: contains all the pure detection helpers — `detect_milestone`, `detect_streak`, `is_birthday`, `should_greet`, `roll_random_drop_with_seed`, plus the `CelebrationKind` mapping for milestone thresholds. Detection logic stays out of `hooks.rs` and `statusline.rs` so those files keep their orchestration role.
- **`speech.rs`**: gains the new pools + extends `compute_celebration_speech` + adds `compute_birthday_speech`.
- **`hooks.rs`** (`feed_inner`): calls into `events::*` for detection, retains the priority match.
- **`hooks.rs`** (`turn`): grows from 1 line to ~10 lines (load/save/greet/reset).
- **`statusline.rs`**: adds the birthday branch in the existing `if celebration_kind != None / else` block, and extends sprite-color and speech-color matches with the new variants.
- **`state.rs`**: three new fields with `serde(default)`, `State::base` updates.

---

## 9. Conflict and edge-case rules

| Scenario | Behavior |
|---|---|
| Two milestones cross in one feed call (rare jump) | Higher one wins; watermark advances to highest |
| Milestone + Levelup-Evolving same call | Milestone wins speech; level/stage update still happens silently |
| Streak hits 5 but milestone also fires | Milestone wins; streak counter does not reset (only `turn()` resets it) |
| Random drop hits but milestone fires | Milestone speech shown; +10 coins still applied |
| First `turn()` after fresh hatch (`last_turn_at == 0`) | Initialize timestamp only; **no greeting** |
| `purrgotchi feed` invoked manually outside any session | Streak counter grows; harmless — resets on next real turn |
| Birthday + Greeting collision | Greeting plays 3 renders → birthday mode resumes for rest of day |
| Birthday + Levelup/Milestone | Active celebration takes priority; birthday resumes after celebration ends |
| Feb 29 kitten in non-leap year | Birthday celebrated Feb 28 |
| State rehatch | All three new fields reset to 0 — fresh kitten can hit Centurion at 100 again |

---

## 10. Testing strategy

**Pure detection helpers (events.rs)**:
- `detect_milestone(prev_total, new_total, watermark) -> Option<CelebrationKind>`
  - 99→100 with watermark 0 → Centurion
  - 100→101 with watermark 100 → None
  - 499→500 with watermark 100 → Veteran
  - 999→1000 with watermark 500 → Master
  - 1999→2000 with watermark 1000 → Legendary (watermark → 2000)
  - 2999→3000 with watermark 2000 → Legendary (watermark → 3000)
  - 2500→2501 with watermark 2000 → None (2501 not a kilo-multiple)
  - Defense-in-depth: 50→1500 (impossible under +1 increment, but if it ever happens,
    watermark prevents firing more than one milestone per call — Centurion wins because
    it's checked first; subsequent calls keep climbing the elif chain via watermark gate)
- `detect_streak(streak_count) -> bool`
  - 4 → false, 5 → true, 6 → false (no re-fire)
- `is_birthday(birth, now) -> bool`
  - Same year as birth → false
  - One year later same month/day → true
  - Feb 29 birth, Feb 28 in non-leap year → true
  - Feb 29 birth, Feb 29 in leap year → true
- `should_greet(last_turn_at, now) -> bool`
  - last == 0 → false (init only)
  - gap < 6h → false
  - gap >= 6h → true
- `roll_random_drop_with_seed(seed) -> bool`
  - Hand-pick a few seeds to assert specific outcomes (deterministic regression test)

**State backward compatibility**:
- v3.4.7-shaped JSON (no `streak_count`, `last_milestone_celebrated`, `last_turn_at`) loads with all three defaulting to 0.

**Integration / behavior**:
- `feed_inner` called 5 times in a row → 5th call sets `celebration_kind = Streak`.
- `feed_inner` called 100 times → at the 100th call, `celebration_kind = MilestoneCenturion`, watermark = 100.
- `turn()` after `last_turn_at` 7 hours ago → `celebration_kind = Greeting`.
- `turn()` resets `streak_count` to 0 even mid-streak.
- `turn()` on fresh hatch → no greeting, `last_turn_at` written.

**Speech**:
- `compute_celebration_speech` returns from each new variant's pool.
- `compute_birthday_speech` rotates over 30s buckets like the existing pools.

**Render count**:
- Milestone fires for 6 renders, then `celebration_kind` clears.
- Streak/Greeting/RandomDrop fire for 3 renders.

Repo convention: use `cargo test` (not `cargo test --lib` — bin-only crate).

---

## 11. Out of scope (explicitly)

- Bespoke sprites for any new event (uses LevelUp sprite).
- Variable random-drop amounts (fixed +10 confirmed; variability deferred to a future "treasure hunt" feature).
- Daily-greeting clock-day variant (we picked the 6h-gap definition).
- Birthday triggering a coin bonus or stat boost (display-only).
- Streak length tuning (fixed 5).
- LLM-flavored speech (Phase 3).
- HUD layout changes.

---

## 12. Master plan checkbox

After ship: tick all five D items in `docs/superpowers/plans/kitten-mechanics-master-plan.md` Phase 2 progress log under a single **v3.4.8** bullet, with a one-line summary linking back to this spec.
