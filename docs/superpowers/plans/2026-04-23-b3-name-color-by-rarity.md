# B3 — Name & Sprite Color by Rarity (v3.4.4) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give kittens a rarity-driven color identity. Name (statusline H1 + card title) and sprite (statusline + card) are colored by rarity. Mood moves from sprite color to speech bubble color. Legendary cycles through a rainbow; shiny shimmers bold/dim.

**Architecture:** A single pure helper `rarity_color(rarity, shiny, now) -> String` in `colors.rs` produces an ANSI prefix. Call sites in `statusline.rs` and `card.rs` wrap the name/sprite regions in that prefix and close with `C_RESET`. Mood color moves to the speech-bubble code path in the statusline. The celebration path is left untouched.

**Tech Stack:** Rust 2021, no new crates. ANSI truecolor escapes. Existing test harness (`cargo test`).

**Spec:** `docs/superpowers/specs/2026-04-23-b3-name-color-by-rarity-design.md`

---

## File Structure

Files touched:

- `rust/src/colors.rs` — add `C_PURPLE`, `C_GOLD`, `C_BLUE`, `RAINBOW_HUES`, `rarity_color()`, and unit tests for the new helper.
- `rust/src/statusline.rs` — rename the mood-color match to `speech_color`; derive `sprite_color` from `rarity_color`; colorize the name inside `title_ansi` (preserving `title_plain`).
- `rust/src/card.rs` — colorize the name inside the title row and colorize the sprite rows. Adjust `card_line_widths_consistent` test so it strips ANSI before measuring.
- `rust/Cargo.toml` — version bump `3.4.3` → `3.4.4`.
- `CHANGELOG.md` — new `## [3.4.4]` entry.
- `docs/superpowers/plans/kitten-mechanics-master-plan.md` — check off B3 in the Phase 2 progress log.

No new files are created. Keeping `rarity_color` inside `colors.rs` (not a new `rarity_color.rs`) because it's a single 20-line function directly about color selection; the home is obvious.

---

## Conventions

- **Build/test dir:** all `cargo` commands run from `rust/`. If your current dir is the repo root, prefix with `cd rust && ...` or `(cd rust && ...)`.
- **Token filter:** prefer `rtk cargo test`, `rtk cargo clippy` per the user's global CLAUDE.md, to minimize output.
- **Commit style:** match existing log — lowercased area prefix, e.g. `colors: add rarity_color helper + shiny shimmer`. No Co-Authored-By lines (per user preference).
- **Spec location:** the spec lives under `docs/superpowers/specs/` and is a working document. Do **not** commit it.

---

## Task 1: Add palette constants (C_PURPLE, C_GOLD, C_BLUE, RAINBOW_HUES)

**Files:**
- Modify: `rust/src/colors.rs`

- [ ] **Step 1: Write the failing test**

Append to `rust/src/colors.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_palette_constants_are_truecolor_escapes() {
        for c in [C_PURPLE, C_GOLD, C_BLUE] {
            assert!(c.starts_with("\x1b[38;2;"), "expected truecolor escape, got {c:?}");
            assert!(c.ends_with('m'), "expected ANSI escape terminator, got {c:?}");
        }
    }

    #[test]
    fn rainbow_hues_has_six_distinct_entries() {
        assert_eq!(RAINBOW_HUES.len(), 6);
        let mut seen = std::collections::HashSet::new();
        for c in RAINBOW_HUES {
            assert!(seen.insert(c), "duplicate hue in RAINBOW_HUES: {c:?}");
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd rust && rtk cargo test --lib colors::tests`
Expected: FAIL — `C_PURPLE`, `C_GOLD`, `C_BLUE`, `RAINBOW_HUES` not defined.

- [ ] **Step 3: Add the constants above the tests module**

Insert after the existing constants (after line 9, `pub const C_RESET`):

```rust
pub const C_PURPLE: &str = "\x1b[38;2;170;100;255m";
pub const C_GOLD:   &str = "\x1b[38;2;255;200;60m";
pub const C_BLUE:   &str = "\x1b[38;2;80;150;255m";

pub const RAINBOW_HUES: [&str; 6] = [
    C_RED,
    C_YELLOW,
    C_GREEN,
    C_CYAN,
    C_BLUE,
    C_MAGENTA,
];
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd rust && rtk cargo test --lib colors::tests`
Expected: PASS (2 tests).

