//! Half-block pixel renderer for the statusline sprite column.
//!
//! A `PxCanvas` is `width x PX_H` pixels. Each terminal cell shows two
//! vertically stacked pixels as `▀` (fg = top, bg = bottom), so `PX_ROWS`
//! terminal rows carry `PX_H` pixel rows. Transparent cells are plain spaces
//! with no background active. Every encoded row ends with `\x1b[0m`.

use crate::colors::{self, Rgb};
use crate::pixel;

pub(crate) mod art;
pub(crate) mod paint;

pub(crate) const PX_ROWS: usize = 6;
pub(crate) const PX_H: usize = 12;

/// Plain width of the bg test card rows.
const CARD_W: usize = 24;

/// `width x PX_H` pixel canvas; `None` is transparent.
pub(crate) struct PxCanvas {
    w: usize,
    px: Vec<Option<Rgb>>,
}

impl PxCanvas {
    pub fn new(w: usize) -> Self {
        Self { w, px: vec![None; w * PX_H] }
    }

    #[allow(dead_code)] // used by the sprite tasks
    pub fn width(&self) -> usize {
        self.w
    }

    fn idx(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= PX_H {
            return None;
        }
        Some(y as usize * self.w + x as usize)
    }

    /// Clipped write.
    pub fn set(&mut self, x: i32, y: i32, c: Rgb) {
        if let Some(i) = self.idx(x, y) {
            self.px[i] = Some(c);
        }
    }

    #[allow(dead_code)] // used by the sprite tasks
    pub fn get(&self, x: i32, y: i32) -> Option<Rgb> {
        self.idx(x, y).and_then(|i| self.px[i])
    }

    /// Clipped sprite blit; `None` pixels are transparent.
    #[allow(dead_code)] // used by the sprite tasks
    pub fn blit(&mut self, s: &PxSprite, x: i32, y: i32) {
        for sy in 0..s.h {
            for sx in 0..s.w {
                if let Some(c) = s.px[sy * s.w + sx] {
                    self.set(x + sx as i32, y + sy as i32, c);
                }
            }
        }
    }
}

