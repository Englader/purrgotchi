//! Idle cat life: what the pixel cat does when nothing else is on stage.
//!
//! Pure and O(1) in `(now_ms, seed, mood, span, tempo)`: the statusline is re-run once a
//! second with no process in between, so nothing is persisted. Time is cut
//! into `BLOCK`-second blocks. Each block has an anchor x (`anchor`), and
//! consecutive anchors are at most 11 px apart (44 in the band), so the
//! cat's path is continuous by construction. A block opens with a walk from its anchor to
//! the next one, then fills the rest with behaviours picked by a per-block die
//! (weighted by mood). Every behaviour starts and ends on the next anchor,
//! facing the way the cat walked in (`out`), which is also how the next block
//! expects her (`in = out(b - 1)`): no jumps, and every facing change is
//! bracketed by a `Turn` (or happens mid-run in zoomies).
//!
//! The plan is in whole seconds at every tempo. In the band
//! (`Tempo::smooth`) walks and zoomies glide over their whole stretch instead
//! of stepping, zoomies spin and skid round, and a few beats get sub-second
//! detail (run bob, grooming licks).
//!
//! Accepted jumps: a mood flip mid-block re-plans the block, and the sleep
//! mood keeps a fixed bed at `span / 3` (entering or leaving it is a jump,
//! about once per 6 h). A terminal resize changes `span` and the anchors.

use super::art::{Pose, Tail};
use super::tempo::Tempo;
use crate::playground::tri;

/// Seconds per block plan.
const BLOCK: u64 = 30;
/// Leftovers shorter than this extend the previous behaviour instead.
const MIN_PICK: u64 = 4;

const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

/// splitmix64 output for `x`: the integer mixer behind every `% n` here
/// (`pixel::hash` is f64 over i32 and stays with the paint patterns).
pub(crate) fn ihash(x: u64) -> u64 {
    let mut z = x.wrapping_add(GOLDEN);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// splitmix64 stream: the per-block die. (`identity::Lcg` would leak the block
/// parity through its low bits.)
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(GOLDEN);
        ihash(self.0)
    }
    /// Uniform in `lo..=hi` (`hi >= lo`).
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next() % (hi - lo + 1)
    }
}