- [ ] **Step 5: Run clippy to stay clean**

Run: `cd rust && rtk cargo clippy --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add rust/src/colors.rs
git commit -m "colors: add purple/gold/blue constants + rainbow hues palette"
```

---

## Task 2: Add `rarity_color` helper with full unit coverage

**Files:**
- Modify: `rust/src/colors.rs`

- [ ] **Step 1: Write the failing tests**

Append to the `#[cfg(test)] mod tests` block in `rust/src/colors.rs`:

```rust
#[test]
fn rarity_color_common_is_cyan() {
    let s = rarity_color("common", false, 0);
    assert_eq!(s, C_CYAN.to_string());
}

#[test]
fn rarity_color_uncommon_is_green() {
    let s = rarity_color("uncommon", false, 0);
    assert_eq!(s, C_GREEN.to_string());
}

#[test]
fn rarity_color_rare_is_purple() {
    let s = rarity_color("rare", false, 0);
    assert_eq!(s, C_PURPLE.to_string());
}

#[test]
fn rarity_color_epic_is_gold() {
    let s = rarity_color("epic", false, 0);
    assert_eq!(s, C_GOLD.to_string());
}

#[test]
fn rarity_color_legendary_cycles_through_all_hues() {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    for now in 0..6 {
        seen.insert(rarity_color("legendary", false, now));
    }
    assert_eq!(seen.len(), 6, "expected 6 distinct hues across now=0..6");
    for hue in RAINBOW_HUES {
        assert!(seen.contains(&hue.to_string()), "missing hue {hue:?}");
    }
}

#[test]
fn rarity_color_legendary_is_deterministic_per_now() {
    for now in [-7_i64, 0, 1, 6, 7, 42, 1_000_000] {
        let a = rarity_color("legendary", false, now);
        let b = rarity_color("legendary", false, now);
        assert_eq!(a, b, "legendary color must be deterministic for now={now}");
    }
}

#[test]
fn rarity_color_unknown_falls_back_to_cyan() {
    assert_eq!(rarity_color("", false, 0), C_CYAN.to_string());
    assert_eq!(rarity_color("garbage", false, 0), C_CYAN.to_string());
}

#[test]
fn rarity_color_shiny_adds_bold_on_even_now() {
    let s = rarity_color("common", true, 0);
    assert!(s.starts_with(C_BOLD), "expected shiny to prefix bold on even now, got {s:?}");
    assert!(s.contains(C_CYAN), "expected shiny to still contain base hue");
}

#[test]
fn rarity_color_shiny_adds_dim_on_odd_now() {
    let s = rarity_color("common", true, 1);
    assert!(s.starts_with(C_DIM), "expected shiny to prefix dim on odd now, got {s:?}");
    assert!(s.contains(C_CYAN), "expected shiny to still contain base hue");
}

#[test]
fn rarity_color_shiny_legendary_combines_shimmer_and_cycle() {
    let s = rarity_color("legendary", true, 2);
    // now=2 is even → bold, RAINBOW_HUES[2] is C_GREEN
    assert!(s.starts_with(C_BOLD));
    assert!(s.contains(C_GREEN));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd rust && rtk cargo test --lib colors::tests`
Expected: FAIL — `rarity_color` not defined.

- [ ] **Step 3: Implement the helper**

Insert after the `RAINBOW_HUES` const in `rust/src/colors.rs`:

