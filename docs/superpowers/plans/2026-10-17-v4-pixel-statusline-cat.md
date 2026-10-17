# v4 pivot: pixel-art live cat in the classic statusline. Implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Drop the Claude Code mod. Keep the classic statusline HUD box byte-identical, and replace the 16-col ASCII sprite column with a live, 1 fps, half-block pixel-art cat (idle life, playground scenes, celebrations). The ASCII sprite stays as the fallback.

**Architecture:** A new `rust/src/px/` module renders a `strip_width × 12` pixel canvas. Each terminal cell is `▀` with a 24-bit fg (top pixel) and bg (bottom pixel), so 6 rows give 12 px of height. The statusline asks `px::cat_area()` for the 6 sprite-column rows, and the box rows are never touched. Everything is a pure function of `(state, now, mood, width)`, so the statusline still writes no state per render. While the work is in progress the pixel cat is opt-in (`PURR_PIXELS=1`). Task 9 flips the default, and that is the only commit that re-records the 45 classic goldens.

**Tech Stack:** Rust (edition as in `rust/Cargo.toml`, no new crates), the existing golden harness (`rust/tests/common/mod.rs`), Python 3 + Pillow 10 for the PNG preview only (dev tool, not shipped).

**Spec:** this document (design + plan in one, per the coordinator). Background: `docs/superpowers/specs/2026-07-11-v3.8.0-playground-scenes-design.md` (scene engine), `rust/src/pixel.rs` header (palette/rim-light origin).

## Global Constraints

- Worktree `/home/englader/Desktop/Work/Personal/purrgotchi/.worktrees/v4.0.0`, branch `feat/v4.0.0-claude-code-mod`. Run cargo from `rust/`.
- The shell is wrapped by `rtk`. Use `/bin/cat` or the Read tool to see real file bytes.
- Never run the binary against the real `~/.claude/purrgotchi`. Always set `PURR_STATE_DIR=$(mktemp -d)` (and `HOME` to a temp dir when hatching).
- Commit messages: conventional (`feat:`, `fix:`, `test:`, `docs:`, `refactor:`, `chore:`). **No `Co-Authored-By` lines.** Never commit `docs/superpowers/specs/` changes.
- HUD box rows (`h1`–`h7` in `statusline.rs`) must stay byte-identical for every golden case. Row 7 (box + speech bubble) is pinned whole.
- Layout facts: `BOX_WIDTH=94` gives rows of 96 visible cols, then a 3-space gap. Sprite column: 6 rows (row 7 is speech). Stage width = `playground::strip_width()` = `COLUMNS − 101`, clamped to `24..=120`, default 60 when `COLUMNS` is unset. `None` means too narrow, which gives the ASCII fallback.
- Frame rate is **1 fps** (statusline `refreshInterval: 1`). Design every animation as one keyframe per second.
- Glyphs: only U+2580 `▀`, U+2584 `▄`, U+2588 `█` and space. Every pixel row ends with `\x1b[0m`. Transparent cells are plain spaces with **no bg active**. Plain width of every row == stage width.
- `PURR_PIXELS`: unset → `DEFAULT_MODE` (`Off` until Task 9). `0` → ASCII. `1` → full (fg+bg). `fg` → fg-only half-blocks. `test` → bg test card. A non-empty `NO_COLOR` forces ASCII. Unknown values → default.
- `PURR_PLAYGROUND=0` disables **scenes only**. The idle pixel cat still shows.
- `frame --json` keeps its ASCII twin (`Attr` gains no `Bg`). The pixel cat is statusline ANSI only.
- Version stays **4.0.0**: it was never tagged or pushed, so the CHANGELOG entry is rewritten, not appended.
- Model tags: **sonnet** unless marked **opus** (art + state machine).

## Decisions (made here, justified once)

- **Removed (mod-only, with ongoing cost):** `mod/`, `.claude-plugin/`, and the `serve` (long-lived streaming process with EOF/broken-pipe handling, only the dock pane read it), `cost` (the statusline already accumulates cost from its payload), `quip-prompt` and `quip-store` subcommands (session-model quips are only reachable from the mod). Also removed: their tests and their README/CHANGELOG text.
- **Kept:** the `--json` surface on action commands, `frame --json` (scripting/debug), `frame --pixels` + the 32×32 `pixel.rs` portrait + `pixel_hashes.txt`. The user named it keep; it is frozen, tested code, and the mini cat reuses its breed table, `mix/shade/light`, rarity colours and rim-light constants. Also kept: `PURR_STATE_DIR`, the hardening, all tests, and `install.sh --uninstall-hooks` (generic and tested; only its comments that mention the mod get reworded).
- **Wrapper:** `purr` is `exec purrgotchi "$@"`, so every verb passes through and nothing needs adding to it. The README documents `! purr pet|play|treat|card|stats|rename` from inside Claude Code, plus the dev verb `pixels`. The wrapper example becomes portable instead of a hardcoded `.exe` path.
- **Celebrations/birthday** render as pixel overlays (cat in a cheer pose plus effect). Scenes keep `playground.rs` scheduling and speech. Only the drawing is new.

## Review Focus

1. **Terminal resize between renders** (`COLUMNS` changes, so the stage width changes): the cat must stay inside `0..=width−cat_w` and never panic. Pinned by the `life_frame` sweep over spans 0..=120 (Task 6).
2. **Host ignores or strips bg escapes, or trims trailing spaces:** `fg` mode must look sane, and no row may leave bg active. Pinned by the encoder hygiene sweep for both modes (Task 1, re-run in Tasks 7–8).
3. **Old or odd state:** an empty `breed` (old states), an unknown breed or rarity, `scene_seed`/`birth_time` near `u64::MAX`, or a clock that regresses under `PURR_NOW`. Everything falls back to Ginger/common, there is no overflow (wrapping/saturating math), and there is no panic. Pinned in Tasks 4 and 6.
4. **Mood or stage flips mid-block** (ctx crosses 80 % → panic; an evolving celebration changes the stage size): the pose stays valid and the cat stays clamped in bounds. Pinned in Task 6.
5. **Narrow terminal / opt-out:** `COLUMNS<125`, `PURR_PIXELS=0` and `NO_COLOR=1` must reproduce today's bytes exactly. Pinned by `statusline_pixels_off_is_classic` (Task 3).

---

## File map

| File | Responsibility |
|---|---|
| `rust/src/px/mod.rs` | `PxCanvas`, `PxSprite`, `PixelMode` + env parse, `encode()` (cells → ANSI rows), `test_card()`, `cat_area()` (top-level chooser) |
| `rust/src/px/art.rs` | Pose/tail/prop enums, interior pixel maps, stage row-drop rule |
| `rust/src/px/paint.rs` | Slot map → coloured `PxSprite`: breed pattern, flip, cel shade, rim light, auto-outline, eyes/blink |
| `rust/src/px/life.rs` | Idle behaviour state machine (pure, `PURR_NOW`-testable), `PURR_FORCE_POSE` |
| `rust/src/px/scene.rs` | Pixel versions of the 6 playground scenes (positions mirror `playground.rs`) |
| `rust/src/px/fx.rs` | Celebration and birthday overlays |
| `rust/src/statusline.rs` | Sprite-column source swap only (box code untouched) |
| `rust/src/main.rs` | `pixels test|sheet` dev subcommand; removal of `serve/cost/quip-*` |
| `rust/tests/golden_statusline.rs`, `rust/tests/common/mod.rs` | Box-freeze goldens, pixels-off == classic, `statusline_px/` cases |
| `rust/tests/golden/statusline_box/`, `statusline_px/`, `pxcat_hashes.txt` | New goldens |
| `rust/tools/px_preview.py`, `rust/tools/px_anim.sh` | PNG sheet renderer; 1 fps statusline player against a temp state dir |
| `README.md`, `CHANGELOG.md`, `install.sh`, `.env.example` | Docs/removal |

