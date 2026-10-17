//! Half-block pixel renderer for the statusline sprite column and the cat band.
//!
//! A `PxCanvas` is `width x h.px()` pixels. Each terminal cell shows two
//! vertically stacked pixels as `▀` (fg = top, bg = bottom), so `h.rows()`
//! terminal rows carry `h.px()` pixel rows. Transparent cells are plain spaces
//! with no background active. Every encoded row ends with `\x1b[0m`.

use crate::colors::{self, Rgb};
use crate::pixel;

pub(crate) mod art;
pub(crate) mod art10;
pub(crate) mod band;
pub(crate) mod fx;
pub(crate) mod life;
pub(crate) mod paint;
pub(crate) mod scene;
pub(crate) mod tempo;

/// Canvas height in pixels: 12 (statusline and band, 6 rows) or 10 (band, 5 rows).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PxHeight {
    H10,
    H12,
}

impl PxHeight {
    pub fn px(self) -> usize {
        match self {
            Self::H10 => 10,
            Self::H12 => 12,
        }
    }

    /// Terminal rows: two pixels per row.
    pub fn rows(self) -> usize {
        self.px() / 2
    }

    pub fn from_rows(r: usize) -> Option<Self> {
        match r {
            5 => Some(Self::H10),
            6 => Some(Self::H12),
            _ => None,
        }
    }

    /// Widest painted sprite, outline included.
    pub fn cat_max_w(self) -> usize {
        match self {
            Self::H10 => 14,
            Self::H12 => 16,
        }
    }

    /// `--height 10|12` of the `pixels` dev verbs.
    pub fn from_px(px: usize) -> Option<Self> {
        Self::from_rows(px / 2).filter(|h| h.px() == px)
    }
}

/// Plain width of the bg test card rows.
const CARD_W: usize = 24;

/// `width x h.px()` pixel canvas; `None` is transparent.
pub(crate) struct PxCanvas {
    w: usize,
    h: PxHeight,
    px: Vec<Option<Rgb>>,
}

impl PxCanvas {
    pub fn new(w: usize, h: PxHeight) -> Self {
        Self { w, h, px: vec![None; w * h.px()] }
    }

    pub fn width(&self) -> usize {
        self.w
    }

    fn idx(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h.px() {
            return None;
        }
        Some(y as usize * self.w + x as usize)
    }