```rust
/// Return an ANSI prefix for the kitten's name / sprite based on rarity.
///
/// Legendary cycles through `RAINBOW_HUES` by `now % 6`.
/// When `shiny`, the prefix is preceded by `C_BOLD` (even `now`) or `C_DIM` (odd `now`).
/// Unknown rarity falls back to `C_CYAN`.
pub fn rarity_color(rarity: &str, shiny: bool, now: i64) -> String {
    let base = match rarity {
        "common"    => C_CYAN,
        "uncommon"  => C_GREEN,
        "rare"      => C_PURPLE,
        "epic"      => C_GOLD,
        "legendary" => RAINBOW_HUES[now.rem_euclid(6) as usize],
        _           => C_CYAN,
    };
    if shiny {
        let shimmer = if now.rem_euclid(2) == 0 { C_BOLD } else { C_DIM };
        format!("{shimmer}{base}")
    } else {
        base.to_string()
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd rust && rtk cargo test --lib colors::tests`
Expected: PASS (all 11 tests in `colors::tests`).

- [ ] **Step 5: Run clippy**

Run: `cd rust && rtk cargo clippy --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add rust/src/colors.rs
git commit -m "colors: add rarity_color(rarity, shiny, now) helper"
```

---

## Task 3: Statusline — split sprite color (rarity) from speech color (mood)

**Files:**
- Modify: `rust/src/statusline.rs` (around lines 120–127, 272–288)

**Behavior today (statusline.rs:120–127):** one `k_color` is derived from mood and used for both the sprite (loop at 278–288) and the speech bubble (line 280–282).

**Behavior after:** `sprite_color` comes from `rarity_color(&rarity, shiny, now)` when no celebration is active; `speech_color` keeps today's mood-derived palette. During a celebration, both keep the mood-derived color (unchanged behavior — spec is explicit that celebration retains today's look).

- [ ] **Step 1: Edit the `k_color` block**

In `rust/src/statusline.rs`, replace the current block at lines 120–127:

```rust
    let k_color: String = match mood {
        "panic" => C_RED.to_string(),
        "alert" => C_YELLOW.to_string(),
        "happy" => C_GREEN.to_string(),
        "sleep" => C_MAGENTA.to_string(),
        "hungry" => format!("{C_DIM}{C_CYAN}"),
        _ => C_CYAN.to_string(),
    };
```

with:

```rust
    let speech_color: String = match mood {
        "panic" => C_RED.to_string(),
        "alert" => C_YELLOW.to_string(),
        "happy" => C_GREEN.to_string(),
        "sleep" => C_MAGENTA.to_string(),
        "hungry" => format!("{C_DIM}{C_CYAN}"),
        _ => C_CYAN.to_string(),
    };

    // Sprite color is rarity-driven outside of celebrations. During a celebration,
    // the celebration sprite + speech path keeps the mood color for continuity.
    let sprite_color: String = if celebration_kind != CelebrationKind::None {
        speech_color.clone()
    } else {
        crate::colors::rarity_color(&rarity, shiny, now)
    };
```

- [ ] **Step 2: Update the render loop to use the new names**

Replace the render loop at lines 278–288. The current loop reads:

```rust
    for (i, row) in rows.iter().enumerate() {
        if i == 6 {
            buf.push_str(&format!(
                "{row}   {k_color}\"{speech}\"{C_RESET}\n"
            ));
        } else {
            let line = sprite_lines.get(i).map(String::as_str).unwrap_or("");
            let padded = pad_sprite(line, SW);
            buf.push_str(&format!("{row}   {k_color}{padded}{C_RESET}\n"));
        }
    }
```

Replace with:

```rust
    for (i, row) in rows.iter().enumerate() {
        if i == 6 {
            buf.push_str(&format!(
                "{row}   {speech_color}\"{speech}\"{C_RESET}\n"
            ));
        } else {
            let line = sprite_lines.get(i).map(String::as_str).unwrap_or("");
            let padded = pad_sprite(line, SW);
            buf.push_str(&format!("{row}   {sprite_color}{padded}{C_RESET}\n"));
        }
    }
```

- [ ] **Step 3: Build to confirm the rename didn't miss a reference**

Run: `cd rust && rtk cargo build`
Expected: compiles cleanly (no references to `k_color` remain).

- [ ] **Step 4: Run the full test suite**

Run: `cd rust && rtk cargo test`
Expected: all tests pass. The H1-rail-length tests (`test_h1_rail_total_len_peak`, `test_h1_rail_total_len_off_peak`) in particular must still pass — this task doesn't touch `title_plain`.