---

### Task 1: Half-block encoder + bg test card (pre-step) [sonnet]

**Files:** Create `rust/src/px/mod.rs`. Modify `rust/src/main.rs` (`mod px;`, `pixels` arm, usage line), `rust/src/statusline.rs` (test-card hook in the compose loop), `rust/tests/golden_statusline.rs` (add a `dir` field to `Case`, plus a `px_cases()` list).

**Interfaces (produces):**
```rust
pub(crate) const PX_ROWS: usize = 6;  pub(crate) const PX_H: usize = 12;
pub(crate) struct PxCanvas { w: usize, px: Vec<Option<Rgb>> }        // w × PX_H
impl PxCanvas { pub fn new(w: usize) -> Self; pub fn width(&self) -> usize;
  pub fn set(&mut self, x: i32, y: i32, c: Rgb);  /* clipped */  pub fn get(&self, x: i32, y: i32) -> Option<Rgb>;
  pub fn blit(&mut self, s: &PxSprite, x: i32, y: i32); /* None = transparent, clipped */ }
pub(crate) struct PxSprite { pub w: usize, pub h: usize, pub px: Vec<Option<Rgb>> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub(crate) enum PixelMode { Off, Full, FgOnly, Test }
pub(crate) const DEFAULT_MODE: PixelMode = PixelMode::Off;          // Task 9 flips it
pub(crate) fn mode_from(pixels: Option<&str>, no_color: Option<&str>, default: PixelMode) -> PixelMode;
pub(crate) fn mode() -> PixelMode;                                   // env PURR_PIXELS / NO_COLOR
pub(crate) fn encode(c: &PxCanvas, mode: PixelMode) -> Vec<String>;  // PX_ROWS rows
pub(crate) fn test_card() -> Vec<String>;                            // 6 rows, plain width 24
```

**Encoding rule** (cell = top px `t`, bottom px `b`). Emit a reset only when the (fg, bg) state changes. Start state = plain. End every row with `\x1b[0m`.
- `(None, None)` → plain `' '`. `(Some t, None)` → fg t, `▀`. `(None, Some b)` → fg b, `▄`. `t == b` → fg t, `█`.
- `t != b`: **Full** → fg t + bg b (`\x1b[48;2;r;g;bm`), `▀`. **FgOnly** → keep the non-outline pixel: if `t` is `OUTLINE` then `▄` in b, else `▀` in t.

**Test card** (labels plain, 24 cols, 6 rows):
- r0 `A ` + 10× `▀` (fg CYAN, bg PINK) + ` fg+bg  `
- r1 `B ` + 10 spaces on bg YELLOW + ` bg     `
- r2 `C ` + `▀▄█▀▄█▀▄█▀` (fg CYAN) + ` fg     `
- r3 `D ` + 10× `▀` with fg CYAN→PINK and bg PINK→CYAN gradient (`pixel::mix`) + 8 spaces
- r4–r5 `E ` / `  ` + `encode(Full)` of a 10×4 px pink heart with one yellow highlight px + 12 spaces

- [ ] **Step 1: Failing unit tests** in `px/mod.rs`:
```rust
#[test] fn mode_parse() {
    use PixelMode::*;
    assert_eq!(mode_from(None, None, Off), Off);
    assert_eq!(mode_from(Some("1"), None, Off), Full);
    assert_eq!(mode_from(Some("fg"), None, Off), FgOnly);
    assert_eq!(mode_from(Some("test"), None, Off), Test);
    assert_eq!(mode_from(Some("0"), None, Full), Off);
    assert_eq!(mode_from(Some("1"), Some("1"), Full), Off);   // NO_COLOR wins
    assert_eq!(mode_from(None, Some(""), Full), Full);        // empty NO_COLOR ignored
    assert_eq!(mode_from(Some("junk"), None, Full), Full);
}
#[test] fn full_cell_is_fg_top_bg_bottom() {
    let mut c = PxCanvas::new(1); c.set(0, 0, CYAN); c.set(0, 1, PINK);
    let r = encode(&c, PixelMode::Full);
    assert_eq!(r[0], "\x1b[0m\x1b[38;2;10;189;198m\x1b[48;2;234;0;217m▀\x1b[0m");
    assert!(r[1..].iter().all(|row| row == " \x1b[0m"));
}
#[test] fn fg_only_never_emits_bg() {
    let mut c = PxCanvas::new(3); c.set(0, 0, CYAN); c.set(0, 1, PINK); c.set(1, 1, CYAN); c.set(2, 0, CYAN); c.set(2, 1, CYAN);
    let r = encode(&c, PixelMode::FgOnly);
    assert!(!r[0].contains("\x1b[48;")); assert!(r[0].contains('▄') && r[0].contains('█'));
}
#[test] fn hygiene_sweep() { // random canvases, both modes
    let mut rng = crate::identity::Lcg::new(7);
    for w in [1usize, 24, 60, 120] { for _ in 0..50 {
        let mut c = PxCanvas::new(w);
        for _ in 0..w * 4 { let v = rng.next(); c.set((v % w as u64) as i32, ((v >> 8) % 12) as i32, Rgb(v as u8, (v >> 16) as u8, 9)); }
        for m in [PixelMode::Full, PixelMode::FgOnly] { for row in encode(&c, m) {
            assert_eq!(crate::span::strip_ansi(&row).chars().count(), w);
            assert!(row.ends_with("\x1b[0m"));
            // a space is never emitted while a bg is active
            let mut bg = false; let mut i = 0; let b = row.as_bytes();
            while i < b.len() { if b[i] == 0x1b { let j = row[i..].find('m').unwrap() + i; let seq = &row[i..=j];
                if seq == "\x1b[0m" { bg = false } else if seq.starts_with("\x1b[48;") { bg = true } i = j + 1; }
                else { if b[i] == b' ' { assert!(!bg, "space under live bg: {row:?}") } i += 1; } }
        } }
    } }
}
#[test] fn test_card_shape() { let r = test_card(); assert_eq!(r.len(), 6);
    for row in &r { assert_eq!(crate::span::strip_ansi(row).chars().count(), 24); assert!(row.ends_with("\x1b[0m")); }
    assert!(r[0].contains("\x1b[48;2;234;0;217m")); }
```
- [ ] **Step 2:** `cargo test px::` → FAIL (module missing).
- [ ] **Step 3: Implement** `px/mod.rs` per the rules above. To use `pixel::mix` and `pixel::OUTLINE`, make them `pub(crate)` (visibility only, so `pixel_hashes.txt` stays unchanged).
- [ ] **Step 4: Wire it up.** In `main.rs`, `"pixels"` with arg `test` prints `test_card()` rows (`\n`-joined), and any other arg prints `usage: purrgotchi pixels test` and exits 2. In `statusline::run()`, just before the compose loop:
```rust
let px_rows: Option<Vec<String>> = match (crate::px::mode(), crate::playground::strip_width()) {
    (crate::px::PixelMode::Test, Some(_)) => Some(crate::px::test_card()),
    _ => None,
};
```
In the loop, rows `i < 6` use `px_rows[i]` when `Some`, before the scene/sprite branches. The box code stays untouched.
- [ ] **Step 5: Golden.** Add `dir: &'static str` to `Case` (default `"statusline"`). Add `fn px_cases() -> Vec<Case>` with `Case { dir: "statusline_px", env: &[("PURR_PIXELS", "test")], ..case("pixel_test_card") }`. `statusline_goldens` and the recorder iterate over `cases().into_iter().chain(px_cases())` and use `{dir}/{name}.ansi`. Record only the new file: `cargo test --test golden_statusline -- --ignored record_missing_statusline_goldens`. Then `/bin/cat rust/tests/golden/statusline_px/pixel_test_card.ansi` in a truecolor terminal and eyeball it.
- [ ] **Step 6:** `cargo test` → all pass. `git status rust/tests/golden/statusline` shows nothing (the 45 are untouched).
- [ ] **Step 7: Commit** `feat: half-block pixel encoder and PURR_PIXELS=test bg test card`.