#[allow(dead_code)] // used by the sprite tasks
pub(crate) struct PxSprite {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Option<Rgb>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PixelMode {
    Off,
    Full,
    FgOnly,
    Test,
}

/// Task 9 flips this once the user has judged the bg test card.
pub(crate) const DEFAULT_MODE: PixelMode = PixelMode::Off;

/// Pure parse of `PURR_PIXELS` / `NO_COLOR`. A non-empty `NO_COLOR` wins.
pub(crate) fn mode_from(pixels: Option<&str>, no_color: Option<&str>, default: PixelMode) -> PixelMode {
    if no_color.is_some_and(|v| !v.is_empty()) {
        return PixelMode::Off;
    }
    match pixels.map(str::trim) {
        Some("0") => PixelMode::Off,
        Some("1") => PixelMode::Full,
        Some("fg") => PixelMode::FgOnly,
        Some("test") => PixelMode::Test,
        _ => default,
    }
}

pub(crate) fn mode() -> PixelMode {
    mode_from(
        std::env::var("PURR_PIXELS").ok().as_deref(),
        std::env::var("NO_COLOR").ok().as_deref(),
        DEFAULT_MODE,
    )
}

fn fg_seq(c: Rgb) -> String {
    format!("\x1b[38;2;{};{};{}m", c.0, c.1, c.2)
}

fn bg_seq(c: Rgb) -> String {
    format!("\x1b[48;2;{};{};{}m", c.0, c.1, c.2)
}

/// One cell: (glyph, fg, bg). `None` fg means plain (no colour active).
fn cell(t: Option<Rgb>, b: Option<Rgb>, mode: PixelMode) -> (char, Option<Rgb>, Option<Rgb>) {
    match (t, b) {
        (None, None) => (' ', None, None),
        (Some(t), None) => ('▀', Some(t), None),
        (None, Some(b)) => ('▄', Some(b), None),
        (Some(t), Some(b)) if t == b => ('█', Some(t), None),
        (Some(t), Some(b)) => match mode {
            PixelMode::FgOnly => {
                if t == pixel::OUTLINE {
                    ('▄', Some(b), None)
                } else {
                    ('▀', Some(t), None)
                }
            }
            _ => ('▀', Some(t), Some(b)),
        },
    }
}

/// Canvas to `PX_ROWS` ANSI rows. A reset is emitted only when the (fg, bg)
/// state changes; every row ends with `\x1b[0m`.
pub(crate) fn encode(c: &PxCanvas, mode: PixelMode) -> Vec<String> {
    (0..PX_ROWS)
        .map(|r| {
            let mut row = String::new();
            let mut state: (Option<Rgb>, Option<Rgb>) = (None, None);
            for x in 0..c.w as i32 {
                let (g, fg, bg) = cell(c.get(x, 2 * r as i32), c.get(x, 2 * r as i32 + 1), mode);
                if (fg, bg) != state {
                    row.push_str("\x1b[0m");
                    if let Some(f) = fg {
                        row.push_str(&fg_seq(f));
                    }
                    if let Some(b) = bg {
                        row.push_str(&bg_seq(b));
                    }
                    state = (fg, bg);
                }
                row.push(g);
            }
            row.push_str("\x1b[0m");
            row
        })
        .collect()
}

/// Pad a labelled row to `CARD_W` plain columns and close it with a reset.
fn card_row(head: &str, body: &str, used: usize) -> String {
    format!("{head}{body}{}\x1b[0m", " ".repeat(CARD_W - used))
}

/// Background-support test card: `PURR_PIXELS=test`. 6 rows, plain width 24.
///   A  half-blocks with fg + bg      B  spaces on a bg only
///   C  fg-only glyphs                D  fg+bg gradient
///   E  a small heart through the real encoder
pub(crate) fn test_card() -> Vec<String> {
    const N: usize = 10;
    let first = |c: &PxCanvas| encode(c, PixelMode::Full).swap_remove(0);

    let mut a = PxCanvas::new(N);
    let mut d = PxCanvas::new(N);
    for x in 0..N as i32 {
        a.set(x, 0, colors::CYAN);
        a.set(x, 1, colors::PINK);
        let t = x as f64 / (N - 1) as f64;
        d.set(x, 0, pixel::mix(colors::CYAN, colors::PINK, t));
        d.set(x, 1, pixel::mix(colors::PINK, colors::CYAN, t));
    }

    let mut heart = PxCanvas::new(N);
    let shape = [".XXX..XXX.", "XXXXXXXXXX", ".XXXXXXXX.", "...XXXX..."];
    for (y, line) in shape.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            if ch == 'X' {
                heart.set(x as i32, y as i32, colors::PINK);
            }
        }
    }
    heart.set(1, 0, colors::YELLOW);
    let h = encode(&heart, PixelMode::Full);

    vec![
        card_row("A ", &(first(&a) + " fg+bg"), 2 + N + 6),
        card_row("B ", &format!("{}{}\x1b[0m bg", bg_seq(colors::YELLOW), " ".repeat(N)), 2 + N + 3),
        card_row("C ", &format!("{}▀▄█▀▄█▀▄█▀\x1b[0m fg", fg_seq(colors::CYAN)), 2 + N + 3),
        card_row("D ", &first(&d), 2 + N),
        card_row("E ", &h[0], 2 + N),
        card_row("  ", &h[1], 2 + N),
    ]
}

/// `PURR_FORCE_POSE` values the Task 4 preview understands. Task 6 replaces
/// this with the full behaviour list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreviewPose {
    Sit,
    Wander,
}

pub(crate) fn preview_pose_from(v: Option<&str>) -> PreviewPose {
    match v.map(str::trim) {
        Some("wander" | "walk") => PreviewPose::Wander,
        _ => PreviewPose::Sit,
    }
}

/// Task 4 stub of the sprite column (Gate B live check): Sit with a tail
/// flick and blinks, or `wander` (Walk1/Walk2 at 2 px/s, ping-pong across the
/// stage). Pure in `(state, now, width)`; Task 6 replaces it with idle life.
pub(crate) fn cat_area(st: &crate::state::State, now: u64, width: usize, pose: PreviewPose, mode: PixelMode) -> Vec<String> {
    use art::{Pose, Tail};
    let look = paint::CatLook {
        breed: &st.breed,
        rarity: &st.rarity,
        shiny: st.shiny,
        stage: crate::state::life_stage(st.level),
        tick: now,
    };
    let span = width.saturating_sub(paint::CAT_MAX_W);
    let (p, x, right, tail, blink) = match pose {
        PreviewPose::Sit => {
            let tail = if now % 5 == 0 { Tail::Flick } else if now % 10 < 5 { Tail::Curl } else { Tail::Up };
            (Pose::Sit, span / 3, true, tail, now % 7 == 0)
        }
        PreviewPose::Wander => {
            let t = now.wrapping_mul(2);
            let walk = if now % 2 == 0 { Pose::Walk1 } else { Pose::Walk2 };
            (walk, crate::playground::tri(t, span), crate::playground::tri_rising(t, span), Tail::Curl, false)
        }
    };
    let s = paint::paint(p, &look, right, tail, blink);
    let mut c = PxCanvas::new(width);
    c.blit(&s, x as i32, (PX_H - s.h) as i32);
    encode(&c, mode)
}