/// A once-in-`n` one-second event (blink, tail flick) that never fires two
/// seconds running, so it always reads as a single beat. (How much of that
/// second it shows is the tempo's `blink_ms` / `flick_ms`.)
fn beat(now: u64, salt: u64, n: u64) -> bool {
    let hit = |t: u64| ihash(t ^ salt).is_multiple_of(n);
    hit(now) && !hit(now.wrapping_sub(1))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Behaviour {
    Sit,
    Loaf,
    Sleep,
    Groom,
    Stretch,
    Look,
    Wander,
    Zoomies,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CatFrame {
    pub pose: Pose,
    /// Sprite's left px, always in `0..=span`.
    pub x: i32,
    pub facing_right: bool,
    pub tail: Tail,
    pub blink: bool,
    /// Rising sleep Z, `0..3` (low to high).
    pub z: Option<u8>,
    /// px above the ground: the band's run bobs up on the gathered frame.
    pub lift: u8,
}

/// `PURR_FORCE_POSE=sit|loaf|sleep|groom|stretch|look|wander|zoomies` (dev
/// backdoor, like `PURR_FORCE_SCENE`). `walk` is kept as an alias of `wander`.
pub(crate) fn force_pose_from(v: Option<&str>) -> Option<Behaviour> {
    Some(match v?.trim().to_ascii_lowercase().as_str() {
        "sit" => Behaviour::Sit,
        "loaf" => Behaviour::Loaf,
        "sleep" => Behaviour::Sleep,
        "groom" => Behaviour::Groom,
        "stretch" => Behaviour::Stretch,
        "look" => Behaviour::Look,
        "wander" | "walk" => Behaviour::Wander,
        "zoomies" => Behaviour::Zoomies,
        _ => return None,
    })
}

pub(crate) fn forced() -> Option<Behaviour> {
    force_pose_from(std::env::var("PURR_FORCE_POSE").ok().as_deref())
}

/// Pick weights per mood (unknown moods are calm). `Wander` is a short
/// excursion back the way she came and home again; `Sleep` is never picked
/// outside the sleep mood, which has its own plan.
fn weights(mood: &str) -> &'static [(Behaviour, u64)] {
    use Behaviour::*;
    match mood {
        "happy" => &[(Sit, 20), (Loaf, 10), (Groom, 15), (Stretch, 15), (Look, 10), (Wander, 30), (Zoomies, 8)],
        "alert" => &[(Sit, 25), (Loaf, 5), (Groom, 5), (Look, 40), (Wander, 25)],
        "hungry" => &[(Sit, 30), (Loaf, 10), (Look, 50)],
        "panic" => &[(Look, 30), (Zoomies, 50), (Wander, 20)],
        _ => &[(Sit, 30), (Loaf, 20), (Groom, 15), (Stretch, 10), (Look, 10), (Wander, 15)],
    }
}

fn pick(d: &mut Dice, mood: &str) -> Behaviour {
    let w = weights(mood);
    let mut roll = d.next() % w.iter().map(|(_, n)| n).sum::<u64>();
    for &(b, n) in w {
        if roll < n {
            return b;
        }
        roll -= n;
    }
    unreachable!("roll is below the weight total")
}

/// Seconds a stationary behaviour lasts when picked (Stretch is a fixed beat;
/// Wander and Zoomies take as long as their path).
fn dwell(b: Behaviour) -> (u64, u64) {
    match b {
        Behaviour::Loaf => (8, 14),
        Behaviour::Groom => (6, 10),
        Behaviour::Look => (5, 10),
        Behaviour::Stretch => (4, 4),
        _ => (6, 14),
    }
}

/// Block anchor: the mean of two triangle waves with different speeds, so
/// the cat drifts, dawdles and doubles back instead of pacing a fixed beat.
/// `|anchor(b + 1) - anchor(b)| <= (steps.0 + steps.1) / 2` (11 at the
/// statusline tempo).
fn anchor(b: u64, seed: u64, span: usize, t: &Tempo) -> i32 {
    let a = tri(b.wrapping_mul(t.anchor_steps.0).wrapping_add(seed % 97), span);
    let c = tri(b.wrapping_mul(t.anchor_steps.1).wrapping_add(seed / 97 % 89), span);
    ((a + c) / 2) as i32
}

/// Which way the cat walks (and then faces) during block `b`.
fn out(b: u64, seed: u64, span: usize, t: &Tempo) -> bool {
    let (from, to) = (anchor(b, seed, span, t), anchor(b.wrapping_add(1), seed, span, t));
    if from != to { to > from } else { ihash(b ^ seed) & 1 == 0 }
}

#[derive(Clone, Copy, Debug)]
enum Kind {
    Turn,
    Walk { to: i32 },
    /// Out `reach` px (signed) at the run speed and straight back.
    Zoom { reach: i32 },
    Act(Behaviour),
    /// Sleep-mood sleep: curled up, after one Loaf frame when `settle`.
    Doze { settle: bool },
}

/// One stretch of a block plan, with the cat's x and facing at its start
/// (for `Turn`, the facing after it).
#[derive(Clone, Copy, Debug)]
struct Seg {
    kind: Kind,
    len: u64,
    x: i32,
    right: bool,
}

fn zoom_out_ticks(reach: i32, t: &Tempo) -> u64 {
    (reach.unsigned_abs() as u64).div_ceil(t.run_px_s as u64)
}

/// Builds a block plan while tracking where the cat is and which way she faces.
#[derive(Clone)]
struct Plan<'a> {
    t: &'a Tempo,
    segs: Vec<Seg>,
    x: i32,
    right: bool,
    used: u64,
}

impl Plan<'_> {
    /// Appends a stretch; a behaviour picked twice running becomes one longer
    /// one (no stand-up-and-sit-again between two loaves).
    fn push(&mut self, kind: Kind, len: u64) {
        self.used += len;
        match (self.segs.last_mut(), kind) {
            (Some(last), Kind::Act(b)) if matches!(last.kind, Kind::Act(a) if a == b) => last.len += len,
            _ => self.segs.push(Seg { kind, len, x: self.x, right: self.right }),
        }
    }

    fn turn(&mut self, right: bool) {
        if right != self.right {
            self.right = right;
            self.push(Kind::Turn, 1);
        }
    }

    fn walk(&mut self, to: i32) {
        let d = to - self.x;
        if d != 0 {
            self.turn(d > 0);
            // At least a full gait cycle: a lone Walk1 between two Sits reads as a twitch.
            self.push(Kind::Walk { to }, (d.unsigned_abs() as u64).div_ceil(self.t.walk_px_s as u64).max(2));
            self.x = to;
        }
    }

    /// Run toward the roomier side and back; ends facing `home`.
    fn zoom(&mut self, span: i32, home: bool) -> bool {
        let cap = match self.t.run_reach_span_div {
            0 => self.t.run_reach_max,
            div => self.t.run_reach_max.min(span / div),
        };
        let reach = if span - self.x >= self.x { (span - self.x).min(cap) } else { -self.x.min(cap) };
        if reach.abs() < self.t.run_px_s {
            return false;
        }
        let len = if self.t.smooth {
            (zoom_spin_ms(reach, self.right) + 2 * zoom_leg_ms(reach, self.t) + SKID_MS).div_ceil(1000)
        } else {
            2 * zoom_out_ticks(reach, self.t) + 1
        };
        self.push(Kind::Zoom { reach }, len);
        self.right = reach < 0;
        self.turn(home);
        true
    }

    /// A few steps back the way she came, then home, ending facing `home`.
    fn excursion(&mut self, d: &mut Dice, span: i32, home: bool) -> bool {
        let (x, back) = (self.x, if home { -1 } else { 1 });
        let away = (x + back * d.range(self.t.wander_px.0, self.t.wander_px.1) as i32).clamp(0, span);
        if (away - x).abs() < self.t.walk_px_s {
            return false;
        }
        self.walk(away);
        self.walk(x);
        self.turn(home);
        true
    }

    /// Hand the last `rem` seconds to the previous behaviour when it can
    /// stretch, else to a Sit.
    fn pad(&mut self, rem: u64) {
        match self.segs.last_mut() {
            Some(s) if matches!(s.kind, Kind::Act(Behaviour::Sit | Behaviour::Loaf | Behaviour::Groom | Behaviour::Look | Behaviour::Sleep)) => {
                s.len += rem;
                self.used += rem;
            }
            _ => self.push(Kind::Act(Behaviour::Sit), rem),
        }
    }
}