**GATE A (human, coordinator relays):** The user adds `PURR_PIXELS=test` to `~/.claude/purrgotchi/.env`. The binary loads that file, so settings.json is untouched. First the user rebuilds and installs the binary at the path that `jq -r .statusLine.command ~/.claude/settings.json` names (`cargo build --release` + copy, or `cargo install --path rust`; settings stay read-only). Then the user eyeballs the statusline in Claude Code (a terminal at least 125 cols wide) and removes the line afterwards.
- Row A shows cyan-over-magenta halves and row B shows yellow blocks → bg works → Task 9 sets `DEFAULT_MODE = Full`.
- Row A shows only cyan tops and row B is blank → Task 9 sets `DEFAULT_MODE = FgOnly`.

Tasks 2–8 do not depend on the answer: both modes are built and tested.

---

### Task 2: Remove the mod and mod-only surface [sonnet]

**Files:** Delete `mod/`, `.claude-plugin/`, `rust/tests/json_serve.rs`. Modify `rust/src/main.rs` (drop the `serve`, `cost`, `quip-prompt`, `quip-store` arms and their usage-line entries), `rust/src/frame.rs` (drop `serve` and its helpers/tests), `rust/src/cost_acc.rs` (drop `run_cmd`, keep `accumulate_from_payload`), `rust/src/react.rs` + `rust/src/quip_cache.rs` (drop `quip_prompt_cmd`/`quip_store_cmd` and code only they use), `rust/src/json.rs` (drop types only those used), `rust/tests/json_small_cmds.rs` + `rust/tests/json_error_paths.rs` (drop cost/quip cases), `install.sh` (reword the comments/help on lines ~14, 31, 42–47 that mention the mod; behaviour unchanged), `README.md`, `CHANGELOG.md`, `.env.example` (grep).

- [ ] **Step 1:** `git rm -r mod .claude-plugin rust/tests/json_serve.rs`.
- [ ] **Step 2:** Remove the arms and functions above. Then `cargo build 2>&1 | grep -E "warning|error"` and delete dead code until the build is clean. Do **not** touch `frame --json`/`--pixels`, `pixel.rs` or `statusline.rs`.
- [ ] **Step 3:** Fix the tests. In `json_error_paths.rs`, drop the `cost`/`quip-prompt` entries from the arg tables (lines ~14–15, 69, 85, 96). In `json_small_cmds.rs`, drop the cost tests. Add one regression test that the removed verbs now hit the usage path:
```rust
#[test] fn removed_mod_verbs_are_unknown() {
    let sb = Sandbox::with_state("removed", json!({}));
    for v in ["serve", "cost", "quip-prompt", "quip-store"] {
        let out = sb.run(&[v], "", &[]);
        assert_eq!(out.status.code(), Some(2), "{v}");
    }
}
```
- [ ] **Step 4: Docs.**
  - README: line 3 becomes "A cyberpunk Claude Code statusline with a persistent tamagotchi kitten". Delete line 5, the mod requirement (line 15), "Install as a mod" through Troubleshooting (~lines 24–105), the mod migration sentence in the classic install section, and the `serve`/`cost`/`quip-*` rows. The `frame` row becomes "Read-only JSON snapshot (scripting/debug)".
  - CHANGELOG: rewrite the `v4.0.0` entry around the classic statusline. Keep the JSON surface, `frame`, `PURR_STATE_DIR` and hardening bullets. Remove every mod/slash-command/pane/theme/serve/quip claim. Leave a `### Added` list that Task 9 extends.
- [ ] **Step 5: Verify.** Run `grep -rnE "mod/|marketplace|/purr-|plugin install|serve|quip-(prompt|store)|session-model" README.md CHANGELOG.md install.sh .env.example rust/src`. The only allowed hits are unrelated words; list each in the report. Then `cargo test` → all pass. `git diff --stat HEAD -- rust/tests/golden` must be empty.
- [ ] **Step 6: Commit** `refactor!: remove the Claude Code mod and mod-only commands (serve, cost, quip-*)`.

---

### Task 3: Freeze the box: box goldens + pixels-off == classic [sonnet]

**Files:** Modify `rust/tests/common/mod.rs` and `rust/tests/golden_statusline.rs`. Create `rust/tests/golden/statusline_box/*.ansi` (45 files, derived from the existing goldens, **not** from a binary).

**Interfaces (produces):** `pub fn box_prefix(row: &str, i: usize) -> &str` in `common`, `fn run_case_with(c: &Case, extra: &[(&str, &str)]) -> (Vec<u8>, Sandbox)`, and `const CLASSIC_DIR: &str = "statusline";` (Task 9 renames it).

- [ ] **Step 1: Helper.** The split rule: in today's bytes the reset+spaces sequence occurs inside the box, so never split on it. `║` occurs only at row edges (inner separators are `│`).
```rust
/// Bytes of statusline row `i` up to and including the box's right border + reset.
/// Row 6 (bottom rail + speech bubble) is pinned whole.
pub fn box_prefix(row: &str, i: usize) -> &str {
    let end = match i { 0 => "╗\x1b[0m", 1..=5 => "║\x1b[0m", _ => return row };
    let at = row.rfind(end).unwrap_or_else(|| panic!("row {i}: no right border in {row:?}"));
    &row[..at + end.len()]
}
```
- [ ] **Step 2: Derive the box goldens once** with an ignored test that reads `golden/statusline/<name>.ansi` and writes `statusline_box/<name>.ansi` through `record_missing_golden` (it never overwrites):
```rust
fn box_rows(text: &str) -> String {
    text.lines().enumerate().map(|(i, l)| common::box_prefix(l, i)).collect::<Vec<_>>().join("\n") + "\n"
}
#[test] #[ignore = "one-off: derive statusline_box/ from the classic goldens"]
fn derive_box_goldens() {
    for c in cases() {
        let classic = std::fs::read_to_string(common::golden_path(&format!("statusline/{}.ansi", c.name))).unwrap();
        common::record_missing_golden(&format!("statusline_box/{}.ansi", c.name), box_rows(&classic).as_bytes());
    }
}
```
(Make `golden_path` `pub` if it is not.) Run it: `cargo test --test golden_statusline -- --ignored derive_box_goldens`. Check: `ls rust/tests/golden/statusline_box | wc -l` → 45. Each file has 7 lines, and lines 1–6 end with `╗`/`║` + reset.
- [ ] **Step 3: Permanent tests.**
```rust
const CLASSIC_DIR: &str = "statusline";
#[test] fn statusline_box_goldens() {
    for c in cases() {
        for extra in [&[][..], &[("PURR_PIXELS", "1")][..], &[("PURR_PIXELS", "fg")][..]] {
            let (bytes, _sb) = run_case_with(&c, extra);
            assert_golden(&format!("statusline_box/{}.ansi", c.name), box_rows(&String::from_utf8(bytes).unwrap()).as_bytes());
        }
    }
}
#[test] fn statusline_pixels_off_is_classic() {
    for c in cases() {
        for extra in [&[("PURR_PIXELS", "0")][..], &[("NO_COLOR", "1"), ("PURR_PIXELS", "1")][..]] {
            let (bytes, _sb) = run_case_with(&c, extra);
            assert_golden(&format!("{CLASSIC_DIR}/{}.ansi", c.name), &bytes);
        }
    }
}
```
`run_case` becomes `run_case_with(c, &[])`. `extra` is appended after `c.env` so it wins.
- [ ] **Step 4:** `cargo test --test golden_statusline` → PASS (no renderer changed yet). Then prove the guard bites: temporarily change one `═` in `h7`, run, and expect FAIL in `statusline_box_goldens`. Revert.
- [ ] **Step 5: Commit** `test: freeze the HUD box per row and pin PURR_PIXELS=0 / NO_COLOR to the classic bytes`.

