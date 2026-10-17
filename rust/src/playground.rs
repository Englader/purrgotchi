//! Playground scenes — Thalia plays in the strip right of the HUD card.
//!
//! Deterministic engine: every frame is a pure function of
//! (kind, elapsed, duration, seed, strip_width, blink). State is written only
//! when a scene starts or ends (scheduler in statusline.rs; instant trigger in
//! hooks.rs `play`). Spec: docs/superpowers/specs/2026-07-11-v3.8.0-playground-scenes-design.md

use crate::identity::Lcg;
use crate::state::PlaygroundScene;

/// The playground is 6 rows tall — the sprite rows of the 7-row HUD (row 6 is speech).
pub const ROWS: usize = 6;
/// Below this many cells the strip can't stage a scene; render falls back to
/// the static home sprite.
pub const MIN_STRIP: usize = 24;
/// Wider than this stops adding charm and increases wrap risk on mis-reported COLUMNS.
pub const MAX_STRIP: usize = 120;
/// Used when COLUMNS is absent and no override is set.
pub const DEFAULT_STRIP: usize = 60;
/// Card renders 96 visible chars + 3-space gap + 2 safety cells. Lines longer
/// than the terminal wrap and shatter the 7-row layout, so always under-shoot.
pub const STRIP_OVERHEAD: usize = 101;
pub const SCENE_MIN_SECS: u64 = 20;
pub const SCENE_MAX_SECS: u64 = 40;
pub const IDLE_MIN_SECS: u64 = 120; // 2 min
pub const IDLE_MAX_SECS: u64 = 300; // 5 min
/// Last N seconds of a scene play its ending beat (catch/pounce).
pub const FINALE_SECS: u64 = 4;

/// Every startable scene. `PlaygroundScene::None` is deliberately absent.
pub const ACTIVE_SCENES: [PlaygroundScene; 6] = [
    PlaygroundScene::Run,
    PlaygroundScene::Zoomies,
    PlaygroundScene::BallChase,
    PlaygroundScene::ButterflyChase,
    PlaygroundScene::Roll,
    PlaygroundScene::YarnPush,
];

/// Kill switch: PURR_PLAYGROUND=0 disables scheduling and rendering.
pub fn enabled() -> bool {
    enabled_from(std::env::var("PURR_PLAYGROUND").ok().as_deref())
}

pub(crate) fn enabled_from(v: Option<&str>) -> bool {
    v != Some("0")
}

/// Usable playground width. Claude Code (>= v2.1.153) sets COLUMNS for the
/// statusline process. PURR_PLAYGROUND_WIDTH overrides everything (for hosts
/// where COLUMNS is absent or wrong). None = too narrow, don't render scenes.
pub fn strip_width() -> Option<usize> {
    strip_width_from(
        std::env::var("COLUMNS").ok().as_deref(),
        std::env::var("PURR_PLAYGROUND_WIDTH").ok().as_deref(),
    )
}

pub(crate) fn strip_width_from(
    columns: Option<&str>,
    override_w: Option<&str>,
) -> Option<usize> {
    let w = if let Some(o) = override_w.and_then(|v| v.trim().parse::<usize>().ok()) {
        o
    } else if let Some(c) = columns.and_then(|v| v.trim().parse::<usize>().ok()) {
        c.saturating_sub(STRIP_OVERHEAD)
    } else {
        DEFAULT_STRIP
    };
    clamp_strip(w)
}

/// MAX_STRIP cap; below MIN_STRIP there is no room to stage a scene.
pub(crate) fn clamp_strip(w: usize) -> Option<usize> {
    let w = w.min(MAX_STRIP);
    if w < MIN_STRIP { None } else { Some(w) }
}

/// Dev backdoor (keep out of README): PURR_FORCE_SCENE=run|zoomies|ball|butterfly|roll|yarn
/// renders that scene on a repeating 30 s loop without touching state.
pub fn force_scene() -> Option<PlaygroundScene> {
    force_scene_from(std::env::var("PURR_FORCE_SCENE").ok().as_deref())
}

pub(crate) fn force_scene_from(v: Option<&str>) -> Option<PlaygroundScene> {
    match v? {
        "run" => Some(PlaygroundScene::Run),
        "zoomies" => Some(PlaygroundScene::Zoomies),
        "ball" => Some(PlaygroundScene::BallChase),
        "butterfly" => Some(PlaygroundScene::ButterflyChase),
        "roll" => Some(PlaygroundScene::Roll),
        "yarn" => Some(PlaygroundScene::YarnPush),
        _ => None,
    }
}

/// What the scheduler should do this render. Applied inside locked_update on a
/// FRESH state copy — plan_transition is re-run there, so a stale snapshot can
/// never clobber a concurrent write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneTransition {
    None,
    Bootstrap { next_at: u64 },
    End { last_kind: PlaygroundScene, next_at: u64 },
    Start { kind: PlaygroundScene, duration_secs: u64, seed: u64 },
}

fn roll_range(rng: &mut Lcg, lo: u64, hi: u64) -> u64 {
    lo + rng.next() % (hi - lo + 1)
}

