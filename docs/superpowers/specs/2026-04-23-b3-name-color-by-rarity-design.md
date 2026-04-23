# B3 — Name & Sprite Color by Rarity (v3.4.4)

_Design spec — 2026-04-23_

## Summary

Give the kitten's **name** and **sprite** a rarity-driven color identity. Mood moves from the sprite to the speech bubble. Legendary kittens get a rainbow-cycle name; shiny kittens get a bold/dim shimmer overlay.

This is the B3 item in the Phase 2 roadmap (`docs/superpowers/plans/kitten-mechanics-master-plan.md`).

## Motivation

Rarity currently surfaces only as stars in the HUD and card. Players don't feel their rarity when they look at their cat — a common and a legendary render identically apart from the star row. Color gives rarity an immediate, glanceable presence. It also frees the sprite color channel from mood duty (a job the sprite's *shape* and the speech bubble can do just as well).

## Locked design decisions

- **Scope:** statusline H1 rail (name), statusline sprite (all 6 rows), card title (name), card sprite (6 rows).
- **Mood signal:** moves to the speech bubble text, which keeps today's mood-colored behavior. Sprite no longer reflects mood via color.
- **Celebration override:** unchanged. When a celebration is active, the existing celebration sprite + speech path runs and takes precedence over the rarity color. No new celebration color is introduced.
- **Rainbow cadence (legendary):** whole-word color cycle, stepping once per second (`now % 6` across 6 hues).
- **Shimmer (shiny):** toggles between `C_BOLD` and `C_DIM` modifier on the rarity color, alternating by `now % 2`.
- **No per-character ANSI** in the name — keeps the H1 rail's plain-length math intact.

## Color palette

| Rarity | Color | Source |
|---|---|---|
| common | cyan | existing `C_CYAN` |
| uncommon | green | existing `C_GREEN` |
| rare | purple | new `C_PURPLE` (brighter than existing `C_MAGENTA`) |
| epic | gold | new `C_GOLD` (warmer/brighter than `C_YELLOW`) |
| legendary | rainbow cycle | new `RAINBOW_HUES: [&str; 6]` — red, yellow, green, cyan, blue, magenta, selected by `now % 6` |

