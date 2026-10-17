//! Pixel celebrations and the birthday: the cat cheers, centred, with a prop
//! effect per `CelebrationKind`. Triggers, timing and speech stay with
//! `kitten_view` and the celebration counter; this only decides what each
//! second looks like, as a pure function of `(overlay, now, width, look, h)`
//! (nothing persisted: `f = now % 2` drives the 2-frame props, `k = now % 6`
//! the rising / falling motion, `now / 6` re-rolls the confetti).
//!
//! Geometry first (`fx_frame`, a `SceneFrame` the scene drawer can paint),
//! then `overlay_canvas` draws it. Props sit in the stage margins or on the
//! cat's own empty corners and never clip: the 12-px cat is 16 px wide and
//! the narrowest stage is 24, so a side prop is at most 4 px wide.

use super::art::{self, Pose, Prop};
use super::life::ihash;
use super::paint::{self, CatLook};
use super::scene::{self, SceneFrame};
use super::{PxCanvas, PxHeight};
use crate::pixel;
use crate::state::CelebrationKind;

/// Seconds per rising / falling cycle (`k = now % CYCLE`).
const CYCLE: u64 = 6;
/// Px per second for rising sparkles and falling confetti: one cycle spans
/// exactly the 12-px canvas, so the wrap is seamless there.
const SPEED: i32 = 2;
const BIRTHDAY_CONFETTI: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Overlay {
    Celebration(CelebrationKind),
    Birthday,
}

impl Overlay {
    /// A celebration outranks the birthday (as in `kitten_view`).
    pub(crate) fn pick(kind: CelebrationKind, birthday: bool) -> Option<Self> {
        match (kind, birthday) {
            (CelebrationKind::None, false) => None,
            (CelebrationKind::None, true) => Some(Self::Birthday),
            (k, _) => Some(Self::Celebration(k)),
        }
    }
}

/// Confetti pieces of a milestone tier.
fn confetti_count(k: CelebrationKind) -> usize {
    match k {
        CelebrationKind::MilestoneCenturion => 8,
        CelebrationKind::MilestoneVeteran => 12,
        CelebrationKind::MilestoneMaster => 16,
        CelebrationKind::MilestoneLegendary => 24,
        _ => 0,
    }
}

fn put(f: &mut SceneFrame, w: usize, prop: Prop, frame: u8, x: i32, y: i32, h: PxHeight) {
    // Never clipped by the canvas. Behind the cat, except the hat and heart
    // that sit on her.
    f.put((w, h), prop, frame, x, y, matches!(prop, Prop::Hat | Prop::Heart));
}

/// Falling confetti: piece `i` of this 6 s block, wrapping top to bottom.
fn confetti(f: &mut SceneFrame, w: usize, n: usize, now: u64, h: PxHeight) {
    let (block, k) = (now / CYCLE, (now % CYCLE) as i32);
    for i in 0..n as u64 {
        let hv = ihash(i << 32 ^ block);
        let (x, y0) = ((hv % w.max(1) as u64) as i32, ((hv >> 24) % scene::ph(h) as u64) as i32);
        put(f, w, Prop::Confetti, ((hv >> 48) % 6) as u8, x, (y0 + SPEED * k) % scene::ph(h), h);
    }
}

/// `n` sparkles rising `SPEED` px/s on both sides of the cat, alternating
/// sides, staggered 4 px apart; one is hidden while it wraps.
fn sparkles(f: &mut SceneFrame, w: usize, n: usize, now: u64, cx: i32, cw: i32, h: PxHeight) {
    let (k, frame) = ((now % CYCLE) as i32, (now % 2) as u8);
    let sh = art::prop(Prop::Sparkle, 0, h).h as i32;
    for i in 0..n as i32 {
        let rise = (SPEED * k + 4 * i) % scene::ph(h);
        if rise <= scene::ph(h) - sh {
            let x = if i % 2 == 0 { cx - 4 } else { cx + cw + 1 };
            put(f, w, Prop::Sparkle, frame, x, scene::ph(h) - sh - rise, h);
        }
    }
}

