//! Generated 32×32 pixel-art kitten (spec §4.8): a Rust port of the prototype
//! `docs/superpowers/specs/assets/2026-10-17-sprites/sprite.mjs`.
//!
//! Ported for fidelity of look, not bits: the layer order and every constant
//! (stage geometry, breed table, palette, eye tables, glyphs) are the
//! prototype's, but `hash` uses exact i32 wrapping math and the JS rounding
//! quirks are not chased. The output is pinned by its own snapshot
//! (`tests/golden/pixel_hashes.txt`); the PNGs next to the prototype are the
//! visual reference only. The classic ASCII sprites (`sprites.rs`) are separate.
//!
//! Layers, in order: silhouette → traits → coat → lighting → outline/rim →
//! collar → face → props → rarity/shiny.

use std::f64::consts::PI;

use crate::colors::{self, Rgb};

pub(crate) const W: usize = 32;
pub(crate) const H: usize = 32;
pub(crate) type Grid = [[Option<Rgb>; W]; H];

/// What to draw. Unknown strings fall back (breed → Ginger, rarity → common,
/// stage → adult, mood → calm); `frame` is taken `% 2`; `tick` only cycles the
/// legendary rainbow.
pub(crate) struct PixelOpts<'a> {
    pub breed: &'a str,
    pub rarity: &'a str,
    pub stage: &'a str,
    pub mood: &'a str,
    pub frame: u8,
    pub shiny: bool,
    pub tick: u64,
}

// ── palette: colors.rs plus the prototype's art constants ──────────────────

const fn rgb(hex: u32) -> Rgb {
    Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

const OUTLINE: Rgb = rgb(0x160b26);
const LID: Rgb = rgb(0x160b26);
const SHADOW: Rgb = rgb(0x0d0618);
const WHITE: Rgb = rgb(0xffffff);
const BLACK: Rgb = rgb(0x000000);
const NEON_CYAN: Rgb = colors::CYAN; // #0abdc6, rim light left
const NEON_MAGENTA: Rgb = colors::PINK; // #ea00d9, rim light right
const NOSE: Rgb = rgb(0xff7aa8);
const INNER_EAR: Rgb = rgb(0xf39bb8);
const VOID_INNER_EAR: Rgb = rgb(0x3a2a5e);
const PUPIL: Rgb = rgb(0x0b0612);
const PANIC_WHITE: Rgb = rgb(0xf4f1ff);
const LID_ON_DARK: Rgb = rgb(0xc9c3dd);
const MOUTH_OPEN: Rgb = rgb(0x2a0f2a);
const SPLIT_MUZZLE_DARK: Rgb = rgb(0x3a3346);
const SPLIT_MUZZLE_LIGHT: Rgb = rgb(0xf3c99a);
const RARE: Rgb = rgb(0xb45cff);
const EPIC: Rgb = rgb(0xffc93c);
const RAINBOW: [Rgb; 7] = [colors::RED, EPIC, colors::YELLOW, colors::GREEN, colors::CYAN, RARE, colors::PINK];

// ── tables ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pattern {
    Tabby,
    Tuxedo,
    Calico,
    Solid,
    Points,
    Spots,
    Van,
    Wrinkles,
    Void,
    Rosettes,
    Split,
}

struct Breed {
    name: &'static str,
    base: Rgb,
    mark: Rgb,
    mark2: Option<Rgb>,
    light: Option<Rgb>,
    pattern: Pattern,
    fluff: bool,
    tufts: bool,
    bushy: bool,
    thin_tail: bool,
    big_ears: bool,
    folded: bool,
    round: bool,
    glow: bool,
    hetero: bool,
    sheen: Option<f64>,
}

const PLAIN: Breed = Breed {
    name: "",
    base: BLACK,
    mark: BLACK,
    mark2: None,
    light: None,
    pattern: Pattern::Solid,
    fluff: false,
    tufts: false,
    bushy: false,
    thin_tail: false,
    big_ears: false,
    folded: false,
    round: false,
    glow: false,
    hetero: false,
    sheen: None,
};