fn pick_scene(rng: &mut Lcg, last: PlaygroundScene) -> (PlaygroundScene, u64, u64) {
    let mut kind = ACTIVE_SCENES[(rng.next() % ACTIVE_SCENES.len() as u64) as usize];
    if kind == last {
        // One re-roll keeps the picker a pure fold over the seed. With this
        // Lcg (odd multiplier + odd increment) consecutive outputs alternate
        // parity, so the re-roll can never land on the same slot — but do not
        // rely on that: treat a post-re-roll repeat as accepted if the RNG
        // ever changes.
        kind = ACTIVE_SCENES[(rng.next() % ACTIVE_SCENES.len() as u64) as usize];
    }
    let duration_secs = roll_range(rng, SCENE_MIN_SECS, SCENE_MAX_SECS);
    let seed = rng.next();
    (kind, duration_secs, seed)
}

/// Pure scheduler. `overlay_active` = celebration running or birthday.
pub fn plan_transition(
    scene_kind: PlaygroundScene,
    scene_started_at: u64,
    scene_duration_secs: u64,
    scene_next_at: u64,
    scene_last_kind: PlaygroundScene,
    overlay_active: bool,
    now: u64,
) -> SceneTransition {
    let mut rng = Lcg::new(now ^ 0x504C_4159); // "PLAY"
    if scene_kind != PlaygroundScene::None {
        let expired = now >= scene_started_at.saturating_add(scene_duration_secs);
        let regressed = now < scene_started_at;
        if expired || regressed {
            return SceneTransition::End {
                last_kind: scene_kind,
                next_at: now + roll_range(&mut rng, IDLE_MIN_SECS, IDLE_MAX_SECS),
            };
        }
        return SceneTransition::None;
    }
    if scene_next_at == 0 {
        return SceneTransition::Bootstrap {
            next_at: now + roll_range(&mut rng, IDLE_MIN_SECS, IDLE_MAX_SECS),
        };
    }
    if !overlay_active && now >= scene_next_at {
        let (kind, duration_secs, seed) = pick_scene(&mut rng, scene_last_kind);
        return SceneTransition::Start { kind, duration_secs, seed };
    }
    SceneTransition::None
}

/// Start a scene immediately (the `purr play` trigger). Mutates only scene
/// fields; call inside an existing locked_update closure.
pub fn start_scene_now(s: &mut crate::state::State, now: u64) {
    let mut rng = Lcg::new(now ^ s.scene_seed ^ 0x6D65_6F77); // "meow"
    if s.scene_kind != PlaygroundScene::None {
        s.scene_last_kind = s.scene_kind;
    }
    let (kind, duration_secs, seed) = pick_scene(&mut rng, s.scene_last_kind);
    s.scene_kind = kind;
    s.scene_started_at = now;
    s.scene_duration_secs = duration_secs;
    s.scene_seed = seed;
    s.scene_next_at = now + roll_range(&mut rng, IDLE_MIN_SECS, IDLE_MAX_SECS);
}

pub const COL_BG: u8 = 0;
pub const COL_CAT: u8 = 1;
pub const COL_YELLOW: u8 = 2;
pub const COL_MAGENTA: u8 = 3;
pub const COL_PINK: u8 = 4;
pub const COL_DIM: u8 = 5;

/// 6-row cell grid. Chars carry a color id; emit() converts runs of same-color
/// cells into ANSI segments so the plain width of every row == canvas width —
/// the repo-wide plain/ANSI convention (render.rs display_width counts chars).
pub struct Canvas {
    w: usize,
    cells: Vec<(char, u8)>,
}

impl Canvas {
    pub fn new(w: usize) -> Self {
        Canvas { w, cells: vec![(' ', COL_BG); w * ROWS] }
    }

    pub fn width(&self) -> usize {
        self.w
    }

    /// Blit art with its top-left at (x, y). Cells outside the canvas are
    /// clipped; spaces in the art are transparent.
    pub fn place(&mut self, x: isize, y: isize, art: &[String], color: u8) {
        for (dy, line) in art.iter().enumerate() {
            let row = y + dy as isize;
            if !(0..ROWS as isize).contains(&row) { continue; }
            for (dx, ch) in line.chars().enumerate() {
                if ch == ' ' { continue; }
                let col = x + dx as isize;
                if !(0..self.w as isize).contains(&col) { continue; }
                self.cells[row as usize * self.w + col as usize] = (ch, color);
            }
        }
    }

    pub fn put(&mut self, x: isize, y: isize, ch: char, color: u8) {
        if !(0..self.w as isize).contains(&x) || !(0..ROWS as isize).contains(&y) {
            return;
        }
        self.cells[y as usize * self.w + x as usize] = (ch, color);
    }

    pub fn put_str(&mut self, x: isize, y: isize, s: &str, color: u8) {
        for (i, ch) in s.chars().enumerate() {
            self.put(x + i as isize, y, ch, color);
        }
    }