/// One celebration second as geometry. The cat faces right and cheers
/// (Greeting waves on odd seconds); `top` is where her sprite starts.
pub(crate) fn fx_frame(o: Overlay, now: u64, width: usize, look: &CatLook, h: PxHeight) -> SceneFrame {
    use CelebrationKind::*;
    let pose = match o {
        Overlay::Celebration(Greeting) if now % 2 == 1 => Pose::Wave,
        _ => Pose::Cheer,
    };
    let cat = paint::paint(pose, look, true, art::Tail::Up, false, h);
    let (cw, top) = (cat.w as i32, scene::ph(h) - cat.h as i32);
    let cx = (width as i32 - cw) / 2;
    let mut f = SceneFrame { pose, x: cx, right: true, lift: 0, props: Vec::new(), dots: Vec::new() };
    let frame = (now % 2) as u8;
    match o {
        Overlay::Celebration(LevelUp) => sparkles(&mut f, width, 3, now, cx, cw, h),
        Overlay::Celebration(EvolvingYoung | EvolvingAdult) => sparkles(&mut f, width, 2, now, cx, cw, h),
        Overlay::Celebration(Streak) => {
            let fh = art::prop(Prop::Flame, 0, h).h as i32;
            put(&mut f, width, Prop::Flame, frame, cx - 4, scene::ph(h) - fh, h);
            put(&mut f, width, Prop::Flame, frame ^ 1, cx + cw + 1, scene::ph(h) - fh, h);
        }
        Overlay::Celebration(k @ (MilestoneCenturion | MilestoneVeteran | MilestoneMaster | MilestoneLegendary)) => {
            confetti(&mut f, width, confetti_count(k), now, h)
        }
        Overlay::Celebration(Greeting) => {
            // Over the cat's empty back corner, bobbing 1 px. The small cats
            // have no room there, so it floats beside her back instead.
            let x = if h == PxHeight::H12 { cx + 1 } else { cx - art::prop(Prop::Heart, 0, h).w as i32 - 1 };
            put(&mut f, width, Prop::Heart, 0, x, top + i32::from(frame), h);
        }
        Overlay::Celebration(RandomDrop) => {
            // Falls from the top over 3 s (0, 4, 8 px), then glints on the ground.
            let k = (now % CYCLE) as i32;
            let ch = art::prop(Prop::Coin, 0, h).h as i32;
            put(&mut f, width, Prop::Coin, if k < 3 { 0 } else { frame }, cx + cw, (scene::ph(h) - ch) * k.min(2) / 2, h);
        }
        Overlay::Celebration(None) => {}
        Overlay::Birthday => {
            confetti(&mut f, width, BIRTHDAY_CONFETTI, now, h);
            // On the head between the ears: brim over the ear tips, so 2 px
            // above her sprite (an adult has no headroom and wears it low).
            let over_head = match h {
                PxHeight::H12 => 7,
                PxHeight::H10 => 6,
            };
            put(&mut f, width, Prop::Hat, 0, cx + cw - over_head, (top - 2).max(0), h);
        }
    }
    f
}

/// The cat's coat flashes white; the outline stays so the silhouette reads.
fn flash(c: &mut PxCanvas, x: i32, w: i32) {
    for y in 0..c.h.px() as i32 {
        for x in x..x + w {
            if c.get(x, y).is_some_and(|p| p != paint::OUTLINE) {
                c.set(x, y, pixel::WHITE);
            }
        }
    }
}

