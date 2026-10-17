//! Pixel playground scenes: the six `playground.rs` scenes drawn with the
//! pixel poses and props. Scheduling, timing and speech stay in
//! `playground.rs` / `kitten_view`; this only decides what each second looks
//! like, as a pure function of `(kind, elapsed, duration, seed, width)`.
//!
//! Run, zoomies, roll and yarn mirror the ASCII paths (same speeds and
//! phases; a `Turn` or a dizzy `Sit` marks each wall bounce). The two chase
//! scenes re-simulate the cat second by second from `elapsed = 0` (at most
//! `SIM_CAP` steps), so she can react to the prop: crouch when the ball rolls
//! back at her, pounce, miss, turn and chase again; stalk the butterfly, look
//! up and swat. The last `FINALE_SECS` seconds are the payoff, in step with the
//! finale speech: crouch, airborne pounce, land (ball trapped / butterfly gone).

use super::art::{self, Pose, Prop, Tail};
use super::paint::{self, CatLook, CAT_MAX_W};
use super::{PxCanvas, PX_H};
use crate::colors::Rgb;
use crate::pixel;
use crate::playground::{
    tri, tri_rising, BALL_SPEED, BUTTERFLY_SPEED, FINALE_SECS, ROLL_SPEED, RUN_SPEED, YARN_SPEED, ZOOMIES_SPEED,
};
use crate::state::PlaygroundScene;

const CAT_W: i32 = CAT_MAX_W as i32;
const H: i32 = PX_H as i32;
/// Fastest the cat moves in a chase (px per 1 fps tick), leap included.
const DASH: i32 = 4;
/// Upper bound on simulated seconds (scenes last at most 40 s; a stale state
/// with a huge `elapsed` is in its finale anyway).
const SIM_CAP: u64 = 64;
/// Px either side of the yarn walk, so the yarn fits ahead in both facings.
const YARN_MARGIN: i32 = 4;
const BALL_W: i32 = 3;
const BFLY_W: i32 = 5;
/// Px of stage the finale needs ahead of the crouching cat: the 6 px leap
/// plus the 3 px ball rolling in (the butterfly needs less).
const FINALE_ROOM: i32 = DASH + 2 + BALL_W;

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
    fn trail(mut self, col: Rgb) -> Self {
        let (x, right) = (self.x, self.right);
        self.dots = [2, 4, 6].iter().map(|&d| (if right { x - d } else { x + CAT_W - 1 + d }, H - 1, col)).collect();
        self
    }

    /// Mid-leap: raised off the ground over her own shadow. (An adult pounce
    /// has 1 px of headroom, so the shadow is what sells the jump.)
    fn airborne(mut self) -> Self {
        self.lift = 2;
        self.dots.extend((4..CAT_W - 4).map(|dx| (self.x + dx, H - 1, paint::OUTLINE)));
        self
    }

    /// Add a prop, clamped so it never clips the canvas.
    fn put(&mut self, w: usize, prop: Prop, frame: u8, x: i32, y: i32, front: bool) {
        let s = art::prop(prop, frame);
        let (x, y) = (x.clamp(0, (w as i32 - s.w as i32).max(0)), y.clamp(0, H - s.h as i32));
        self.props.push(Placed { prop, frame, x, y, front });
    }
}

fn gait(e: u64, a: Pose, b: Pose) -> Pose {
    if e.is_multiple_of(2) { a } else { b }
}

fn dir(right: bool) -> i32 {
    if right { 1 } else { -1 }
}

/// Room for the cat's left px.
fn cat_span(w: usize) -> usize {
    w.saturating_sub(CAT_MAX_W)
}

/// Left px of a `pw`-wide prop `gap` px ahead of the cat's nose.
fn ahead(x: i32, right: bool, gap: i32, pw: i32) -> i32 {
    if right { x + CAT_W + gap } else { x - gap - pw }
}

/// Back-and-forth path at `speed` px/s: (x, facing right, just bounced).
fn bounce(e: u64, speed: u64, phase: u64, span: usize) -> (i32, bool, bool) {
    let t = |e: u64| e.wrapping_mul(speed).wrapping_add(phase);
    let right = tri_rising(t(e), span);
    let turned = e > 0 && right != tri_rising(t(e - 1), span);
    (tri(t(e), span) as i32, right, turned)
}