    /// Render to ROWS ANSI strings. `cat_color` is the ANSI prefix for COL_CAT
    /// cells (rarity/shiny tint computed by the caller).
    pub fn emit(&self, cat_color: &str) -> Vec<String> {
        use crate::colors::{C_DIM, C_MAGENTA, C_PINK, C_RESET, C_YELLOW};
        let mut rows = Vec::with_capacity(ROWS);
        for y in 0..ROWS {
            let mut out = String::with_capacity(self.w + 24);
            let mut cur = COL_BG;
            for &(ch, col) in &self.cells[y * self.w..(y + 1) * self.w] {
                if col != cur {
                    out.push_str(C_RESET);
                    match col {
                        COL_CAT => out.push_str(cat_color),
                        COL_YELLOW => out.push_str(C_YELLOW),
                        COL_MAGENTA => out.push_str(C_MAGENTA),
                        COL_PINK => out.push_str(C_PINK),
                        COL_DIM => out.push_str(C_DIM),
                        _ => {} // COL_BG and unknown ids render unstyled
                    }
                    cur = col;
                }
                out.push(ch);
            }
            out.push_str(C_RESET);
            rows.push(out);
        }
        rows
    }

    /// JSON twin of `emit`: one span per run of same-colour cells. Plain text
    /// per row == the ANSI row stripped. (Not used for ANSI output — `emit`'s
    /// reset-on-every-change byte pattern is what the statusline goldens pin.)
    pub fn emit_spans(&self, cat_attrs: &[crate::colors::Attr]) -> Vec<Vec<crate::span::Span>> {
        use crate::colors::{Attr, MAGENTA, PINK, YELLOW};
        use crate::span::Span;
        let attrs_for = |col: u8| -> Vec<Attr> {
            match col {
                COL_CAT => cat_attrs.to_vec(),
                COL_YELLOW => vec![Attr::Fg(YELLOW)],
                COL_MAGENTA => vec![Attr::Fg(MAGENTA)],
                COL_PINK => vec![Attr::Fg(PINK)],
                COL_DIM => vec![Attr::Dim],
                _ => vec![], // COL_BG and unknown ids are unstyled, as in emit()
            }
        };
        let mut rows = Vec::with_capacity(ROWS);
        for y in 0..ROWS {
            let mut row: Vec<Span> = Vec::new();
            let mut cur: Option<u8> = None;
            for &(ch, col) in &self.cells[y * self.w..(y + 1) * self.w] {
                if cur != Some(col) {
                    row.push(Span::new(String::new(), &attrs_for(col)));
                    cur = Some(col);
                }
                row.last_mut().expect("pushed above").text.push(ch);
            }
            rows.push(row);
        }
        rows
    }
}

/// One rendered frame: exactly ROWS ANSI strings, plain width == strip width.
pub struct SceneFrame {
    pub rows: Vec<String>,
}

/// Triangle wave 0..=max..=0 as t grows: position for back-and-forth motion.
pub(crate) fn tri(t: u64, max: usize) -> usize {
    if max == 0 { return 0; }
    let period = (2 * max) as u64;
    let p = (t % period) as usize;
    if p <= max { p } else { 2 * max - p }
}

/// True while the triangle wave is in its rising (rightward) half.
pub(crate) fn tri_rising(t: u64, max: usize) -> bool {
    if max == 0 { return true; }
    ((t % (2 * max) as u64) as usize) < max
}

pub(crate) fn art_width(art: &[String]) -> usize {
    art.iter().map(|l| l.chars().count()).max().unwrap_or(0)
}

const RUN_SPEED: u64 = 3; // cells per second reads as a trot at 1 fps

/// Horizontal cell budget every playground pose fits in (art test enforces <= 12).
pub(crate) const CAT_W: usize = 12;

fn cat_art(key: &str, blink: u8) -> Vec<String> {
    crate::sprites::get_playground_sprite(key, blink)
}

/// Playground art keeps its true height; the ground line is canvas row 5.
fn place_bottom(canvas: &mut Canvas, x: isize, art: &[String], color: u8) {
    let y = ROWS as isize - art.len() as isize;
    canvas.place(x, y, art, color);
}

/// Shared back-and-forth run movement. Returns (x, facing_right).
fn run_leg(canvas: &mut Canvas, t: u64, blink: u8) -> (usize, bool) {
    let span = canvas.width().saturating_sub(CAT_W);
    let x = tri(t, span);
    let right = tri_rising(t, span);
    let key = if right { "run_right" } else { "run_left" };
    place_bottom(canvas, x as isize, &cat_art(key, blink), COL_CAT);
    (x, right)
}

fn run_scene(canvas: &mut Canvas, elapsed: u64, seed: u64, blink: u8) {
    run_leg(canvas, elapsed * RUN_SPEED + seed % 97, blink);
}

const ZOOMIES_SPEED: u64 = 8;

fn zoomies_scene(canvas: &mut Canvas, elapsed: u64, seed: u64, blink: u8) {
    let (x, right) = run_leg(canvas, elapsed * ZOOMIES_SPEED + seed % 97, blink);
    // skid dust kicked up behind her
    for d in [2isize, 4, 6] {
        let tx = if right {
            x as isize - d
        } else {
            x as isize + CAT_W as isize - 1 + d
        };
        canvas.put(tx, 5, '-', COL_DIM);
    }
}