    /// The whole canvas as a sprite (sheet cells).
    fn into_sprite(self) -> PxSprite {
        PxSprite { w: self.w, h: self.h.px(), px: self.px }
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

/// Canvas to `c.h.rows()` ANSI rows. A reset is emitted only when the (fg, bg)
/// state changes; every row ends with `\x1b[0m`.
pub(crate) fn encode(c: &PxCanvas, mode: PixelMode) -> Vec<String> {
    (0..c.h.rows())
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

    let mut a = PxCanvas::new(N, PxHeight::H12);
    let mut d = PxCanvas::new(N, PxHeight::H12);
    for x in 0..N as i32 {
        a.set(x, 0, colors::CYAN);
        a.set(x, 1, colors::PINK);
        let t = x as f64 / (N - 1) as f64;
        d.set(x, 0, pixel::mix(colors::CYAN, colors::PINK, t));
        d.set(x, 1, pixel::mix(colors::PINK, colors::CYAN, t));
    }

    let mut heart = PxCanvas::new(N, PxHeight::H12);
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

/// The `h.rows()` sprite-column rows, in `kitten_view` priority: a celebration, the
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
    h: PxHeight,
) -> Vec<String> {
    encode(&band_canvas(view, st, now.saturating_mul(1000), width, forced, h, &tempo::Tempo::STATUSLINE), mode)
}

/// The canvas behind [`cat_area`] (at `Tempo::STATUSLINE` and whole seconds)
/// and the cat band: the cat at `now_ms`, `cols` px wide. (`forced` is the
/// `PURR_FORCE_POSE` dev backdoor.)
pub(crate) fn band_canvas(
    view: &crate::statusline::KittenView,
    st: &crate::state::State,
    now_ms: u64,
    cols: usize,
    forced: Option<life::Behaviour>,
    h: PxHeight,
    t: &tempo::Tempo,
) -> PxCanvas {
    match (fx::Overlay::pick(st.celebration_kind, view.is_birthday), &view.scene) {
        (Some(o), _) => fx::overlay_canvas_ms(o, now_ms, cols, &cat_look(st, now_ms / 1000), h, t),
        (None, Some(sv)) => scene_canvas(st, sv, now_ms, cols, h, t),
        (None, None) => life_canvas(st, view.mood, now_ms, cols, forced, h, t),
    }
}

fn cat_look(st: &crate::state::State, now: u64) -> paint::CatLook<'_> {
    paint::CatLook { breed: &st.breed, rarity: &st.rarity, stage: crate::state::life_stage(st.level), tick: now }
}

/// A playground scene second; a shiny cat twinkles here too.
fn scene_canvas(st: &crate::state::State, sv: &crate::statusline::SceneView, now_ms: u64, width: usize, h: PxHeight, t: &tempo::Tempo) -> PxCanvas {
    let (now, elapsed_ms) = (now_ms / 1000, sv.elapsed.saturating_mul(1000) + now_ms % 1000);
    let f = scene::scene_frame_ms(sv.kind, elapsed_ms, sv.duration, sv.seed, width, h, t);
    let mut c = scene::draw(&f, width, &cat_look(st, now), h);
    if st.shiny && now.is_multiple_of(2) {
        sparkle(&mut c, f.x, h.cat_max_w() as i32, (now / 2) ^ st.birth_time);
    }
    c
}

/// The idle cat on a `width`-px stage, plus her sleep Z and shiny sparkle.
fn life_canvas(st: &crate::state::State, mood: &str, now_ms: u64, width: usize, forced: Option<life::Behaviour>, h: PxHeight, t: &tempo::Tempo) -> PxCanvas {
    let (span, now) = (width.saturating_sub(h.cat_max_w()), now_ms / 1000);
    let f = life::life_frame_ms(now_ms, st.birth_time, mood, span, forced, t);
    let cat = paint::paint(f.pose, &cat_look(st, now), f.facing_right, f.tail, f.blink, h);
    let (w, top) = (cat.w as i32, (h.px() - cat.h) as i32);
    let mut c = PxCanvas::new(width, h);
    c.blit(&cat, f.x, top);
    if let Some(k) = f.z {
        // Drifts up and away from the tucked head (right end of the Curl
        // map), mirrored when she faces left; the top one is the dim frame.
        let z = art::prop(art::Prop::Z, u8::from(k == 2));
        let (k, zw) = (i32::from(k), z.w as i32);
        let (zx, zy) = match (h, f.facing_right) {
            (PxHeight::H12, true) => (f.x + w - 4 + k, (top - 3 - k).max(0)),
            (PxHeight::H12, false) => (f.x + 4 - k - zw, (top - 3 - k).max(0)),
            // The small cats have no headroom: the Z drifts out past her nose.
            (_, true) => (f.x + w + k, 0),
            (_, false) => (f.x - k - zw, 0),
        };
        c.blit(&z, zx, zy);
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
        let (sx, sy) = (x - sw + (h % (w + sw) as u64) as i32, ((h >> 20) % (c.h.px() as u64 - 4)) as i32);
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
fn sheet_cells(breed: &str, h: PxHeight) -> Vec<(String, PxSprite)> {
    use art::{ALL_POSES, Pose, Tail};
    use paint::{CatLook, paint};
    let rarity = canonical_rarity(breed);
    let look = |stage| CatLook { breed, rarity, stage, tick: 0 };
    let mut cells = Vec::new();
    for st in ["kitten", "young", "adult"] {
        for p in ALL_POSES {
            for f in [true, false] {
                cells.push((format!("{p:?} {st} {}", if f { ">" } else { "<" }), paint(p, &look(st), f, Tail::Curl, false, h)));
            }
        }
    }
    for (label, tail, blink) in [("tail up", Tail::Up, false), ("tail flick", Tail::Flick, false), ("blink", Tail::Curl, true)] {
        cells.push((format!("Sit adult {label}"), paint(Pose::Sit, &look("adult"), true, tail, blink, h)));
    }
    for p in [Pose::Loaf, Pose::Look, Pose::Cheer, Pose::Wave] {
        cells.push((format!("{p:?} adult tail up"), paint(p, &look("adult"), true, Tail::Up, false, h)));
    }
    for p in art::ALL_PROPS {
        for f in 0..2 {
            cells.push((format!("{p:?} {f}"), art::prop(p, f)));
        }
    }
    cells
}

const SHEET_COLS: usize = 6;

/// Dev verb `pixels sheet [--breed B] [--height 12|10] [--json]`: every pose
/// and prop.
pub(crate) fn sheet(breed: &str, h: PxHeight, json: bool) -> String {
    let title = if h == PxHeight::H12 { breed.to_string() } else { format!("{breed} ({} px)", h.px()) };
    render_sheet(&title, sheet_cells(breed, h), SHEET_COLS, json)
}

/// Dev verb `pixels scene <run|zoomies|ball|butterfly|roll|yarn> [--width W]
/// [--breed B] [--stage S] [--json]`: the 30 s `PURR_FORCE_SCENE` loop
/// (duration 30, seed 42), one whole-stage cell per second.
pub(crate) fn scene_sheet(kind: crate::state::PlaygroundScene, width: usize, breed: &str, stage: &str, h: PxHeight, json: bool) -> String {
    let look = paint::CatLook { breed, rarity: canonical_rarity(breed), stage, tick: 0 };
    let cells = (0..30u64)
        .map(|e| {
            let c = scene::draw(&scene::scene_frame(kind, e, 30, 42, width, h), width, &look, h);
            (format!("{kind:?} {stage} t={e}"), c.into_sprite())
        })
        .collect();
    render_sheet(&format!("{kind:?} ({breed} {stage}, {width} px)"), cells, 3, json)
}

/// Dev verb `pixels celebrate <levelup|young|adult|streak|centurion|veteran|
/// master|legendary|greeting|drop|birthday> [--width W] [--breed B] [--stage S]
/// [--json]`: twelve consecutive seconds of the overlay.
pub(crate) fn celebrate_sheet(name: &str, width: usize, breed: &str, stage: &str, h: PxHeight, json: bool) -> Option<String> {
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
            (format!("{name} {stage} +{s}"), fx::overlay_canvas(o, start + s, width, &look, h).into_sprite())
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
                let mut c = PxCanvas::new(cw, PxHeight::H12);
                c.blit(s, 1, (PxHeight::H12.px() - s.h) as i32);
                encode(&c, PixelMode::Full)
            })
            .collect();
        for r in 0..PxHeight::H12.rows() {
            out.push(encoded.iter().map(|e| e[r].as_str()).collect::<Vec<_>>().join(" "));
        }
        out.push(chunk.iter().map(|(l, _)| format!("{:<cw$}", l.chars().take(cw).collect::<String>())).collect::<Vec<_>>().join(" "));
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    const H12: PxHeight = PxHeight::H12;
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
    fn heights_size_the_canvas() {
        for (h, rows) in [(PxHeight::H10, 5), (PxHeight::H12, 6)] {
            assert_eq!(encode(&PxCanvas::new(10, h), PixelMode::Full).len(), rows, "{h:?}");
            assert_eq!(h.rows(), rows);
            assert_eq!(PxHeight::from_rows(rows), Some(h));
        }
        for bad in [3, 4, 7] {
            assert_eq!(PxHeight::from_rows(bad), None, "{bad}");
        }
        assert_eq!((PxHeight::H12.px(), PxHeight::H10.px()), (12, 10));
        assert_eq!((PxHeight::H12.cat_max_w(), PxHeight::H10.cat_max_w()), (16, 14));
        assert_eq!((PxHeight::from_px(10), PxHeight::from_px(12), PxHeight::from_px(8), PxHeight::from_px(6)), (Some(PxHeight::H10), Some(PxHeight::H12), None, None));
    }

    #[test]
    fn full_cell_is_fg_top_bg_bottom() {
        let mut c = PxCanvas::new(1, H12);
        c.set(0, 0, CYAN);
        c.set(0, 1, PINK);
        let r = encode(&c, PixelMode::Full);
        assert_eq!(r[0], "\x1b[0m\x1b[38;2;10;189;198m\x1b[48;2;234;0;217m▀\x1b[0m");
        assert!(r[1..].iter().all(|row| row == " \x1b[0m"));
    }

    #[test]
    fn fg_only_never_emits_bg() {
        let mut c = PxCanvas::new(3, H12);
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
        let mut c = PxCanvas::new(1, H12);
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
                let mut c = PxCanvas::new(w, H12);
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
        let mut c = PxCanvas::new(2, H12);
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
        assert_eq!(rows.len(), H12.rows(), "{what}");
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
                            assert_rows(&encode(&life_canvas(&st, mood, now * 1000, w, None, H12, &tempo::Tempo::STATUSLINE), m), w, &format!("{level} {w} {mood} {now}"));
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
                    let c = life_canvas(&st, "sleep", now * 1000, w, None, H12, &tempo::Tempo::STATUSLINE);
                    (0..H12.px() as i32).find(|&y| (0..w as i32).any(|x| c.get(x, y) == Some(zc))).expect("a Z")
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
            let c = life_canvas(st, "calm", now * 1000, 60, Some(life::Behaviour::Sit), H12, &tempo::Tempo::STATUSLINE);
            (0..60).any(|x| (0..H12.px() as i32).any(|y| c.get(x, y) == white))
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
        let rows = cat_area(&view, &st, now, 60, PixelMode::Full, None, H12);
        assert_rows(&rows, 60, "cat_area");
        assert_eq!(rows, encode(&life_canvas(&st, "sleep", now * 1000, 60, None, H12, &tempo::Tempo::STATUSLINE), PixelMode::Full));
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
        let rows = cat_area(&v, &st, now, 60, PixelMode::Full, None, H12);
        assert_rows(&rows, 60, "scene");
        let f = scene::scene_frame(PlaygroundScene::BallChase, 5, 30, 7, 60, H12);
        assert_eq!(rows, encode(&scene::draw(&f, 60, &cat_look(&st, now), H12), PixelMode::Full));
        let life = encode(&life_canvas(&st, v.mood, now * 1000, 60, None, H12, &tempo::Tempo::STATUSLINE), PixelMode::Full);
        assert_ne!(rows, life);
        let off = view(&st, false);
        assert!(off.scene.is_none());
        assert_eq!(cat_area(&off, &st, now, 60, PixelMode::Full, None, H12), life);
        st.celebration_kind = CelebrationKind::LevelUp;
        let v = view(&st, true);
        assert!(v.scene.is_none());
        let rows = cat_area(&v, &st, now, 60, PixelMode::Full, None, H12);
        let o = fx::Overlay::Celebration(CelebrationKind::LevelUp);
        assert_eq!(rows, encode(&fx::overlay_canvas(o, now, 60, &cat_look(&st, now), H12), PixelMode::Full));
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
            let c = scene_canvas(&st, &sv, now * 1000, 60, H12, &tempo::Tempo::STATUSLINE);
            (0..60).any(|x| (0..H12.px() as i32).any(|y| c.get(x, y) == Some(crate::pixel::WHITE)))
        };
        assert!((1_800_000_000..1_800_000_040u64).step_by(2).any(white));
        assert!(!(1_800_000_001..1_800_000_040u64).step_by(2).any(white));
    }

