# Peak Times HUD Label — Design Spec

**Date:** 2026-04-21  
**Version target:** v3.3.1 (patch, display-only)  
**Scope:** Embed a dynamic Anthropic API peak-time indicator in the Purrgotchi statusline top rail

---

## 1. Goal

Show the user whether Anthropic's API is currently in peak or off-peak load at a glance, embedded in the HUD's top rail above the W2 (middle) column — matching the Netrunner card aesthetic already established by the `╡ NETRUNNER // Thalia ╞` title label.

---

## 2. Data Source

Peak windows are based on community-reported patterns (source: techpulse.news/tools/claude-peak-hours/). Anthropic has not published official data. The global peak window is **1 PM – 7 PM UTC**, which maps to **3 PM – 9 PM local time** for Switzerland and Macedonia (both CET/CEST, UTC+1/UTC+2).

The CH↔US offset is always 9 hours regardless of DST (both regions shift clocks simultaneously), so no DST logic is needed. The local hour thresholds are stable year-round.

The displayed window is widened slightly to **2 PM – 8 PM** (14:00–20:59) to include the borderline hours on either side of the strict source window. 2PM–3PM and 8PM–9PM are considered grey-zone and absorbed into PEAK.

---

## 3. Two States

| State | Local hours (CET/CEST) | Rail label (plain) |
|---|---|---|
| PEAK | 14:00 – 20:59 | `╡ PEAK 2PM-8PM ╞` |
| OFF PEAK | 21:00 – 13:59 | `╡ OFF PEAK 9PM-3PM ╞` |

---

## 4. Placement — Top Rail (H1)

The label is embedded in the 94-char top rail, centered above the W2 column.

**W2 visual center calculation:**
- Content rows layout: `1 (leading space) + 28 (W1) + 3 (sep) + 15 (W2/2) = position 47`
- The peak label is centered at rail position 47

**Rail segments:**

```
╔═╡ NETRUNNER // Thalia ╞══════════╡ PEAK 2PM-8PM ╞═══════════════════════════╗
  [────── title (25) ──────][pre ─][──── label (16) ────][───── post (39) ────]
```

For OFF PEAK (label is 20 chars):

```
╔═╡ NETRUNNER // Thalia ╞════════╡ OFF PEAK 9PM-3PM ╞═══════════════════════╗
  [────── title (25) ──────][pre][────── label (20) ──────][──── post ───────]
```

**Math:**
```
w2_center    = 1 + W1 + 3 + W2/2   // = 47
peak_start   = w2_center - label_plain_len / 2
pre_rail_len = peak_start.saturating_sub(title_plain_len).max(1)
post_rail_len = BOX_WIDTH - title_plain_len - pre_rail_len - label_plain_len
```

If a very long kitten name crowds the label, `pre_rail_len` saturates to 1 `═` (never zero, never overlaps).

---

## 5. Colors

| Part | Color |
|---|---|
| `╡` and `╞` | Pink (matches box border color `C_PINK`) |
| `PEAK` / `OFF PEAK` | Cyan (`C_CYAN`) |
| `2PM-8PM` / `9PM-3PM` | Yellow (`C_YELLOW`) |
| Rail `═` segments | Pink (`C_PINK`) |

ANSI example for PEAK state:
```
{C_PINK}╡{C_RESET} {C_CYAN}PEAK{C_RESET} {C_YELLOW}2PM-8PM{C_RESET} {C_PINK}╞{C_RESET}
```

---

## 6. Local Time — No New Dependencies

The project uses zero crates beyond serde/serde_json. Local hour is obtained without adding a dependency:

**Windows (primary target):**  
FFI call to `GetLocalTime` from `kernel32` — ~10 lines of `unsafe`, no `winapi` crate needed. Returns the system local hour directly.

```rust
#[cfg(windows)]
fn local_hour() -> u32 {
    #[repr(C)]
    struct SYSTEMTIME { year: u16, month: u16, dow: u16, day: u16,
                        hour: u16, minute: u16, second: u16, ms: u16 }
    extern "system" { fn GetLocalTime(lp: *mut SYSTEMTIME); }
    let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut st); }
    st.hour as u32
}
```

**Non-Windows fallback:**  
`SystemTime` UTC seconds + hardcoded `+1h` offset. Accepts a 1-hour DST error; acceptable since the primary deployment target is Windows.

```rust
#[cfg(not(windows))]
fn local_hour() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0);
    ((secs / 3600 + 1) % 24) as u32
}
```

---

## 7. Code Changes

All changes are confined to `rust/src/statusline.rs`:

1. Add `local_hour()` function (platform-conditional, ~15 lines)
2. Add `peak_label()` function returning `(plain: String, ansi: String)` based on local hour
3. Modify H1 construction to split rail into three segments and embed the peak label

No new files. No new crates. No state changes. No CLI changes.

---

## 8. Version

Bump `Cargo.toml` from `3.3.0` → `3.3.1`. Patch release.

---

## 9. Out of Scope

- User-configurable timezone (always CET/CEST)
- Color-coding by peak/off-peak state (green/red signal)
- Weekend vs. weekday distinction
- Syncing with actual Anthropic server metrics (unofficial data only)