fn block_plan<'a>(b: u64, seed: u64, mood: &str, span: usize, forced: Option<Behaviour>, t: &'a Tempo) -> Plan<'a> {
    let mut d = Dice(b ^ seed ^ 0x4C49_4645);
    let sp = span as i32;
    if mood == "sleep" && forced.is_none() {
        // A fixed bed and a fixed side; about every 2.5 min she wakes for a
        // stretch and settles back down.
        let mut p = Plan { t, segs: Vec::new(), x: sp / 3, right: (seed >> 3) & 1 == 0, used: 0 };
        let wake = ihash(b ^ seed ^ 0x5A5A).is_multiple_of(5);
        if wake {
            p.push(Kind::Act(Behaviour::Stretch), 4);
        }
        p.push(Kind::Doze { settle: wake }, BLOCK - p.used);
        return p;
    }
    let home = out(b, seed, span, t);
    let mut p = Plan { t, segs: Vec::new(), x: anchor(b, seed, span, t), right: out(b.wrapping_sub(1), seed, span, t), used: 0 };
    p.walk(anchor(b.wrapping_add(1), seed, span, t));
    p.turn(home);
    while p.used < BLOCK {
        let rem = BLOCK - p.used;
        if rem < MIN_PICK {
            p.pad(rem);
            break;
        }
        let mut beh = forced.unwrap_or_else(|| pick(&mut d, mood));
        if matches!(beh, Behaviour::Wander | Behaviour::Zoomies) {
            let before = p.clone();
            let placed = if beh == Behaviour::Wander { p.excursion(&mut d, sp, home) } else { p.zoom(sp, home) };
            // A path must end the block or leave room for a real pick: a
            // 1-3 s sit between two walks reads as a stutter.
            let left = BLOCK.saturating_sub(p.used);
            if placed && (p.used > BLOCK || (1..MIN_PICK).contains(&left)) {
                p = before;
                p.pad(rem);
                break;
            }
            if placed {
                continue;
            }
            // Against a wall with no room to run: she looks around instead.
            beh = Behaviour::Look;
        }
        let (lo, hi) = dwell(beh);
        let len = d.range(lo, hi).min(rem);
        p.push(Kind::Act(beh), if rem - len < MIN_PICK { rem } else { len });
    }
    p
}

/// Poses whose map has eyes to blink.
fn has_eyes(p: Pose) -> bool {
    super::art::pose_map(p, super::PxHeight::H12).iter().any(|r| r.contains('e'))
}

/// Alternates two poses every 2 s on the wall clock (breathing), so the
/// rhythm carries across behaviour and block boundaries.
fn breathe(now: u64, a: Pose, b: Pose) -> Pose {
    if (now / 2).is_multiple_of(2) { a } else { b }
}

/// Pose of a stationary behaviour at tick `k` of `len`.
fn act_pose(b: Behaviour, k: u64, len: u64, now: u64, mood: &str) -> Pose {
    let last = k + 1 == len;
    match b {
        Behaviour::Loaf if k == 0 || last => Pose::Sit,
        Behaviour::Loaf => breathe(now, Pose::Loaf, Pose::LoafBreath),
        Behaviour::Sleep if k == 0 => Pose::Loaf,
        Behaviour::Sleep if last => Pose::Sit,
        Behaviour::Sleep => breathe(now, Pose::Curl, Pose::CurlBreath),
        Behaviour::Groom if k == 0 || last => Pose::Sit,
        // lick, lick, wipe
        Behaviour::Groom if k.is_multiple_of(3) => Pose::GroomPaw,
        Behaviour::Groom => Pose::GroomLick,
        Behaviour::Stretch => [Pose::Crouch, Pose::Stretch, Pose::Stretch].get(k as usize).copied().unwrap_or(Pose::Sit),
        Behaviour::Look if k == 0 => Pose::Sit,
        Behaviour::Look if mood == "hungry" && k.is_multiple_of(3) => Pose::Meow,
        Behaviour::Look => Pose::Look,
        _ => Pose::Sit,
    }
}