    // ---- motion oracle -------------------------------------------------

    /// One FNV-1a line per (seed, mood, span) over an hour of whole-second
    /// life, per (scene, width, height) over 40 s of scene, and per overlay
    /// over 12 s of fx. `scene` takes (kind, elapsed, width, height); `fx`
    /// takes (overlay, now, height).
    fn motion_lines(
        life: impl Fn(u64, u64, &str, usize) -> life::CatFrame,
        scene: impl Fn(crate::state::PlaygroundScene, u64, usize, PxHeight) -> scene::SceneFrame,
        fx: impl Fn(fx::Overlay, u64, PxHeight) -> scene::SceneFrame,
    ) -> Vec<String> {
        const NOW: u64 = 1_800_000_000;
        let mut lines = Vec::new();
        for seed in [0u64, 12345, u64::MAX] {
            for mood in ["calm", "happy", "alert", "hungry", "panic", "sleep"] {
                for span in [8usize, 29, 104] {
                    let mut acc = String::new();
                    for now in NOW..NOW + 3600 {
                        let f = life(now, seed, mood, span);
                        acc += &format!("{:?}|{}|{}|{:?}|{}|{:?}\n", f.pose, f.x, f.facing_right, f.tail, f.blink, f.z);
                    }
                    lines.push(format!("life {seed} {mood} {span} {:016x}", crate::identity::fnv1a(&acc)));
                }
            }
        }
        for kind in crate::playground::ACTIVE_SCENES {
            for width in [45usize, 120] {
                for h in [PxHeight::H12, PxHeight::H10] {
                    let acc: String = (0..40).map(|e| format!("{:?}\n", scene(kind, e, width, h))).collect();
                    lines.push(format!("scene {kind:?} {width} {} {:016x}", h.px(), crate::identity::fnv1a(&acc)));
                }
            }
        }
        use crate::state::CelebrationKind as K;
        let kinds = [
            K::LevelUp, K::EvolvingYoung, K::EvolvingAdult, K::Streak, K::MilestoneCenturion, K::MilestoneVeteran,
            K::MilestoneMaster, K::MilestoneLegendary, K::Greeting, K::RandomDrop,
        ];
        for o in kinds.into_iter().map(fx::Overlay::Celebration).chain([fx::Overlay::Birthday]) {
            for h in [PxHeight::H12, PxHeight::H10] {
                let acc: String = (NOW..NOW + 12).map(|now| format!("{:?}\n", fx(o, now, h))).collect();
                lines.push(format!("fx {o:?} {} {:016x}", h.px(), crate::identity::fnv1a(&acc)));
            }
        }
        lines
    }