New constants added to `colors.rs`:
- `C_PURPLE` — suggested `\x1b[38;2;170;100;255m` (bright purple, visually distinct from `C_MAGENTA`'s dark `113,28,145`)
- `C_GOLD` — suggested `\x1b[38;2;255;200;60m` (warm gold, visually distinct from `C_YELLOW`'s lime-yellow `243,243,27`)
- `C_BLUE` — suggested `\x1b[38;2;80;150;255m` (for the rainbow cycle only)

Exact RGB values may be tuned during implementation; the spec commits to the *role* (bright purple, warm gold, readable blue), not specific values.

## New helper: `rarity_color`

Lives in `colors.rs`:

```rust
pub fn rarity_color(rarity: &str, shiny: bool, now: i64) -> String
```

Returns an ANSI prefix string that should be followed by the text to color and then `C_RESET`.

Behavior:
1. Pick base hue:
   - `"common"` → `C_CYAN`
   - `"uncommon"` → `C_GREEN`
   - `"rare"` → `C_PURPLE`
   - `"epic"` → `C_GOLD`
   - `"legendary"` → `RAINBOW_HUES[(now.rem_euclid(6)) as usize]`
   - fallback (empty / unknown) → `C_CYAN`
2. If `shiny`, prepend `C_BOLD` on even `now`, `C_DIM` on odd `now`.
3. Concatenate and return as `String`.

Rationale for returning `String` (not `&'static str`): legendary + shiny combine two dynamic pieces. A small allocation per render is negligible.

## File changes

### `rust/src/colors.rs`
- Add `C_PURPLE`, `C_GOLD`, `C_BLUE` constants.
- Add `RAINBOW_HUES: [&str; 6]` const array.
- Add `rarity_color` function.

### `rust/src/statusline.rs`
- **H1 title:** split the existing `title_ansi` so `NETRUNNER //` and the `╡`/`╞` glyphs keep `C_PINK`, but the name segment is wrapped in `rarity_color(&rarity, shiny, now)` ... `C_RESET`. The `title_plain` remains unchanged, so the rail width math at lines 138–147 is unaffected.
- **Sprite color:** the current `k_color` match on `mood` is split into two values:
  - `sprite_color` = `rarity_color(&rarity, shiny, now)` when no celebration is active.
  - `speech_color` = the existing mood-to-color match (today's behavior).
  - Sprite rows use `sprite_color`; the speech bubble line uses `speech_color`.
- **Celebration path:** when `celebration_kind != CelebrationKind::None`, keep today's behavior (sprite and speech both use mood color). No rarity color applied during a celebration.

### `rust/src/card.rs`
- `render(s: &State)` signature stays the same. Compute `now` inside `render` via `SystemTime::now().duration_since(UNIX_EPOCH)` — same approach as `statusline.rs`. A failing clock yields `0`, which still produces a valid color (first hue, even shimmer).
- **Title row (line 46):** wrap `s.name` in `rarity_color(&s.rarity, s.shiny, now)` ... `C_RESET`. Breed and stars stay uncolored so the title still reads as "BREED // Name // STARS" with only the name tinted.
- **Sprite rows (lines 75–77):** each line needs to be padded to `sprite_inner` *before* being wrapped in color codes, then written without further padding, because Rust's `{:<width$}` formatter counts bytes and will miscount ANSI escapes. Approach: build `format!("{:<width$}", line, width = sprite_inner)` first, then wrap in `rarity_color(...)` + `C_RESET`, then write with `writeln!(out, "║  {raw}║", raw = colored_padded)`.
- Title-row width (line 48) uses the same pre-pad-then-wrap pattern for the name segment. Compute the plain title with the name uncolored for width purposes, then rebuild the ANSI variant.

## Edge cases

- **Card is a one-shot command.** Each invocation uses wall-clock `now`, so a legendary card displays one of 6 hues each time you run `purrgotchi card`. Shiny alternates bold/dim per invocation. No continuous animation; acceptable for a printed card.
- **H1 rail width math.** `title_plain` is used to compute `pre_rail`/`post_rail` and must stay ANSI-free. Splitting the color wrapper around the name only changes `title_ansi`; `title_plain` is unchanged. Existing tests `test_h1_rail_total_len_peak` and `test_h1_rail_total_len_off_peak` should still pass unchanged.
- **Unknown rarity string.** `rarity_color` falls back to `C_CYAN`. Matches today's behavior for fresh / empty state.
- **`shiny = true` on a pre-v3.4.0 state.** Impossible — state migration hard-rejects v1; all current states have a rarity and shiny bool.
- **Card rendering a state where `now` = 0.** The helper still returns a valid color (first rainbow hue, even shimmer). Safe fallback.

## Testing

Unit tests in `colors.rs`:
- `rarity_color("common", false, 0)` contains `C_CYAN` and no bold/dim modifier.
- `rarity_color("uncommon", false, 0)` contains `C_GREEN`.
- `rarity_color("rare", false, 0)` contains `C_PURPLE`.
- `rarity_color("epic", false, 0)` contains `C_GOLD`.
- `rarity_color("legendary", false, n)` cycles through all 6 `RAINBOW_HUES` as `n` iterates 0..=5.
- `rarity_color("common", true, 0)` starts with `C_BOLD`.
- `rarity_color("common", true, 1)` starts with `C_DIM`.
- `rarity_color("", false, 0)` falls back to `C_CYAN`.
- `rarity_color("garbage", false, 0)` falls back to `C_CYAN`.

Regression test in `statusline.rs`:
- Existing H1 rail-length tests remain unchanged and must still pass.

Smoke test (manual, documented in plan):
- Run `purrgotchi card` with a forced rarity at each tier (`PURR_FORCE_RARITY=<tier>`) and with `--shiny`. Verify visual output.
- Run `purrgotchi statusline` with piped mock payload at each rarity; verify sprite and name color match rarity and speech bubble still reflects mood.
- Trigger a level-up and confirm celebration still overrides rarity color.

## Version & changelog

- Bump `Cargo.toml` to `3.4.4`.
- `CHANGELOG.md` entry under `## [3.4.4]`:
  - **Added:** rarity-driven color for the kitten's name (H1 rail + card title) and sprite. Legendary kittens cycle through a rainbow; shiny kittens shimmer with a bold/dim overlay. New palette constants `C_PURPLE`, `C_GOLD`, `C_BLUE`.
  - **Changed:** sprite color is now driven by rarity (not mood). Mood is expressed through the speech bubble text color, which keeps its previous behavior.
  - **Backwards compatibility:** no state schema change. Existing kittens keep their rarity + shiny flags and render in the new palette immediately.

## Out of scope

- No new rarity tiers.
- No changes to sprite ASCII art.
- No changes to mood logic or speech pools.
- No animated effects beyond the legendary hue cycle and shiny bold/dim shimmer.
- No color on the stars row, breed, or personality text — only the name and the sprite.
- No per-character rainbow effects — whole-name single-color cycle only.

## Workflow checkpoints

1. Spec self-review (inline in brainstorming).
2. User reviews spec.
3. `/superpowers:writing-plans` — turn this spec into a task-by-task implementation plan.
4. `/superpowers:executing-plans` — build it on a feature branch.
5. `/superpowers:verification-before-completion` — run tests + smoke tests before claiming done.
6. `/superpowers:requesting-code-review`.
7. Commit → tag `v3.4.4` → push → merge → update master plan progress log.