pub(crate) fn scene_frame(kind: PlaygroundScene, elapsed: u64, duration: u64, seed: u64, width: usize) -> SceneFrame {
    let finale = elapsed.saturating_add(FINALE_SECS) >= duration;
    match kind {
        PlaygroundScene::Run => run(elapsed, seed, width),
        PlaygroundScene::Zoomies => zoomies(elapsed, seed, width),
        PlaygroundScene::Roll => roll(elapsed, seed, width),
        PlaygroundScene::YarnPush => yarn(elapsed, seed, width),
        PlaygroundScene::BallChase if finale => ball_finale(elapsed, duration, seed, width),
        PlaygroundScene::BallChase => ball(elapsed, seed, width),
        PlaygroundScene::ButterflyChase if finale => butterfly_finale(elapsed, duration, seed, width),
        PlaygroundScene::ButterflyChase => butterfly(elapsed, seed, width),
        PlaygroundScene::None => SceneFrame::cat(Pose::Sit, cat_span(width) as i32 / 2, true),
    }
}

/// Laps at a trot; a `Turn` at each wall.
fn run(e: u64, seed: u64, w: usize) -> SceneFrame {
    let (x, right, turned) = bounce(e, RUN_SPEED, seed % 97, cat_span(w));
    SceneFrame::cat(if turned { Pose::Turn } else { gait(e, Pose::Run1, Pose::Run2) }, x, right)
}

/// Flat-out laps with skid dust behind her; no time to turn around.
fn zoomies(e: u64, seed: u64, w: usize) -> SceneFrame {
    let (x, right, _) = bounce(e, ZOOMIES_SPEED, seed % 97, cat_span(w));
    SceneFrame::cat(gait(e, Pose::Run1, Pose::Run2), x, right).trail(pixel::mix(paint::OUTLINE, pixel::WHITE, 0.5))
}

/// Belly-up rolls across the stage and back, a dizzy sit at each end.
fn roll(e: u64, seed: u64, w: usize) -> SceneFrame {
    let (x, right, turned) = bounce(e, ROLL_SPEED, seed % 13, cat_span(w));
    SceneFrame::cat(if turned { Pose::Sit } else { gait(e, Pose::Roll1, Pose::Roll2) }, x, right)
}

/// Pushes the yarn along the ground, string unravelling behind; at a wall she
/// turns with the yarn tucked under her and pushes it back.
fn yarn(e: u64, seed: u64, w: usize) -> SceneFrame {
    let span = w.saturating_sub(CAT_MAX_W + 2 * YARN_MARGIN as usize);
    let (x, right, turned) = bounce(e, YARN_SPEED, seed % 79, span);
    let x = x + YARN_MARGIN;
    if turned {
        let mut f = SceneFrame::cat(Pose::Turn, x, right);
        f.put(w, Prop::Yarn, 0, x + CAT_W / 2 - 2, H - 4, false);
        return f;
    }
    let mut f = SceneFrame::cat(gait(e, Pose::Walk1, Pose::Walk2), x, right).trail(pixel::NOSE);
    f.put(w, Prop::Yarn, 0, ahead(x, right, 0, 4), H - 4, false);
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
    Swat,
}

/// The simulated cat in a chase scene.
#[derive(Clone, Copy, Debug)]
struct Cat {
    x: i32,
    right: bool,
    beat: Beat,
    /// Seconds until the next swat may start.
    cool: u8,
}

/// Run `step` for seconds `1..=to` from `start`, keeping x on the stage.
fn simulate(start: Cat, to: u64, w: usize, step: impl Fn(Cat, u64) -> Cat) -> Cat {
    (1..=to.min(SIM_CAP)).fold(start, |c, e| {
        let n = step(c, e);
        Cat { x: n.x.clamp(0, cat_span(w) as i32), ..n }
    })
}

/// The seconds the finale has run (0 = the crouch), and the cat it starts
/// from: the chase as it stood just before, turned toward the open stage when
/// she has no room to leap.
fn finale_start(elapsed: u64, duration: u64, w: usize, sim: impl Fn(u64) -> Cat) -> (u64, Cat) {
    let start = duration.saturating_sub(FINALE_SECS);
    let mut c = sim(start.saturating_sub(1));
    let room = if c.right { cat_span(w) as i32 - c.x } else { c.x };
    if room < FINALE_ROOM {
        c.right = !c.right;
    }
    (elapsed.saturating_sub(start).min(FINALE_SECS - 1), c)
}

/// Crouch, leap (airborne, 4 px), land (2 px more): the finale cat.
fn pounce(k: u64, c: Cat, w: usize) -> SceneFrame {
    let (step, pose) = match k {
        0 => (0, Pose::Crouch),
        1 => (DASH, Pose::Pounce),
        _ => (DASH + 2, Pose::Pounce),
    };
    let x = (c.x + step * dir(c.right)).clamp(0, cat_span(w) as i32);
    let f = SceneFrame::cat(pose, x, c.right);
    if k == 1 { f.airborne() } else { f }
}

