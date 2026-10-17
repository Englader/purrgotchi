//! Half-block pixel renderer for the statusline sprite column.
//!
//! A `PxCanvas` is `width x PX_H` pixels. Each terminal cell shows two
//! vertically stacked pixels as `▀` (fg = top, bg = bottom), so `PX_ROWS`
//! terminal rows carry `PX_H` pixel rows. Transparent cells are plain spaces
//! with no background active. Every encoded row ends with `\x1b[0m`.

use crate::colors::{self, Rgb};
use crate::pixel;

pub(crate) mod art;
pub(crate) mod fx;
pub(crate) mod life;
pub(crate) mod paint;
pub(crate) mod scene;

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

    pub fn get(&self, x: i32, y: i32) -> Option<Rgb> {
        self.idx(x, y).and_then(|i| self.px[i])
    }

    /// Clipped sprite blit; `None` pixels are transparent.
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

/// Pixel cat on by default (Gate A: the 24-bit bg test card rendered). Opt out with `PURR_PIXELS=0` or `NO_COLOR`.
pub(crate) const DEFAULT_MODE: PixelMode = PixelMode::Full;

/// Pure parse of `PURR_PIXELS` / `NO_COLOR`. A non-empty `NO_COLOR` wins.
pub(crate) fn mode_from(pixels: Option<&str>, no_color: Option<&str>, default: PixelMode) -> PixelMode {
    if no_color.is_some_and(|v| !v.is_empty()) {
        return PixelMode::Off;
    }
    let Some(v) = pixels.map(|v| v.trim().to_ascii_lowercase()) else { return default };
    match v.as_str() {
        "0" | "off" | "false" | "no" => PixelMode::Off,
        "1" => PixelMode::Full,
        "fg" => PixelMode::FgOnly,
        "test" => PixelMode::Test,
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
                if t == paint::OUTLINE {
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

/// The 6 sprite-column rows, in `kitten_view` priority: a celebration, the
/// birthday, a visible playground scene, else idle life. (`view.scene` is
/// already `None` behind the first two, `PURR_PLAYGROUND=0` or a too-narrow
/// strip.)
pub(crate) fn cat_area(
    view: &crate::statusline::KittenView,
    st: &crate::state::State,
    now: u64,
    width: usize,
    mode: PixelMode,
    forced: Option<life::Behaviour>,
) -> Vec<String> {
    let canvas = match (fx::Overlay::pick(st.celebration_kind, view.is_birthday), &view.scene) {
        (Some(o), _) => fx::overlay_canvas(o, now, width, &cat_look(st, now)),
        (None, Some(sv)) => scene_canvas(st, sv, now, width),
        (None, None) => life_canvas(st, view.mood, now, width, forced),
    };
    encode(&canvas, mode)
}

fn cat_look(st: &crate::state::State, now: u64) -> paint::CatLook<'_> {
    paint::CatLook { breed: &st.breed, rarity: &st.rarity, stage: crate::state::life_stage(st.level), tick: now }
}

/// A playground scene second; a shiny cat twinkles here too.
fn scene_canvas(st: &crate::state::State, sv: &crate::statusline::SceneView, now: u64, width: usize) -> PxCanvas {
    let f = scene::scene_frame(sv.kind, sv.elapsed, sv.duration, sv.seed, width);
    let mut c = scene::draw(&f, width, &cat_look(st, now));
    if st.shiny && now.is_multiple_of(2) {
        sparkle(&mut c, f.x, paint::CAT_MAX_W as i32, (now / 2) ^ st.birth_time);
    }
    c
}

/// The idle cat on a `width`-px stage, plus her sleep Z and shiny sparkle.
fn life_canvas(st: &crate::state::State, mood: &str, now: u64, width: usize, forced: Option<life::Behaviour>) -> PxCanvas {
    let span = width.saturating_sub(paint::CAT_MAX_W);
    let f = life::life_frame(now, st.birth_time, mood, span, forced);
    let cat = paint::paint(f.pose, &cat_look(st, now), f.facing_right, f.tail, f.blink);
    let (w, top) = (cat.w as i32, (PX_H - cat.h) as i32);
    let mut c = PxCanvas::new(width);
    c.blit(&cat, f.x, top);
    if let Some(k) = f.z {
        // Drifts up and away from the tucked head (right end of the Curl
        // map), mirrored when she faces left; the top one is the dim frame.
        let z = art::prop(art::Prop::Z, u8::from(k == 2));
        let (k, zw) = (i32::from(k), z.w as i32);
        let zx = if f.facing_right { f.x + w - 4 + k } else { f.x + 4 - k - zw };
        c.blit(&z, zx, (top - 3 - k).max(0));
    }
    if st.shiny && now.is_multiple_of(2) {
        sparkle(&mut c, f.x, w, (now / 2) ^ st.birth_time);
    }
    c
}

/// A shiny cat twinkles: a sparkle on even seconds at a free spot near her,
/// a new spot every 2 s. Skipped when no candidate is free.
fn sparkle(c: &mut PxCanvas, x: i32, w: i32, key: u64) {
    let s = art::prop(art::Prop::Sparkle, 0);
    let (sw, sh) = (s.w as i32, s.h as i32);
    for i in 0..4 {
        let h = life::ihash(key ^ (i << 60));
        let (sx, sy) = (x - sw + (h % (w + sw) as u64) as i32, ((h >> 20) % (PX_H as u64 - 4)) as i32);
        let inside = sx >= 0 && sx + sw <= c.width() as i32;
        if inside && (0..sh).all(|dy| (0..sw).all(|dx| c.get(sx + dx, sy + dy).is_none())) {
            c.blit(&s, sx, sy);
            return;
        }
    }
}

/// The rarity a breed always hatches with (sheet previews only).
fn canonical_rarity(breed: &str) -> &'static str {
    ["common", "uncommon", "rare", "epic", "legendary"]
        .into_iter()
        .find(|r| crate::identity::breeds_for_rarity(r).contains(&breed))
        .unwrap_or("common")
}

/// `pixels sheet` cells for one breed: every pose × stage × facing
/// (tail Curl, eyes open), then adult Sit with tail Up / Flick / blink, the
/// tail-socket poses with tail Up, and every prop frame.
fn sheet_cells(breed: &str) -> Vec<(String, PxSprite)> {
    use art::{ALL_POSES, Pose, Tail};
    use paint::{CatLook, paint};
    let rarity = canonical_rarity(breed);
    let look = |stage| CatLook { breed, rarity, stage, tick: 0 };
    let mut cells = Vec::new();
    for st in ["kitten", "young", "adult"] {
        for p in ALL_POSES {
            for f in [true, false] {
                cells.push((format!("{p:?} {st} {}", if f { ">" } else { "<" }), paint(p, &look(st), f, Tail::Curl, false)));
            }
        }
    }
    for (label, tail, blink) in [("tail up", Tail::Up, false), ("tail flick", Tail::Flick, false), ("blink", Tail::Curl, true)] {
        cells.push((format!("Sit adult {label}"), paint(Pose::Sit, &look("adult"), true, tail, blink)));
    }
    for p in [Pose::Loaf, Pose::Look, Pose::Cheer, Pose::Wave] {
        cells.push((format!("{p:?} adult tail up"), paint(p, &look("adult"), true, Tail::Up, false)));
    }
    for p in art::ALL_PROPS {
        for f in 0..2 {
            cells.push((format!("{p:?} {f}"), art::prop(p, f)));
        }
    }
    cells
}

const SHEET_COLS: usize = 6;

/// Dev verb `pixels sheet [--breed B] [--json]`: every pose and prop.
pub(crate) fn sheet(breed: &str, json: bool) -> String {
    render_sheet(breed, sheet_cells(breed), SHEET_COLS, json)
}

/// Dev verb `pixels scene <run|zoomies|ball|butterfly|roll|yarn> [--width W]
/// [--breed B] [--stage S] [--json]`: the 30 s `PURR_FORCE_SCENE` loop
/// (duration 30, seed 42), one whole-stage cell per second.
pub(crate) fn scene_sheet(kind: crate::state::PlaygroundScene, width: usize, breed: &str, stage: &str, json: bool) -> String {
    let look = paint::CatLook { breed, rarity: canonical_rarity(breed), stage, tick: 0 };
    let cells = (0..30u64)
        .map(|e| {
            let c = scene::draw(&scene::scene_frame(kind, e, 30, 42, width), width, &look);
            (format!("{kind:?} {stage} t={e}"), PxSprite { w: c.w, h: PX_H, px: c.px })
        })
        .collect();
    render_sheet(&format!("{kind:?} ({breed} {stage}, {width} px)"), cells, 3, json)
}

/// Dev verb `pixels celebrate <levelup|young|adult|streak|centurion|veteran|
/// master|legendary|greeting|drop|birthday> [--width W] [--breed B] [--stage S]
/// [--json]`: twelve consecutive seconds of the overlay.
pub(crate) fn celebrate_sheet(name: &str, width: usize, breed: &str, stage: &str, json: bool) -> Option<String> {
    use crate::state::CelebrationKind as K;
    let o = fx::Overlay::pick(
        match name {
            "levelup" => K::LevelUp,
            "young" => K::EvolvingYoung,
            "adult" => K::EvolvingAdult,
            "streak" => K::Streak,
            "centurion" => K::MilestoneCenturion,
            "veteran" => K::MilestoneVeteran,
            "master" => K::MilestoneMaster,
            "legendary" => K::MilestoneLegendary,
            "greeting" => K::Greeting,
            "drop" => K::RandomDrop,
            "birthday" => K::None,
            _ => return None,
        },
        name == "birthday",
    )?;
    let look = paint::CatLook { breed, rarity: canonical_rarity(breed), stage, tick: 0 };
    let start = 1_800_000_000u64 / 6 * 6;
    let cells = (0..12u64)
        .map(|s| {
            let c = fx::overlay_canvas(o, start + s, width, &look);
            (format!("{name} {stage} +{s}"), PxSprite { w: c.w, h: PX_H, px: c.px })
        })
        .collect();
    Some(render_sheet(&format!("{name} ({breed} {stage}, {width} px)"), cells, 6, json))
}

/// Labelled cells as ANSI blocks, or
/// `{"title","cols","cells":[{"label","px":[[hex|null]]}]}` for `px_preview.py`.
fn render_sheet(title: &str, cells: Vec<(String, PxSprite)>, cols: usize, json: bool) -> String {
    if json {
        let cells: Vec<serde_json::Value> = cells
            .iter()
            .map(|(label, s)| {
                let px: Vec<Vec<Option<String>>> =
                    (0..s.h).map(|y| (0..s.w).map(|x| s.px[y * s.w + x].map(|c| c.hex())).collect()).collect();
                serde_json::json!({ "label": label, "px": px })
            })
            .collect();
        return serde_json::json!({ "title": title, "cols": cols, "cells": cells }).to_string();
    }
    let cw = cells.iter().map(|(_, s)| s.w).max().unwrap_or(0) + 2;
    let mut out = vec![title.to_string()];
    for chunk in cells.chunks(cols) {
        let encoded: Vec<Vec<String>> = chunk
            .iter()
            .map(|(_, s)| {
                let mut c = PxCanvas::new(cw);
                c.blit(s, 1, (PX_H - s.h) as i32);
                encode(&c, PixelMode::Full)
            })
            .collect();
        for r in 0..PX_ROWS {
            out.push(encoded.iter().map(|e| e[r].as_str()).collect::<Vec<_>>().join(" "));
        }
        out.push(chunk.iter().map(|(l, _)| format!("{:<cw$}", l.chars().take(cw).collect::<String>())).collect::<Vec<_>>().join(" "));
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
        assert_eq!(mode_from(Some("junk"), None, Off), Off);
        for off in ["OFF", " Off ", "false", "FALSE", "no", "No", "0"] {
            assert_eq!(mode_from(Some(off), None, Full), Off, "{off:?}");
        }
        assert_eq!(mode_from(Some("FG"), None, Full), FgOnly);
        assert_eq!(mode_from(Some("Test"), None, Full), Test);
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
        c.set(0, 0, paint::OUTLINE);
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
        assert_rows(&r, CARD_W, "test card");
        assert!(r[0].contains("\x1b[48;2;234;0;217m"));
        assert!(r[1].contains("\x1b[48;2;243;243;27m"));
    }

    fn assert_rows(rows: &[String], w: usize, what: &str) {
        assert_eq!(rows.len(), PX_ROWS, "{what}");
        for r in rows {
            assert_eq!(crate::span::strip_ansi(r).chars().count(), w, "{what}");
            assert!(r.ends_with("\x1b[0m"), "{what}");
        }
    }

    #[test]
    fn life_rows_keep_their_shape() {
        let mut st = crate::state::State::base(1_000);
        for (level, shiny, breed) in [(1, false, "Ginger"), (10, true, "Void Cat"), (50, true, ""), (50, false, "Dragon")] {
            (st.level, st.shiny, st.breed) = (level, shiny, breed.to_string());
            for w in [24usize, 60, 120] {
                for mood in ["calm", "happy", "alert", "hungry", "panic", "sleep"] {
                    for m in [PixelMode::Full, PixelMode::FgOnly] {
                        for now in 1_800_000_000..1_800_000_120u64 {
                            assert_rows(&encode(&life_canvas(&st, mood, now, w, None), m), w, &format!("{level} {w} {mood} {now}"));
                        }
                    }
                }
            }
        }
    }

    /// The sleeping cat gets a Z over her head, on the canvas, both facings.
    #[test]
    fn sleep_shows_a_rising_z() {
        let zc = crate::pixel::NEON_CYAN;
        let mut st = crate::state::State::base(1_000);
        for (seed, w) in [(1_000u64, 24usize), (1_008, 24), (1_000, 120), (1_008, 60)] {
            st.birth_time = seed;
            // 1_800_000_012 is mid-block (dozing either way) and divisible by 3: Z k = 0, then 1.
            let ys: Vec<i32> = (1_800_000_012..1_800_000_014u64)
                .map(|now| {
                    let c = life_canvas(&st, "sleep", now, w, None);
                    (0..PX_H as i32).find(|&y| (0..w as i32).any(|x| c.get(x, y) == Some(zc))).expect("a Z")
                })
                .collect();
            assert!(ys[1] < ys[0], "{seed} {w}: Z does not rise {ys:?}");
        }
    }

    #[test]
    fn shiny_twinkles_on_even_seconds_only() {
        let white = Some(crate::pixel::WHITE);
        let mut st = crate::state::State::base(1_000);
        st.breed = "Ginger".into();
        let has = |st: &crate::state::State, now| {
            let c = life_canvas(st, "calm", now, 60, Some(life::Behaviour::Sit));
            (0..60).any(|x| (0..PX_H as i32).any(|y| c.get(x, y) == white))
        };
        assert!(!(1_800_000_000..1_800_000_060u64).any(|t| has(&st, t)), "plain cat sparkles");
        st.shiny = true;
        assert!((1_800_000_000..1_800_000_060u64).step_by(2).filter(|&t| has(&st, t)).count() > 20);
        assert!(!(1_800_000_001..1_800_000_060u64).step_by(2).any(|t| has(&st, t)));
    }

    /// The statusline entry point, through a real `kitten_view`.
    #[test]
    fn cat_area_uses_the_view_mood() {
        use crate::statusline::{kitten_view, ViewInput};
        let mut st = crate::state::State::base(1_000);
        st.last_fed_at = 1;
        let now = 1_800_000_010u64;
        let view = kitten_view(
            &ViewInput { st: &st, now: now as i64, frame: 0, ctx_pct: 0.0, forced_scene: None, playground_enabled: true, strip_width: Some(60) },
            || None,
        );
        assert_eq!(view.mood, "sleep");
        let rows = cat_area(&view, &st, now, 60, PixelMode::Full, None);
        assert_rows(&rows, 60, "cat_area");
        assert_eq!(rows, encode(&life_canvas(&st, "sleep", now, 60, None), PixelMode::Full));
    }

    /// A visible scene replaces idle life; `PURR_PLAYGROUND=0` (or a
    /// celebration) hides it and the idle cat is back.
    #[test]
    fn cat_area_plays_the_scene_then_life_when_disabled() {
        use crate::state::{CelebrationKind, PlaygroundScene};
        use crate::statusline::{kitten_view, ViewInput};
        let now = 1_800_000_010u64;
        let mut st = crate::state::State::base(1_000);
        st.last_fed_at = now - 60;
        (st.scene_kind, st.scene_started_at, st.scene_duration_secs, st.scene_seed) = (PlaygroundScene::BallChase, now - 5, 30, 7);
        let view = |st: &crate::state::State, playground_enabled| {
            kitten_view(
                &ViewInput { st, now: now as i64, frame: 0, ctx_pct: 0.0, forced_scene: None, playground_enabled, strip_width: Some(60) },
                || None,
            )
        };
        let v = view(&st, true);
        let rows = cat_area(&v, &st, now, 60, PixelMode::Full, None);
        assert_rows(&rows, 60, "scene");
        let f = scene::scene_frame(PlaygroundScene::BallChase, 5, 30, 7, 60);
        assert_eq!(rows, encode(&scene::draw(&f, 60, &cat_look(&st, now)), PixelMode::Full));
        let life = encode(&life_canvas(&st, v.mood, now, 60, None), PixelMode::Full);
        assert_ne!(rows, life);
        let off = view(&st, false);
        assert!(off.scene.is_none());
        assert_eq!(cat_area(&off, &st, now, 60, PixelMode::Full, None), life);
        st.celebration_kind = CelebrationKind::LevelUp;
        let v = view(&st, true);
        assert!(v.scene.is_none());
        let rows = cat_area(&v, &st, now, 60, PixelMode::Full, None);
        let o = fx::Overlay::Celebration(CelebrationKind::LevelUp);
        assert_eq!(rows, encode(&fx::overlay_canvas(o, now, 60, &cat_look(&st, now)), PixelMode::Full));
        assert_ne!(rows, life);
    }

    #[test]
    fn shiny_twinkles_in_scenes_too() {
        let mut st = crate::state::State::base(1_000);
        st.shiny = true;
        let sv = crate::statusline::SceneView {
            kind: crate::state::PlaygroundScene::Run, elapsed: 3, duration: 30, seed: 7, width: 60, finale: false,
        };
        let white = |now| {
            let c = scene_canvas(&st, &sv, now, 60);
            (0..60).any(|x| (0..PX_H as i32).any(|y| c.get(x, y) == Some(crate::pixel::WHITE)))
        };
        assert!((1_800_000_000..1_800_000_040u64).step_by(2).any(white));
        assert!(!(1_800_000_001..1_800_000_040u64).step_by(2).any(white));
    }
}