- [ ] **Step 5: Run clippy**

Run: `cd rust && rtk cargo clippy --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add rust/src/statusline.rs
git commit -m "statusline: split sprite (rarity) from speech (mood) color"
```

---

## Task 4: Statusline — color the name in the H1 rail

**Files:**
- Modify: `rust/src/statusline.rs` (around lines 134–137)

**Today's H1 wraps the entire title (`NETRUNNER // {kitten_name}`) in `C_PINK`.** We split that wrapper so `NETRUNNER //` and the `╡`/`╞` glyphs keep `C_PINK`, but the name gets the rarity color. `title_plain` is unchanged, so the rail width math stays correct.

- [ ] **Step 1: Edit the title_ansi builder**

Replace lines 134–137 of `rust/src/statusline.rs`:

```rust
    let title_plain = format!("═╡ NETRUNNER // {kitten_name} ╞");
    let title_ansi = format!(
        "{C_PINK}═╡{C_RESET} {C_BOLD}{C_CYAN}NETRUNNER{C_RESET} {C_MAGENTA}//{C_RESET} {C_PINK}{kitten_name}{C_RESET} {C_PINK}╞{C_RESET}"
    );
```

with:

```rust
    let title_plain = format!("═╡ NETRUNNER // {kitten_name} ╞");
    let name_color = crate::colors::rarity_color(&rarity, shiny, now);
    let title_ansi = format!(
        "{C_PINK}═╡{C_RESET} {C_BOLD}{C_CYAN}NETRUNNER{C_RESET} {C_MAGENTA}//{C_RESET} {name_color}{kitten_name}{C_RESET} {C_PINK}╞{C_RESET}"
    );
```

- [ ] **Step 2: Build**

Run: `cd rust && rtk cargo build`
Expected: compiles cleanly.

- [ ] **Step 3: Run the full test suite**

Run: `cd rust && rtk cargo test`
Expected: all tests pass, including the two rail-length tests.

- [ ] **Step 4: Quick visual smoke test (optional, recommended)**

Pipe an empty JSON to the statusline binary; confirm output renders without ANSI garbage:

```bash
cd rust && echo '{}' | cargo run --quiet --bin purrgotchi -- statusline | head -3
```

Expected: the H1 rail prints; the kitten's name appears in its rarity color (visible in a truecolor terminal). No mojibake.

- [ ] **Step 5: Commit**

```bash
git add rust/src/statusline.rs
git commit -m "statusline: color H1 name by rarity (keeping rail pink)"
```

---

## Task 5: Card — color the name in the title row

**Files:**
- Modify: `rust/src/card.rs` (around lines 32–48)

**Problem:** `card.rs:46–48` builds `title_raw = format!("{} // {} // {}", breed_upper, s.name, stars)`, truncates to `inner_content`, and pads with `{:<width$}`. Wrapping `s.name` in ANSI codes before the format would break the char-count width. Strategy: **pad the plain form first, then splice color codes around the name.**

- [ ] **Step 1: Compute `now` inside `render`**

At the top of `render` (after line 34 `let mut out = String::new();`), add:

```rust
    let now: i64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let name_color = crate::colors::rarity_color(&s.rarity, s.shiny, now);
```

- [ ] **Step 2: Replace the title-row block (lines 43–48) with an ANSI-aware version**

```rust
    // Title: BREED // Name // STARS — name is colored by rarity; breed and stars stay plain.
    let inner_content = inner - 2;
    let title_raw = format!("{} // {} // {}", breed_upper, s.name, stars);
    let title: String = title_raw.chars().take(inner_content).collect();
    // Pad plain first (width-correct), then splice color around the name.
    let padded_plain = format!("{:<width$}", title, width = inner_content);
    let colored = if padded_plain.contains(&s.name) {
        padded_plain.replacen(
            s.name.as_str(),
            &format!("{name_color}{}{}", s.name, crate::colors::C_RESET),
            1,
        )
    } else {
        // Name was truncated out of the visible title — no coloring possible.
        padded_plain
    };
    writeln!(out, "║  {colored}║").unwrap();
```