/// The ball rolls the full stage at `BALL_SPEED`, hopping every other second:
/// (x, rolling right).
fn ball_at(e: u64, seed: u64, w: usize) -> (i32, bool) {
    let (x, right, _) = bounce(e, BALL_SPEED, seed % 89, w.saturating_sub(BALL_W as usize));
    (x, right)
}

fn ball_y(e: u64) -> i32 {
    if e.is_multiple_of(2) { H - BALL_W } else { H - BALL_W - 4 }
}

/// She starts a dozen px behind the ball facing the way it rolls or, with no
/// room behind it, a dozen px ahead of it facing it.
fn ball_cat(to: u64, seed: u64, w: usize) -> Cat {
    let (bx, br) = ball_at(0, seed, w);
    let behind = if br { bx - 12 - CAT_W } else { bx + BALL_W + 12 };
    let start = if (0..=cat_span(w) as i32).contains(&behind) {
        Cat { x: behind, right: br, beat: Beat::Chase, cool: 0 }
    } else {
        let x = if br { bx + BALL_W + 12 } else { bx - 12 - CAT_W };
        Cat { x: x.clamp(0, cat_span(w) as i32), right: !br, beat: Beat::Chase, cool: 0 }
    };
    simulate(start, to, w, |c, e| {
        let (bx, br) = ball_at(e, seed, w);
        let d = dir(c.right);
        let gap = if c.right { bx - (c.x + CAT_W) } else { c.x - (bx + BALL_W) };
        let behind = (bx + BALL_W / 2 - (c.x + CAT_W / 2)) * d < 0;
        match c.beat {
            Beat::Crouch => Cat { x: c.x + DASH * d, beat: Beat::Leap, ..c },
            Beat::Leap => Cat { x: c.x + 2 * d, beat: Beat::Land, ..c },
            _ if behind => Cat { right: !c.right, beat: Beat::Turn, ..c },
            // rolling back at her face: get low
            _ if br != c.right && gap <= 10 => Cat { beat: Beat::Crouch, ..c },
            _ => {
                let s = (gap - 1).clamp(0, DASH);
                Cat { x: c.x + s * d, beat: if s > 0 { Beat::Chase } else { Beat::Still }, ..c }
            }
        }
    })
}

/// "after it!": she chases the ball; when it bounces off a wall and rolls back
/// at her she crouches and pounces, it slips past, she turns and chases again.
fn ball(e: u64, seed: u64, w: usize) -> SceneFrame {
    let c = ball_cat(e, seed, w);
    let pose = match c.beat {
        Beat::Chase | Beat::Dash => gait(e, Pose::Run1, Pose::Run2),
        Beat::Crouch => Pose::Crouch,
        Beat::Leap | Beat::Land => Pose::Pounce,
        Beat::Turn => Pose::Turn,
        Beat::Still | Beat::Look | Beat::Swat => Pose::Sit,
    };
    let mut f = SceneFrame::cat(pose, c.x, c.right);
    if c.beat == Beat::Leap {
        f = f.airborne();
    }
    let (bx, _) = ball_at(e, seed, w);
    f.put(w, Prop::Ball, 0, bx, ball_y(e), false);
    f
}

/// "gotcha!!": crouch with the ball rolling in, leap over it, land with it
/// trapped under the front paws.
fn ball_finale(elapsed: u64, duration: u64, seed: u64, w: usize) -> SceneFrame {
    let (k, c) = finale_start(elapsed, duration, w, |to| ball_cat(to, seed, w));
    let mut f = pounce(k, c, w);
    let bx = match k {
        0 => ahead(c.x, c.right, 6, BALL_W),
        1 => ahead(c.x, c.right, 2, BALL_W),
        _ if f.right => f.x + CAT_W - 4,
        _ => f.x + 1,
    };
    f.put(w, Prop::Ball, 0, bx, H - BALL_W, true);
    f
}

/// The butterfly drifts the full stage at `BUTTERFLY_SPEED`, fluttering
/// between px rows 0, 2 and 4 (the ASCII rows 0..2).
fn butterfly_at(e: u64, seed: u64, w: usize) -> (i32, i32) {
    let (x, _, _) = bounce(e, BUTTERFLY_SPEED, seed % 83, w.saturating_sub(BFLY_W as usize));
    // (e % 3 + seed % 3) never overflows, unlike e + seed near u64::MAX.
    (x, 2 * ((e % 3 + seed % 3) % 3) as i32)
}