const ROLL_SPEED: u64 = 3;

fn roll_scene(canvas: &mut Canvas, elapsed: u64, seed: u64, blink: u8) {
    let span = canvas.width().saturating_sub(CAT_W);
    let x = ((elapsed * ROLL_SPEED + seed % 13) as usize).min(span);
    let key = if x >= span { "sit" } else { "roll" };
    place_bottom(canvas, x as isize, &cat_art(key, blink), COL_CAT);
}

const BALL_SPEED: u64 = 4;
const CHASE_LAG_SECS: u64 = 2;

fn ball_scene(canvas: &mut Canvas, elapsed: u64, duration: u64, seed: u64, blink: u8) {
    let w = canvas.width();
    let finale = elapsed + FINALE_SECS >= duration;
    let phase = seed % 89;
    let cspan = w.saturating_sub(CAT_W);
    // the cat chases the ball's trajectory with a 2 s lag
    let ct = elapsed.saturating_sub(CHASE_LAG_SECS) * BALL_SPEED + phase;
    let cx = tri(ct, cspan);
    if finale {
        let art = cat_art("pounce", blink);
        place_bottom(canvas, cx as isize, &art, COL_CAT);
        // ball trapped under her paws
        canvas.put(cx as isize + art_width(&art) as isize - 2, 5, 'o', COL_YELLOW);
    } else {
        let key = if tri_rising(ct, cspan) { "run_right" } else { "run_left" };
        place_bottom(canvas, cx as isize, &cat_art(key, blink), COL_CAT);
        // ball leads, bouncing between the ground and one row up
        let bx = tri(elapsed * BALL_SPEED + phase, w.saturating_sub(1));
        let by = if elapsed.is_multiple_of(2) { 5 } else { 4 };
        canvas.put(bx as isize, by, 'o', COL_YELLOW);
    }
}

const BUTTERFLY_SPEED: u64 = 2;

fn butterfly_scene(canvas: &mut Canvas, elapsed: u64, duration: u64, seed: u64, blink: u8) {
    let w = canvas.width();
    let finale = elapsed + FINALE_SECS >= duration;
    let phase = seed % 83;
    let bspan = w.saturating_sub(3);
    let bfly = if blink == 0 { "}{" } else { "}i{" };
    let cspan = w.saturating_sub(CAT_W);
    if finale {
        // she commits to the pounce where the chase ended; the butterfly
        // escapes to the upper air
        let ct = duration.saturating_sub(FINALE_SECS + 2) * BUTTERFLY_SPEED + phase;
        let cx = tri(ct, cspan);
        place_bottom(canvas, cx as isize, &cat_art("pounce", blink), COL_CAT);
        let ex = tri(elapsed * 5 + phase, bspan).max(bspan / 2);
        canvas.put_str(ex as isize, 0, bfly, COL_MAGENTA);
    } else {
        let bx = tri(elapsed * BUTTERFLY_SPEED + phase, bspan);
        // (elapsed % 3 + seed % 3) never exceeds 4 — a plain elapsed + seed
        // would overflow in debug builds when seed is near u64::MAX.
        let by = ((elapsed % 3 + seed % 3) % 3) as isize; // flutter across rows 0..2
        let ct = elapsed.saturating_sub(2) * BUTTERFLY_SPEED + phase;
        let cx = tri(ct, cspan);
        let key = if tri_rising(ct, cspan) { "run_right" } else { "run_left" };
        place_bottom(canvas, cx as isize, &cat_art(key, blink), COL_CAT);
        canvas.put_str(bx as isize, by, bfly, COL_MAGENTA);
    }
}

const YARN_SPEED: u64 = 2;

fn yarn_scene(canvas: &mut Canvas, elapsed: u64, seed: u64, blink: u8) {
    let w = canvas.width();
    let span = w.saturating_sub(CAT_W + 2); // room for the yarn ball ahead
    let t = elapsed * YARN_SPEED + seed % 79;
    let x = tri(t, span) as isize;
    let right = tri_rising(t, span);
    let key = if right { "run_right" } else { "run_left" };
    place_bottom(canvas, x, &cat_art(key, blink), COL_CAT);
    // yarn ball rides just ahead of her paws; unraveled string curls behind
    let yx = if right { x + CAT_W as isize } else { x - 2 };
    canvas.put(yx, 5, '@', COL_PINK);
    for d in 1..=3isize {
        let tx = if right {
            x - 2 * d
        } else {
            x + CAT_W as isize - 1 + 2 * d
        };
        canvas.put(tx, 5, '~', COL_PINK);
    }
}