---

### Task 4: Art pipeline + core poses + preview tools [opus]

**Files:** Create `rust/src/px/art.rs`, `rust/src/px/paint.rs`, `rust/tools/px_preview.py`, `rust/tests/golden/pxcat_hashes.txt`. Modify `rust/src/pixel.rs` (visibility only, all `pub(crate)`: `breed()`, `Breed` + its fields, the `Rarity` enum + `parse/colour`, `mix/shade/light/lum`, `OUTLINE`, `NOSE`, `INNER_EAR`, `VOID_INNER_EAR`, `LID_ON_DARK`, `EPIC`, `NEON_CYAN`, `NEON_MAGENTA`, `WHITE`, `hash`) and `rust/src/main.rs` (`pixels sheet [--breed B] [--json]`).

**Interfaces (produces):**
```rust
// art.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub(crate) enum Pose { Sit, Walk1, Walk2, Loaf, LoafBreath, Curl, CurlBreath,
  GroomPaw, GroomLick, Stretch, Turn, Look, Meow, Run1, Run2, Crouch, Pounce, Roll1, Roll2, Cheer, Wave }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub(crate) enum Tail { Curl, Up, Flick }
pub(crate) const ALL_POSES: [Pose; 21];
pub(crate) fn pose_map(p: Pose) -> &'static [&'static str];      // interior slot map, faces right
pub(crate) fn stage_rows<'a>(map: &'a [&'a str], stage: &str) -> Vec<&'a str>;
// paint.rs
pub(crate) struct CatLook<'a> { pub breed: &'a str, pub rarity: &'a str, pub shiny: bool, pub stage: &'a str, pub tick: u64 }
pub(crate) fn paint(p: Pose, look: &CatLook, facing_right: bool, tail: Tail, blink: bool) -> PxSprite;
pub(crate) const CAT_MAX_W: usize = 16;   // sprite incl. outline, every pose and stage
```
Task 4 draws `Sit`, `Walk1`, `Walk2`. The other variants map to `Sit` until Task 5, and a test marks them as such.

**Slot legend** (interior maps only, since the outline is generated): `.` transparent, `h` head coat, `b` body coat, `a` paw (light), `r` ear outer, `i` inner ear, `l` chest/muzzle light, `e` eye, `n` nose, `m` dark line (mouth/paw split, uses the outline colour), `*` tail socket (paints as `b`; Sit/Loaf/Look/Cheer/Wave draw the tail layer from here). A row prefixed `-` is droppable: young drops the first `-` row and kitten drops all of them. The head keeps its size, so kittens read chibi.

**Size budget:** interior ≤ 14 wide × 10 tall (adult), so the outlined sprite is ≤ 16 × 12. Ground = the sprite's last row sits on canvas px row 11. Heights with outline: adult 12, young 11, kitten 10.

**Starting map for SIT** (adult, faces right, 3/4 face toward the viewer, nose offset toward facing). The art task may redraw it in the preview loop but keeps the legend, budget and ground anchor:
```
....r...r...
...rir.rir..
...hhhhhhhh.
...hehhhheh.
...hhhlnlhh.
-....hllllh..
-..bbbbllb...
.bbbbbbllb..
*bbbbbbbbb..
.bbbbamama..
```
**Walk1/Walk2:** a horizontal body (≈ 13×8 interior). The head is at the right end, raised one row above the back, with the same 2-eye face. Four legs: Walk1 has legs apart (front-left and back-right forward), Walk2 has legs together under the body. The tail is part of the map, a straight-back line with the tip up one px. Keep the ears and eye positions identical across both frames so only the legs move.

**Tail layer** (offsets from `*`, faces right, so −x points behind the cat; each px is a `t` coat px, two px thick for `bushy` breeds): `Curl` = (−1,0),(−2,1),(−3,1),(−4,0) along the ground. `Up` = (−1,0),(−2,−1),(−2,−2),(−2,−3),(−1,−4), a question mark. `Flick` = the same as Up but the last px goes to (−3,−4).

**Paint pipeline** (order is load-bearing):
1. **Slots → colour** from `pixel::breed(look.breed)`. Unknown or empty breed falls back to Ginger. `h/b/t` = base. `l/a` = `light`, else `mix(base, WHITE, 0.25)`. `r` = base. `i` = `INNER_EAR` (Void: `VOID_INNER_EAR`). `e` = `Rarity::parse(rarity).colour(tick)` (hetero breeds: the right eye is `NEON_MAGENTA`). `n` = `NOSE`. `m` = `OUTLINE`.
2. **Pattern**, on `h/b/t/r` in map coordinates:
   - Tabby: `(x+y)%3==0` → mark (b/t); top head row with `x%2==0` → mark.
   - Points: `r/t/a` → mark; `l` → `mix(base, mark, 0.6)`.
   - Spots/Rosettes: `hash(x,y,3)>0.78` → mark (b/t).
   - Van: `r`, `t` and the top head row → mark.
   - Calico: `hash(x/3,y/3,7)>0.6` → mark, `>0.85` → mark2.
   - Void: `hash(x,y,9)>0.93` → `NEON_CYAN`/`NEON_MAGENTA` by `hash(x,y,4)`.
   - Split: `x < map_w/2` → base, else mark.
   - Tuxedo/Solid/Wrinkles: none.
3. **Flip** horizontally when `!facing_right`.
4. **Cel shade (2-tone):** a coat px with a transparent px above → `light(c, 0.12)`. With a transparent px below or to the right → `shade(c, 0.25)`.
5. **Rim light**, on rows with ≥ 3 coat px: leftmost → `mix(c, NEON_CYAN, 0.45)`, rightmost → `mix(c, NEON_MAGENTA, 0.45)`. This applies after the flip, so the rim is always cyan on the left.
6. **Auto-outline:** every transparent px 4-adjacent to a filled px → `OUTLINE`.
7. **Blink:** `e` px → `OUTLINE`, or `pixel::LID_ON_DARK` when `lum(under) < 0.3`.