/// Where she wants the butterfly's middle: just past her nose. (An adult's
/// head fills the top rows, so "over her head" would cover her face.)
fn reach(c: &Cat) -> i32 {
    if c.right { c.x + CAT_W + 2 } else { c.x - 3 }
}

/// She stalks the butterfly; when it hovers at her nose she looks up, then swats.
fn butterfly_cat(to: u64, seed: u64, w: usize) -> Cat {
    let (bx, _) = butterfly_at(0, seed, w);
    let x = bx + BFLY_W / 2 - (CAT_W + 2) - 12;
    simulate(Cat { x: x.clamp(0, cat_span(w) as i32), right: true, beat: Beat::Chase, cool: 2 }, to, w, |c, e| {
        let (bx, _) = butterfly_at(e, seed, w);
        let d = dir(c.right);
        let bc = bx + BFLY_W / 2;
        let dx = (bc - reach(&c)) * d;
        let cool = c.cool.saturating_sub(1);
        match c.beat {
            Beat::Look => Cat { beat: Beat::Swat, cool, ..c },
            Beat::Swat => Cat { beat: Beat::Still, cool: 4, ..c },
            // past her middle: turn round (over her body she just watches)
            _ if (bc - (c.x + CAT_W / 2)) * d < 0 => Cat { right: !c.right, beat: Beat::Turn, cool, ..c },
            _ if dx.abs() <= 2 && c.cool == 0 => Cat { beat: Beat::Look, cool, ..c },
            _ => {
                let s = dx.clamp(0, DASH);
                let beat = match s {
                    0 => Beat::Still,
                    1 | 2 => Beat::Chase,
                    _ => Beat::Dash,
                };
                Cat { x: c.x + s * d, beat, cool, ..c }
            }
        }
    })
}

/// "patience... patience...": stalking walk (a trot to catch up), a look up
/// and a two-paw swat when it hovers at her nose; it flits up out of reach.
fn butterfly(e: u64, seed: u64, w: usize) -> SceneFrame {
    let c = butterfly_cat(e, seed, w);
    let pose = match c.beat {
        Beat::Dash => gait(e, Pose::Run1, Pose::Run2),
        Beat::Chase => gait(e, Pose::Walk1, Pose::Walk2),
        Beat::Look => Pose::Look,
        Beat::Swat => Pose::Cheer,
        Beat::Still => Pose::Sit,
        Beat::Turn => Pose::Turn,
        Beat::Crouch | Beat::Leap | Beat::Land => Pose::Crouch,
    };
    let mut f = SceneFrame::cat(pose, c.x, c.right);
    let (bx, by) = butterfly_at(e, seed, w);
    let by = if c.beat == Beat::Swat { 0 } else { by };
    // behind her: flying back over her it passes behind her head
    f.put(w, Prop::Butterfly, (e % 2) as u8, bx, by, false);
    f
}

/// "pounce!! ...it got away": crouch under it, leap, land; it escapes up and
/// away and she sits looking after it.
fn butterfly_finale(elapsed: u64, duration: u64, seed: u64, w: usize) -> SceneFrame {
    let (k, c) = finale_start(elapsed, duration, w, |to| butterfly_cat(to, seed, w));
    let mut f = pounce(k.min(2), c, w);
    if k == 3 {
        f = SceneFrame::cat(Pose::Look, f.x, f.right);
    }
    let (bx, by) = match k {
        0 => (ahead(c.x, c.right, 0, BFLY_W), 4),
        _ => (ahead(c.x, c.right, 2 + 5 * k as i32, BFLY_W), 0),
    };
    f.put(w, Prop::Butterfly, (elapsed % 2) as u8, bx, by, true);
    f
}