/// The prototype's `BREEDS`, verbatim. Names match `identity::BREEDS_*`.
const BREEDS: [Breed; 16] = [
    Breed { name: "Tabby", base: rgb(0x9a7b5f), mark: rgb(0x5c4332), light: Some(rgb(0xe8dccb)), pattern: Pattern::Tabby, ..PLAIN },
    Breed { name: "Tuxedo", base: rgb(0x24212d), mark: rgb(0x24212d), light: Some(rgb(0xeeeaf2)), pattern: Pattern::Tuxedo, ..PLAIN },
    Breed { name: "Calico", base: rgb(0xf1ece4), mark: rgb(0xe5893a), mark2: Some(rgb(0x2a2530)), pattern: Pattern::Calico, ..PLAIN },
    Breed { name: "Ginger", base: rgb(0xe8913a), mark: rgb(0xc26a22), light: Some(rgb(0xf7d9b5)), pattern: Pattern::Tabby, ..PLAIN },
    Breed { name: "Grey Shorthair", base: rgb(0x8d94a5), mark: rgb(0x8d94a5), light: Some(rgb(0xc9ced9)), pattern: Pattern::Solid, ..PLAIN },
    Breed { name: "Siamese", base: rgb(0xefe2cc), mark: rgb(0x4a3328), pattern: Pattern::Points, big_ears: true, ..PLAIN },
    Breed { name: "Russian Blue", base: rgb(0x6e8199), mark: rgb(0x6e8199), pattern: Pattern::Solid, sheen: Some(0.22), ..PLAIN },
    Breed { name: "Bengal", base: rgb(0xd9a54c), mark: rgb(0x5a3a1e), light: Some(rgb(0xf3e2bd)), pattern: Pattern::Spots, ..PLAIN },
    Breed { name: "Maine Coon", base: rgb(0x8a6a4c), mark: rgb(0x4f3a29), light: Some(rgb(0xeadfcf)), pattern: Pattern::Tabby, fluff: true, tufts: true, bushy: true, ..PLAIN },
    Breed { name: "Norwegian Forest", base: rgb(0x9aa1ad), mark: rgb(0x646b78), light: Some(rgb(0xf0f0f3)), pattern: Pattern::Tabby, fluff: true, tufts: true, bushy: true, ..PLAIN },
    Breed { name: "Turkish Van", base: rgb(0xf3efe8), mark: rgb(0xb5552a), pattern: Pattern::Van, ..PLAIN },
    Breed { name: "Sphinx", base: rgb(0xdcab95), mark: rgb(0xb98270), pattern: Pattern::Wrinkles, big_ears: true, thin_tail: true, ..PLAIN },
    Breed { name: "Scottish Fold", base: rgb(0xb9bdc9), mark: rgb(0x8c91a0), light: Some(rgb(0xe3e5ec)), pattern: Pattern::Tabby, folded: true, round: true, ..PLAIN },
    Breed { name: "Void Cat", base: rgb(0x16121f), mark: rgb(0x2c2145), pattern: Pattern::Void, glow: true, ..PLAIN },
    Breed { name: "Snow Leopard Cat", base: rgb(0xe2e5ec), mark: rgb(0x7b8292), pattern: Pattern::Rosettes, fluff: true, bushy: true, ..PLAIN },
    Breed { name: "Chimera", base: rgb(0x221d2b), mark: rgb(0xe08a3c), pattern: Pattern::Split, hetero: true, ..PLAIN },
];

const GINGER: usize = 3;

fn breed(name: &str) -> &'static Breed {
    BREEDS.iter().find(|b| b.name == name).unwrap_or(&BREEDS[GINGER])
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Kitten,
    Young,
    Adult,
}

impl Stage {
    fn parse(s: &str) -> Stage {
        match s {
            "kitten" => Stage::Kitten,
            "young" => Stage::Young,
            _ => Stage::Adult,
        }
    }
}

struct StageGeom {
    head: [f64; 4],
    body: [f64; 4],
    ear: [f64; 6],
    eye_y: f64,
    eye_dx: f64,
    tail: [[f64; 2]; 3],
    paws: f64,
    eye_h: u8,
}

