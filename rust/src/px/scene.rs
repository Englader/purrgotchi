//! Pixel playground scenes: the six `playground.rs` scenes drawn with the
//! pixel poses and props. Scheduling, timing and speech stay in
//! `playground.rs` / `kitten_view`; this only decides what each second looks
//! like, as a pure function of `(kind, elapsed, duration, seed, width, h)`.
//!
//! Run, zoomies, roll and yarn mirror the ASCII paths (same speeds and
//! phases; a `Turn` or a dizzy `Sit` marks each wall bounce). The two chase
//! scenes re-simulate the cat second by second from `elapsed = 0` (at most
//! `SIM_CAP` steps), so she can react to the prop: crouch when the ball rolls
//! back at her, pounce, miss, turn and chase again; stalk the butterfly, look
//! up and swat. The last `FINALE_SECS` seconds are the payoff, in step with the
//! finale speech: crouch, airborne pounce, land (ball trapped / butterfly gone).

use super::art::{self, Pose, Prop, Tail};
use super::paint::{self, CatLook};
use super::tempo::Tempo;
use super::{PxCanvas, PxHeight};
use crate::colors::Rgb;
use crate::pixel;
use crate::playground::{tri, tri_rising, FINALE_SECS};
use crate::state::PlaygroundScene;

/// Canvas height in px, for the signed geometry.
pub(super) fn ph(h: PxHeight) -> i32 {
    h.px() as i32
}

/// Widest cat sprite in px, for the signed geometry.
fn cat_w(h: PxHeight) -> i32 {
    h.cat_max_w() as i32
}

/// How far a pounce carries her before she lands (2 px more on landing).
const LEAP: i32 = 4;
/// Upper bound on simulated seconds (scenes last at most 40 s; a stale state
/// with a huge `elapsed` is in its finale anyway).
const SIM_CAP: u64 = 64;
/// Px either side of the yarn walk, so the yarn fits ahead in both facings.
const YARN_MARGIN: i32 = 4;
const BALL_W: i32 = 3;
const BFLY_W: i32 = 5;
/// Px of stage the finale needs ahead of the crouching cat: the 6 px leap
/// plus the 3 px ball rolling in (the butterfly needs less).
const FINALE_ROOM: i32 = LEAP + 2 + BALL_W;
/// The band finale (`Tempo::smooth`): the crouch holds this long, then the
/// leap is airborne for `LEAP_MS` before she lands.
const CROUCH_MS: u64 = 1000;
const LEAP_MS: u64 = 400;
/// In the band, roll's dizzy sit at a wall lasts this many turns.
const DIZZY: u64 = 3;

/// A prop on the stage. `front` props are drawn over the cat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Placed {
    pub prop: Prop,
    pub frame: u8,
    pub x: i32,
    pub y: i32,
    pub front: bool,
}

/// One scene second as geometry: the cat (left px, facing, px off the
/// ground), her props and loose ground pixels (dust, yarn trail).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SceneFrame {
    pub pose: Pose,
    pub x: i32,
    pub right: bool,
    pub lift: i32,
    pub props: Vec<Placed>,
    pub dots: Vec<(i32, i32, Rgb)>,
}

impl SceneFrame {
    fn cat(pose: Pose, x: i32, right: bool) -> Self {
        Self { pose, x, right, lift: 0, props: Vec::new(), dots: Vec::new() }
    }

    /// Three ground px trailing behind her (skid dust, unravelled yarn).
    fn trail(mut self, col: Rgb, h: PxHeight) -> Self {
        let (x, right) = (self.x, self.right);
        self.dots = [2, 4, 6].iter().map(|&d| (if right { x - d } else { x + cat_w(h) - 1 + d }, ph(h) - 1, col)).collect();
        self
    }

    /// Mid-leap: raised off the ground over her own shadow. (An adult pounce
    /// has 1 px of headroom, so the shadow is what sells the jump.)
    fn airborne(mut self, h: PxHeight) -> Self {
        self.lift = 2;
        self.dots.extend((4..cat_w(h) - 4).map(|dx| (self.x + dx, ph(h) - 1, paint::OUTLINE)));
        self
    }

    /// Add a prop, clamped so it never clips the canvas.
    pub(super) fn put(&mut self, (w, h): (usize, PxHeight), prop: Prop, frame: u8, x: i32, y: i32, front: bool) {
        let s = art::prop(prop, frame);
        let (x, y) = (x.clamp(0, (w as i32 - s.w as i32).max(0)), y.clamp(0, ph(h) - s.h as i32));
        self.props.push(Placed { prop, frame, x, y, front });
    }
}

/// Alternates `a` and `b` every `gait_ms`, the walking gait (every second
/// at the statusline tempo).
fn gait(ms: u64, a: Pose, b: Pose, t: &Tempo) -> Pose {
    if (ms / t.gait_ms).is_multiple_of(2) { a } else { b }
}

/// Run1 / Run2 at the running gait, as in life.
fn run_gait(ms: u64, t: &Tempo) -> Pose {
    if (ms / t.run_gait_ms).is_multiple_of(2) { Pose::Run1 } else { Pose::Run2 }
}

/// The 2-frame phase (0 or 1) of a prop at `ms`: butterfly wings.
fn frame(ms: u64, t: &Tempo) -> u8 {
    (ms / t.prop_ms % 2) as u8
}

/// Px a chasing cat covers per sim step at most (her run), and at a stroll.
fn dash(t: &Tempo) -> i32 {
    (t.run_px_s as u64 * t.sim_step_ms / 1000) as i32
}

fn stroll(t: &Tempo) -> i32 {
    (t.walk_px_s as u64 * t.sim_step_ms / 1000) as i32
}

/// Sim steps in `ms` (a cooldown).
fn steps(ms: u64, t: &Tempo) -> u8 {
    (ms / t.sim_step_ms).min(u64::from(u8::MAX)) as u8
}