/// A scene second painted: ground dots, props behind her, the cat (lifted
/// when airborne), props in front.
pub(crate) fn draw(f: &SceneFrame, width: usize, look: &CatLook) -> PxCanvas {
    let tail = if matches!(f.pose, Pose::Sit | Pose::Look | Pose::Cheer) { Tail::Up } else { Tail::Curl };
    let cat = paint::paint(f.pose, look, f.right, tail, false);
    let mut c = PxCanvas::new(width);
    for &(x, y, col) in &f.dots {
        c.set(x, y, col);
    }
    let draw = |c: &mut PxCanvas, front: bool| {
        for p in f.props.iter().filter(|p| p.front == front) {
            c.blit(&art::prop(p.prop, p.frame), p.x, p.y);
        }
    };
    draw(&mut c, false);
    c.blit(&cat, f.x, (H - cat.h as i32 - f.lift).max(0));
    draw(&mut c, true);
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playground::ACTIVE_SCENES;
    use crate::px::{encode, PixelMode};
    use PlaygroundScene::*;

    fn scene_pose(kind: PlaygroundScene, elapsed: u64, duration: u64, seed: u64, width: usize) -> Pose {
        scene_frame(kind, elapsed, duration, seed, width).pose
    }

    fn scene_canvas(kind: PlaygroundScene, elapsed: u64, duration: u64, seed: u64, width: usize, look: &CatLook) -> PxCanvas {
        draw(&scene_frame(kind, elapsed, duration, seed, width), width, look)
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
                        let f = scene_frame(kind, e, d, seed, w);
                        assert!((0..=(w as i32 - CAT_W)).contains(&f.x), "{kind:?} {w} {e}: cat x {}", f.x);
                        for p in &f.props {
                            let (pw, ph) = size(p);
                            assert!(p.x >= 0 && p.x + pw <= w as i32 && p.y >= 0 && p.y + ph <= PX_H as i32, "{kind:?} {w} {e}: {p:?} clips");
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
                        let (a, b) = (scene_frame(kind, e - 1, d, seed, w), scene_frame(kind, e, d, seed, w));
                        assert!((a.x - b.x).abs() <= cap, "{kind:?} {w} {seed}: {} -> {} at {e}", a.x, b.x);
                    }
                }
            }
        }
    }

    #[test]
    fn run_runs_and_zoomies_run_faster() {
        let max_step = |kind| (1..12u64).map(|e| (scene_frame(kind, e, 30, 7, 80).x - scene_frame(kind, e - 1, 30, 7, 80).x).abs()).max().unwrap();
        let moving = (1..20u64).filter(|&e| scene_frame(Run, e, 30, 7, 80).x != scene_frame(Run, e - 1, 30, 7, 80).x).count();
        assert!(moving >= 15, "run mostly stands still: {moving}/19");
        assert!((0..20u64).all(|e| matches!(scene_pose(Run, e, 30, 7, 80), Pose::Run1 | Pose::Run2 | Pose::Turn)));
        assert!(max_step(Zoomies) > max_step(Run));
        assert!((0..20u64).any(|e| !scene_frame(Zoomies, e, 30, 7, 80).dots.is_empty()), "no dust");
    }

    #[test]
    fn finale_has_anticipation_then_pounce() {
        let poses = |e| scene_pose(BallChase, e, 30, 7, 80);
        assert_eq!(poses(26), Pose::Crouch);
        assert_eq!(poses(27), Pose::Pounce);
        for kind in [BallChase, ButterflyChase] {
            for w in [24usize, 60, 120] {
                for (d, seed) in CASES {
                    let f = |e| scene_frame(kind, e, d, seed, w);
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
                    assert_eq!(props_of(&scene_frame(BallChase, e, d, seed, w), Prop::Ball).len(), 1, "{w} {e}");
                }
                for e in d - 2..d {
                    let f = scene_frame(BallChase, e, d, seed, w);
                    let b = props_of(&f, Prop::Ball)[0];
                    let paws = if f.right { f.x + CAT_W - 4 } else { f.x + 1 };
                    assert!((b.x - paws).abs() <= 1 && b.y + 3 == PX_H as i32, "{w} {seed} {e}: ball {b:?} not under the paws of {f:?}");
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
                    let b = props_of(&scene_frame(ButterflyChase, e, d, seed, w), Prop::Butterfly);
                    assert_eq!(b.len(), 1);
                    assert!(b[0].y <= 4, "{w} {e}: butterfly low at {}", b[0].y);
                }
            }
        }
        for w in [60usize, 120] {
            let poses: Vec<Pose> = (0..36u64).map(|e| scene_pose(ButterflyChase, e, 40, 7, w)).collect();
            let at = poses.iter().position(|&p| p == Pose::Look).expect("looks up");
            assert_eq!(poses[at + 1], Pose::Cheer, "{w}: swat after the look: {poses:?}");
        }
    }

    #[test]
    fn yarn_rolls_on_the_ground_with_a_trail() {
        for w in [24usize, 60, 120] {
            for e in 0..30u64 {
                let f = scene_frame(YarnPush, e, 30, 7, w);
                let y = props_of(&f, Prop::Yarn);
                assert_eq!(y.len(), 1);
                assert_eq!(y[0].y + 4, PX_H as i32, "on the ground");
                assert!(f.dots.iter().all(|&(_, dy, _)| dy == PX_H as i32 - 1), "trail on row 11");
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
}