pub(crate) fn overlay_canvas(o: Overlay, now: u64, width: usize, look: &CatLook, h: PxHeight) -> PxCanvas {
    let f = fx_frame(o, now, width, look, h);
    let mut c = scene::draw(&f, width, look, h);
    if matches!(o, Overlay::Celebration(CelebrationKind::EvolvingYoung | CelebrationKind::EvolvingAdult)) && now % 2 == 1 {
        flash(&mut c, f.x, h.cat_max_w() as i32);
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    const H12: PxHeight = PxHeight::H12;
    const H: i32 = 12;
    use crate::px::{encode, PixelMode};
    use CelebrationKind::*;

    const KINDS: [CelebrationKind; 10] = [
        LevelUp, EvolvingYoung, EvolvingAdult, Streak, MilestoneCenturion, MilestoneVeteran, MilestoneMaster,
        MilestoneLegendary, Greeting, RandomDrop,
    ];
    const NOW: u64 = 1_800_000_000;
    const STAGES: [&str; 3] = ["kitten", "young", "adult"];

    fn look(stage: &str) -> CatLook<'_> {
        CatLook { breed: "Ginger", rarity: "common", stage, tick: 0 }
    }

    fn overlays() -> Vec<Overlay> {
        KINDS.iter().map(|&k| Overlay::Celebration(k)).chain([Overlay::Birthday]).collect()
    }

    #[test]
    fn zero_width_does_not_panic() {
        for o in overlays() {
            for now in [NOW, NOW + 3] {
                let _ = overlay_canvas(o, now, 0, &look("adult"), H12);
            }
        }
    }

    #[test]
    fn pick_orders_celebration_over_birthday() {
        assert_eq!(Overlay::pick(None, false), Option::None);
        assert_eq!(Overlay::pick(None, true), Some(Overlay::Birthday));
        assert_eq!(Overlay::pick(Streak, true), Some(Overlay::Celebration(Streak)));
    }

    #[test]
    fn rows_stay_aligned() {
        for o in overlays() {
            for w in [24usize, 60, 120] {
                for stage in STAGES {
                    for now in NOW..NOW + 12 {
                        for m in [PixelMode::Full, PixelMode::FgOnly] {
                            for row in encode(&overlay_canvas(o, now, w, &look(stage), H12), m) {
                                assert_eq!(crate::span::strip_ansi(&row).chars().count(), w, "{o:?} {w} {now}");
                                assert!(row.ends_with("\x1b[0m"));
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn every_overlay_differs_from_plain_cheer() {
        for o in overlays() {
            let plain = |now| {
                let f = SceneFrame { props: Vec::new(), dots: Vec::new(), ..fx_frame(o, now, 60, &look("adult"), H12) };
                encode(&scene::draw(&f, 60, &look("adult"), H12), PixelMode::Full)
            };
            let differs = |stage| (NOW..NOW + 6).any(|n| encode(&overlay_canvas(o, n, 60, &look(stage), H12), PixelMode::Full) != plain(n));
            assert!(differs("adult"), "{o:?}");
        }
    }

    #[test]
    fn props_never_clip_and_stay_clear_of_the_cat() {
        for o in overlays() {
            for w in [24usize, 60, 120] {
                for stage in STAGES {
                    for now in NOW..NOW + 12 {
                        let f = fx_frame(o, now, w, &look(stage), H12);
                        assert_eq!(f.x, (w as i32 - 16) / 2, "cat is centred");
                        for p in &f.props {
                            let s = art::prop(p.prop, p.frame, H12);
                            let (sw, sh) = (s.w as i32, s.h as i32);
                            assert!(p.x >= 0 && p.y >= 0 && p.x + sw <= w as i32 && p.y + sh <= H, "{o:?} {p:?}");
                            let on_cat = p.x < f.x + 16 && p.x + sw > f.x;
                            // Only the hat and the heart sit on the cat; confetti falls behind her.
                            if on_cat && !matches!(p.prop, Prop::Hat | Prop::Heart | Prop::Confetti) {
                                panic!("{o:?} {p:?} overlaps the cat");
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn hat_sits_on_the_head_for_every_stage() {
        for stage in STAGES {
            let f = fx_frame(Overlay::Birthday, NOW, 60, &look(stage), H12);
            let hat = f.props.iter().find(|p| p.prop == Prop::Hat).expect("a hat");
            let cat = paint::paint(Pose::Cheer, &look(stage), true, art::Tail::Up, false, H12);
            assert_eq!((hat.x, hat.y), (f.x + cat.w as i32 - 7, (H - cat.h as i32 - 2).max(0)), "{stage}");
            assert!(hat.y + 4 <= H);
        }
    }

    #[test]
    fn confetti_rises_with_the_tier() {
        let count = |k| fx_frame(Overlay::Celebration(k), NOW, 60, &look("adult"), H12).props.iter().filter(|p| p.prop == Prop::Confetti).count();
        let tiers = [MilestoneCenturion, MilestoneVeteran, MilestoneMaster, MilestoneLegendary].map(count);
        assert_eq!(tiers, [8, 12, 16, 24]);
        assert_eq!(count(Streak), 0);
        let bday = fx_frame(Overlay::Birthday, NOW, 60, &look("adult"), H12).props.iter().filter(|p| p.prop == Prop::Confetti).count();
        assert_eq!(bday, 10);
    }

    #[test]
    fn confetti_falls_two_px_a_second_and_wraps() {
        let ys = |now| {
            fx_frame(Overlay::Celebration(MilestoneVeteran), now, 60, &look("adult"), H12).props.iter().map(|p| p.y).collect::<Vec<_>>()
        };
        let base = NOW - NOW % CYCLE; // start of a block
        for k in 0..5u64 {
            let (a, b) = (ys(base + k), ys(base + k + 1));
            assert!(a.iter().zip(&b).all(|(a, b)| *b == (a + 2) % H), "k={k}");
        }
        assert_eq!(ys(base), ys(base + CYCLE - 1).iter().map(|y| (y + 2) % H).collect::<Vec<_>>());
    }

    #[test]
    fn evolving_flashes_white_on_odd_seconds() {
        let o = Overlay::Celebration(EvolvingAdult);
        let coat = |c: &PxCanvas| {
            (0..60).flat_map(|x| (0..H).map(move |y| (x, y))).filter_map(|(x, y)| c.get(x, y)).filter(|p| *p != paint::OUTLINE && *p != pixel::WHITE).count()
        };
        let (even, odd) = (overlay_canvas(o, NOW - NOW % 2, 60, &look("adult"), H12), overlay_canvas(o, NOW - NOW % 2 + 1, 60, &look("adult"), H12));
        assert!(coat(&even) > 40);
        assert_eq!(coat(&odd), 0);
    }

    #[test]
    fn coin_drops_then_glints() {
        let base = NOW - NOW % CYCLE;
        let coin_y = |k| fx_frame(Overlay::Celebration(RandomDrop), base + k, 60, &look("adult"), H12).props[0].y;
        assert_eq!([coin_y(0), coin_y(1), coin_y(2), coin_y(3), coin_y(4)], [0, 4, 8, 8, 8]);
    }

    #[test]
    fn greeting_alternates_wave_and_cheer() {
        let pose = |now| fx_frame(Overlay::Celebration(Greeting), now, 60, &look("adult"), H12).pose;
        assert_eq!((pose(NOW - NOW % 2), pose(NOW - NOW % 2 + 1)), (Pose::Cheer, Pose::Wave));
    }
}