/// A hop of `peak` px: one parabola every `period` ms, `peak` at its middle.
pub(super) fn hop(ms: u64, period: u64, peak: i32) -> i32 {
    let (q, p) = ((ms % period) as i64, period as i64);
    (i64::from(peak) * 4 * q * (p - q) / (p * p)) as i32
}

fn dir(right: bool) -> i32 {
    if right { 1 } else { -1 }
}

/// Room for the cat's left px.
fn cat_span(w: usize, h: PxHeight) -> usize {
    w.saturating_sub(h.cat_max_w())
}

/// Left px of a `pw`-wide prop `gap` px ahead of the cat's nose.
fn ahead(x: i32, right: bool, gap: i32, pw: i32, h: PxHeight) -> i32 {
    if right { x + cat_w(h) + gap } else { x - gap - pw }
}

/// Back-and-forth path at `speed` px/s, starting `phase` px in: (x, facing
/// right, turning at a wall). At 1 fps she turns over the `turn_ms` after
/// a bounce while moving on. In the band she stops at each wall for `dwell`
/// ms to turn (0 for props and zoomies), and x rounds to the nearest px, so
/// a 30-fps clock a ms early or late lands on the same px.
fn bounce(ms: u64, speed: u64, phase: u64, span: usize, dwell: u64, t: &Tempo) -> (i32, bool, bool) {
    if !t.smooth {
        let pos = |ms: u64| ms.wrapping_mul(speed) / 1000 + phase;
        let right = tri_rising(pos(ms), span);
        let turned = ms >= t.turn_ms && right != tri_rising(pos(ms - t.turn_ms), span);
        return (tri(pos(ms), span) as i32, right, turned);
    }
    if span == 0 || speed == 0 {
        return (0, true, false);
    }
    let leg = span as u64 * 1000 / speed;
    let cycle = 2 * (leg + dwell);
    // `phase` px along the outbound leg, or past the far wall's dwell.
    let start = phase % (2 * span as u64) * 1000 / speed;
    let start = if start < leg { start } else { start + dwell };
    let k = (ms % cycle + start) % cycle;
    let px = |ms: u64| ((ms * speed + 500) / 1000).min(span as u64) as i32;
    match k {
        k if k < leg => (px(k), true, false),
        k if k < leg + dwell => (span as i32, false, true),
        k if k < 2 * leg + dwell => (span as i32 - px(k - leg - dwell), false, false),
        _ => (0, true, true),
    }
}

/// The scene at whole second `elapsed` at the statusline tempo.
pub(crate) fn scene_frame(kind: PlaygroundScene, elapsed: u64, duration: u64, seed: u64, width: usize, h: PxHeight) -> SceneFrame {
    scene_frame_ms(kind, elapsed.saturating_mul(1000), duration, seed, width, h, &Tempo::STATUSLINE)
}

/// The scene `elapsed_ms` in. `duration` stays in seconds: the finale starts
/// `FINALE_SECS` before it, whatever the tempo, so the caption stays in step.
pub(crate) fn scene_frame_ms(kind: PlaygroundScene, elapsed_ms: u64, duration: u64, seed: u64, width: usize, h: PxHeight, t: &Tempo) -> SceneFrame {
    let finale = elapsed_ms.saturating_add(FINALE_SECS * 1000) >= duration.saturating_mul(1000);
    match kind {
        PlaygroundScene::Run => run(elapsed_ms, seed, width, h, t),
        PlaygroundScene::Zoomies => zoomies(elapsed_ms, seed, width, h, t),
        PlaygroundScene::Roll => roll(elapsed_ms, seed, width, h, t),
        PlaygroundScene::YarnPush => yarn(elapsed_ms, seed, width, h, t),
        PlaygroundScene::BallChase if finale => ball_finale(elapsed_ms, duration, seed, width, h, t),
        PlaygroundScene::BallChase => ball(elapsed_ms, seed, width, h, t),
        PlaygroundScene::ButterflyChase if finale => butterfly_finale(elapsed_ms, duration, seed, width, h, t),
        PlaygroundScene::ButterflyChase => butterfly(elapsed_ms, seed, width, h, t),
        PlaygroundScene::None => SceneFrame::cat(Pose::Sit, cat_span(width, h) as i32 / 2, true),
    }
}

/// Laps at a trot; a `Turn` at each wall.
fn run(e: u64, seed: u64, w: usize, h: PxHeight, t: &Tempo) -> SceneFrame {
    let (x, right, turned) = bounce(e, t.scene.run, seed % 97, cat_span(w, h), t.turn_ms, t);
    SceneFrame::cat(if turned { Pose::Turn } else { run_gait(e, t) }, x, right)
}

/// Flat-out laps with skid dust behind her; no time to turn around.
fn zoomies(e: u64, seed: u64, w: usize, h: PxHeight, t: &Tempo) -> SceneFrame {
    let (x, right, _) = bounce(e, t.scene.zoomies, seed % 97, cat_span(w, h), 0, t);
    SceneFrame::cat(run_gait(e, t), x, right).trail(pixel::mix(paint::OUTLINE, pixel::WHITE, 0.5), h)
}

/// Belly-up rolls across the stage and back, a dizzy sit at each end.
fn roll(e: u64, seed: u64, w: usize, h: PxHeight, t: &Tempo) -> SceneFrame {
    let (x, right, turned) = bounce(e, t.scene.roll, seed % 13, cat_span(w, h), DIZZY * t.turn_ms, t);
    SceneFrame::cat(if turned { Pose::Sit } else { gait(e, Pose::Roll1, Pose::Roll2, t) }, x, right)
}