- [ ] **Step 1: Failing tests** (`paint.rs`):
```rust
fn look(b: &'static str, st: &'static str) -> CatLook<'static> { CatLook { breed: b, rarity: "common", shiny: false, stage: st, tick: 0 } }
#[test] fn sizes_and_ground() { for p in ALL_POSES { for st in ["kitten", "young", "adult"] { for f in [true, false] {
    let s = paint(p, &look("Ginger", st), f, Tail::Curl, false);
    assert!(s.w <= CAT_MAX_W && s.h <= 12, "{p:?} {st}");
    assert!((0..s.w).any(|x| s.px[(s.h - 1) * s.w + x].is_some()), "{p:?} {st}: nothing on the ground row");
} } } }
#[test] fn stages_scale() { let h = |st| paint(Pose::Sit, &look("Ginger", st), true, Tail::Curl, false).h;
    assert!(h("kitten") < h("young") && h("young") < h("adult")); }
#[test] fn flip_mirrors_silhouette() { let r = paint(Pose::Walk1, &look("Tabby", "adult"), true, Tail::Curl, false);
    let l = paint(Pose::Walk1, &look("Tabby", "adult"), false, Tail::Curl, false);
    for y in 0..r.h { for x in 0..r.w { assert_eq!(r.px[y * r.w + x].is_some(), l.px[y * l.w + (l.w - 1 - x)].is_some()); } } }
#[test] fn rim_signature() { let s = paint(Pose::Sit, &look("Ginger", "adult"), true, Tail::Curl, false);
    let row = &s.px[4 * s.w..5 * s.w]; let coat: Vec<_> = row.iter().flatten().filter(|c| **c != crate::pixel::OUTLINE).collect();
    let (l, r) = (coat[0], coat[coat.len() - 1]); assert!(l.1 > l.0, "left not cyan {l:?}"); assert!(r.0 > r.1, "right not magenta {r:?}"); }
#[test] fn blink_hides_eyes() { let eye = crate::colors::CYAN;
    assert!(paint(Pose::Sit, &look("Tabby", "adult"), true, Tail::Curl, false).px.contains(&Some(eye)));
    assert!(!paint(Pose::Sit, &look("Tabby", "adult"), true, Tail::Curl, true).px.contains(&Some(eye))); }
#[test] fn unknown_or_empty_breed_is_ginger() { let g = paint(Pose::Sit, &look("Ginger", "adult"), true, Tail::Up, false).px;
    assert_eq!(paint(Pose::Sit, &look("", "adult"), true, Tail::Up, false).px, g);
    assert_eq!(paint(Pose::Sit, &look("Dragon", "elder"), true, Tail::Up, false).px, g); }
```
plus `art.rs` tests: every map row in a pose has equal width, only legend chars appear, the interior is ≤ 14×10, and there is exactly one `*` in Sit/Loaf/Look/Cheer/Wave.
- [ ] **Step 2:** `cargo test px::` → FAIL.
- [ ] **Step 3: Implement** `art.rs`, `paint.rs` and the `pixels sheet` dev verb. The verb prints, for a breed (default Ginger), every pose × stage × facing as ANSI blocks with a label line. With `--json` it prints `{"cols":6,"cells":[{"label":..,"px":[[hex|null]]}]}` (the `pixel.rs::sheet_json` shape).
- [ ] **Step 4: Preview tool.** `rust/tools/px_preview.py` is `preview.py` with the cell size taken from the largest sprite (`cw = max w + 2`), scale 10, and labels drawn under each cell. Run:
```bash
cd rust && cargo build -q && SCR=/tmp/claude-1000/-home-englader-Desktop-Work-Personal-purrgotchi/75d9d16e-8f90-4059-866a-4511be028341/scratchpad
for b in Ginger Tuxedo Siamese "Void Cat" Calico Chimera; do ./target/debug/purrgotchi pixels sheet --breed "$b" --json | python3 -I tools/px_preview.py "$SCR/px-$b.png"; done
./target/debug/purrgotchi pixels sheet   # ANSI, eyeball in a truecolor terminal
```
(Use the coordinator-provided scratch dir if it differs.) Iterate on the maps until the cat reads as a chibi side-view cat with its face toward the viewer.
- [ ] **Step 5: Art snapshot.** `pxcat_hashes.txt` holds one line per `(pose, stage, facing, tail, blink)` for Ginger (21×3×2 + tail/blink variants for Sit), plus Sit/adult/right for all 16 breeds and for legendary ticks 0..6. Format `pose|stage|facing|tail|blink|breed|tick <fnv1a hex>`, with the `UPDATE_GOLDEN=1` record pattern copied from `pixel.rs::snapshot_all_combinations`. Record it now (`UPDATE_GOLDEN=1 cargo test pxcat`). Task 5 re-records it once the art is final.
- [ ] **Step 6:** `cargo test` → all pass, and `pixel_hashes.txt` is unchanged.
- [ ] **Step 7: Commit** `feat: pixel cat paint pipeline, core poses (sit, walk) and preview tools`.

**GATE B (human):** The coordinator shows the user the PNGs (sit + walk for 6 breeds × 3 stages). The user approves the look or gives notes before Task 5 multiplies the art.

---

### Task 5: Remaining poses + props [opus]

**Files:** Modify `rust/src/px/art.rs` and `rust/src/px/paint.rs`. Create the prop art in `art.rs`. Re-record `pxcat_hashes.txt`.

**Interfaces (produces):**
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub(crate) enum Prop { Ball, Butterfly, Yarn, Coin, Heart, Hat, Sparkle, Confetti, Flame, Z }
pub(crate) fn prop(p: Prop, frame: u8) -> PxSprite;   // already coloured, outline included where noted
```
**Pose specs** (same legend, budget and anchor; each pose keeps the Sit head where it can, so pose changes read as one cat):

| Pose | Silhouette rule |
|---|---|
| Loaf / LoafBreath | Sit head lowered 2 rows, body a flat oval 12×5, paws hidden. Breath frame: back +1 px |
| Curl / CurlBreath | Donut 11×6 seen from the side, head tucked at the right, eyes closed (`m`), tail wrapped along the front. Breath: top row +1 px |
| GroomPaw / GroomLick | Sit. GroomPaw: front paw raised to the mouth (2 `a` px at the nose row). GroomLick: head tilted down 1 row, tongue 1 px `n` |
| Stretch | Front legs extended forward on the ground, chest low, rear up, tail up. Width ≤ 14 interior |
| Turn | Front view: symmetric head, body foreshortened 8 wide, used for one tick between facings |
| Look / Meow | Sit with the head raised 1 row and ears tall (+1 `r`). Meow: mouth 2 px `m` open |
| Run1 / Run2 | Stretched body 14×6: Run1 legs fully extended front/back, Run2 legs gathered under. Tail straight back |
| Crouch | Run body lowered to 4 rows, rear high, eyes wide: the anticipation frame |
| Pounce | Body diagonal (rear low-left, head high-right), front paws forward/down, `>`-shaped squint eyes (`m` px) |
| Roll1 / Roll2 | Roll1 on its back, 4 paws up. Roll2 on its side, belly toward the viewer. Both 13×6 |
| Cheer / Wave | Sit with happy eyes (`^^`: `m` px in an arch) and tail Up. Wave: one front paw raised above the head |

**Props** (`frame` toggles 2-frame flaps/flickers):
- Ball 3×3: yellow with a 1 px white highlight.
- Butterfly 5×3: magenta wings, 1 px dark body. Frame 1 has the wings folded (3×3).
- Yarn 4×4: pink with a dark strand diagonal.
- Coin 4×4: `pixel::EPIC` gold. Frame 1 has a white glint.
- Heart 5×4: pink.
- Hat 3×4: magenta cone, yellow pom-pom.
- Sparkle 3×3: a `+` in white. Frame 1 is the centre only.
- Confetti 1×1: colour from `colors::RAINBOW`, chosen by the caller.
- Flame 2×3: orange/yellow. Frame 1 is 1 px shorter.
- Z 3×3: `NEON_CYAN` z. Frame 1 is a dimmer 2×2.

- [ ] **Step 1: Failing tests.** Every pose has its own map (`pose_map(p)` differs from `pose_map(Pose::Sit)` for every `p != Sit`). Each breath/flap pair differs in at most 6 px. Run2/Walk2 keep the head px identical to Run1/Walk1. Every `prop` stays within its stated size and is deterministic.
```rust
#[test] fn every_pose_is_drawn() { for p in ALL_POSES { if p != Pose::Sit { assert_ne!(pose_map(p), pose_map(Pose::Sit), "{p:?}"); } } }
#[test] fn breath_is_subtle() { let l = |p| paint(p, &look("Ginger", "adult"), true, Tail::Curl, false).px;
    for (a, b) in [(Pose::Loaf, Pose::LoafBreath), (Pose::Curl, Pose::CurlBreath)] {
        let d = l(a).iter().zip(l(b).iter()).filter(|(x, y)| x != y).count(); assert!(d > 0 && d <= 6, "{a:?}: {d}"); } }