/// The prototype's `STAGES`, verbatim.
fn geom(stage: Stage) -> &'static StageGeom {
    const KITTEN: StageGeom = StageGeom { head: [16.0, 18.6, 9.2, 7.4], body: [16.0, 27.8, 5.4, 3.9], ear: [6.8, 8.6, 4.6, 16.4, 13.2, 12.2], eye_y: 18.4, eye_dx: 4.0, tail: [[20.0, 30.0], [25.5, 30.5], [25.0, 25.0]], paws: 30.8, eye_h: 4 };
    const YOUNG: StageGeom = StageGeom { head: [16.0, 14.2, 9.4, 7.2], body: [16.0, 24.6, 7.2, 6.0], ear: [6.8, 3.8, 4.4, 12.4, 13.4, 8.2], eye_y: 13.6, eye_dx: 4.4, tail: [[22.0, 29.0], [29.5, 29.0], [28.0, 19.5]], paws: 30.5, eye_h: 3 };
    const ADULT: StageGeom = StageGeom { head: [16.0, 11.8, 9.8, 7.4], body: [16.0, 22.8, 8.8, 7.8], ear: [6.6, 1.2, 4.2, 10.2, 13.6, 5.6], eye_y: 11.8, eye_dx: 4.6, tail: [[23.0, 29.5], [31.0, 29.5], [28.5, 16.5]], paws: 30.6, eye_h: 3 };
    match stage {
        Stage::Kitten => &KITTEN,
        Stage::Young => &YOUNG,
        Stage::Adult => &ADULT,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mood {
    Calm,
    Happy,
    Alert,
    Hungry,
    Panic,
    Sleep,
}

impl Mood {
    fn parse(s: &str) -> Mood {
        match s {
            "happy" => Mood::Happy,
            "alert" => Mood::Alert,
            "hungry" => Mood::Hungry,
            "panic" => Mood::Panic,
            "sleep" => Mood::Sleep,
            _ => Mood::Calm,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    fn parse(s: &str) -> Rarity {
        match s {
            "uncommon" => Rarity::Uncommon,
            "rare" => Rarity::Rare,
            "epic" => Rarity::Epic,
            "legendary" => Rarity::Legendary,
            _ => Rarity::Common,
        }
    }

    /// Eye/collar colour; legendary cycles the rainbow by `tick % 7`.
    fn colour(self, tick: u64) -> Rgb {
        match self {
            Rarity::Common => colors::CYAN,
            Rarity::Uncommon => colors::GREEN,
            Rarity::Rare => RARE,
            Rarity::Epic => EPIC,
            Rarity::Legendary => RAINBOW[(tick % 7) as usize],
        }
    }

    fn sparks(self) -> usize {
        match self {
            Rarity::Common | Rarity::Uncommon => 0,
            Rarity::Rare => 2,
            Rarity::Epic => 3,
            Rarity::Legendary => 4,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Region {
    Tail,
    Body,
    Paw,
    Ear,
    Head,
    Inner,
    Chest,
    Muzzle,
}

type Regions = [[Option<Region>; W]; H];

// ── helpers ─────────────────────────────────────────────────────────────────

/// `Math.round` for the non-negative values the prototype rounds.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

fn in_ell(x: i32, y: i32, cx: f64, cy: f64, rx: f64, ry: f64, p: f64) -> bool {
    ((x as f64 + 0.5 - cx) / rx).abs().powf(p) + ((y as f64 + 0.5 - cy) / ry).abs().powf(p) <= 1.0
}

fn in_tri(x: i32, y: i32, [a, b, c]: [[f64; 2]; 3]) -> bool {
    let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
    let s = |p1: [f64; 2], p2: [f64; 2]| (px - p2[0]) * (p1[1] - p2[1]) - (p1[0] - p2[0]) * (py - p2[1]);
    let (d1, d2, d3) = (s(a, b), s(b, c), s(c, a));
    !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0))
}

/// FNV-style integer hash in [0, 1), exact i32/u32 wrapping math.
fn hash(x: i32, y: i32, s: i32) -> f64 {
    let h = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263))
        .wrapping_add(s.wrapping_mul(2147483647)) as u32;
    let h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    (h ^ (h >> 16)) as f64 / 4294967296.0
}

struct Spot {
    d: f64,
    on: bool,
    a: f64,
}

/// Jittered hex-grid spot field (Bengal spots, snow-leopard rosettes).
fn spot(x: i32, y: i32, seed: i32, cell: i32) -> Spot {
    let c = cell as f64;
    let gy = y.div_euclid(cell);
    let odd = (gy % 2) as f64;
    let gx = ((x as f64 + odd * c / 2.0) / c).floor() as i32;
    let cx = gx as f64 * c - odd * c / 2.0 + c / 2.0 + (hash(gx, gy, seed) - 0.5) * 1.2;
    let cy = gy as f64 * c + c / 2.0 + (hash(gx, gy, seed + 1) - 0.5) * 1.2;
    let (dx, dy) = (x as f64 + 0.5 - cx, y as f64 + 0.5 - cy);
    Spot { d: dx.hypot(dy), on: hash(gx, gy, seed + 2) > 0.25, a: dy.atan2(dx) }
}

/// Linear blend, quantised to u8 per channel like the prototype's `toHex`.
fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let ch = |u: u8, v: u8| {
        let (u, v) = (u as f64, v as f64);
        js_round(u + (v - u) * t).clamp(0.0, 255.0) as u8
    };
    Rgb(ch(a.0, b.0), ch(a.1, b.1), ch(a.2, b.2))
}

fn lum(c: Rgb) -> f64 {
    (0.299 * c.0 as f64 + 0.587 * c.1 as f64 + 0.114 * c.2 as f64) / 255.0
}

fn shade(c: Rgb, t: f64) -> Rgb {
    mix(c, SHADOW, t)
}

fn light(c: Rgb, t: f64) -> Rgb {
    mix(c, WHITE, t)
}

fn in_bounds(x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && (x as usize) < W && (y as usize) < H
}

fn reg_at(reg: &Regions, x: i32, y: i32) -> Option<Region> {
    if in_bounds(x, y) { reg[y as usize][x as usize] } else { None }
}

fn set_reg(reg: &mut Regions, x: i32, y: i32, r: Region) {
    if in_bounds(x, y) {
        reg[y as usize][x as usize] = Some(r);
    }
}

fn px_at(out: &Grid, x: i32, y: i32) -> Option<Rgb> {
    if in_bounds(x, y) { out[y as usize][x as usize] } else { None }
}

fn put(out: &mut Grid, x: i32, y: i32, c: Rgb) {
    if in_bounds(x, y) {
        out[y as usize][x as usize] = Some(c);
    }
}

fn cells() -> impl Iterator<Item = (i32, i32)> {
    (0..H as i32).flat_map(|y| (0..W as i32).map(move |x| (x, y)))
}

/// Paint `#` cells of a small bitmap glyph.
fn glyph(out: &mut Grid, rows: &[&str], x0: i32, y0: i32, c: Rgb) {
    for (j, row) in rows.iter().enumerate() {
        for (i, ch) in row.bytes().enumerate() {
            if ch == b'#' {
                put(out, x0 + i as i32, y0 + j as i32, c);
            }
        }
    }
}

/// Eye bitmaps (3 wide): E iris, k pupil, w/W whites, L lid, `.` untouched.
fn eye_rows(mood: Mood, blink: bool) -> [&'static str; 3] {
    match mood {
        Mood::Alert => ["wEE", "EkE", "EEE"],
        Mood::Hungry => ["wEE", "EkE", "EkE"],
        Mood::Calm if blink => ["...", "...", "LLL"],
        Mood::Calm => ["...", "LLL", "EkE"],
        Mood::Happy => ["...", ".L.", "L.L"],
        Mood::Panic => ["WWW", "WkW", "WWW"],
        Mood::Sleep => ["...", "...", "LLL"],
    }
}

// ── render ──────────────────────────────────────────────────────────────────

pub(crate) fn render(o: &PixelOpts) -> Grid {
    use Region::*;

    let b = breed(o.breed);
    let stage = Stage::parse(o.stage);
    let s = geom(stage);
    let mood = Mood::parse(o.mood);
    let rarity = Rarity::parse(o.rarity);
    let frame = (o.frame % 2) as usize;
    let f1 = frame == 1;
    let eye = rarity.colour(o.tick);

    let mut reg: Regions = [[None; W]; H];
    let oxi: i32 = if mood == Mood::Panic && f1 { 1 } else { 0 };
    let ox = oxi as f64;
    let oy = if mood == Mood::Sleep { 1.0 } else { 0.0 };
    let [mut hx, mut hy, mut hrx, hry] = s.head;
    hy += oy;
    hx += ox;
    let [bx, by, brx, bry] = s.body;
    let hp = if b.round { 2.0 } else if b.fluff { 2.5 } else { 2.25 };
    if b.fluff {
        hrx += 0.4;
    }

    // tail (behind body)
    let [p0, p1, p2] = s.tail;
    let happy = mood == Mood::Happy;
    let sway = if happy { if f1 { -1.5 } else { 0.0 } } else if f1 { 1.0 } else { 0.0 };
    let end = [p2[0] + sway, p2[1] - if happy { 2.0 } else { 0.0 }];
    let tr = if b.bushy { 2.3 } else if b.thin_tail { 1.05 } else { 1.6 };
    // The prototype's `t += 0.02` loop accumulates to 0.98 before overshooting 1: 50 steps.
    for i in 0..50 {
        let t = i as f64 * 0.02;
        let cx = (1.0 - t).powi(2) * p0[0] + 2.0 * (1.0 - t) * t * p1[0] + t * t * end[0];
        let cy = (1.0 - t).powi(2) * p0[1] + 2.0 * (1.0 - t) * t * p1[1] + t * t * end[1];
        let r = tr * if b.thin_tail { 1.0 } else { 0.85 + 0.25 * (t * PI).sin() };
        // Only the disc's bounding box can pass `in_ell` (50 discs per render).
        let span = |c: f64| (c - r - 1.0).floor() as i32..=(c + r).ceil() as i32;
        for y in span(cy) {
            for x in span(cx) {
                if in_bounds(x, y) && in_ell(x, y, cx, cy, r, r, 2.0) {
                    set_reg(&mut reg, x, y, Tail);
                }
            }
        }
    }
    for (x, y) in cells() {
        if in_ell(x, y, bx, by, brx, bry, 2.2) {
            set_reg(&mut reg, x, y, Body);
        }
    }
    for dx in [-3.6, 3.6] {
        for (x, y) in cells() {
            if in_ell(x, y, bx + dx, s.paws, 2.6, 1.5, 2.0) {
                set_reg(&mut reg, x, y, Paw);
            }
        }
    }

    // ears
    let [mut tx, mut ty, mut b1x, mut b1y, mut b2x, mut b2y] = s.ear;
    ty += oy;
    b1y += oy;
    b2y += oy;
    tx += ox;
    b1x += ox;
    b2x += ox;
    b1x = b1x.max(hx - hrx + 0.9);
    if b.big_ears {
        tx -= 0.8;
        ty -= 1.4;
        b1x -= 0.3;
    }
    if mood == Mood::Alert && f1 {
        tx -= 1.0;
    }
    if mood == Mood::Sleep || mood == Mood::Hungry {
        ty += 1.0;
    }
    if b.folded {
        ty = b2y + 1.2;
        tx = b1x + 3.4;
    }
    let ear_l = [[tx, ty], [b1x, b1y], [b2x, b2y]];
    let ear_r = ear_l.map(|[x, y]| [32.0 - x + 2.0 * ox, y]);
    for (x, y) in cells() {
        if in_tri(x, y, ear_l) || in_tri(x, y, ear_r) {
            set_reg(&mut reg, x, y, Ear);
        }
    }
    if b.tufts && !b.folded {
        let tip_y = ty.floor() as i32 - 1;
        set_reg(&mut reg, js_round(tx) as i32, tip_y, Ear);
        set_reg(&mut reg, js_round(32.0 - tx + 2.0 * ox) as i32 - 1, tip_y, Ear);
    }
    // head
    for (x, y) in cells() {
        if in_ell(x, y, hx, hy, hrx, hry, hp) {
            set_reg(&mut reg, x, y, Head);
        }
    }
    if b.fluff {
        for side in [-1.0, 1.0] {
            for k in [0, 2] {
                let x = js_round(hx + side * (hrx + 0.2)) as i32 - if side > 0.0 { 1 } else { 0 };
                let y = js_round(hy + 2.0 + k as f64 * 1.5) as i32;
                set_reg(&mut reg, x, y, Head);
            }
        }
    }
    // inner ear
    let shrink = |tri: [[f64; 2]; 3], f: f64| {
        let c = [(tri[0][0] + tri[1][0] + tri[2][0]) / 3.0, (tri[0][1] + tri[1][1] + tri[2][1]) / 3.0 + 0.6];
        tri.map(|[x, y]| [c[0] + (x - c[0]) * f, c[1] + (y - c[1]) * f])
    };
    if !b.folded {
        for tri in [shrink(ear_l, 0.62), shrink(ear_r, 0.62)] {
            for (x, y) in cells() {
                if reg_at(&reg, x, y) == Some(Ear) && in_tri(x, y, tri) {
                    set_reg(&mut reg, x, y, Inner);
                }
            }
        }
    }
    // chest + muzzle
    let ny = js_round(hy + 3.4 - if stage == Stage::Kitten { 0.4 } else { 0.0 }) as i32;
    for (x, y) in cells() {
        let r = &mut reg[y as usize][x as usize];
        if *r == Some(Body) && in_ell(x, y, bx, by - 1.0, brx * if b.fluff { 0.55 } else { 0.42 }, bry * 0.8, 2.0) {
            *r = Some(Chest);
        }
        if *r == Some(Head) && in_ell(x, y, hx, ny as f64 + 1.2, 2.7, 1.7, 2.0) {
            *r = Some(Muzzle);
        }
    }

    // coat colour
    let mut col: Grid = [[None; W]; H];
    let light_c = b.light.unwrap_or(b.base);
    for (x, y) in cells() {
        let Some(r) = reg[y as usize][x as usize] else { continue };
        let yf = y as f64;
        let mut c = b.base;
        if r == Inner {
            c = INNER_EAR;
        } else if (r == Chest || r == Paw) && b.light.is_some() {
            c = light_c;
        } else if r == Muzzle && b.light.is_some() {
            c = mix(b.base, light_c, 0.7);
        }
        let lx = x - oxi;
        let dl = (lx - 16).abs();
        match b.pattern {
            Pattern::Tabby => {
                if r == Head && yf < hy - 2.5 && dl < 5 && (lx - 16) % 2 == 0 {
                    c = b.mark;
                }
                if r == Head && dl as f64 > hrx - 3.5 && yf > hy - 1.0 && y % 2 == 0 {
                    c = b.mark;
                }
                if r == Body && (yf + dl as f64 * 0.55).floor() as i32 % 3 == 0 && dl > 2 {
                    c = b.mark;
                }
                if r == Tail && (x + y) % 3 == 0 {
                    c = b.mark;
                }
            }
            Pattern::Tuxedo => {
                if r == Head && (dl as f64) < 1.5 && yf > hy - 2.0 {
                    c = light_c;
                }
                if r == Tail && y > 28 {
                    c = light_c;
                }
            }
            Pattern::Calico => {
                let mark2 = b.mark2.unwrap_or(b.mark);
                let blobs = [(9.0, hy - 4.0, 4.2, b.mark), (22.0, hy - 3.0, 3.6, mark2), (10.0, by + 1.0, 3.6, mark2), (23.0, by + 2.0, 4.0, b.mark), (28.0, 20.0, 3.0, b.mark)];
                for (cx, cy, rr, k) in blobs {
                    if r != Inner && in_ell(lx, y, cx, cy, rr, rr * 0.85, 2.0) {
                        c = k;
                    }
                }
            }
            Pattern::Solid => {}
            Pattern::Points => {
                if r == Ear || r == Tail || r == Paw {
                    c = b.mark;
                } else if (r == Muzzle || r == Head) && in_ell(lx, y, 16.0, ny as f64 - 0.8, 4.4, 3.6, 2.0) {
                    c = mix(b.base, b.mark, if r == Muzzle { 0.9 } else { 0.6 });
                }
            }
            Pattern::Spots => {
                let sp = spot(x, y, 3, 4);
                if (r == Body || r == Tail || (r == Head && yf < hy - 1.0)) && sp.on && sp.d < 1.25 {
                    c = if sp.d < 0.6 { mix(b.mark, b.base, 0.45) } else { b.mark };
                }
                if r == Head && yf < hy - 2.5 && dl < 4 && (lx - 16) % 2 == 0 {
                    c = b.mark;
                }
            }
            Pattern::Van => {
                if (r == Ear || r == Head) && yf < hy - 2.0 && dl as f64 > 1.6 {
                    c = b.mark;
                }
                if r == Tail {
                    c = if (x + y) % 4 == 0 { mix(b.mark, BLACK, 0.25) } else { b.mark };
                }
            }
            Pattern::Wrinkles => {
                if r == Head && yf < hy - 2.5 && (dl as f64) < 3.5 && y % 2 == 1 {
                    c = b.mark;
                }
                if r == Body && dl as f64 > brx - 3.0 && y % 3 == 0 {
                    c = b.mark;
                }
            }
            Pattern::Void => {
                if r != Inner && hash(x, y, 9) > 0.965 {
                    c = if hash(x, y, 4) > 0.5 { NEON_CYAN } else { NEON_MAGENTA };
                }
                if r == Inner {
                    c = VOID_INNER_EAR;
                }
            }
            Pattern::Rosettes => {
                let sp = spot(x, y, 7, 5);
                if r != Inner && r != Muzzle && r != Chest && sp.on && sp.d > 0.9 && sp.d < 1.9 && (sp.a * 3.0 + 1.0).sin() > -0.4 {
                    c = b.mark;
                }
                if r == Head && yf < hy - 2.0 && sp.on && sp.d < 0.8 {
                    c = b.mark;
                }
                if r == Tail && (x + y) % 4 < 2 {
                    c = b.mark;
                }
            }
            Pattern::Split => {
                if r != Inner {
                    c = if lx < 16 { b.base } else { b.mark };
                }
                if r == Muzzle {
                    c = if lx < 16 { SPLIT_MUZZLE_DARK } else { SPLIT_MUZZLE_LIGHT };
                }
            }
        }
        col[y as usize][x as usize] = Some(c);
    }

    // shading: highlight up-left, shadow down-right, chin occlusion
    let filled = |x: i32, y: i32| reg_at(&reg, x, y).is_some();
    let mut out: Grid = [[None; W]; H];
    let sheen = b.sheen.unwrap_or(0.12) + if o.shiny { 0.12 } else { 0.0 };
    for (x, y) in cells() {
        let Some(r) = reg[y as usize][x as usize] else { continue };
        let Some(mut c) = col[y as usize][x as usize] else { continue };
        if !filled(x - 1, y) || !filled(x, y - 1) {
            c = light(c, sheen);
        }
        if !filled(x + 1, y) || !filled(x, y + 1) {
            c = shade(c, 0.28);
        } else if !filled(x + 2, y) || !filled(x, y + 2) {
            c = shade(c, 0.12);
        }
        let above = reg_at(&reg, x, y - 1);
        let left = reg_at(&reg, x - 1, y);
        if (r == Body || r == Chest) && (above == Some(Head) || above == Some(Muzzle)) {
            c = shade(c, 0.35);
        }
        if r == Tail && matches!(left, Some(Body | Chest | Paw)) {
            c = shade(c, 0.3);
        }
        if r == Paw && left == Some(Paw) && reg_at(&reg, x + 1, y) != Some(Paw) {
            c = shade(c, 0.2);
        }
        out[y as usize][x as usize] = Some(c);
    }
    // outline + neon rim (cyan left, magenta right)
    for (x, y) in cells() {
        let (ux, uy) = (x as usize, y as usize);
        if reg[uy][ux].is_some() {
            if let Some(c) = out[uy][ux] {
                if !filled(x - 1, y) && x < 16 + oxi {
                    out[uy][ux] = Some(mix(c, NEON_CYAN, 0.45));
                }
                if !filled(x + 1, y) && x >= 16 + oxi {
                    out[uy][ux] = Some(mix(c, NEON_MAGENTA, 0.45));
                }
            }
            continue;
        }
        if filled(x - 1, y) || filled(x + 1, y) || filled(x, y - 1) || filled(x, y + 1) {
            out[uy][ux] = Some(OUTLINE);
        }
    }
    // paw separators
    for dx in [-3.6, 3.6] {
        let (x, y) = (js_round(bx + dx) as i32, js_round(s.paws) as i32);
        if reg_at(&reg, x, y) == Some(Paw) {
            if let Some(c) = px_at(&out, x, y) {
                put(&mut out, x, y, shade(c, 0.45));
            }
        }
    }

    // collar in rarity colour
    let cy = js_round(hy + hry) as i32 - 1;
    for x in 0..W as i32 {
        let below = reg_at(&reg, x, cy + 1);
        let at = reg_at(&reg, x, cy);
        if matches!(below, Some(Body | Chest)) && matches!(at, Some(Head | Muzzle)) && (x - 16 - oxi).abs() < 5 {
            put(&mut out, x, cy + 1, mix(eye, BLACK, 0.15));
        }
    }
    put(&mut out, 16 + oxi, cy + 2, eye);

    // face
    let ey = js_round(hy + (s.eye_y - s.head[1]) - 1.0) as i32;
    let mut eyes: Vec<String> = eye_rows(mood, f1).iter().map(|r| r.to_string()).collect();
    if s.eye_h == 4 && eyes.concat().contains('E') && eyes[1].contains('k') {
        let tall = eyes[1].replacen('k', "E", 1);
        eyes.insert(1, tall);
    }
    let eye_col = |side: i32| if b.hetero && side > 0 { NEON_MAGENTA } else { eye };
    for side in [-1, 1] {
        let ex = js_round(hx + side as f64 * s.eye_dx) as i32 - 1;
        for (j, row) in eyes.iter().enumerate() {
            for (i, ch) in row.chars().enumerate() {
                let (x, y) = (ex + i as i32, ey + j as i32);
                let under = px_at(&out, x, y).unwrap_or(b.base);
                let c = match ch {
                    'E' => eye_col(side),
                    'k' => PUPIL,
                    'w' => WHITE,
                    'W' => PANIC_WHITE,
                    'L' if lum(under) < 0.3 => mix(eye_col(side), WHITE, 0.35),
                    'L' => LID,
                    _ => continue,
                };
                put(&mut out, x, y, if ch == 'E' && b.glow { light(c, 0.15) } else { c });
            }
        }
        if b.glow && matches!(mood, Mood::Alert | Mood::Hungry | Mood::Calm | Mood::Panic) {
            let gx = ex + if side < 0 { -1 } else { 3 };
            if reg_at(&reg, gx, ey + 1).is_some() {
                let under = px_at(&out, gx, ey + 1).unwrap_or(b.base);
                put(&mut out, gx, ey + 1, mix(under, eye_col(side), 0.5));
            }
        }
    }
    let nx = js_round(hx) as i32 - 1;
    put(&mut out, nx, ny, NOSE);
    put(&mut out, nx + 1, ny, NOSE);
    let mouth = |out: &mut Grid, pts: &[(i32, i32)], c: Rgb| {
        for &(dx, dy) in pts {
            let (x, y) = (nx + dx, ny + dy);
            let u = px_at(out, x, y).unwrap_or(b.base);
            put(out, x, y, if c == LID && lum(u) < 0.3 { LID_ON_DARK } else { c });
        }
    };
    match mood {
        Mood::Happy => {
            mouth(&mut out, &[(-1, 1), (2, 1), (0, 1), (1, 1)], LID);
            mouth(&mut out, &[(0, 2), (1, 2)], NOSE);
        }
        Mood::Panic => mouth(&mut out, &[(0, 1), (1, 1), (0, 2), (1, 2)], MOUTH_OPEN),
        Mood::Hungry => {
            mouth(&mut out, &[(0, 1), (1, 1)], MOUTH_OPEN);
            mouth(&mut out, &[(1, 2)], NOSE);
            if f1 {
                mouth(&mut out, &[(0, 3)], NEON_CYAN);
            }
        }
        Mood::Calm | Mood::Alert | Mood::Sleep => mouth(&mut out, &[(-1, 1), (2, 1)], LID),
    }

    // mood props
    match mood {
        Mood::Sleep => {
            glyph(&mut out, &["####", "..#.", ".#..", "####"], 27, if f1 { 0 } else { 1 }, NEON_CYAN);
            glyph(&mut out, &["##.", ".#.", ".##"], if f1 { 29 } else { 28 }, if f1 { 6 } else { 7 }, mix(NEON_CYAN, BLACK, 0.35));
        }
        Mood::Happy if f1 => glyph(&mut out, &["#.#", "###", ".#."], 26, 3, NEON_MAGENTA),
        Mood::Panic => glyph(&mut out, &[".#.", "###", "###", ".#."], 28, 6 + frame as i32, NEON_CYAN),
        Mood::Happy | Mood::Calm | Mood::Alert | Mood::Hungry => {}
    }

    // rarity aura / shiny sparkle
    let sparks = rarity.sparks();
    const SPOTS: [(i32, i32); 5] = [(2, 4), (28, 11), (3, 18), (29, 25), (1, 27)];
    for i in 0..sparks + if o.shiny { 2 } else { 0 } {
        let (x, y) = SPOTS[(i + frame) % SPOTS.len()];
        if reg_at(&reg, x, y).is_some() || px_at(&out, x, y).is_some() {
            continue;
        }
        let c = if o.shiny && i >= sparks {
            WHITE
        } else if rarity == Rarity::Legendary {
            RAINBOW[((o.tick % 7) as usize + i) % RAINBOW.len()]
        } else {
            eye
        };
        put(&mut out, x, y, c);
        if (i + frame) % 2 == 0 {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (ax, ay) = (x + dx, y + dy);
                if px_at(&out, ax, ay).is_none() && reg_at(&reg, ax, ay).is_none() {
                    put(&mut out, ax, ay, mix(c, BLACK, 0.45));
                }
            }
        }
    }
    out
}