- [ ] **Step 3: Update the `card_line_widths_consistent` test to strip ANSI before measuring**

The current test (card.rs:244–255) counts `line.chars().count()` which now includes ANSI escape chars. Replace it with:

```rust
    #[test]
    fn card_line_widths_consistent() {
        let output = render(&test_state());
        let expected = CARD_WIDTH + 2; // e.g. 44
        for (i, line) in output.lines().enumerate() {
            let stripped = strip_ansi(line);
            let len = stripped.chars().count();
            assert_eq!(len, expected,
                "line {i} stripped width {len} != {expected}: {:?}", line);
        }
    }

    /// Strip CSI (ESC [ ... letter) sequences so char counts reflect visible width.
    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' && chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // consume until a letter (inclusive) — CSI final byte is 0x40..=0x7E
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }
```

- [ ] **Step 4: Add a new test that the title row contains a rarity color**

Append inside `mod tests`:

```rust
    #[test]
    fn card_title_contains_rarity_color_for_name() {
        // test_state is legendary shiny → rarity_color will include a RAINBOW hue
        // and either C_BOLD or C_DIM.
        let output = render(&test_state());
        let has_any_hue = crate::colors::RAINBOW_HUES
            .iter()
            .any(|h| output.contains(h));
        assert!(has_any_hue, "expected a rainbow hue in legendary card output:\n{output}");
    }

    #[test]
    fn card_title_contains_rarity_color_for_common() {
        let mut s = test_state();
        s.rarity = "common".to_string();
        s.shiny = false;
        let output = render(&s);
        assert!(output.contains(crate::colors::C_CYAN),
            "expected C_CYAN in common card output:\n{output}");
    }
```

- [ ] **Step 5: Run tests**

Run: `cd rust && rtk cargo test --lib card`
Expected: all card tests pass (existing + 2 new).

- [ ] **Step 6: Run clippy**

Run: `cd rust && rtk cargo clippy --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add rust/src/card.rs
git commit -m "card: color name in title row by rarity; strip ANSI in width test"
```

---

## Task 6: Card — color the sprite rows

**Files:**
- Modify: `rust/src/card.rs` (sprite loop at lines 70–77)

**Strategy:** pad each sprite line to `sprite_inner` width first (plain, char-counted), *then* wrap the padded string in `{name_color}` + `C_RESET`, *then* write without a second `{:<width$}` formatter. This keeps the box edges aligned regardless of ANSI length.

- [ ] **Step 1: Replace the sprite render loop**

Replace lines 70–77 of `rust/src/card.rs`:

```rust
    // Sprite (6 lines, each padded to fit inside the box)
    // Sprite line max width = inner - 2 (two leading spaces + content + trailing pad)
    let sprite_inner = inner - 2;
    let stage = crate::state::life_stage(s.level);
    let sprite = crate::sprites::get_sprite_lines(stage, &s.rarity, "calm", 0);
    for line in sprite.iter().take(6) {
        writeln!(out, "║  {:<width$}║", line, width = sprite_inner).unwrap();
    }
```

with:

```rust
    // Sprite (6 lines, each padded to fit inside the box, then wrapped in rarity color)
    let sprite_inner = inner - 2;
    let stage = crate::state::life_stage(s.level);
    let sprite = crate::sprites::get_sprite_lines(stage, &s.rarity, "calm", 0);
    for line in sprite.iter().take(6) {
        let padded = format!("{:<width$}", line, width = sprite_inner);
        writeln!(out, "║  {name_color}{padded}{}║", crate::colors::C_RESET).unwrap();
    }
```

- [ ] **Step 2: Add a test that sprite rows carry a color prefix**

Append inside `mod tests`:

```rust
    #[test]
    fn card_sprite_rows_wrapped_in_rarity_color() {
        let mut s = test_state();
        s.rarity = "common".to_string();
        s.shiny = false;
        let output = render(&s);
        // Expect at least one line where a plain sprite glyph is preceded by C_CYAN in
        // the same line. Simpler: confirm C_CYAN occurs more than once in the output
        // (title + at least one sprite row).
        let count = output.matches(crate::colors::C_CYAN).count();
        assert!(count >= 2, "expected C_CYAN in title + sprite rows, saw {count} in:\n{output}");
    }
```