/// The rarity a breed always hatches with (sheet previews only).
fn canonical_rarity(breed: &str) -> &'static str {
    ["common", "uncommon", "rare", "epic", "legendary"]
        .into_iter()
        .find(|r| crate::identity::breeds_for_rarity(r).contains(&breed))
        .unwrap_or("common")
}

/// `pixels sheet` cells for one breed: every drawn pose × stage × facing
/// (tail Curl, eyes open), then adult Sit with tail Up / Flick / blink.
fn sheet_cells(breed: &str) -> Vec<(String, PxSprite)> {
    use art::{ALL_POSES, Pose, Tail};
    use paint::{CatLook, paint};
    let rarity = canonical_rarity(breed);
    let look = |stage| CatLook { breed, rarity, shiny: false, stage, tick: 0 };
    let drawn = ALL_POSES.into_iter().filter(|&p| p == Pose::Sit || !std::ptr::eq(art::pose_map(p), art::pose_map(Pose::Sit)));
    let drawn: Vec<Pose> = drawn.collect();
    let mut cells = Vec::new();
    for st in ["kitten", "young", "adult"] {
        for &p in &drawn {
            for f in [true, false] {
                cells.push((format!("{p:?} {st} {}", if f { ">" } else { "<" }), paint(p, &look(st), f, Tail::Curl, false)));
            }
        }
    }
    for (label, tail, blink) in [("tail up", Tail::Up, false), ("tail flick", Tail::Flick, false), ("blink", Tail::Curl, true)] {
        cells.push((format!("Sit adult {label}"), paint(Pose::Sit, &look("adult"), true, tail, blink)));
    }
    cells
}

const SHEET_COLS: usize = 6;