/// JSON wire form for `frame --pixels`: `pixels[y][x]` is `"#rrggbb"` or null.
pub(crate) fn grid_hex(g: &Grid) -> Vec<Vec<Option<String>>> {
    g.iter().map(|row| row.iter().map(|p| p.map(|c| c.hex())).collect()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::Rgb;
    use crate::identity;

    const STAGES: [&str; 3] = ["kitten", "young", "adult"];
    const MOODS: [&str; 6] = ["calm", "happy", "alert", "hungry", "panic", "sleep"];
    const CYAN: Rgb = Rgb(0x0a, 0xbd, 0xc6);

    fn all_breeds() -> Vec<&'static str> {
        identity::BREEDS_COMMON
            .iter()
            .chain(identity::BREEDS_UNCOMMON.iter())
            .chain(identity::BREEDS_RARE.iter())
            .chain(identity::BREEDS_EPIC.iter())
            .chain(identity::BREEDS_LEGENDARY.iter())
            .copied()
            .collect()
    }

    /// The rarity a breed is always hatched with (identity::breeds_for_rarity).
    fn canonical_rarity(breed: &str) -> &'static str {
        ["common", "uncommon", "rare", "epic", "legendary"]
            .into_iter()
            .find(|r| identity::breeds_for_rarity(r).contains(&breed))
            .unwrap_or("common")
    }

    fn opts<'a>(breed: &'a str, stage: &'a str, mood: &'a str) -> PixelOpts<'a> {
        PixelOpts {
            breed,
            rarity: canonical_rarity(breed),
            stage,
            mood,
            frame: 0,
            shiny: false,
            tick: 0,
        }
    }

    fn contains(g: &Grid, c: Rgb) -> bool {
        g.iter().flatten().any(|p| *p == Some(c))
    }

    fn serialise(g: &Grid) -> String {
        let mut s = String::with_capacity(W * H * 7);
        for row in g {
            for p in row {
                match p {
                    Some(c) => s.push_str(&c.hex()),
                    None => s.push('-'),
                }
            }
        }
        s
    }

    #[test]
    fn breed_table_covers_identity() {
        for b in all_breeds() {
            assert!(BREEDS.iter().any(|e| e.name == b), "no BREEDS entry for {b}");
        }
        assert_eq!(BREEDS.len(), all_breeds().len());
    }

    #[test]
    fn dimensions_and_transparent_corners() {
        for b in all_breeds() {
            for st in STAGES {
                let g = render(&opts(b, st, "calm"));
                assert_eq!(g.len(), H);
                assert_eq!(g[0].len(), W);
                assert!(g[0][0].is_none(), "{b} {st}: top-left corner painted");
                assert!(g[31][0].is_none(), "{b} {st}: bottom-left corner painted");
            }
        }
    }

    #[test]
    fn deterministic() {
        let o = PixelOpts { shiny: true, frame: 1, tick: 5, ..opts("Void Cat", "young", "panic") };
        assert!(render(&o) == render(&o));
    }

    #[test]
    fn rim_light_signature() {
        // Ginger adult calm, a head row: leftmost coat pixel leans cyan
        // (green over red, blue above the base coat), rightmost leans
        // magenta (red over green, blue above the base coat).
        let base = Rgb(0xe8, 0x91, 0x3a);
        let g = render(&opts("Ginger", "adult", "calm"));
        let outline = Rgb(0x16, 0x0b, 0x26);
        let row = &g[12];
        let coat = |p: &&Option<Rgb>| matches!(p, Some(c) if *c != outline);
        let left = row.iter().find(coat).unwrap().unwrap();
        let right = row.iter().rev().find(coat).unwrap().unwrap();
        assert!(left.1 > left.0 && left.2 > base.2, "left edge not cyan-tinted: {left:?}");
        assert!(right.0 > right.1 && right.2 > base.2, "right edge not magenta-tinted: {right:?}");
        assert!(left != right);
    }

    #[test]
    fn common_eye_colour() {
        assert!(contains(&render(&opts("Tabby", "adult", "alert")), CYAN));
    }

    #[test]
    fn legendary_cycles_with_tick() {
        let at = |tick| render(&PixelOpts { tick, ..opts("Void Cat", "adult", "alert") });
        assert!(at(0) != at(1), "legendary eye must cycle with tick");
        assert!(at(0) == at(7), "rainbow period is 7");
    }

    #[test]
    fn shiny_adds_white() {
        let white = Rgb(255, 255, 255);
        let shiny = render(&PixelOpts { shiny: true, ..opts("Tabby", "adult", "calm") });
        assert!(contains(&shiny, white));
        assert!(!contains(&render(&opts("Tabby", "adult", "calm")), white));
    }

    #[test]
    fn sleep_has_z_only_when_sleeping() {
        assert_eq!(render(&opts("Ginger", "adult", "sleep"))[1][27], Some(CYAN));
        assert_ne!(render(&opts("Ginger", "adult", "calm"))[1][27], Some(CYAN));
    }

    #[test]
    fn unknown_inputs_fall_back() {
        let ginger = render(&opts("Ginger", "adult", "calm"));
        assert!(render(&PixelOpts { breed: "Dragon", ..opts("Ginger", "adult", "calm") }) == ginger);
        assert!(render(&opts("Ginger", "elder", "calm")) == ginger);
        assert!(render(&opts("Ginger", "adult", "bored")) == ginger);
        assert!(render(&PixelOpts { rarity: "mythic", ..opts("Ginger", "adult", "calm") }) == ginger);
    }

    #[test]
    fn frame_is_taken_mod_two() {
        let at = |frame| render(&PixelOpts { frame, ..opts("Ginger", "adult", "happy") });
        assert!(at(0) == at(2));
        assert!(at(1) == at(3));
        assert!(at(0) != at(1));
    }

    #[test]
    fn grid_hex_wire_form() {
        let g = render(&opts("Tabby", "adult", "alert"));
        let w = grid_hex(&g);
        assert_eq!(w.len(), H);
        assert!(w.iter().all(|r| r.len() == W));
        assert_eq!(w[0][0], None);
        for (y, row) in w.iter().enumerate() {
            for (x, p) in row.iter().enumerate() {
                assert_eq!(p.clone(), g[y][x].map(|c| c.hex()));
            }
        }
    }

    fn snapshot_lines() -> Vec<String> {
        let mut lines = Vec::new();
        let mut push = |b: &str, st: &str, m: &str, frame: u8, shiny: bool, tick: u64| {
            let g = render(&PixelOpts { frame, shiny, tick, ..opts(b, st, m) });
            lines.push(format!(
                "{b}|{st}|{m}|{frame}|{shiny}|{tick} {:016x}",
                identity::fnv1a(&serialise(&g))
            ));
        };
        for b in all_breeds() {
            for st in STAGES {
                for m in MOODS {
                    for frame in 0..2 {
                        for shiny in [false, true] {
                            push(b, st, m, frame, shiny, 0);
                        }
                    }
                }
            }
        }
        for b in identity::BREEDS_LEGENDARY {
            for st in STAGES {
                for m in MOODS {
                    for frame in 0..2 {
                        for shiny in [false, true] {
                            push(b, st, m, frame, shiny, 3);
                        }
                    }
                }
            }
        }
        lines
    }

    #[test]
    fn snapshot_all_combinations() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/pixel_hashes.txt");
        let lines = snapshot_lines();
        assert_eq!(lines.len(), 1368);
        if std::env::var("UPDATE_GOLDEN").as_deref() == Ok("1") {
            std::fs::write(path, lines.join("\n") + "\n").unwrap();
            return;
        }
        let golden = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{path}: {e} (record with UPDATE_GOLDEN=1)"));
        let want: Vec<&str> = golden.lines().collect();
        let mismatches: Vec<String> = lines
            .iter()
            .zip(want.iter())
            .filter(|(got, want)| got != *want)
            .take(5)
            .map(|(got, want)| format!("  want {want}\n  got  {got}"))
            .collect();
        assert!(
            mismatches.is_empty() && want.len() == lines.len(),
            "pixel snapshot drift ({} golden lines, {} rendered); first mismatches:\n{}",
            want.len(),
            lines.len(),
            mismatches.join("\n")
        );
    }

    fn sheet_json(cols: usize, grids: &[Grid]) -> String {
        let cells: Vec<serde_json::Value> = grids
            .iter()
            .map(|g| serde_json::json!({ "px": grid_hex(g) }))
            .collect();
        serde_json::json!({ "cols": cols, "cells": cells }).to_string()
    }

    /// Visual check: `preview.py` renders these the same way as the prototype's `dump.mjs`.
    #[test]
    #[ignore = "writes target/pixel-sheet-*.json for the visual check; run by hand"]
    fn dump_pixel_sheet() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/target");
        let breeds: Vec<Grid> = all_breeds().iter().map(|b| render(&opts(b, "adult", "alert"))).collect();
        let mut moods = Vec::new();
        for st in STAGES {
            for m in MOODS {
                moods.push(render(&opts("Ginger", st, m)));
            }
        }
        std::fs::write(format!("{dir}/pixel-sheet-breeds.json"), sheet_json(4, &breeds)).unwrap();
        std::fs::write(format!("{dir}/pixel-sheet-moods.json"), sheet_json(6, &moods)).unwrap();
    }
}