```
- [ ] **Step 2:** FAIL. **Step 3:** Draw them, iterating with `pixels sheet` + `px_preview.py` (as in Task 4, Step 4).
- [ ] **Step 4:** Re-record `pxcat_hashes.txt` with `UPDATE_GOLDEN=1 cargo test pxcat`, then `cargo test` → pass.
- [ ] **Step 5: Commit** `feat: pixel cat poses (loaf, sleep, groom, stretch, turn, look, run, pounce, roll, cheer) and props`.

---

### Task 6: Idle life state machine + opt-in statusline wiring [opus]

**Files:** Create `rust/src/px/life.rs` and `rust/tools/px_anim.sh`. Modify `rust/src/px/mod.rs` (`cat_area`), `rust/src/statusline.rs` (use `cat_area` for Full/FgOnly), and `rust/tests/golden_statusline.rs` (`px_cases`).

**Interfaces:**
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub(crate) enum Behaviour { Sit, Loaf, Sleep, Groom, Stretch, Look, Wander, Zoomies }
pub(crate) struct CatFrame { pub pose: Pose, pub x: i32, pub facing_right: bool, pub tail: Tail, pub blink: bool, pub z: Option<u8> }
pub(crate) fn life_frame(now: u64, seed: u64, mood: &str, span: usize, forced: Option<Behaviour>) -> CatFrame;
pub(crate) fn force_pose_from(v: Option<&str>) -> Option<Behaviour>;   // PURR_FORCE_POSE=sit|loaf|sleep|groom|stretch|look|wander|zoomies
// mod.rs: life only for now. Tasks 7-8 add the scene and celebration branches.
pub(crate) fn cat_area(view: &crate::statusline::KittenView, st: &crate::state::State, now: u64, width: usize, mode: PixelMode) -> Vec<String>;
```
`span = width − CAT_MAX_W`. `seed = st.birth_time`. `x` is the sprite's left px, always in `0..=span`.

**Schedule (pure, O(1), no state writes):**
- `BLOCK = 30` s, `b = now / BLOCK`, `t = now % BLOCK`.
- **Anchor:** `X(b) = tri(b.wrapping_mul(12).wrapping_add(seed % 97), span)` (reuse `playground::tri`, made `pub(crate)`). Consecutive anchors differ by ≤ 12 px, so motion is continuous by construction.
- **Facing:** `out(b) = X(b+1) > X(b)` if they differ, else `ihash(b) & 1`. `in(b) = out(b−1)`.
- **Block plan** (`Lcg::new(b ^ seed ^ 0x4C494645)`): 
  1. `Turn` for 1 s if `in(b)` ≠ the walk direction.
  2. `Wander` from `X(b)` to `X(b+1)` at 2 px/s (`Walk1`/`Walk2` alternating per second, last tick `Sit`).
  3. Pick 1 for 6–14 s, weighted by mood.
  4. Pick 2 fills the rest. If the rest is < 4 s, it merges into pick 1.
- **Mood weights** (Sit, Loaf, Sleep, Groom, Stretch, Look, Zoomies):
  - calm 30/20/0/15/10/10/0 (+15 extra Wander picks: Turn, walk 4 px against `out(b)`, Turn, walk back, so the block still ends facing `out(b)`)
  - happy 20/10/0/15/15/10/0 + 30 Wander
  - alert 25/5/0/5/0/40/0 + 25 Wander
  - hungry 30/10/0/0/0/50/0 (Look alternates Look/Meow)
  - panic 0/0/0/0/0/30/50 + 20 Wander
  - sleep: the whole block is `Sleep` at a fixed bed `x = span / 3`, every 4th block opens with a 2 s Stretch.
  
  Entering sleep mood jumps the cat to the bed once, which is accepted (a once-per-6 h mood flip).
- **Keyframes (1 fps):**
  - Sit: tail Curl, with Flick for 1 s when `ihash(now) % 5 == 0`.
  - Loaf: Sit 1 s → Loaf/LoafBreath alternating every 2 s → Sit 1 s.
  - Sleep: Loaf 1 s → Curl/CurlBreath every 2 s, `z = Some(t % 3)` (a rising Z, drawn by `cat_area`).
  - Groom: Sit → GroomPaw/GroomLick alternating → Sit.
  - Stretch: Crouch 1 → Stretch 2 → Sit 1.
  - Look: Sit 1 → Look (hungry: Look/Meow alternating).
  - Zoomies: Run1/Run2 with `x = clamp(X(b+1) + tri(k·4, 16), 0, span)` for `k = 0..=8` (9 ticks, ending on the anchor), facing = `tri_rising`. The cat ends where it started. If the cat ends facing away from `out(b)`, a 1 s Turn follows.
- **Blink:** when `ihash(now ^ seed) % 7 == 0`, for poses with open eyes. `ihash(u64) -> u64` is a small integer mixer in `life.rs` (splitmix64 finaliser). Use it for every `hash(..) % n` in this task (Sit flick: `ihash(now) % 5 == 0`, facing tie-break `ihash(b) & 1`). `pixel::hash` is f64 and takes i32 inputs, so keep it for paint patterns only.
- `forced` replaces the picks (anchor, wander and facing unchanged). Sleep under `forced` uses the anchor x, not the bed.

**`cat_area` (life branch):**
1. Canvas = `PxCanvas::new(width)`.
2. `paint(pose, look, facing, tail, blink)` blitted at `(x, 12 − h)`.
3. Tail layer as in Task 4.
4. When `z` is `Some(k)`, `Prop::Z` at `(x + w − 2, 4 − 2k)`.
5. Shiny: a `Prop::Sparkle` at a hash-picked empty spot near the cat, on even seconds.
6. `encode(mode)`.

In the statusline, `px_rows` = `Some(cat_area(...))` when the mode is Full/FgOnly and `strip_width()` is `Some`. The Test branch stays.