fn is_gait(p: Pose) -> bool {
    matches!(p, Pose::Walk1 | Pose::Walk2 | Pose::Run1 | Pose::Run2)
}

/// `a / b` rounded to the nearest whole (halves away from zero), like the
/// scenes' x, so a gait steps evenly whichever way she faces.
fn div_round(a: i64, b: i64) -> i64 {
    (a + a.signum() * b / 2) / b
}

/// How long a smooth grooming lick keeps the head down in its second.
const LICK_MS: u64 = 600;

/// A gliding zoom spins round (a `Turn`) for this long before it bolts the
/// other way, and skids round for at least this long at the far end.
const SKID_MS: u64 = 250;

/// Spin-round before a gliding zoom: she bolts away from where she faces.
fn zoom_spin_ms(reach: i32, right: bool) -> u64 {
    if (reach > 0) == right { 0 } else { SKID_MS }
}

/// Run time of one leg of a gliding zoom, ms.
fn zoom_leg_ms(reach: i32, t: &Tempo) -> u64 {
    u64::from(reach.unsigned_abs()) * 1000 / t.run_px_s as u64
}

/// A gliding zoom, `ms` into its `len_ms`, started facing `right`: a spin
/// round when she faces the wrong way, out `reach` px (signed) at the run
/// speed, a skid round at the far end for the rest of the whole seconds, and
/// straight back. Returns (pose, offset from the start, facing out).
fn glide_zoom(reach: i32, right: bool, ms: u64, len_ms: u64, t: &Tempo) -> (Pose, i32, bool) {
    let dist = u64::from(reach.unsigned_abs());
    let spin = zoom_spin_ms(reach, right);
    let leg = zoom_leg_ms(reach, t).min(len_ms.saturating_sub(spin) / 2).max(1);
    let back_at = len_ms - leg;
    let gait = |ms: u64| if ms / t.run_gait_ms % 2 == 1 { Pose::Run2 } else { Pose::Run1 };
    let (pose, off, out) = if ms < spin {
        (Pose::Turn, 0, true)
    } else if ms < spin + leg {
        (gait(ms - spin), div_round(dist as i64 * (ms - spin) as i64, leg as i64) as u64, true)
    } else if ms < back_at {
        (Pose::Turn, dist, false)
    } else {
        let back = ms - back_at;
        (gait(back), dist - (div_round(dist as i64 * back as i64, leg as i64) as u64).min(dist), false)
    };
    (pose, off as i32 * reach.signum(), out)
}

/// The cat at whole second `now`, `sub_ms` (< 1000) into it. Position and
/// pose are those of tick `now`; the gait frame and the blink / flick beats
/// follow `sub_ms`.
fn tick_frame(now: u64, sub_ms: u64, seed: u64, mood: &str, span: usize, forced: Option<Behaviour>, t: &Tempo) -> CatFrame {
    let (b, tk) = (now / BLOCK, now % BLOCK);
    let plan = block_plan(b, seed, mood, span, forced, t);
    let mut start = 0;
    let seg = plan
        .segs
        .iter()
        .find(|s| {
            start += s.len;
            tk < start
        })
        .copied()
        .expect("a block plan covers the whole block");
    let k = tk - (start - seg.len);
    // ms into the segment; gait frames flip every period from its start
    // (`k % 2` at 1 s).
    let (ms, len_ms) = (k * 1000 + sub_ms, seg.len * 1000);
    let flip = |period: u64| ms / period % 2 == 1;
    let (pose, x, right) = match seg.kind {
        Kind::Turn => (Pose::Turn, seg.x, seg.right),
        Kind::Walk { to } => {
            let gait = if flip(t.gait_ms) { Pose::Walk2 } else { Pose::Walk1 };
            if t.smooth {
                // At least half the walk speed: a few px over a 2 s walk
                // would tread in place, so she steps over and sits.
                let d = to - seg.x;
                let moving = len_ms.min(u64::from(d.unsigned_abs()) * 2000 / t.walk_px_s as u64);
                if ms < moving {
                    (gait, seg.x + div_round(i64::from(d) * ms as i64, moving as i64) as i32, seg.right)
                } else {
                    (Pose::Sit, to, seg.right)
                }
            } else {
                (gait, seg.x + (to - seg.x) * (k as i32 + 1) / seg.len as i32, seg.right)
            }
        }
        Kind::Zoom { reach } if t.smooth => {
            let (pose, off, out) = glide_zoom(reach, seg.right, ms, len_ms, t);
            (pose, seg.x + off, out == (reach > 0))
        }
        Kind::Zoom { reach } => {
            let n = zoom_out_ticks(reach, t);
            let off = if k <= n { k as i32 * t.run_px_s } else { reach.abs() - (k - n) as i32 * t.run_px_s };
            let gait = if flip(t.run_gait_ms) { Pose::Run2 } else { Pose::Run1 };
            (gait, seg.x + off.clamp(0, reach.abs()) * reach.signum(), (k <= n) == (reach > 0))
        }
        // Smooth: each lick dips the head and comes back up, so two licks
        // read as two rather than one held pose.
        Kind::Act(beh) => match act_pose(beh, k, seg.len, now, mood) {
            Pose::GroomLick if t.smooth && sub_ms >= LICK_MS => (Pose::Sit, seg.x, seg.right),
            p => (p, seg.x, seg.right),
        },
        Kind::Doze { settle } if settle && k == 0 => (Pose::Loaf, seg.x, seg.right),
        Kind::Doze { .. } => (breathe(now, Pose::Curl, Pose::CurlBreath), seg.x, seg.right),
    };
    let base_up = ihash(b ^ seed ^ 0x7A11) % 10 < if mood == "happy" { 7 } else { 3 };
    let tail = match pose {
        Pose::Loaf | Pose::LoafBreath => Tail::Curl,
        _ if base_up && beat(now, !seed, 5) && sub_ms < t.flick_ms => Tail::Flick,
        _ if base_up => Tail::Up,
        _ => Tail::Curl,
    };
    CatFrame {
        pose,
        x: x.clamp(0, span as i32),
        facing_right: right,
        tail,
        blink: has_eyes(pose) && beat(now, seed, 7) && sub_ms < t.blink_ms,
        z: matches!(pose, Pose::Curl | Pose::CurlBreath).then_some((now % 3) as u8),
        lift: u8::from(t.smooth && pose == Pose::Run2),
    }
}