/// Dev verb `pixels sheet [--breed B] [--json]`: ANSI blocks with labels, or
/// `{"title","cols","cells":[{"label","px":[[hex|null]]}]}` for `px_preview.py`.
pub(crate) fn sheet(breed: &str, json: bool) -> String {
    let cells = sheet_cells(breed);
    if json {
        let cells: Vec<serde_json::Value> = cells
            .iter()
            .map(|(label, s)| {
                let px: Vec<Vec<Option<String>>> =
                    (0..s.h).map(|y| (0..s.w).map(|x| s.px[y * s.w + x].map(|c| c.hex())).collect()).collect();
                serde_json::json!({ "label": label, "px": px })
            })
            .collect();
        return serde_json::json!({ "title": breed, "cols": SHEET_COLS, "cells": cells }).to_string();
    }
    const CW: usize = paint::CAT_MAX_W + 2;
    let mut out = vec![breed.to_string()];
    for chunk in cells.chunks(SHEET_COLS) {
        let encoded: Vec<Vec<String>> = chunk
            .iter()
            .map(|(_, s)| {
                let mut c = PxCanvas::new(CW);
                c.blit(s, 1, (PX_H - s.h) as i32);
                encode(&c, PixelMode::Full)
            })
            .collect();
        for r in 0..PX_ROWS {
            out.push(encoded.iter().map(|e| e[r].as_str()).collect::<Vec<_>>().join(" "));
        }
        out.push(chunk.iter().map(|(l, _)| format!("{:<CW$}", l.chars().take(CW).collect::<String>())).collect::<Vec<_>>().join(" "));
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use colors::{CYAN, PINK};

    #[test]
    fn mode_parse() {
        use PixelMode::*;
        assert_eq!(mode_from(None, None, Off), Off);
        assert_eq!(mode_from(Some("1"), None, Off), Full);
        assert_eq!(mode_from(Some("fg"), None, Off), FgOnly);
        assert_eq!(mode_from(Some("test"), None, Off), Test);
        assert_eq!(mode_from(Some("0"), None, Full), Off);
        assert_eq!(mode_from(Some("1"), Some("1"), Full), Off); // NO_COLOR wins
        assert_eq!(mode_from(None, Some(""), Full), Full); // empty NO_COLOR ignored
        assert_eq!(mode_from(Some("junk"), None, Full), Full);
    }

    #[test]
    fn full_cell_is_fg_top_bg_bottom() {
        let mut c = PxCanvas::new(1);
        c.set(0, 0, CYAN);
        c.set(0, 1, PINK);
        let r = encode(&c, PixelMode::Full);
        assert_eq!(r[0], "\x1b[0m\x1b[38;2;10;189;198m\x1b[48;2;234;0;217m▀\x1b[0m");
        assert!(r[1..].iter().all(|row| row == " \x1b[0m"));
    }

    #[test]
    fn fg_only_never_emits_bg() {
        let mut c = PxCanvas::new(3);
        c.set(0, 0, CYAN);
        c.set(0, 1, PINK);
        c.set(1, 1, CYAN);
        c.set(2, 0, CYAN);
        c.set(2, 1, CYAN);
        let r = encode(&c, PixelMode::FgOnly);
        assert!(!r[0].contains("\x1b[48;"));
        assert!(r[0].contains('▄') && r[0].contains('█'));
    }

    #[test]
    fn fg_only_outline_top_keeps_bottom() {
        let mut c = PxCanvas::new(1);
        c.set(0, 0, pixel::OUTLINE);
        c.set(0, 1, PINK);
        let r = encode(&c, PixelMode::FgOnly);
        assert_eq!(r[0], "\x1b[0m\x1b[38;2;234;0;217m▄\x1b[0m");
    }

    #[test]
    fn hygiene_sweep() {
        // random canvases, both modes
        let mut rng = crate::identity::Lcg::new(7);
        for w in [1usize, 24, 60, 120] {
            for _ in 0..50 {
                let mut c = PxCanvas::new(w);
                for _ in 0..w * 4 {
                    let v = rng.next();
                    c.set((v % w as u64) as i32, ((v >> 8) % 12) as i32, Rgb(v as u8, (v >> 16) as u8, 9));
                }
                for m in [PixelMode::Full, PixelMode::FgOnly] {
                    for row in encode(&c, m) {
                        assert_eq!(crate::span::strip_ansi(&row).chars().count(), w);
                        assert!(row.ends_with("\x1b[0m"));
                        // a space is never emitted while a bg is active
                        let mut bg = false;
                        let mut i = 0;
                        let b = row.as_bytes();
                        while i < b.len() {
                            if b[i] == 0x1b {
                                let j = row[i..].find('m').unwrap() + i;
                                let seq = &row[i..=j];
                                if seq == "\x1b[0m" {
                                    bg = false
                                } else if seq.starts_with("\x1b[48;") {
                                    bg = true
                                }
                                i = j + 1;
                            } else {
                                if b[i] == b' ' {
                                    assert!(!bg, "space under live bg: {row:?}")
                                }
                                i += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn canvas_clips_and_blits() {
        let mut c = PxCanvas::new(2);
        c.set(-1, 0, CYAN);
        c.set(2, 0, CYAN);
        c.set(0, 12, CYAN);
        assert!((0..2).all(|x| (0..12).all(|y| c.get(x, y).is_none())));
        let s = PxSprite { w: 2, h: 1, px: vec![Some(CYAN), None] };
        c.blit(&s, 1, 3); // second pixel falls on x=2, clipped; first lands at (1,3)
        assert_eq!(c.get(1, 3), Some(CYAN));
        assert_eq!(c.width(), 2);
    }

    #[test]
    fn test_card_shape() {
        let r = test_card();
        assert_eq!(r.len(), 6);
        for row in &r {
            assert_eq!(crate::span::strip_ansi(row).chars().count(), 24);
            assert!(row.ends_with("\x1b[0m"));
        }
        assert!(r[0].contains("\x1b[48;2;234;0;217m"));
        assert!(r[1].contains("\x1b[48;2;243;243;27m"));
    }

    #[test]
    fn preview_cat_area_shape() {
        let mut st = crate::state::State::base(1_000);
        for level in [1, 10, 50] {
            st.level = level;
            for w in [24usize, 60, 120] {
                for pose in [PreviewPose::Sit, PreviewPose::Wander] {
                    for m in [PixelMode::Full, PixelMode::FgOnly] {
                        for now in 1_800_000_000..1_800_000_120u64 {
                            let rows = cat_area(&st, now, w, pose, m);
                            assert_eq!(rows.len(), PX_ROWS);
                            for r in &rows {
                                assert_eq!(crate::span::strip_ansi(r).chars().count(), w);
                                assert!(r.ends_with("\x1b[0m"));
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(preview_pose_from(Some("wander")), PreviewPose::Wander);
        assert_eq!(preview_pose_from(Some("junk")), PreviewPose::Sit);
        assert_eq!(preview_pose_from(None), PreviewPose::Sit);
    }
}