- [ ] **Step 3: Run tests**

Run: `cd rust && rtk cargo test --lib card`
Expected: all tests pass. The `card_line_widths_consistent` test (now ANSI-stripped) must still hold.

- [ ] **Step 4: Run clippy**

Run: `cd rust && rtk cargo clippy --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 5: Visual smoke test**

Run:

```bash
cd rust && cargo run --quiet --bin purrgotchi -- card
```

Expected: the card prints; the name and sprite are tinted in whatever color matches the current kitten's rarity. Box edges (`║`, `╚`, `╝`) are aligned. If you're a legendary, running `card` back to back shifts the hue.

- [ ] **Step 6: Commit**

```bash
git add rust/src/card.rs
git commit -m "card: color sprite rows by rarity with ANSI-aware padding"
```

---

## Task 7: Version bump + CHANGELOG

**Files:**
- Modify: `rust/Cargo.toml` (line 3)
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Bump the crate version**

Edit `rust/Cargo.toml` line 3:

```toml
version = "3.4.4"
```

- [ ] **Step 2: Add the changelog entry**

Open `CHANGELOG.md`. Above the current top entry (`## [3.4.3]`), insert:

```markdown
## [3.4.4] — 2026-04-23

### Added
- Rarity-driven color for the kitten's name and sprite, in both the statusline HUD (H1 rail + sprite column) and the `purrgotchi card` output. Common → cyan, uncommon → green, rare → purple, epic → gold.
- Legendary kittens cycle through six rainbow hues, stepping once per second (`now % 6`).
- Shiny kittens shimmer: the rarity color is prefixed by `C_BOLD` on even seconds and `C_DIM` on odd seconds. Combines with legendary for a shimmering rainbow.
- New palette constants in `colors.rs`: `C_PURPLE`, `C_GOLD`, `C_BLUE`, plus a `RAINBOW_HUES` array and a `rarity_color(rarity, shiny, now)` helper.

### Changed
- Sprite color is now driven by rarity, not mood. Mood continues to be expressed through the speech-bubble text color (previous palette: red/yellow/green/magenta/dim-cyan/cyan).
- The statusline H1 rail keeps its pink framing; only the kitten's name segment adopts the rarity color. `title_plain` is unchanged, so rail width math is unaffected.
- The `card_line_widths_consistent` test now strips ANSI escapes before measuring visible width.

### Backwards compatibility
- No state schema change. Existing kittens keep their rarity and shiny flag and render in the new palette on first statusline/card run after upgrade.
- Celebration rendering (level-up and evolving overlays) is untouched and retains its mood-colored look.
```

- [ ] **Step 3: Rebuild to regenerate `Cargo.lock`**

Run: `cd rust && rtk cargo build`
Expected: builds cleanly; `Cargo.lock` is touched.

- [ ] **Step 4: Commit**

```bash
git add rust/Cargo.toml rust/Cargo.lock CHANGELOG.md
git commit -m "v3.4.4: name & sprite color by rarity"
```

---

## Task 8: Verification + master plan update

**Files:**
- Modify: `docs/superpowers/plans/kitten-mechanics-master-plan.md`

- [ ] **Step 1: Full test suite one more time**

Run: `cd rust && rtk cargo test`
Expected: all tests pass.

- [ ] **Step 2: Clippy final pass**

Run: `cd rust && rtk cargo clippy --all-targets -- -D warnings`
Expected: no warnings. The previous release was blocked on a redundant `'static` clippy; watch for similar gotchas on the new additions.

- [ ] **Step 3: Manual smoke — each rarity tier via dev backdoor**

For each of the five tiers, re-hatch and visually inspect. From the repo root:

```bash
# Re-hatch at each tier; then run card + statusline to eyeball the color.
for tier in common uncommon rare epic legendary; do
  echo "=== $tier ==="
  PURR_FORCE_RARITY=$tier cd rust && cargo run --quiet --bin purrgotchi -- hatch --force
  cd .. && cd rust && cargo run --quiet --bin purrgotchi -- card
done
```