- [ ] **Step 1: Failing tests** (`life.rs`):
```rust
const MOODS: [&str; 6] = ["calm", "happy", "alert", "hungry", "panic", "sleep"];
#[test] fn continuous_and_in_bounds() { for m in MOODS { for span in [0usize, 8, 44, 104] {
    let mut prev = life_frame(1_800_000_000, 42, m, span, None);
    for now in 1_800_000_001..1_800_003_600u64 { let f = life_frame(now, 42, m, span, None);
        assert!((0..=span as i32).contains(&f.x), "{m} {span} {now}");
        assert!((f.x - prev.x).abs() <= 4, "{m} teleport at {now}: {} -> {}", prev.x, f.x);   // zoomies peak = 4 px/s
        if f.facing_right != prev.facing_right { assert!(f.pose == Pose::Turn || prev.pose == Pose::Turn || matches!(f.pose, Pose::Run1 | Pose::Run2), "{m} flip without turn at {now}"); }
        prev = f; } } } }
#[test] fn mood_biases_behaviour() { let count = |m: &str, p: &[Pose]| (0..3600u64).filter(|t| p.contains(&life_frame(1_800_000_000 + t, 7, m, 44, None).pose)).count();
    assert!(count("sleep", &[Pose::Curl, Pose::CurlBreath]) > 2800);
    assert!(count("panic", &[Pose::Run1, Pose::Run2]) > 600);
    assert!(count("hungry", &[Pose::Look, Pose::Meow]) > count("calm", &[Pose::Look, Pose::Meow])); }
#[test] fn every_behaviour_reachable_in_a_day() { for m in ["calm", "happy"] { let mut seen = std::collections::HashSet::new();
    for t in 0..86_400u64 { seen.insert(life_frame(t, 3, m, 44, None).pose); }
    for p in [Pose::Sit, Pose::Walk1, Pose::Loaf, Pose::GroomLick, Pose::Stretch, Pose::Turn] { assert!(seen.contains(&p), "{m} never {p:?}"); } } }
#[test] fn extreme_seeds_and_clock_do_not_panic() { for seed in [0, u64::MAX] { for now in [0, 29, 30, u64::MAX / 2, u64::MAX - 31] { let _ = life_frame(now, seed, "calm", 44, None); } } }
#[test] fn forced_pose_parses_and_wins() { assert_eq!(force_pose_from(Some("groom")), Some(Behaviour::Groom)); assert_eq!(force_pose_from(Some("x")), None);
    let f = life_frame(1_800_000_020, 1, "calm", 44, Some(Behaviour::Sleep)); assert!(matches!(f.pose, Pose::Curl | Pose::CurlBreath | Pose::Loaf)); }
```
and `cat_area` tests: rows == 6, plain width == `width` for widths {24, 60, 120} × 6 moods × 120 s × both modes; rows end with a reset.
- [ ] **Step 2:** FAIL. **Step 3:** Implement, and wire up `PURR_FORCE_POSE` (dev backdoor, keep it out of the README like `PURR_FORCE_SCENE`).
- [ ] **Step 4: `px_anim.sh`**, a 1 fps player that never touches real state:
```bash
#!/usr/bin/env bash
# usage: rust/tools/px_anim.sh [--mood calm|happy|alert|hungry|panic|sleep] [--secs N] [--fast] [--pose P] [--scene S]
set -euo pipefail; BIN="$(dirname "$0")/../target/debug/purrgotchi"; T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
export HOME="$T" PURR_STATE_DIR="$T/state" PURR_PIXELS="${PURR_PIXELS:-1}" COLUMNS="${COLUMNS:-170}"
# flags → MOOD/SECS/FAST/PURR_FORCE_POSE/PURR_FORCE_SCENE; hatch, then patch last_fed_at/last_pet_at with jq for the mood;
# ctx for alert/panic goes in the payload's context_window.used_percentage
NOW0=1800000000; for ((i=0;i<SECS;i++)); do printf '\033[H\033[2J'; PURR_NOW=$((NOW0+i)) "$BIN" statusline <<<"$PAYLOAD"; [ -n "$FAST" ] || sleep 1; done
```
(Fill in the elided flag parsing in the script. It is a dev tool, so keep it short.)
- [ ] **Step 5: Goldens.** In `px_cases()` (dir `statusline_px`, env `PURR_PIXELS=1`), add `px_life_calm`, `px_life_sleep_epic` (state from `sleep_epic`), `px_life_panic` (ctx 92), `px_life_young_rare_frame1` (state from `alert_rare_young_peak_frame1`), `px_life_fg_mode` (`PURR_PIXELS=fg`), `px_life_forced_groom` (`PURR_FORCE_POSE=groom`), `px_life_narrow_is_ascii` (`COLUMNS=120`, every other field copied from `calm_common_offpeak` including its anchor; its bytes must equal `{CLASSIC_DIR}/calm_common_offpeak.ansi`, so assert that directly too). Record them with the recorder, then eyeball them with `/bin/cat`.
- [ ] **Step 6:** `bash rust/tools/px_anim.sh --mood calm --secs 90` and `--mood sleep --secs 30`, watched in a terminal (the coordinator may ask the user to watch). `cargo test` → pass. The classic and box goldens are unchanged.
- [ ] **Step 7: Commit** `feat: pixel cat idle life (sit, loaf, sleep, groom, stretch, look, wander, zoomies) behind PURR_PIXELS=1`.

---

### Task 7: Pixel playground scenes [sonnet]

**Files:** Create `rust/src/px/scene.rs`. Modify `rust/src/playground.rs` (make `tri`, `tri_rising` and the speed/lag constants `pub(crate)`, no behaviour change), `rust/src/px/mod.rs` (scene branch in `cat_area`), and `rust/tests/golden_statusline.rs`.

**Interface:** `pub(crate) fn scene_canvas(kind: PlaygroundScene, elapsed: u64, duration: u64, seed: u64, width: usize, look: &CatLook) -> PxCanvas;` and `pub(crate) fn scene_pose(kind: PlaygroundScene, elapsed: u64, duration: u64, seed: u64, width: usize) -> Pose;` (the cat pose, which `scene_canvas` uses and the tests call). `cat_area` uses it when `view.scene` is `Some` (so `kitten_view` still decides visibility and speech).

**Rules:** mirror each `playground.rs` scene function line for line. Swap `CAT_W` for `CAT_MAX_W`, cells for px on x, and art keys for poses (`run_right/left` → `Run1/Run2` by `elapsed % 2` with facing; `sit` → `Sit`; `roll` → `Roll1/Roll2` by `elapsed % 2`; `pounce` → `Pounce`). Ground = px row 11, and a cell row `r` maps to px y = `2r`.
- **Ball:** `Prop::Ball` at px y 9 (even `elapsed`) or y 5 (odd).
- **Butterfly:** `Prop::Butterfly` (flap = `elapsed % 2`) at y = `2·by`. In the finale it escapes to y 0.
- **Yarn:** `Prop::Yarn` ahead of the paws at y 8, with the trail as 3 single pink px on row 11 every 2 px behind.
- **Zoomies:** dust as 3 dim-grey (`mix(OUTLINE, WHITE, .5)`) px on row 11 behind.
- **Finale:** the first finale second (`elapsed + FINALE_SECS == duration`) shows `Crouch` (anticipation), then `Pounce`. The ball sits under the front paws.

- [ ] **Step 1: Failing tests:** an alignment sweep (port `alignment_sweep_every_scene_width_and_second`: every scene × widths {24, 60, 120} × elapsed 0..=45 × (duration, seed) ∈ {(20,0),(30,12345),(40,u64::MAX)} × both modes, checking the plain width and the trailing reset), the cat moving in Run, zoomies moving faster than run, the ball being yellow mid-scene, Pounce appearing only in the finale, the butterfly in px rows 0..5, and the yarn on row 11.
```rust
#[test] fn finale_has_anticipation_then_pounce() { let poses = |e| scene_pose(PlaygroundScene::BallChase, e, 30, 7, 80);
    assert_eq!(poses(26), Pose::Crouch); assert_eq!(poses(27), Pose::Pounce); assert_ne!(poses(20), Pose::Pounce); }
```
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** Add `px_cases`: `px_scene_<kind>_mid` for all 6 (states as in the existing `scene_*` cases, `COLUMNS=160`), `px_scene_ballchase_finale` and `px_scene_disabled_still_shows_life` (`PURR_PLAYGROUND=0` → life cat, no scene). Record and eyeball them. Watch `bash rust/tools/px_anim.sh --scene ball --secs 30`.
- [ ] **Step 5:** `cargo test` → pass. `playground.rs` tests are unchanged and green.
- [ ] **Step 6: Commit** `feat: pixel versions of the playground scenes with anticipation before the pounce`.

---

### Task 8: Celebration and birthday overlays [sonnet]