fn compose(kind: PlaygroundScene, elapsed: u64, duration: u64, seed: u64, strip_width: usize, blink: u8) -> Canvas {
    let mut canvas = Canvas::new(strip_width);
    match kind {
        PlaygroundScene::None => {}
        PlaygroundScene::Run => run_scene(&mut canvas, elapsed, seed, blink),
        PlaygroundScene::Zoomies => zoomies_scene(&mut canvas, elapsed, seed, blink),
        PlaygroundScene::BallChase => ball_scene(&mut canvas, elapsed, duration, seed, blink),
        PlaygroundScene::ButterflyChase => butterfly_scene(&mut canvas, elapsed, duration, seed, blink),
        PlaygroundScene::Roll => roll_scene(&mut canvas, elapsed, seed, blink),
        PlaygroundScene::YarnPush => yarn_scene(&mut canvas, elapsed, seed, blink),
    }
    canvas
}

/// Pure frame renderer. `blink` = (now % 2), same parity the home sprite uses.
pub fn render_scene(
    kind: PlaygroundScene,
    elapsed: u64,
    duration: u64,
    seed: u64,
    strip_width: usize,
    blink: u8,
    cat_color: &str,
) -> SceneFrame {
    SceneFrame { rows: compose(kind, elapsed, duration, seed, strip_width, blink).emit(cat_color) }
}

