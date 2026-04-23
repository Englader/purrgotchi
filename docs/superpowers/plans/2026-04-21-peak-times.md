# Peak Times HUD Label Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Embed a dynamic Anthropic API peak/off-peak indicator in the Purrgotchi HUD top rail, centered above the W2 column, computed from local system time.

**Architecture:** All changes are in `rust/src/statusline.rs`. Two new pure functions handle the logic: `peak_period(hour)` maps a 0–23 hour to a static label pair, `peak_label(hour)` formats it as plain+ANSI string pair. The H1 rail in `run()` is split into three segments — pre-rail `═`, peak label, post-rail `═` — with the label centered at W2's visual midpoint (position 47 in the 94-char rail). Local time comes from a platform-conditional `local_hour()`.

**Tech Stack:** Rust 2024 edition — no new dependencies.

---

## File Map

| File | Change |
|---|---|
| `rust/src/statusline.rs` | Add `peak_period()`, `local_hour()`, `peak_label()`, modify H1 block, add test module |
| `rust/Cargo.toml` | Version bump `3.3.0` → `3.3.1` |

---

## Task 1: Add `peak_period()` with unit tests

**Files:**
- Modify: `rust/src/statusline.rs`

- [ ] **Step 1: Add failing test module at the bottom of statusline.rs**

Append this block after the closing brace of `read_git_branch()` (currently the last function, ending at line 312):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peak_period_peak_hours() {
        assert_eq!(peak_period(14), ("PEAK", "2PM-8PM"));
        assert_eq!(peak_period(17), ("PEAK", "2PM-8PM"));
        assert_eq!(peak_period(20), ("PEAK", "2PM-8PM"));
    }

    #[test]
    fn test_peak_period_off_peak_hours() {
        assert_eq!(peak_period(0),  ("OFF PEAK", "9PM-3PM"));
        assert_eq!(peak_period(13), ("OFF PEAK", "9PM-3PM"));
        assert_eq!(peak_period(21), ("OFF PEAK", "9PM-3PM"));
        assert_eq!(peak_period(23), ("OFF PEAK", "9PM-3PM"));
    }
}
```

- [ ] **Step 2: Run tests — expect compile failure**

```bash
rtk cargo test --manifest-path rust/Cargo.toml
```

Expected: compile error — `peak_period` not found. This is the red phase.

- [ ] **Step 3: Add `peak_period()` before the test module**

Insert this function after `read_git_branch()` and before the `#[cfg(test)]` block:

```rust
fn peak_period(hour: u32) -> (&'static str, &'static str) {
    if hour >= 14 && hour <= 20 {
        ("PEAK", "2PM-8PM")
    } else {
        ("OFF PEAK", "9PM-3PM")
    }
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
rtk cargo test --manifest-path rust/Cargo.toml
```

Expected output: `test tests::test_peak_period_peak_hours ... ok` and `test tests::test_peak_period_off_peak_hours ... ok`.

- [ ] **Step 5: Commit**

```bash
rtk git add rust/src/statusline.rs && rtk git commit -m "feat: add peak_period() with unit tests"
```

---

## Task 2: Add `local_hour()` and `peak_label()`

**Files:**
- Modify: `rust/src/statusline.rs`

- [ ] **Step 1: Add failing tests for `peak_label()` in the existing test module**

In the `mod tests` block (at the bottom of the file), add these two tests after `test_peak_period_off_peak_hours`:

```rust
    #[test]
    fn test_peak_label_plain_content() {
        let (plain, _) = peak_label(17);
        assert_eq!(plain, "╡ PEAK 2PM-8PM ╞");
        let (plain, _) = peak_label(8);
        assert_eq!(plain, "╡ OFF PEAK 9PM-3PM ╞");
    }

    #[test]
    fn test_peak_label_plain_len() {
        let (plain, _) = peak_label(17);
        assert_eq!(plain.chars().count(), 16);
        let (plain, _) = peak_label(8);
        assert_eq!(plain.chars().count(), 20);
    }
```

- [ ] **Step 2: Run tests — expect compile failure**

```bash
rtk cargo test --manifest-path rust/Cargo.toml
```

Expected: compile error — `peak_label` not found.

- [ ] **Step 3: Add `local_hour()` and `peak_label()` before `peak_period()`**

Insert both functions immediately before `peak_period()` (after `read_git_branch()`, before `peak_period()`):

```rust
#[cfg(windows)]
fn local_hour() -> u32 {
    #[repr(C)]
    struct SYSTEMTIME {
        year: u16, month: u16, dow: u16, day: u16,
        hour: u16, minute: u16, second: u16, ms: u16,
    }
    extern "system" {
        fn GetLocalTime(lp: *mut SYSTEMTIME);
    }
    let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut st); }
    st.hour as u32
}

#[cfg(not(windows))]
fn local_hour() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    ((secs / 3600 + 1) % 24) as u32
}

fn peak_label(hour: u32) -> (String, String) {
    let (label, range) = peak_period(hour);
    let plain = format!("╡ {label} {range} ╞");
    let ansi = format!(
        "{C_PINK}╡{C_RESET} {C_CYAN}{label}{C_RESET} {C_YELLOW}{range}{C_RESET} {C_PINK}╞{C_RESET}"
    );
    (plain, ansi)
}
```