(If `hatch --force` isn't the right flag in this repo, use whatever Phase 1 shipped — `purrgotchi hatch --rarity <tier>` per the master plan.)

Expected: card renders in a different color per tier; common=cyan, uncommon=green, rare=purple, epic=gold, legendary=one of 6 hues.

- [ ] **Step 4: Manual smoke — shiny shimmer**

```bash
PURR_FORCE_RARITY=epic PURR_FORCE_SHINY=1 cd rust && cargo run --quiet --bin purrgotchi -- hatch
cd rust && cargo run --quiet --bin purrgotchi -- card
cd rust && cargo run --quiet --bin purrgotchi -- card
```

Expected: two consecutive `card` runs show the same gold hue but alternating bold/dim shimmer (if the runs fall on different seconds).

- [ ] **Step 5: Manual smoke — celebration path is untouched**

Trigger a level-up (feed until level threshold crossed) and observe the statusline.
Expected: the celebration sprite appears in mood color (red/yellow/green/etc.) — not the rarity color — and the `levelup_renders_remaining` counter decrements normally. This confirms we didn't regress B2.

- [ ] **Step 6: Check off B3 in the master plan**

Edit `docs/superpowers/plans/kitten-mechanics-master-plan.md`. In the Phase 2 progress log block (currently lines 109–112), change:

```markdown
- [ ] Phase 2 — Character depth (v3.4.x)
  - [x] B1 + B2 bundle (v3.4.3) — life-stage sprite evolution + level-up celebration
  - Remaining B/C/D items TBD per iteration
```

to:

```markdown
- [ ] Phase 2 — Character depth (v3.4.x)
  - [x] B1 + B2 bundle (v3.4.3) — life-stage sprite evolution + level-up celebration
  - [x] B3 (v3.4.4) — name & sprite color by rarity (rainbow legendary, shimmer shiny)
  - Remaining C/D items TBD per iteration
```

- [ ] **Step 7: Commit master plan update**

```bash
git add docs/superpowers/plans/kitten-mechanics-master-plan.md
git commit -m "master plan: mark B3 (v3.4.4) complete"
```

- [ ] **Step 8: Offer merge & tag**

At this point the feature branch is ready. Per repo convention:
- Tag `v3.4.4` on the merge commit once merged to `main`.
- Push tags.
- (Merge, tag, push are user-gated actions per global preferences — don't do them unsolicited.)

---

## Self-Review Notes

**Spec coverage check:**
- Palette (common cyan / uncommon green / rare purple / epic gold / legendary rainbow / shiny shimmer) → Task 1 + Task 2.
- `rarity_color(rarity, shiny, now)` helper → Task 2.
- Statusline sprite tinted by rarity → Task 3.
- Statusline H1 name tinted by rarity → Task 4.
- Card title name tinted by rarity → Task 5.
- Card sprite rows tinted by rarity → Task 6.
- Speech bubble keeps mood color → Task 3 (renamed to `speech_color`, logic unchanged).
- Celebration path untouched → Task 3 (explicit `if celebration_kind != None { speech_color.clone() }` carve-out).
- `title_plain` unchanged (rail width math intact) → Task 4 (only `title_ansi` mutated).
- ANSI-aware card padding → Tasks 5 and 6 (pad plain first, then wrap).
- Updated `card_line_widths_consistent` test → Task 5 with `strip_ansi` helper.
- New tests for each rarity tier, legendary cycle, shiny shimmer, fallback → Task 2.
- Version bump + CHANGELOG → Task 7.
- Master plan checkmark → Task 8.

**Placeholder scan:** no `TBD`, no "similar to", no "add appropriate". Every code block is complete and copy-pastable.

**Type consistency:** `rarity_color(rarity: &str, shiny: bool, now: i64) -> String` signature used identically in Tasks 3, 4, 5, 6. `speech_color`/`sprite_color` naming consistent between their definition (Task 3) and use.

**Out-of-scope creep:** none. No sprite ASCII changes, no state schema changes, no new commands, no mood logic changes.