/// Span twin of [`render_scene`] for `frame --json`.
pub fn render_scene_spans(
    kind: PlaygroundScene,
    elapsed: u64,
    duration: u64,
    seed: u64,
    strip_width: usize,
    blink: u8,
    cat_attrs: &[crate::colors::Attr],
) -> Vec<Vec<crate::span::Span>> {
    compose(kind, elapsed, duration, seed, strip_width, blink).emit_spans(cat_attrs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_only_zero_disables() {
        assert!(enabled_from(None));
        assert!(enabled_from(Some("1")));
        assert!(enabled_from(Some("")));
        assert!(!enabled_from(Some("0")));
    }

    #[test]
    fn strip_width_from_columns() {
        // 200 cols - 101 overhead = 99
        assert_eq!(strip_width_from(Some("200"), None), Some(99));
        // clamped to MAX_STRIP
        assert_eq!(strip_width_from(Some("500"), None), Some(MAX_STRIP));
        // 120 cols - 101 = 19 < MIN_STRIP -> None
        assert_eq!(strip_width_from(Some("120"), None), None);
        // garbage COLUMNS -> DEFAULT_STRIP
        assert_eq!(strip_width_from(Some("wat"), None), Some(DEFAULT_STRIP));
        // absent COLUMNS -> DEFAULT_STRIP
        assert_eq!(strip_width_from(None, None), Some(DEFAULT_STRIP));
    }

    #[test]
    fn strip_width_override_wins() {
        assert_eq!(strip_width_from(Some("200"), Some("40")), Some(40));
        assert_eq!(strip_width_from(None, Some("40")), Some(40));
        assert_eq!(strip_width_from(None, Some("10")), None); // below MIN
        assert_eq!(strip_width_from(None, Some("999")), Some(MAX_STRIP));
        assert_eq!(strip_width_from(Some("200"), Some("junk")), Some(99)); // bad override ignored
    }

    #[test]
    fn force_scene_parses_known_names_only() {
        assert_eq!(force_scene_from(Some("run")), Some(PlaygroundScene::Run));
        assert_eq!(force_scene_from(Some("ball")), Some(PlaygroundScene::BallChase));
        assert_eq!(force_scene_from(Some("yarn")), Some(PlaygroundScene::YarnPush));
        assert_eq!(force_scene_from(Some("nope")), None);
        assert_eq!(force_scene_from(None), None);
    }

    #[test]
    fn bootstrap_schedules_first_scene() {
        let t = plan_transition(PlaygroundScene::None, 0, 0, 0, PlaygroundScene::None, false, 1000);
        match t {
            SceneTransition::Bootstrap { next_at } => {
                assert!((1000 + IDLE_MIN_SECS..=1000 + IDLE_MAX_SECS).contains(&next_at));
            }
            SceneTransition::None | SceneTransition::End { .. } | SceneTransition::Start { .. } => {
                panic!("expected Bootstrap, got {t:?}")
            }
        }
    }

    #[test]
    fn start_fires_only_at_or_after_next_at_and_without_overlay() {
        // before next_at -> None
        let t = plan_transition(PlaygroundScene::None, 0, 0, 5000, PlaygroundScene::None, false, 4999);
        assert_eq!(t, SceneTransition::None);
        // at next_at with overlay -> None (deferred)
        let t = plan_transition(PlaygroundScene::None, 0, 0, 5000, PlaygroundScene::None, true, 5000);
        assert_eq!(t, SceneTransition::None);
        // at next_at, no overlay -> Start with sane duration
        let t = plan_transition(PlaygroundScene::None, 0, 0, 5000, PlaygroundScene::None, false, 5000);
        match t {
            SceneTransition::Start { kind, duration_secs, seed: _ } => {
                assert_ne!(kind, PlaygroundScene::None);
                assert!((SCENE_MIN_SECS..=SCENE_MAX_SECS).contains(&duration_secs));
            }
            SceneTransition::None | SceneTransition::Bootstrap { .. } | SceneTransition::End { .. } => {
                panic!("expected Start, got {t:?}")
            }
        }
    }

    #[test]
    fn active_scene_ends_after_duration_and_on_clock_regression() {
        // mid-scene -> None
        let t = plan_transition(PlaygroundScene::Run, 1000, 30, 0, PlaygroundScene::None, false, 1015);
        assert_eq!(t, SceneTransition::None);
        // expired -> End carrying last_kind
        let t = plan_transition(PlaygroundScene::Run, 1000, 30, 0, PlaygroundScene::None, false, 1030);
        match t {
            SceneTransition::End { last_kind, next_at } => {
                assert_eq!(last_kind, PlaygroundScene::Run);
                assert!((1030 + IDLE_MIN_SECS..=1030 + IDLE_MAX_SECS).contains(&next_at));
            }
            SceneTransition::None | SceneTransition::Bootstrap { .. } | SceneTransition::Start { .. } => {
                panic!("expected End, got {t:?}")
            }
        }
        // clock went backwards -> End too
        let t = plan_transition(PlaygroundScene::Run, 1000, 30, 0, PlaygroundScene::None, false, 500);
        assert!(matches!(t, SceneTransition::End { .. }));
    }

    #[test]
    fn overlay_does_not_block_end_transition() {
        let t = plan_transition(PlaygroundScene::Run, 1000, 30, 0, PlaygroundScene::None, true, 2000);
        assert!(matches!(t, SceneTransition::End { .. }));
    }

    #[test]
    fn picked_scene_rarely_repeats_last_kind() {
        // One re-roll on collision. With the current Lcg, consecutive outputs
        // alternate parity so repeats are actually impossible (expect 0); the
        // loose <30/300 bound only guards against a future RNG swap.
        let mut repeats = 0;
        for now in 0..300u64 {
            let t = plan_transition(
                PlaygroundScene::None, 0, 0, 1, PlaygroundScene::Run, false, 10_000 + now,
            );
            if let SceneTransition::Start { kind, .. } = t {
                if kind == PlaygroundScene::Run { repeats += 1; }
            } else {
                panic!("expected Start");
            }
        }
        assert!(repeats < 30, "no-repeat re-roll looks broken: {repeats}/300");
    }

    #[test]
    fn start_scene_now_activates_and_defers_scheduler() {
        let mut s = crate::state::State::base(1000);
        s.scene_kind = PlaygroundScene::Roll; // pretend one is already active
        start_scene_now(&mut s, 5000);
        assert_ne!(s.scene_kind, PlaygroundScene::None);
        assert_eq!(s.scene_started_at, 5000);
        assert!((SCENE_MIN_SECS..=SCENE_MAX_SECS).contains(&s.scene_duration_secs));
        assert!((5000 + IDLE_MIN_SECS..=5000 + IDLE_MAX_SECS).contains(&s.scene_next_at));
        assert_eq!(s.scene_last_kind, PlaygroundScene::Roll);
    }

    pub(super) fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut in_esc = false;
        for ch in s.chars() {
            if in_esc {
                if ch == 'm' { in_esc = false; }
            } else if ch == '\x1b' {
                in_esc = true;
            } else {
                out.push(ch);
            }
        }
        out
    }

    #[test]
    fn canvas_emit_rows_have_exact_plain_width() {
        let c = Canvas::new(30);
        let rows = c.emit(crate::colors::C_CYAN);
        assert_eq!(rows.len(), ROWS);
        for r in &rows {
            assert_eq!(strip_ansi(r).chars().count(), 30);
            assert!(r.ends_with(crate::colors::C_RESET));
        }
    }

    #[test]
    fn place_clips_and_treats_spaces_as_transparent() {
        let mut c = Canvas::new(10);
        let art = vec!["ab".to_string(), " c".to_string()];
        // partially off the left edge and off the top
        c.place(-1, -1, &art, COL_CAT);
        let rows = c.emit(crate::colors::C_CYAN);
        // row -1 clipped entirely; art row 1 lands on canvas row 0: " c" at x=-1
        // -> 'c' at x=0. Space stayed transparent (background remains ' ').
        assert_eq!(strip_ansi(&rows[0]), "c         ");
        // off the right edge
        let mut c = Canvas::new(10);
        c.place(9, 0, &art, COL_CAT);
        let rows = c.emit(crate::colors::C_CYAN);
        assert_eq!(strip_ansi(&rows[0]), "         a"); // 'b' clipped
    }

    #[test]
    fn put_and_put_str_land_where_told() {
        let mut c = Canvas::new(12);
        c.put(3, 5, 'o', COL_YELLOW);
        c.put_str(6, 2, "}{", COL_MAGENTA);
        let rows = c.emit(crate::colors::C_CYAN);
        assert_eq!(strip_ansi(&rows[5]).chars().nth(3), Some('o'));
        assert_eq!(&strip_ansi(&rows[2])[6..8], "}{");
        // out-of-bounds puts are silently clipped
        c.put(-5, 0, 'x', COL_PINK);
        c.put(99, 0, 'x', COL_PINK);
    }

    #[test]
    fn emit_uses_cat_color_for_cat_cells_and_named_colors_for_props() {
        let mut c = Canvas::new(8);
        c.put(0, 0, 'C', COL_CAT);
        c.put(2, 0, 'o', COL_YELLOW);
        let rows = c.emit(crate::colors::C_GREEN);
        assert!(rows[0].contains(crate::colors::C_GREEN));
        assert!(rows[0].contains(crate::colors::C_YELLOW));
    }

    #[test]
    fn tri_wave_bounces_between_zero_and_max() {
        assert_eq!(tri(0, 10), 0);
        assert_eq!(tri(10, 10), 10);
        assert_eq!(tri(15, 10), 5);
        assert_eq!(tri(20, 10), 0);
        assert_eq!(tri(25, 10), 5);
        assert_eq!(tri(7, 0), 0); // degenerate strip
        assert!(tri_rising(3, 10));
        assert!(!tri_rising(13, 10));
    }

    #[test]
    fn render_scene_rows_are_exact_width_for_all_kinds() {
        for kind in ACTIVE_SCENES {
            let f = render_scene(kind, 7, 30, 999, 48, 1, crate::colors::C_CYAN);
            assert_eq!(f.rows.len(), ROWS);
            for r in &f.rows {
                assert_eq!(strip_ansi(r).chars().count(), 48, "{kind:?}");
            }
        }
    }

    #[test]
    fn render_scene_is_deterministic() {
        let a = render_scene(PlaygroundScene::Run, 12, 30, 5, 60, 0, crate::colors::C_CYAN);
        let b = render_scene(PlaygroundScene::Run, 12, 30, 5, 60, 0, crate::colors::C_CYAN);
        assert_eq!(a.rows, b.rows);
    }

    #[test]
    fn run_scene_cat_moves_over_time() {
        // The cat's leftmost inked column must differ between two elapsed values
        // closer together than a full bounce period.
        let inked_start = |f: &SceneFrame| -> Option<usize> {
            f.rows.iter()
                .filter_map(|r| strip_ansi(r).chars().position(|c| c != ' '))
                .min()
        };
        let f0 = render_scene(PlaygroundScene::Run, 0, 30, 0, 60, 0, crate::colors::C_CYAN);
        let f2 = render_scene(PlaygroundScene::Run, 2, 30, 0, 60, 0, crate::colors::C_CYAN);
        assert_ne!(inked_start(&f0), inked_start(&f2), "cat did not move");
    }

    #[test]
    fn render_scene_none_is_blank() {
        let f = render_scene(PlaygroundScene::None, 5, 30, 1, 40, 0, crate::colors::C_CYAN);
        for r in &f.rows {
            assert!(strip_ansi(r).trim().is_empty());
        }
    }

    #[test]
    fn run_uses_side_view_art_not_placeholder() {
        let f = render_scene(PlaygroundScene::Run, 1, 30, 0, 60, 0, crate::colors::C_CYAN);
        let plain: String = f.rows.iter().map(|r| strip_ansi(r)).collect::<Vec<_>>().join("\n");
        // side-view run art has the ~~ tail + > muzzle; placeholder was the
        // front-facing 6-line HUD sprite
        assert!(plain.contains("( o.o )>") || plain.contains("<( o.o )"), "{plain}");
    }

    #[test]
    fn zoomies_moves_faster_than_run() {
        let right_edge = |kind: PlaygroundScene, t: u64| -> usize {
            let f = render_scene(kind, t, 30, 0, 100, 0, crate::colors::C_CYAN);
            f.rows.iter().filter_map(|r| {
                let p = strip_ansi(r);
                p.chars().enumerate().filter(|(_, c)| *c != ' ').map(|(i, _)| i).max()
            }).max().unwrap()
        };
        let run_delta = right_edge(PlaygroundScene::Run, 1) - right_edge(PlaygroundScene::Run, 0);
        let zoom_delta = right_edge(PlaygroundScene::Zoomies, 1) - right_edge(PlaygroundScene::Zoomies, 0);
        assert!(zoom_delta > run_delta, "zoomies {zoom_delta} vs run {run_delta}");
    }

    #[test]
    fn roll_travels_and_sits_at_the_end() {
        // span = 100 - 12 = 88; seed 0 -> arrival once elapsed*3 >= 88
        let plain = |t: u64| -> Vec<String> {
            render_scene(PlaygroundScene::Roll, t, 40, 0, 100, 0, crate::colors::C_CYAN)
                .rows.iter().map(|r| strip_ansi(r)).collect()
        };
        assert!(plain(5).iter().any(|r| r.contains("'-..-'")), "mid-scene shows roll art");
        assert!(plain(35).iter().any(|r| r.contains("/\\_/\\")), "end shows sitting cat ears");
    }

    #[test]
    fn ball_chase_shows_yellow_ball_mid_scene_and_catch_at_finale() {
        let mid = render_scene(PlaygroundScene::BallChase, 5, 30, 7, 80, 0, crate::colors::C_CYAN);
        assert!(
            mid.rows.iter().any(|r| r.contains(crate::colors::C_YELLOW)),
            "ball (yellow) visible mid-scene"
        );
        let plain_mid: String = mid.rows.iter().map(|r| strip_ansi(r)).collect::<Vec<_>>().join("\n");
        assert!(!plain_mid.contains(">.<"), "no pounce before the finale");

        let fin = render_scene(PlaygroundScene::BallChase, 27, 30, 7, 80, 0, crate::colors::C_CYAN);
        let plain_fin: String = fin.rows.iter().map(|r| strip_ansi(r)).collect::<Vec<_>>().join("\n");
        assert!(plain_fin.contains(">.<"), "pounce face at finale");
        assert!(fin.rows.iter().any(|r| r.contains(crate::colors::C_YELLOW)), "ball trapped under paws");
    }

    #[test]
    fn butterfly_flutters_in_the_upper_rows() {
        let f = render_scene(PlaygroundScene::ButterflyChase, 6, 30, 11, 80, 0, crate::colors::C_CYAN);
        assert!(f.rows.iter().any(|r| r.contains(crate::colors::C_MAGENTA)), "butterfly is magenta");
        let plain: Vec<String> = f.rows.iter().map(|r| strip_ansi(r)).collect();
        assert!(
            plain[0].contains("}{") || plain[1].contains("}{") || plain[2].contains("}{"),
            "butterfly in rows 0-2"
        );
    }

    #[test]
    fn butterfly_finale_pounces_and_butterfly_escapes_to_top_row() {
        let f = render_scene(PlaygroundScene::ButterflyChase, 27, 30, 11, 80, 1, crate::colors::C_CYAN);
        let plain: Vec<String> = f.rows.iter().map(|r| strip_ansi(r)).collect();
        assert!(plain.iter().any(|r| r.contains(">.<")), "pounce at finale");
        assert!(plain[0].contains("}i{"), "butterfly escaped to the top row");
    }

    #[test]
    fn yarn_scene_shows_ball_and_trail_on_ground_row() {
        let f = render_scene(PlaygroundScene::YarnPush, 8, 30, 3, 80, 0, crate::colors::C_CYAN);
        let plain: Vec<String> = f.rows.iter().map(|r| strip_ansi(r)).collect();
        assert!(plain[5].contains('@'), "yarn ball on the ground row");
        assert!(plain[5].contains('~'), "trail behind her");
        assert!(f.rows.iter().any(|r| r.contains(crate::colors::C_PINK)), "yarn is pink");
    }

    #[test]
    fn alignment_sweep_every_scene_width_and_second() {
        // The load-bearing invariant: no rendered row may ever deviate from the
        // strip width — a longer row wraps the terminal line and shatters the
        // whole 7-row layout.
        for kind in ACTIVE_SCENES {
            for w in [MIN_STRIP, 60, MAX_STRIP] {
                for elapsed in 0..=45u64 {
                    for (duration, seed) in [(20u64, 0u64), (30, 12345), (40, u64::MAX)] {
                        let f = render_scene(
                            kind, elapsed, duration, seed, w,
                            (elapsed % 2) as u8, crate::colors::C_CYAN,
                        );
                        assert_eq!(f.rows.len(), ROWS);
                        for (i, row) in f.rows.iter().enumerate() {
                            assert_eq!(
                                strip_ansi(row).chars().count(), w,
                                "{kind:?} w={w} t={elapsed} d={duration} seed={seed} row={i}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn clamp_strip_bounds() {
        assert_eq!(clamp_strip(MIN_STRIP - 1), None);
        assert_eq!(clamp_strip(MIN_STRIP), Some(MIN_STRIP));
        assert_eq!(clamp_strip(500), Some(MAX_STRIP));
    }

    #[test]
    fn emit_spans_carries_the_same_text_as_emit() {
        use crate::colors::{Attr, CYAN};
        for kind in ACTIVE_SCENES {
            for t in [0u64, 7, 27] {
                let ansi = render_scene(kind, t, 30, 11, 64, (t % 2) as u8, crate::colors::C_CYAN);
                let spans = render_scene_spans(kind, t, 30, 11, 64, (t % 2) as u8, &[Attr::Fg(CYAN)]);
                assert_eq!(spans.len(), ROWS);
                for (a, s) in ansi.rows.iter().zip(&spans) {
                    assert_eq!(crate::span::to_plain(s), crate::span::strip_ansi(a), "{kind:?} t={t}");
                    assert_eq!(crate::span::width(s), 64);
                }
            }
        }
    }

    #[test]
    fn emit_spans_colours_cat_and_props() {
        use crate::colors::{Attr, CYAN, YELLOW};
        let rows = render_scene_spans(PlaygroundScene::BallChase, 5, 30, 7, 80, 0, &[Attr::Fg(CYAN)]);
        let all: Vec<_> = rows.iter().flatten().collect();
        assert!(all.iter().any(|s| s.attrs == vec![Attr::Fg(YELLOW)]), "ball is yellow");
        assert!(all.iter().any(|s| s.attrs == vec![Attr::Fg(CYAN)]), "cat carries the rarity tint");
    }
}