    fn check_motion(lines: Vec<String>) {
        let path = format!("{}/tests/golden/motion_hashes.txt", env!("CARGO_MANIFEST_DIR"));
        if std::env::var("UPDATE_GOLDEN").as_deref() == Ok("1") {
            std::fs::write(&path, lines.join("\n") + "\n").unwrap();
            return;
        }
        let golden = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e} (record with UPDATE_GOLDEN=1)"));
        let want: Vec<&str> = golden.lines().collect();
        let bad: Vec<String> = lines.iter().zip(&want).filter(|(g, w)| g != w).take(5).map(|(g, w)| format!("  want {w}\n  got  {g}")).collect();
        assert!(bad.is_empty() && want.len() == lines.len(), "motion drift ({} golden, {} rendered):\n{}", want.len(), lines.len(), bad.join("\n"));
    }

    fn oracle_look() -> paint::CatLook<'static> {
        paint::CatLook { breed: "Ginger", rarity: "common", stage: "adult", tick: 0 }
    }

    /// Whole-second motion snapshot, recorded before the `Tempo` plumbing.
    /// Re-record only on purpose: `UPDATE_GOLDEN=1 cargo test motion_oracle`.
    #[test]
    fn motion_oracle() {
        check_motion(motion_lines(
            |now, seed, mood, span| life::life_frame(now, seed, mood, span, None),
            |kind, e, w, h| scene::scene_frame(kind, e, 40, 42, w, h),
            |o, now, h| fx::fx_frame(o, now, 60, &oracle_look(), h),
        ));
    }

    /// The `*_ms` samplers at whole seconds and `Tempo::STATUSLINE` are the
    /// statusline's own output.
    #[test]
    fn whole_seconds_match_statusline_tempo() {
        let t = &tempo::Tempo::STATUSLINE;
        check_motion(motion_lines(
            |now, seed, mood, span| life::life_frame_ms(now * 1000, seed, mood, span, None, t),
            |kind, e, w, h| scene::scene_frame_ms(kind, e * 1000, 40, 42, w, h, t),
            |o, now, h| fx::fx_frame_ms(o, now * 1000, 60, &oracle_look(), h, t),
        ));
    }
}