**Files:** Create `rust/src/px/fx.rs`. Modify `rust/src/px/mod.rs` (top priority in `cat_area`: celebration > birthday > scene > life, mirroring `kitten_view`) and `rust/tests/golden_statusline.rs`.

**Interface:** `pub(crate) enum Overlay { Celebration(CelebrationKind), Birthday }` and `pub(crate) fn overlay_canvas(o: Overlay, now: u64, width: usize, look: &CatLook) -> PxCanvas;`.

**Rules:** The cat is centred (`x = (width − w)/2`). Its pose is `Cheer`, alternating with `Wave` for Greeting. `f = now % 2` drives 2-frame props, and `k = now % 6` drives rising/falling motion.

| Kind | Effect |
|---|---|
| LevelUp | 3 `Sparkle` rising 2 px/s on both sides of the cat |
| EvolvingYoung / EvolvingAdult | odd seconds: every coat px of the cat → `WHITE` (flash); even: normal art (new stage). 2 sparkles |
| Streak | `Flame` left and right of the cat, flickering with `f` |
| Milestone* | confetti: N px (Centurion 8, Veteran 12, Master 16, Legendary 24) from `RAINBOW`, x = `hash(i, b)·width`, falling 2 px/s, wrapping |
| Greeting | `Heart` above the head, bobbing 1 px with `f` |
| RandomDrop | `Coin` drops from y 0 to the ground beside the cat over 3 s, then glints |
| Birthday | `Hat` on the head (anchored to the sprite top, x + w/2 − 1) + 10 confetti |

- [ ] **Step 1: Failing tests:** an alignment sweep (every overlay × widths {24, 60, 120} × 12 s × both modes), every kind differing from plain Cheer at some second in 0..6, the Hat staying inside the canvas for kitten/young/adult, and confetti counts rising with milestone tier.
- [ ] **Step 2:** FAIL. **Step 3:** Implement. **Step 4:** Add `px_cases`: `px_celebration_<kind>` for all 10 kinds (states as in the existing `celebration_*` cases) + `px_birthday` + `px_quip_keeps_pixel_cat` (quip fixture: speech pink, cat unchanged). Record and eyeball them.
- [ ] **Step 5:** `cargo test` → pass.
- [ ] **Step 6: Commit** `feat: pixel celebration and birthday overlays`.

---

### Task 9: Flip the default, re-record the classic goldens, docs [sonnet]

**Files:** Modify `rust/src/px/mod.rs` (`DEFAULT_MODE` per the Gate A answer), `rust/tests/golden_statusline.rs` (`CLASSIC_DIR = "statusline_ascii"`), golden dirs, `README.md`, `CHANGELOG.md`, `.env.example`, `rust/src/main.rs` (usage line includes `pixels`).

- [ ] **Step 1: Deliberate re-record** (the only commit that rewrites the classic goldens):
```bash
cd rust
git mv tests/golden/statusline tests/golden/statusline_ascii      # classic bytes now pinned under PURR_PIXELS=0
sed -i 's/const CLASSIC_DIR: &str = "statusline";/const CLASSIC_DIR: \&str = "statusline_ascii";/' tests/golden_statusline.rs
# set DEFAULT_MODE in src/px/mod.rs (Full, or FgOnly if Gate A failed)
cargo test --test golden_statusline -- --ignored record_missing_statusline_goldens   # writes the 45 new default-mode files
cargo test                                                                           # box guard + classic pin + new goldens all green
```
Delete the one-off `derive_box_goldens` test (it would now read the pixel dir; its job is done). Change the recorder doc comment from "pre-change binary only!" to "records missing files only; a re-record = git mv/delete + run + eyeball".
- [ ] **Step 2: Verify the box is untouched.** `statusline_box_goldens` passes. As an extra check, for 3 cases diff the box prefix of `statusline/x.ansi` against `statusline_ascii/x.ansi` (Python one-liner using the `box_prefix` rule). `/bin/cat` 5 new files and eyeball them. `scene_too_narrow_shows_sprite` must be byte-identical between the two dirs (`cmp`).
- [ ] **Step 3: Docs.**
  - README: a "Your pixel cat" section (idle life list, scenes, celebrations, 1 fps, ~125+ cols wide).
  - README: `PURR_PIXELS=0|fg` and `NO_COLOR` fallback in Customize, with a note that `PURR_PIXELS=test` checks bg colour support.
  - README: a portable wrapper:
```bash
mkdir -p ~/.local/bin && cat > ~/.local/bin/purr <<'EOF'
#!/usr/bin/env bash
exec "${PURRGOTCHI_BIN:-$(command -v purrgotchi || echo "$HOME/path/to/purrgotchi/rust/target/release/purrgotchi")}" "$@"
EOF
chmod +x ~/.local/bin/purr
```
    It also needs a note that inside Claude Code you type `! purr pet` (also `play`, `treat`, `card`, `stats`, `rename <name>`).
  - README: a `pixels test|sheet` row in Subcommands.
  - CHANGELOG `v4.0.0` `### Added`: pixel-art live cat (half-block 24-bit, idle behaviours, pixel scenes, celebration overlays, ASCII fallback). `### Removed`: the mod-era verbs never shipped, so there is nothing to list; say "statusline sprite column redrawn, HUD box unchanged" under `### Changed`.
  - `.env.example`: a commented `# PURR_PIXELS=0` line.
- [ ] **Step 4:** `cargo test` and `cargo clippy --all-targets` (no new warnings vs. the branch base). `cargo build --release`.
- [ ] **Step 5: Commit** `feat!: pixel cat is the default statusline sprite (PURR_PIXELS=0 restores ASCII)`.

---

### Task 10: Live check, review, simplify [coordinator]

- [ ] **GATE C (human):** These checks are read-only. Run `jq '.hooks, .statusLine, .enabledPlugins' ~/.claude/settings.json`. If the feed/turn hooks are missing (from an earlier `--uninstall-hooks` during mod testing), tell the user to run `bash install.sh` (idempotent). If a `purrgotchi` plugin is enabled, tell the user to `claude plugin uninstall purrgotchi`. The user installs the release binary and watches the statusline for ~2 min (idle life), runs `! purr play` (a scene) and confirms the box looks unchanged.
- [ ] Whole-branch review: superpowers:requesting-code-review on `pre-v4.0.0-backup..HEAD` (Rust + docs). Fix findings in one `fix: pixel cat review fixes` commit.
- [ ] Run `/simplify` on the branch diff and commit as `refactor: simplify pixel cat`. Re-run `cargo test`. The goldens must stay unchanged unless a deliberate fix says otherwise.
- [ ] Update `docs/superpowers/plans/2026-10-17-v4.0.0-HANDOFF.md` status (one paragraph: pivot done, gates passed, known limits).

## Risks

- **NO_COLOR inherited from the user's shell** would silently give the ASCII cat. Gate C checks `echo $NO_COLOR` if the cat doesn't show.
- **Host bg support unknown.** Gate A decides. `fg` mode is the tested fallback, but it loses one colour per two-colour cell (half the vertical detail).
- **12 px is tiny.** The cat's readability depends on the art. Gate B exists for this, and if 12 px fails, the fallback is a 16 px-wide, less chibi adult.
- **Width math:** CJK/emoji names already overflow the box by ~5 cells (pre-existing). Combined with a wide stage, row 1 may wrap on exact-fit terminals. `STRIP_OVERHEAD` keeps its 2 safety cells; do not reduce it.
- **Mood-flip jumps** (into the sleep bed, and pose resets at mood changes) are accepted at 1 fps and documented in `life.rs`.
- **Golden churn:** Task 9 rewrites 45 files at once. The box guard + classic pin make that diff reviewable as "sprite column only".