/// The cat at `now_ms`. `seed` is the cat's `birth_time`, `span` the
/// room for the sprite's left px (`stage width - cat_max_w`). `forced`
/// replaces the mood picks (the walk between anchors stays), and a forced
/// Sleep happens on the anchor, not the bed.
///
/// The plan runs in whole seconds. While the pose of second `k` is a gait
/// pose, x slides linearly to the next second's x, so whole seconds give the
/// same cat as a 1-fps clock and everything between is smooth.
pub(crate) fn life_frame_ms(now_ms: u64, seed: u64, mood: &str, span: usize, forced: Option<Behaviour>, t: &Tempo) -> CatFrame {
    let (sec, sub) = (now_ms / 1000, now_ms % 1000);
    let mut f = tick_frame(sec, sub, seed, mood, span, forced, t);
    if !t.smooth && sub > 0 && is_gait(f.pose) {
        let next = tick_frame(sec.wrapping_add(1), 0, seed, mood, span, forced, t);
        f.x += ((i64::from(next.x - f.x) * sub as i64) / 1000) as i32;
    }
    f
}

/// The cat at whole second `now` at the statusline tempo.
#[cfg(test)]
pub(crate) fn life_frame(now: u64, seed: u64, mood: &str, span: usize, forced: Option<Behaviour>) -> CatFrame {
    life_frame_ms(now.saturating_mul(1000), seed, mood, span, forced, &Tempo::STATUSLINE)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOODS: [&str; 6] = ["calm", "happy", "alert", "hungry", "panic", "sleep"];

    /// One hour from 1_800_000_000 in `step_ms` samples: x stays in
    /// `0..=span`, moves at most `max_dx` per sample, and facing only flips
    /// around a Turn or mid-run.
    fn assert_continuous_at(t: &Tempo, step_ms: u64, max_dx: i32, seed: u64, mood: &str, span: usize, forced: Option<Behaviour>) {
        let t0 = 1_800_000_000_000;
        let mut prev = life_frame_ms(t0, seed, mood, span, forced, t);
        for i in 1..3_600_000 / step_ms {
            let now = t0 + i * step_ms;
            let f = life_frame_ms(now, seed, mood, span, forced, t);
            assert!((0..=span as i32).contains(&f.x), "{mood} {span} {forced:?}: x {} at {now}", f.x);
            assert!((f.x - prev.x).abs() <= max_dx, "{mood} {forced:?}: teleport at {now}: {} -> {}", prev.x, f.x);
            if f.facing_right != prev.facing_right {
                assert!(
                    f.pose == Pose::Turn || prev.pose == Pose::Turn || matches!(f.pose, Pose::Run1 | Pose::Run2),
                    "{mood} {forced:?}: flip without turn at {now}"
                );
            }
            prev = f;
        }
    }

    /// At the statusline tempo, whole seconds, at most 4 px/s (the zoomies peak).
    fn assert_continuous(seed: u64, mood: &str, span: usize, forced: Option<Behaviour>) {
        assert_continuous_at(&Tempo::STATUSLINE, 1000, 4, seed, mood, span, forced);
    }

    #[test]
    fn continuous_and_in_bounds() {
        for m in MOODS {
            for span in [0usize, 8, 44, 104] {
                assert_continuous(42, m, span, None);
            }
        }
    }

    /// The band re-runs the continuity rules on its own 30-fps frames: at
    /// most 2 px a frame (a 32 px/s run), for every mood and forced behaviour.
    #[test]
    fn band_continuous_and_in_bounds() {
        let t = &Tempo::BAND;
        for m in MOODS {
            for span in [0usize, 10, 44, 106, 110] {
                assert_continuous_at(t, 33, 2, 42, m, span, None);
            }
        }
        use Behaviour::*;
        for b in [Sit, Loaf, Sleep, Groom, Stretch, Look, Wander, Zoomies] {
            assert_continuous_at(t, 33, 2, 5, "calm", 106, Some(b));
        }
    }

    /// `n` frames from `t0_ms`, `step_ms` apart, at the band tempo.
    fn band_frames(seed: u64, mood: &str, span: usize, forced: Option<Behaviour>, t0_ms: u64, step_ms: u64, n: u64) -> Vec<CatFrame> {
        (0..n).map(|i| life_frame_ms(t0_ms + i * step_ms, seed, mood, span, forced, &Tempo::BAND)).collect()
    }

    /// Over 30 min in a 124-col band she visits at least 90 % of it.
    #[test]
    fn band_roams_full_width() {
        let span = 110;
        for seed in [7, 1_000, 123_456_789] {
            for mood in ["calm", "happy", "alert"] {
                let fr = band_frames(seed, mood, span, None, 1_800_000_000_000, 250, 30 * 60 * 4);
                let (lo, hi) = fr.iter().fold((i32::MAX, i32::MIN), |(lo, hi), f| (lo.min(f.x), hi.max(f.x)));
                assert!(hi - lo >= span as i32 * 9 / 10, "{seed} {mood}: x only in {lo}..={hi}");
            }
        }
    }

    fn walking(p: Pose) -> bool {
        matches!(p, Pose::Walk1 | Pose::Walk2)
    }

    fn running(p: Pose) -> bool {
        matches!(p, Pose::Run1 | Pose::Run2)
    }

    /// 33-ms frames: at most 1 px a frame while she walks, 2 while she runs,
    /// she never stands still in a gait pose for more than 300 ms (no
    /// treadmill), and she always moves the way she faces (no moonwalk).
    #[test]
    fn band_walk_is_smooth() {
        let (mut walked, mut ran) = (0, 0);
        for (mood, forced) in [("calm", None), ("happy", None), ("panic", None), ("calm", Some(Behaviour::Wander)), ("calm", Some(Behaviour::Zoomies))] {
            let fr = band_frames(3, mood, 106, forced, 1_800_000_000_000, 33, 20 * 60 * 30);
            let mut still = 0;
            for w in fr.windows(2) {
                let (a, b) = (w[0], w[1]);
                let dx = b.x - a.x;
                if walking(a.pose) && walking(b.pose) {
                    assert!(dx.abs() <= 1, "{mood} {forced:?}: walk step {} -> {}", a.x, b.x);
                    walked += 1;
                }
                if running(a.pose) && running(b.pose) {
                    assert!(dx.abs() <= 2, "{mood} {forced:?}: run step {} -> {}", a.x, b.x);
                    ran += 1;
                }
                if (walking(b.pose) || running(b.pose)) && a.facing_right == b.facing_right && dx != 0 {
                    assert_eq!(dx > 0, b.facing_right, "{mood} {forced:?}: moonwalk {} -> {} ({:?})", a.x, b.x, b.pose);
                }
                still = if (walking(b.pose) || running(b.pose)) && dx == 0 { still + 1 } else { 0 };
                assert!(still * 33 <= 300, "{mood} {forced:?}: gait pose standing still at {} ({:?})", b.x, b.pose);
            }
        }
        assert!(walked > 1000 && ran > 1000, "walked {walked}, ran {ran} frames");
    }

    /// Steady gait at 33-ms frames moves by even steps: each step is one of
    /// two neighbouring whole px (walk 0 or 1, run 1 or 2, never a 0 next to
    /// a 2), and every 8-frame window moves within 1.5 px of the run's own
    /// average rate.
    #[test]
    fn band_steps_are_even() {
        const WIN: usize = 8;
        let (mut walks, mut runs) = (0, 0);
        for (mood, forced) in [("calm", Some(Behaviour::Wander)), ("calm", Some(Behaviour::Zoomies)), ("happy", None)] {
            let fr = band_frames(5, mood, 106, forced, 1_800_000_000_000, 33, 20 * 60 * 30);
            let mut i = 0;
            while i < fr.len() {
                let (p0, r0) = (fr[i].pose, fr[i].facing_right);
                let kind = if walking(p0) { 1 } else if running(p0) { 2 } else { 0 };
                let mut j = i + 1;
                while kind != 0 && j < fr.len() && fr[j].facing_right == r0 && (if kind == 1 { walking(fr[j].pose) } else { running(fr[j].pose) }) {
                    j += 1;
                }
                let run = &fr[i..j];
                i = j.max(i + 1);
                if kind == 0 || run.len() < WIN + 2 {
                    continue;
                }
                // The first and last frames sit on a segment edge.
                let run = &run[1..run.len() - 1];
                let dx: Vec<i32> = run.windows(2).map(|w| (w[1].x - w[0].x).abs()).collect();
                let (lo, hi) = (*dx.iter().min().unwrap(), *dx.iter().max().unwrap());
                let rate = f64::from(dx.iter().sum::<i32>()) / dx.len() as f64;
                // Every step is the whole px either side of the rate, so a
                // 1.06 px/frame run steps 1 or 2, a 0.26 walk 0 or 1.
                let (floor, ceil) = ((rate - 0.1).floor() as i32, (rate + 0.1).ceil() as i32);
                assert!(lo >= floor && hi <= ceil, "{mood} {forced:?}: steps {lo}..{hi} vs rate {rate} at x {}", run[0].x);
                for w in dx.windows(WIN) {
                    let moved = f64::from(w.iter().sum::<i32>());
                    assert!((moved - rate * WIN as f64).abs() <= 1.5, "{mood} {forced:?}: {w:?} vs rate {rate}");
                }
                if kind == 1 { walks += 1 } else { runs += 1 }
            }
        }
        assert!(walks > 20 && runs > 20, "walks {walks}, runs {runs}");
    }

    /// Lengths in ms of the unbroken runs of frames (1-ms samples) where
    /// `is` holds and the pose stays the same, skipping the first and last
    /// run (cut by the window).
    fn runs_ms(fr: &[CatFrame], is: impl Fn(&CatFrame) -> bool) -> Vec<u64> {
        let mut out = Vec::new();
        let mut len = 0u64;
        for w in fr.windows(2) {
            if is(&w[0]) {
                len += 1;
                if !is(&w[1]) || w[1].pose != w[0].pose {
                    out.push(len);
                    len = 0;
                }
            } else {
                len = 0;
            }
        }
        if out.len() > 2 { out[1..out.len() - 1].to_vec() } else { Vec::new() }
    }

    /// The walk flips Walk1/Walk2 every 100-200 ms (sub-second, a stroll
    /// rather than a scurry), and the run flips faster.
    #[test]
    fn band_gait_cycles_sub_second() {
        let fr = band_frames(3, "calm", 106, Some(Behaviour::Wander), 1_800_000_000_000, 1, 120_000);
        let walk = runs_ms(&fr, |f| walking(f.pose));
        assert!(walk.len() > 50, "{} walk frames", walk.len());
        // A walk can end mid-frame; whole frames are the gait period.
        let full = *walk.iter().max().unwrap();
        assert!((100..=200).contains(&full), "walk frame {full} ms");
        let fr = band_frames(3, "calm", 106, Some(Behaviour::Zoomies), 1_800_000_000_000, 1, 120_000);
        let run = runs_ms(&fr, |f| running(f.pose));
        assert!(run.len() > 50, "{} run frames", run.len());
        assert!(*run.iter().max().unwrap() < full, "run frame {} ms", run.iter().max().unwrap());
    }

    /// A blink is a quick shut-open, at most 200 ms, and a tail flick is
    /// shorter than a second.
    #[test]
    fn band_blink_is_short() {
        let fr = band_frames(9, "calm", 44, None, 1_800_000_000_000, 1, 600_000);
        let blinks = runs_ms(&fr, |f| f.blink);
        assert!(!blinks.is_empty(), "never blinks");
        assert!(blinks.iter().all(|&ms| ms <= 200), "blinks {blinks:?}");
        let flicks = runs_ms(&fr, |f| f.tail == Tail::Flick);
        assert!(!flicks.is_empty() && flicks.iter().all(|&ms| ms < 1000), "flicks {flicks:?}");
    }

    #[test]
    fn mood_biases_behaviour() {
        let count = |m: &str, p: &[Pose]| (0..3600u64).filter(|t| p.contains(&life_frame(1_800_000_000 + t, 7, m, 44, None).pose)).count();
        assert!(count("sleep", &[Pose::Curl, Pose::CurlBreath]) > 2800);
        assert!(count("panic", &[Pose::Run1, Pose::Run2]) > 600);
        assert!(count("hungry", &[Pose::Look, Pose::Meow]) > count("calm", &[Pose::Look, Pose::Meow]));
    }

    #[test]
    fn every_behaviour_reachable_in_a_day() {
        for m in ["calm", "happy"] {
            let mut seen = std::collections::HashSet::new();
            for t in 0..86_400u64 {
                seen.insert(life_frame(t, 3, m, 44, None).pose);
            }
            for p in [Pose::Sit, Pose::Walk1, Pose::Loaf, Pose::GroomLick, Pose::Stretch, Pose::Turn] {
                assert!(seen.contains(&p), "{m} never {p:?}");
            }
        }
    }

    #[test]
    fn extreme_seeds_and_clock_do_not_panic() {
        for seed in [0, u64::MAX] {
            for now in [0, 29, 30, u64::MAX / 2, u64::MAX - 31, u64::MAX] {
                for m in MOODS {
                    let _ = life_frame(now, seed, m, 44, None);
                }
            }
        }
    }

    #[test]
    fn forced_pose_parses_and_wins() {
        assert_eq!(force_pose_from(Some("groom")), Some(Behaviour::Groom));
        assert_eq!(force_pose_from(Some(" Zoomies ")), Some(Behaviour::Zoomies));
        assert_eq!(force_pose_from(Some("walk")), Some(Behaviour::Wander));
        assert_eq!(force_pose_from(Some("x")), None);
        assert_eq!(force_pose_from(None), None);
        let f = life_frame(1_800_000_020, 1, "calm", 44, Some(Behaviour::Sleep));
        assert!(matches!(f.pose, Pose::Curl | Pose::CurlBreath | Pose::Loaf));
    }

    /// Forced behaviours keep the same continuity rules as the mood picks.
    #[test]
    fn forced_is_continuous_too() {
        use Behaviour::*;
        for b in [Sit, Loaf, Sleep, Groom, Stretch, Look, Wander, Zoomies] {
            for span in [0usize, 44] {
                assert_continuous(5, "calm", span, Some(b));
            }
        }
    }

    /// No jitter: a pose shown for a single second between two identical
    /// neighbours (A B A) is only allowed for deliberate 1 s beats: gaits,
    /// grooming licks, a meow, a turn, a stretch's crouch.
    #[test]
    fn no_single_frame_blips() {
        let beat = |a: Pose, b: Pose| {
            let pair = |x, y| (a == x && b == y) || (a == y && b == x);
            b == Pose::Turn
                || pair(Pose::Walk1, Pose::Walk2)
                || pair(Pose::Run1, Pose::Run2)
                || pair(Pose::GroomLick, Pose::GroomPaw)
                || pair(Pose::Look, Pose::Meow)
        };
        for m in MOODS {
            let poses: Vec<Pose> = (0..7200u64).map(|t| life_frame(1_800_000_000 + t, 11, m, 60, None).pose).collect();
            let band: Vec<Pose> = band_frames(11, m, 106, None, 1_800_000_000_000, 33, 60 * 60 * 30).iter().map(|f| f.pose).collect();
            for (poses, at) in [(poses, "1 fps"), (band, "band")] {
                for w in poses.windows(3) {
                    if w[0] == w[2] && w[1] != w[0] {
                        assert!(beat(w[0], w[1]), "{m} {at}: blip {:?} {:?} {:?}", w[0], w[1], w[2]);
                    }
                }
            }
        }
    }

    /// Blinks and tail flicks never fire two seconds running.
    #[test]
    fn blinks_are_single_frames() {
        let fr: Vec<CatFrame> = (0..3600u64).map(|t| life_frame(1_800_000_000 + t, 9, "calm", 44, None)).collect();
        assert!(fr.iter().any(|f| f.blink), "never blinks");
        for w in fr.windows(2) {
            assert!(!(w[0].blink && w[1].blink), "double blink");
            assert!(!(w[0].tail == Tail::Flick && w[1].tail == Tail::Flick), "double flick");
        }
    }

    /// At the statusline tempo, 33 ms samples never move more than 1 px while
    /// she walks (the gait interpolates between whole seconds).
    #[test]
    fn ms_samples_are_continuous() {
        let t = &Tempo::STATUSLINE;
        let mut walked = 0;
        for mood in ["calm", "happy"] {
            let mut prev = life_frame_ms(1_800_000_000_000, 5, mood, 104, None, t);
            for i in 1..(600_000 / 33) {
                let f = life_frame_ms(1_800_000_000_000 + i * 33, 5, mood, 104, None, t);
                if matches!(f.pose, Pose::Walk1 | Pose::Walk2) && matches!(prev.pose, Pose::Walk1 | Pose::Walk2) {
                    assert!((f.x - prev.x).abs() <= 1, "{mood}: {} -> {} at sample {i}", prev.x, f.x);
                    walked += 1;
                }
                prev = f;
            }
        }
        assert!(walked > 100, "only {walked} walking samples");
    }
}