/// Pushes the yarn along the ground, string unravelling behind; at a wall she
/// turns with the yarn tucked under her and pushes it back.
fn yarn(e: u64, seed: u64, w: usize, h: PxHeight, t: &Tempo) -> SceneFrame {
    let span = w.saturating_sub(h.cat_max_w() + 2 * YARN_MARGIN as usize);
    let (x, right, turned) = bounce(e, t.scene.yarn, seed % 79, span, t.turn_ms, t);
    let x = x + YARN_MARGIN;
    if turned {
        let mut f = SceneFrame::cat(Pose::Turn, x, right);
        f.put((w, h), Prop::Yarn, 0, x + cat_w(h) / 2 - 2, ph(h) - 4, false);
        return f;
    }
    let mut f = SceneFrame::cat(gait(e, Pose::Walk1, Pose::Walk2, t), x, right).trail(pixel::NOSE, h);
    f.put((w, h), Prop::Yarn, 0, ahead(x, right, 0, 4, h), ph(h) - 4, false);
    f
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Beat {
    Chase,
    /// Chasing at more than a walk (butterfly scene: catching up).
    Dash,
    Still,
    Crouch,
    Leap,
    Land,
    Turn,
    Look,
    /// Head up, following the butterfly (it hovers or drifts behind her nose).
    Watch,
    Swat,
}

/// The simulated cat in a chase scene.
#[derive(Clone, Copy, Debug)]
struct Cat {
    x: i32,
    right: bool,
    beat: Beat,
    /// Sim steps until the next swat may start.
    cool: u8,
    /// Sim steps the beat holds on for (a crouch or a look lasts a moment
    /// in the band; one step, a second, at 1 fps).
    hold: u8,
}

impl Cat {
    fn new(x: i32, right: bool, cool: u8) -> Self {
        Self { x, right, beat: Beat::Chase, cool, hold: 0 }
    }

    /// Start `beat`, held for about `ms` (at least one step).
    fn begin(self, beat: Beat, ms: u64, t: &Tempo) -> Self {
        Self { beat, hold: steps(ms, t).saturating_sub(1), ..self }
    }
}

/// How long a mid-scene crouch and a look up last in the band.
const CROUCH_HOLD_MS: u64 = 400;
const LOOK_HOLD_MS: u64 = 600;

/// Run `step` for sim steps `1..=to` from `start`, keeping x on the stage.
/// (`SIM_CAP` seconds' worth of steps at most.)
fn simulate(start: Cat, to: u64, w: usize, h: PxHeight, t: &Tempo, step: impl Fn(Cat, u64) -> Cat) -> Cat {
    (1..=to.min(SIM_CAP * 1000 / t.sim_step_ms)).fold(start, |c, e| {
        let n = if c.hold > 0 { Cat { hold: c.hold - 1, ..c } } else { step(c, e) };
        Cat { x: n.x.clamp(0, cat_span(w, h) as i32), ..n }
    })
}

/// When the finale starts (ms into the scene), the ms it has run (0 = the
/// crouch), and the cat it starts from: the chase as it stood (a second
/// before at 1 fps; at that moment in the band, so she doesn't jump), turned
/// toward the open stage when she has no room to leap.
fn finale_start(elapsed_ms: u64, duration: u64, w: usize, h: PxHeight, t: &Tempo, sim: impl Fn(u64) -> Cat) -> (u64, u64, Cat) {
    let start = duration.saturating_sub(FINALE_SECS);
    let from = if t.smooth { start } else { start.saturating_sub(1) };
    let mut c = sim(from.saturating_mul(1000) / t.sim_step_ms);
    let room = if c.right { cat_span(w, h) as i32 - c.x } else { c.x };
    if room < FINALE_ROOM {
        c.right = !c.right;
    }
    let start = start.saturating_mul(1000);
    (start, elapsed_ms.saturating_sub(start).min(FINALE_SECS * 1000 - 1), c)
}

/// The chase cat at `ms`, simulated in `sim_step_ms` steps. At 1 fps she
/// is step `ms / step`, sliding toward the next step's x while her beat is
/// a chase gait. In the band each step is what she does on the way to it:
/// its beat shows from the step before, while x slides there (so a leap
/// glides from the crouch, and she never runs in place before a crouch).
fn chase_cat(ms: u64, t: &Tempo, sim: impl Fn(u64) -> Cat) -> Cat {
    let (n, sub) = (ms / t.sim_step_ms, ms % t.sim_step_ms);
    let lerp = |a: i32, b: i32| toward(a, b, sub, t.sim_step_ms);
    if sub == 0 {
        return sim(n);
    }
    let (now, next) = (sim(n), sim(n + 1));
    if t.smooth {
        Cat { x: lerp(now.x, next.x), ..next }
    } else if matches!(now.beat, Beat::Chase | Beat::Dash) {
        Cat { x: lerp(now.x, next.x), ..now }
    } else {
        now
    }
}

/// The finale cat `ms` in: crouch, leap (airborne, 4 px), land (2 px more).
/// In the band the crouch holds `CROUCH_MS` and the leap arcs and glides
/// over `LEAP_MS`.
fn pounce(ms: u64, c: Cat, w: usize, h: PxHeight, t: &Tempo) -> SceneFrame {
    let at = |step: i32| (c.x + step * dir(c.right)).clamp(0, cat_span(w, h) as i32);
    if !t.smooth {
        let k = ms / 1000;
        let (step, pose) = match k {
            0 => (0, Pose::Crouch),
            1 => (LEAP, Pose::Pounce),
            _ => (LEAP + 2, Pose::Pounce),
        };
        let f = SceneFrame::cat(pose, at(step), c.right);
        return if k == 1 { f.airborne(h) } else { f };
    }
    match ms.checked_sub(CROUCH_MS) {
        None => SceneFrame::cat(Pose::Crouch, at(0), c.right),
        Some(p) if p < LEAP_MS => {
            let mut f = SceneFrame::cat(Pose::Pounce, at((LEAP + 2) * p as i32 / LEAP_MS as i32), c.right).airborne(h);
            f.lift = hop(p, LEAP_MS, 2);
            f
        }
        Some(_) => SceneFrame::cat(Pose::Pounce, at(LEAP + 2), c.right),
    }
}

/// `a` to `b` as `ms` runs over `len`.
fn toward(a: i32, b: i32, ms: u64, len: u64) -> i32 {
    a + (i64::from(b - a) * ms.min(len) as i64 / len as i64) as i32
}

/// The ball rolls the full stage at the ball speed: (x, rolling right).
fn ball_at(ms: u64, seed: u64, w: usize, t: &Tempo) -> (i32, bool) {
    let (x, right, _) = bounce(ms, t.scene.ball, seed % 89, w.saturating_sub(BALL_W as usize), 0, t);
    (x, right)
}

/// Px the ball is off the ground: a 4-px hop every other gait period (every
/// other second at 1 fps).
fn ball_hop(ms: u64, t: &Tempo) -> i32 {
    hop(ms, 2 * t.gait_ms, 4)
}

/// She starts a dozen px behind the ball facing the way it rolls or, with no
/// room behind it, a dozen px ahead of it facing it.
fn ball_cat(to: u64, seed: u64, w: usize, h: PxHeight, t: &Tempo) -> Cat {
    let (bx, br) = ball_at(0, seed, w, t);
    let behind = if br { bx - 12 - cat_w(h) } else { bx + BALL_W + 12 };
    let start = if (0..=cat_span(w, h) as i32).contains(&behind) {
        Cat::new(behind, br, 0)
    } else {
        let x = if br { bx + BALL_W + 12 } else { bx - 12 - cat_w(h) };
        Cat::new(x.clamp(0, cat_span(w, h) as i32), !br, 0)
    };
    simulate(start, to, w, h, t, |c, e| {
        let (bx, br) = ball_at(e * t.sim_step_ms, seed, w, t);
        let d = dir(c.right);
        let gap = if c.right { bx - (c.x + cat_w(h)) } else { c.x - (bx + BALL_W) };
        let behind = (bx + BALL_W / 2 - (c.x + cat_w(h) / 2)) * d < 0;
        match c.beat {
            Beat::Crouch => Cat { x: c.x + LEAP * d, beat: Beat::Leap, ..c },
            Beat::Leap => Cat { x: c.x + 2 * d, beat: Beat::Land, ..c },
            _ if behind => Cat { right: !c.right, beat: Beat::Turn, ..c },
            // rolling back at her face: get low
            _ if br != c.right && gap <= 10 => c.begin(Beat::Crouch, CROUCH_HOLD_MS, t),
            _ => {
                let s = (gap - 1).clamp(0, dash(t));
                Cat { x: c.x + s * d, beat: if s > 0 { Beat::Chase } else { Beat::Still }, ..c }
            }
        }
    })
}

/// "after it!": she chases the ball; when it bounces off a wall and rolls back
/// at her she crouches and pounces, it slips past, she turns and chases again.
fn ball(e: u64, seed: u64, w: usize, h: PxHeight, t: &Tempo) -> SceneFrame {
    let c = chase_cat(e, t, |n| ball_cat(n, seed, w, h, t));
    let pose = match c.beat {
        Beat::Chase | Beat::Dash => run_gait(e, t),
        Beat::Crouch => Pose::Crouch,
        Beat::Leap | Beat::Land => Pose::Pounce,
        Beat::Turn => Pose::Turn,
        Beat::Still | Beat::Look | Beat::Watch | Beat::Swat => Pose::Sit,
    };
    let mut f = SceneFrame::cat(pose, c.x, c.right);
    if c.beat == Beat::Leap {
        f = f.airborne(h);
        if t.smooth {
            f.lift = hop(e % t.sim_step_ms, t.sim_step_ms, 2);
        }
    }
    let (bx, _) = ball_at(e, seed, w, t);
    f.put((w, h), Prop::Ball, 0, bx, ph(h) - BALL_W - ball_hop(e, t), false);
    f
}

/// "gotcha!!": crouch with the ball rolling in, leap over it, land with it
/// trapped under the front paws. In the band the ball rolls in from where
/// the chase left it, settling on the ground, and on under her as she leaps.
fn ball_finale(elapsed: u64, duration: u64, seed: u64, w: usize, h: PxHeight, t: &Tempo) -> SceneFrame {
    let (start, ms, c) = finale_start(elapsed, duration, w, h, t, |to| ball_cat(to, seed, w, h, t));
    let mut f = pounce(ms, c, w, h, t);
    let trapped = |x: i32| if c.right { x + cat_w(h) - 4 } else { x + 1 };
    let ground = ph(h) - BALL_W;
    let (bx, by) = if t.smooth {
        let (bx0, _) = ball_at(start, seed, w, t);
        let close = ahead(c.x, c.right, 2, BALL_W, h);
        let landed = trapped(pounce(CROUCH_MS + LEAP_MS, c, w, h, t).x);
        match ms.checked_sub(CROUCH_MS) {
            None => (toward(bx0, close, ms, CROUCH_MS), ground - toward(ball_hop(start, t), 0, ms, CROUCH_MS / 3)),
            Some(p) => (toward(close, landed, p, LEAP_MS), ground),
        }
    } else {
        let bx = match ms / 1000 {
            0 => ahead(c.x, c.right, 6, BALL_W, h),
            1 => ahead(c.x, c.right, 2, BALL_W, h),
            _ => trapped(f.x),
        };
        (bx, ground)
    };
    f.put((w, h), Prop::Ball, 0, bx, by, true);
    f
}

/// The butterfly drifts the full stage at the butterfly speed, fluttering
/// between px rows 0, 2 and 4 (the ASCII rows 0..2); in the band it bobs
/// through them a px at a time.
fn butterfly_at(ms: u64, seed: u64, w: usize, t: &Tempo) -> (i32, i32) {
    let (x, _, _) = bounce(ms, t.scene.butterfly, seed % 83, w.saturating_sub(BFLY_W as usize), 0, t);
    if t.smooth {
        return (x, tri(ms / t.gait_ms % 8 + seed % 8, 4) as i32);
    }
    // (e % 3 + seed % 3) never overflows, unlike e + seed near u64::MAX.
    (x, 2 * ((ms / t.gait_ms % 3 + seed % 3) % 3) as i32)
}

/// Where she wants the butterfly's middle: just past her nose. (An adult's
/// head fills the top rows, so "over her head" would cover her face.)
fn reach(c: &Cat, h: PxHeight) -> i32 {
    if c.right { c.x + cat_w(h) + 2 } else { c.x - 3 }
}

/// She stalks the butterfly; when it hovers at her nose she looks up, then swats.
fn butterfly_cat(to: u64, seed: u64, w: usize, h: PxHeight, t: &Tempo) -> Cat {
    let (bx, _) = butterfly_at(0, seed, w, t);
    let x = bx + BFLY_W / 2 - (cat_w(h) + 2) - 12;
    simulate(Cat::new(x.clamp(0, cat_span(w, h) as i32), true, steps(2000, t)), to, w, h, t, |c, e| {
        let (bx, _) = butterfly_at(e * t.sim_step_ms, seed, w, t);
        let d = dir(c.right);
        let bc = bx + BFLY_W / 2;
        let dx = (bc - reach(&c, h)) * d;
        let cool = c.cool.saturating_sub(1);
        match c.beat {
            Beat::Look => Cat { beat: Beat::Swat, cool, ..c },
            Beat::Swat => Cat { beat: Beat::Still, cool: steps(4000, t), ..c },
            // past her middle: turn round (over her body she just watches)
            _ if (bc - (c.x + cat_w(h) / 2)) * d < 0 => Cat { right: !c.right, beat: Beat::Turn, cool, ..c },
            _ if dx.abs() <= 2 && c.cool == 0 => Cat { cool, ..c }.begin(Beat::Look, LOOK_HOLD_MS, t),
            _ => {
                let s = dx.clamp(0, dash(t));
                let beat = match s {
                    0 => Beat::Watch,
                    s if s <= stroll(t) => Beat::Chase,
                    _ => Beat::Dash,
                };
                Cat { x: c.x + s * d, beat, cool, ..c }
            }
        }
    })
}

/// "patience... patience...": stalking walk (a trot to catch up), a look up
/// and a two-paw swat when it hovers at her nose; it flits up out of reach.
fn butterfly(e: u64, seed: u64, w: usize, h: PxHeight, t: &Tempo) -> SceneFrame {
    let c = chase_cat(e, t, |n| butterfly_cat(n, seed, w, h, t));
    let pose = match c.beat {
        Beat::Dash => run_gait(e, t),
        Beat::Chase => gait(e, Pose::Walk1, Pose::Walk2, t),
        Beat::Look | Beat::Watch => Pose::Look,
        Beat::Swat => Pose::Cheer,
        Beat::Still => Pose::Sit,
        Beat::Turn => Pose::Turn,
        Beat::Crouch | Beat::Leap | Beat::Land => Pose::Crouch,
    };
    let mut f = SceneFrame::cat(pose, c.x, c.right);
    let (bx, by) = butterfly_at(e, seed, w, t);
    let by = if c.beat == Beat::Swat { 0 } else { by };
    // behind her: flying back over her it passes behind her head
    f.put((w, h), Prop::Butterfly, frame(e, t), bx, by, false);
    f
}

/// "pounce!! ...it got away": crouch under it, leap, land; it escapes up and
/// away and she sits looking after it. In the band it flutters down to her
/// nose from where it was, and away from the leap.
fn butterfly_finale(elapsed: u64, duration: u64, seed: u64, w: usize, h: PxHeight, t: &Tempo) -> SceneFrame {
    let (start, ms, c) = finale_start(elapsed, duration, w, h, t, |to| butterfly_cat(to, seed, w, h, t));
    let mut f = pounce(ms.min(2999), c, w, h, t);
    if ms >= 3000 {
        f = SceneFrame::cat(Pose::Look, f.x, f.right);
    }
    let (bx, by) = if t.smooth {
        let (bx0, by0) = butterfly_at(start, seed, w, t);
        let hover = ahead(c.x, c.right, 0, BFLY_W, h);
        match ms.checked_sub(CROUCH_MS) {
            None => (toward(bx0, hover, ms, CROUCH_MS * 2 / 3), toward(by0, 4, ms, CROUCH_MS * 2 / 3)),
            Some(p) => (toward(hover, ahead(c.x, c.right, 17, BFLY_W, h), p, 3000 - CROUCH_MS), toward(4, 0, p, LEAP_MS)),
        }
    } else {
        match ms / 1000 {
            0 => (ahead(c.x, c.right, 0, BFLY_W, h), 4),
            k => (ahead(c.x, c.right, 2 + 5 * k as i32, BFLY_W, h), 0),
        }
    };
    f.put((w, h), Prop::Butterfly, frame(elapsed, t), bx, by, true);
    f
}

/// A scene second painted: ground dots, props behind her, the cat (lifted
/// when airborne), props in front.
pub(crate) fn draw(f: &SceneFrame, width: usize, look: &CatLook, h: PxHeight) -> PxCanvas {
    let tail = if matches!(f.pose, Pose::Sit | Pose::Look | Pose::Cheer) { Tail::Up } else { Tail::Curl };
    let cat = paint::paint(f.pose, look, f.right, tail, false, h);
    let mut c = PxCanvas::new(width, h);
    for &(x, y, col) in &f.dots {
        c.set(x, y, col);
    }
    let draw = |c: &mut PxCanvas, front: bool| {
        for p in f.props.iter().filter(|p| p.front == front) {
            c.blit(&art::prop(p.prop, p.frame), p.x, p.y);
        }
    };
    draw(&mut c, false);
    c.blit(&cat, f.x, (ph(h) - cat.h as i32 - f.lift).max(0));
    draw(&mut c, true);
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    const H12: PxHeight = PxHeight::H12;
    const H: i32 = 12;
    const CAT_W: i32 = 16;
    use crate::playground::ACTIVE_SCENES;
    use crate::px::{encode, PixelMode};
    use PlaygroundScene::*;

    fn scene_pose(kind: PlaygroundScene, elapsed: u64, duration: u64, seed: u64, width: usize) -> Pose {
        scene_frame(kind, elapsed, duration, seed, width, H12).pose
    }

    fn scene_canvas(kind: PlaygroundScene, elapsed: u64, duration: u64, seed: u64, width: usize, look: &CatLook) -> PxCanvas {
        draw(&scene_frame(kind, elapsed, duration, seed, width, H12), width, look, H12)
    }

    const CASES: [(u64, u64); 3] = [(20, 0), (30, 12_345), (40, u64::MAX)];

    fn look(stage: &str) -> CatLook<'_> {
        CatLook { breed: "Ginger", rarity: "common", stage, tick: 0 }
    }

    fn size(p: &Placed) -> (i32, i32) {
        let s = art::prop(p.prop, p.frame);
        (s.w as i32, s.h as i32)
    }

    fn props_of(f: &SceneFrame, p: Prop) -> Vec<Placed> {
        f.props.iter().copied().filter(|q| q.prop == p).collect()
    }

    /// Every scene, width, second and both modes: 6 rows of the stage width,
    /// each closed by a reset; the cat and every prop stay on the canvas.
    #[test]
    fn alignment_sweep_every_scene_width_and_second() {
        for kind in ACTIVE_SCENES {
            for w in [24usize, 60, 120] {
                for (d, seed) in CASES {
                    for e in (0..=45u64).chain([u64::MAX / 2, u64::MAX]) {
                        let f = scene_frame(kind, e, d, seed, w, H12);
                        assert!((0..=(w as i32 - CAT_W)).contains(&f.x), "{kind:?} {w} {e}: cat x {}", f.x);
                        for p in &f.props {
                            let (pw, ph) = size(p);
                            assert!(p.x >= 0 && p.x + pw <= w as i32 && p.y >= 0 && p.y + ph <= H, "{kind:?} {w} {e}: {p:?} clips");
                        }
                        for stage in ["kitten", "adult"] {
                            let c = scene_canvas(kind, e, d, seed, w, &look(stage));
                            for m in [PixelMode::Full, PixelMode::FgOnly] {
                                let rows = encode(&c, m);
                                assert_eq!(rows.len(), 6);
                                for r in rows {
                                    assert_eq!(crate::span::strip_ansi(&r).chars().count(), w, "{kind:?} {w} {e}");
                                    assert!(r.ends_with("\x1b[0m"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// No teleports: the cat moves at most its scene speed per second
    /// (zoomies 8 px, everything else 4 px including the pounce leap).
    #[test]
    fn cat_moves_continuously() {
        for kind in ACTIVE_SCENES {
            let cap = if kind == Zoomies { 8 } else { 4 };
            for w in [24usize, 60, 120] {
                for (d, seed) in CASES {
                    for e in 1..d {
                        let (a, b) = (scene_frame(kind, e - 1, d, seed, w, H12), scene_frame(kind, e, d, seed, w, H12));
                        assert!((a.x - b.x).abs() <= cap, "{kind:?} {w} {seed}: {} -> {} at {e}", a.x, b.x);
                    }
                }
            }
        }
    }

    #[test]
    fn run_runs_and_zoomies_run_faster() {
        let max_step = |kind| (1..12u64).map(|e| (scene_frame(kind, e, 30, 7, 80, H12).x - scene_frame(kind, e - 1, 30, 7, 80, H12).x).abs()).max().unwrap();
        let moving = (1..20u64).filter(|&e| scene_frame(Run, e, 30, 7, 80, H12).x != scene_frame(Run, e - 1, 30, 7, 80, H12).x).count();
        assert!(moving >= 15, "run mostly stands still: {moving}/19");
        assert!((0..20u64).all(|e| matches!(scene_pose(Run, e, 30, 7, 80), Pose::Run1 | Pose::Run2 | Pose::Turn)));
        assert!(max_step(Zoomies) > max_step(Run));
        assert!((0..20u64).any(|e| !scene_frame(Zoomies, e, 30, 7, 80, H12).dots.is_empty()), "no dust");
    }

    #[test]
    fn finale_has_anticipation_then_pounce() {
        let poses = |e| scene_pose(BallChase, e, 30, 7, 80);
        assert_eq!(poses(26), Pose::Crouch);
        assert_eq!(poses(27), Pose::Pounce);
        for kind in [BallChase, ButterflyChase] {
            for w in [24usize, 60, 120] {
                for (d, seed) in CASES {
                    let f = |e| scene_frame(kind, e, d, seed, w, H12);
                    assert_eq!(f(d - 4).pose, Pose::Crouch, "{kind:?} {w} {d}");
                    assert_eq!((f(d - 3).pose, f(d - 3).lift > 0), (Pose::Pounce, true), "{kind:?} {w} {d}: airborne");
                    assert_eq!((f(d - 2).pose, f(d - 2).lift), (Pose::Pounce, 0), "{kind:?} {w} {d}: landed");
                }
            }
        }
    }

    /// The ball is on stage the whole scene (red, white glint), and in the
    /// finale it ends up trapped under the front paws.
    #[test]
    fn ball_rolls_ahead_and_is_caught_in_the_finale() {
        for w in [24usize, 60, 120] {
            for (d, seed) in CASES {
                for e in 0..d {
                    assert_eq!(props_of(&scene_frame(BallChase, e, d, seed, w, H12), Prop::Ball).len(), 1, "{w} {e}");
                }
                for e in d - 2..d {
                    let f = scene_frame(BallChase, e, d, seed, w, H12);
                    let b = props_of(&f, Prop::Ball)[0];
                    let paws = if f.right { f.x + CAT_W - 4 } else { f.x + 1 };
                    assert!((b.x - paws).abs() <= 1 && b.y + 3 == H, "{w} {seed} {e}: ball {b:?} not under the paws of {f:?}");
                }
            }
        }
        let c = scene_canvas(BallChase, 9, 30, 7, 60, &look("adult"));
        assert!((0..60).any(|x| (0..12).any(|y| c.get(x, y) == Some(crate::colors::RED))), "ball is red");
    }

    /// Mid-scene the ball rolls back at her: she crouches, pounces and it gets
    /// past her (near miss), so the finale is the only catch.
    #[test]
    fn ball_near_miss_mid_scene() {
        let poses: Vec<Pose> = (0..36u64).map(|e| scene_pose(BallChase, e, 40, 7, 60)).collect();
        let at = poses.iter().position(|&p| p == Pose::Crouch).expect("a mid-scene crouch at width 60");
        assert!(at < 36, "{poses:?}");
        assert_eq!(&poses[at..at + 3], &[Pose::Crouch, Pose::Pounce, Pose::Pounce]);
        assert!(poses[at + 3..].contains(&Pose::Turn), "she turns after the ball: {poses:?}");
    }

    /// The butterfly flutters in the upper air; she stalks it, looks up and swats.
    #[test]
    fn butterfly_flutters_above_and_gets_swatted() {
        for w in [24usize, 60, 120] {
            for (d, seed) in CASES {
                for e in 0..d {
                    let b = props_of(&scene_frame(ButterflyChase, e, d, seed, w, H12), Prop::Butterfly);
                    assert_eq!(b.len(), 1);
                    assert!(b[0].y <= 4, "{w} {e}: butterfly low at {}", b[0].y);
                }
            }
        }
        for w in [60usize, 120] {
            let poses: Vec<Pose> = (0..36u64).map(|e| scene_pose(ButterflyChase, e, 40, 7, w)).collect();
            assert!(poses.windows(2).any(|p| p == [Pose::Look, Pose::Cheer]), "{w}: swat after the look: {poses:?}");
            // never a dead stretch: she follows it with her head, a turn or a swat
            let idle = poses.windows(3).filter(|p| p.iter().all(|&q| q == Pose::Sit)).count();
            assert_eq!(idle, 0, "{w}: sits through the pass: {poses:?}");
        }
    }

    #[test]
    fn yarn_rolls_on_the_ground_with_a_trail() {
        for w in [24usize, 60, 120] {
            for e in 0..30u64 {
                let f = scene_frame(YarnPush, e, 30, 7, w, H12);
                let y = props_of(&f, Prop::Yarn);
                assert_eq!(y.len(), 1);
                assert_eq!(y[0].y + 4, H, "on the ground");
                assert!(f.dots.iter().all(|&(_, dy, _)| dy == H - 1), "trail on row 11");
                if f.pose != Pose::Turn {
                    let ahead = if f.right { y[0].x >= f.x + CAT_W } else { y[0].x + 4 <= f.x };
                    assert!(ahead, "{w} {e}: yarn not ahead of the paws");
                    assert_eq!(f.dots.len(), 3);
                }
            }
        }
    }

    /// Roll: belly-up rolls, a dizzy sit at each end of the stage.
    #[test]
    fn roll_rolls_and_sits_at_the_ends() {
        let poses: Vec<Pose> = (0..40u64).map(|e| scene_pose(Roll, e, 40, 7, 40)).collect();
        assert!(poses.iter().filter(|p| matches!(p, Pose::Roll1 | Pose::Roll2)).count() > 30);
        assert!(poses.contains(&Pose::Sit));
    }

    // The band: scenes sampled at 30 fps under `Tempo::BAND`.

    const BAND_HEIGHTS: [PxHeight; 2] = [PxHeight::H10, PxHeight::H12];

    /// The 30-fps frame times (`i * 1000 / 30` ms) in `0..secs` seconds.
    fn frame_times(secs: u64) -> impl Iterator<Item = u64> {
        (0..secs * 30).map(|i| i * 1000 / 30)
    }

    fn band(kind: PlaygroundScene, ms: u64, d: u64, seed: u64, w: usize, h: PxHeight) -> SceneFrame {
        scene_frame_ms(kind, ms, d, seed, w, h, &Tempo::BAND)
    }

    #[test]
    fn band_scenes_use_full_width() {
        for kind in ACTIVE_SCENES {
            for h in BAND_HEIGHTS {
                for (d, seed) in CASES {
                    let far = (0..d * 10).map(|i| band(kind, i * 100, d, seed, 120, h).x).max().unwrap();
                    let span = cat_span(120, h) as i32;
                    assert!(far * 10 >= span * 8, "{kind:?} {h:?} {seed}: reaches {far} of {span}");
                }
            }
        }
    }

    /// Every 30-fps frame at both band heights: the cat and every prop on
    /// the canvas, and the painted canvas the band's size.
    #[test]
    fn band_scene_props_never_clip() {
        for kind in ACTIVE_SCENES {
            for h in BAND_HEIGHTS {
                for w in [24usize, 60, 120] {
                    let (d, seed) = (30, 12_345);
                    for ms in frame_times(d) {
                        let f = band(kind, ms, d, seed, w, h);
                        assert!((0..=cat_span(w, h) as i32).contains(&f.x), "{kind:?} {h:?} {w} {ms}: cat x {}", f.x);
                        assert!((0..=2).contains(&f.lift), "{kind:?} {h:?} {ms}: lift {}", f.lift);
                        for p in &f.props {
                            let (pw, phh) = size(p);
                            assert!(p.x >= 0 && p.x + pw <= w as i32 && p.y >= 0 && p.y + phh <= ph(h), "{kind:?} {h:?} {w} {ms}: {p:?} clips");
                        }
                        let c = draw(&f, w, &look("adult"), h);
                        assert_eq!((c.width(), encode(&c, PixelMode::Full).len()), (w, h.rows()));
                    }
                    // a stale scene's huge elapsed still lands on the stage
                    for ms in [u64::MAX / 2, u64::MAX] {
                        let f = band(kind, ms, d, seed, w, h);
                        assert!((0..=cat_span(w, h) as i32).contains(&f.x), "{kind:?} {h:?} {w} {ms}");
                    }
                }
            }
        }
    }

    /// The finale (the prop in front of her) runs exactly over the caption's
    /// last `FINALE_SECS`: crouch first, then the pounce, landed by the end.
    #[test]
    fn band_finale_matches_caption_window() {
        for kind in [BallChase, ButterflyChase] {
            for h in BAND_HEIGHTS {
                for w in [60usize, 120] {
                    for (d, seed) in CASES {
                        let start = (d - FINALE_SECS) * 1000;
                        for ms in frame_times(d) {
                            let f = band(kind, ms, d, seed, w, h);
                            let finale = f.props.iter().any(|p| p.front);
                            assert_eq!(finale, ms >= start, "{kind:?} {h:?} {w} {seed} {ms}");
                            if ms >= start && ms < start + CROUCH_MS {
                                assert_eq!(f.pose, Pose::Crouch, "{kind:?} {h:?} {w} {seed} {ms}");
                            } else if ms >= start + CROUCH_MS + LEAP_MS {
                                assert!(matches!(f.pose, Pose::Pounce | Pose::Look) && f.lift == 0, "{kind:?} {h:?} {w} {seed} {ms}: {f:?}");
                            }
                        }
                        let airborne = frame_times(d).filter(|&ms| ms >= start).any(|ms| band(kind, ms, d, seed, w, h).lift > 0);
                        assert!(airborne, "{kind:?} {h:?} {w} {seed}: no leap");
                    }
                }
            }
        }
    }

    /// No pops at 30 fps: the cat moves at most 2 px a frame (zoomies are
    /// 45 px/s), mid-scene leaps and the finale included, and so do the ball
    /// and the butterfly outside the finale's run-in. (The yarn tucks under her
    /// at a wall, out of sight.)
    #[test]
    fn band_scene_motion_is_continuous() {
        for kind in ACTIVE_SCENES {
            for h in BAND_HEIGHTS {
                for (d, seed) in CASES {
                    let frames: Vec<SceneFrame> = frame_times(d).map(|ms| band(kind, ms, d, seed, 120, h)).collect();
                    for (i, p) in frames.windows(2).enumerate() {
                        assert!((p[1].x - p[0].x).abs() <= 2, "{kind:?} {h:?} {seed} frame {i}: {} -> {}", p[0].x, p[1].x);
                        let prop_x = |f: &SceneFrame| f.props.iter().find(|q| !q.front && q.prop != Prop::Yarn).map(|q| q.x);
                        if let (Some(a), Some(b)) = (prop_x(&p[0]), prop_x(&p[1])) {
                            assert!((b - a).abs() <= 2, "{kind:?} {h:?} {seed} frame {i}: prop {a} -> {b}");
                        }
                    }
                }
            }
        }
    }

    /// Her speed and gait match life's: laps run at `run_px_s` on the 100 ms
    /// run gait, the yarn is pushed at `walk_px_s` on the 200 ms walk gait.
    #[test]
    fn band_scene_gaits_match_life() {
        let t = &Tempo::BAND;
        let flips = |kind, a: Pose, b: Pose| {
            let poses: Vec<Pose> = frame_times(20).map(|ms| band(kind, ms, 30, 7, 120, PxHeight::H10).pose).collect();
            let at: Vec<usize> = poses.windows(2).enumerate().filter(|(_, p)| (p[0], p[1]) == (a, b)).map(|(i, _)| i).collect();
            at.windows(2).map(|w| (w[1] - w[0]) as u64 * 1000 / 30).min().unwrap()
        };
        // a full gait cycle is two frames of the gait
        assert!((2 * t.run_gait_ms - 34..=2 * t.run_gait_ms + 34).contains(&flips(Run, Pose::Run1, Pose::Run2)));
        assert!((2 * t.gait_ms - 34..=2 * t.gait_ms + 34).contains(&flips(YarnPush, Pose::Walk1, Pose::Walk2)));
        // px per second mid-leg (the stage is 106 px at H10)
        let speed = |kind| {
            (0..20u64)
                .map(|s| (band(kind, s * 1000 + 1000, 30, 7, 120, PxHeight::H10).x - band(kind, s * 1000, 30, 7, 120, PxHeight::H10).x).abs())
                .max()
                .unwrap()
        };
        assert_eq!(speed(Run), t.run_px_s);
        assert_eq!(speed(YarnPush), t.walk_px_s);
        assert!(speed(Zoomies) > t.run_px_s);
    }

    /// At each wall she stops to turn (roll: a longer dizzy sit) instead of
    /// skating round.
    #[test]
    fn band_turns_stand_still() {
        for (kind, pose, ms) in [(Run, Pose::Turn, Tempo::BAND.turn_ms), (Roll, Pose::Sit, DIZZY * Tempo::BAND.turn_ms)] {
            let frames: Vec<SceneFrame> = frame_times(30).map(|t| band(kind, t, 30, 7, 120, PxHeight::H12)).collect();
            let turns: Vec<&SceneFrame> = frames.iter().filter(|f| f.pose == pose).collect();
            assert!(turns.len() as u64 >= ms * 2 / 34, "{kind:?}: {} turn frames", turns.len());
            assert!(turns.iter().all(|f| f.x == 0 || f.x == cat_span(120, PxHeight::H12) as i32), "{kind:?}: turns off the wall");
        }
    }

    /// The mid-scene near miss holds the crouch, then glides through the leap.
    #[test]
    fn band_near_miss_crouches_then_glides() {
        let frames: Vec<SceneFrame> = frame_times(36).map(|ms| band(BallChase, ms, 40, 7, 60, PxHeight::H12)).collect();
        let at = frames.iter().position(|f| f.pose == Pose::Crouch).expect("a mid-scene crouch");
        let crouch = frames[at..].iter().take_while(|f| f.pose == Pose::Crouch).count() as u64;
        assert!((CROUCH_HOLD_MS * 30 / 1000 - 1..=CROUCH_HOLD_MS * 30 / 1000 + 1).contains(&crouch), "crouch {crouch} frames");
        let leap: Vec<i32> = frames[at..].iter().skip_while(|f| f.pose == Pose::Crouch).take_while(|f| f.lift > 0).map(|f| f.x).collect();
        assert!(leap.len() >= 3 && leap.windows(2).all(|p| (p[1] - p[0]).abs() <= 1), "leap {leap:?}");
    }
}