- [ ] **Step 4: Run tests — expect all 4 pass**

```bash
rtk cargo test --manifest-path rust/Cargo.toml
```

Expected: 4 tests all `ok`.

- [ ] **Step 5: Commit**

```bash
rtk git add rust/src/statusline.rs && rtk git commit -m "feat: add local_hour() and peak_label()"
```

---

## Task 3: Modify H1 rail to embed peak label

**Files:**
- Modify: `rust/src/statusline.rs`

- [ ] **Step 1: Add failing rail-length tests to the test module**

In `mod tests`, add after the existing tests:

```rust
    #[test]
    fn test_h1_rail_total_len_peak() {
        let title_plain = format!("═╡ NETRUNNER // {} ╞", "Thalia");
        let (peak_plain, _) = peak_label(17);
        let title_len = title_plain.chars().count();
        let peak_len = peak_plain.chars().count();
        let w2_center = 1 + W1 + 3 + W2 / 2;
        let peak_start = w2_center.saturating_sub(peak_len / 2);
        let pre_len = peak_start.saturating_sub(title_len).max(1);
        let post_len = BOX_WIDTH.saturating_sub(title_len + pre_len + peak_len);
        assert_eq!(title_len + pre_len + peak_len + post_len, BOX_WIDTH);
    }

    #[test]
    fn test_h1_rail_total_len_off_peak() {
        let title_plain = format!("═╡ NETRUNNER // {} ╞", "Thalia");
        let (peak_plain, _) = peak_label(8);
        let title_len = title_plain.chars().count();
        let peak_len = peak_plain.chars().count();
        let w2_center = 1 + W1 + 3 + W2 / 2;
        let peak_start = w2_center.saturating_sub(peak_len / 2);
        let pre_len = peak_start.saturating_sub(title_len).max(1);
        let post_len = BOX_WIDTH.saturating_sub(title_len + pre_len + peak_len);
        assert_eq!(title_len + pre_len + peak_len + post_len, BOX_WIDTH);
    }
```

- [ ] **Step 2: Run tests — expect the 2 new tests fail**

```bash
rtk cargo test --manifest-path rust/Cargo.toml
```

Expected: the two new tests compile (they only use already-defined symbols and arithmetic) and pass — the math is correct. If they pass here, proceed to Step 3 (the math validates the implementation we're about to write). If they somehow fail, fix the arithmetic before continuing.

- [ ] **Step 3: Replace the H1 block in `run()`**

Find the H1 comment block in `run()` (around line 123) and replace it entirely:

Old block:
```rust
    // H1 — top rail.
    let title_plain = format!("═╡ NETRUNNER // {kitten_name} ╞");
    let title_ansi = format!(
        "{C_PINK}═╡{C_RESET} {C_BOLD}{C_CYAN}NETRUNNER{C_RESET} {C_MAGENTA}//{C_RESET} {C_PINK}{kitten_name}{C_RESET} {C_PINK}╞{C_RESET}"
    );
    let rail_count = BOX_WIDTH.saturating_sub(title_plain.chars().count());
    let rail = "═".repeat(rail_count);
    let h1 = format!("{C_PINK}╔{C_RESET}{title_ansi}{C_PINK}{rail}╗{C_RESET}");
```

New block:
```rust
    // H1 — top rail.
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

- [ ] **Step 4: Run all tests — expect 6 pass**

```bash
rtk cargo test --manifest-path rust/Cargo.toml
```

Expected: all 6 tests `ok`.

- [ ] **Step 5: Build release binary**

```bash
rtk cargo build --release --manifest-path rust/Cargo.toml
```

Expected: `Compiling purrgotchi` then `Finished release`.

- [ ] **Step 6: Smoke test — verify the label appears in the top rail**

```bash
echo | ./rust/target/release/purrgotchi.exe statusline
```

Visually confirm the top rail shows either:
- `╔═╡ NETRUNNER // <name> ╞══...══╡ PEAK 2PM-8PM ╞══...══╗` (if current local time is 2PM–8PM)
- `╔═╡ NETRUNNER // <name> ╞══...══╡ OFF PEAK 9PM-3PM ╞══...╗` (otherwise)

The label text should appear cyan (label) and yellow (hours). No other rows should change.

- [ ] **Step 7: Commit**

```bash
rtk git add rust/src/statusline.rs && rtk git commit -m "feat: embed peak time label in HUD top rail"
```

---

## Task 4: Version bump

**Files:**
- Modify: `rust/Cargo.toml`

- [ ] **Step 1: Bump version in Cargo.toml**

Change line 4:
```toml
version = "3.3.0"
```
to:
```toml
version = "3.3.1"
```

- [ ] **Step 2: Build release to verify**

```bash
rtk cargo build --release --manifest-path rust/Cargo.toml
```

Expected: `Finished release [optimized] target(s)`.

- [ ] **Step 3: Commit**

```bash
rtk git add rust/Cargo.toml && rtk git commit -m "chore: bump version to 3.3.1"
```
