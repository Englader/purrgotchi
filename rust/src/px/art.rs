//! Pixel cat art: pose enums, interior slot maps and the stage row-drop rule,
//! plus the small coloured props. Cat colours, outline and lighting are added
//! by `paint.rs`.

use super::{PxHeight, PxSprite};
use super::paint::OUTLINE;
use crate::colors::{self, Rgb};
use crate::pixel;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Pose {
    Sit,
    Walk1,
    Walk2,
    Loaf,
    LoafBreath,
    Curl,
    CurlBreath,
    GroomPaw,
    GroomLick,
    Stretch,
    Turn,
    Look,
    Meow,
    Run1,
    Run2,
    Crouch,
    Pounce,
    Roll1,
    Roll2,
    Cheer,
    Wave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tail {
    Curl,
    Up,
    Flick,
}

pub(crate) const ALL_POSES: [Pose; 21] = [
    Pose::Sit,
    Pose::Walk1,
    Pose::Walk2,
    Pose::Loaf,
    Pose::LoafBreath,
    Pose::Curl,
    Pose::CurlBreath,
    Pose::GroomPaw,
    Pose::GroomLick,
    Pose::Stretch,
    Pose::Turn,
    Pose::Look,
    Pose::Meow,
    Pose::Run1,
    Pose::Run2,
    Pose::Crouch,
    Pose::Pounce,
    Pose::Roll1,
    Pose::Roll2,
    Pose::Cheer,
    Pose::Wave,
];

#[cfg(test)]
pub(crate) const ALL_TAILS: [Tail; 3] = [Tail::Curl, Tail::Up, Tail::Flick];

/// Slot legend (interior only; the outline is generated):
/// `.` transparent, `h` head coat, `b` body coat, `t` tail coat, `a` paw
/// (light), `r` ear outer, `i` inner ear, `l` chest/muzzle light, `e` eye,
/// `n` nose, `m` dark line (outline colour), `*` tail socket (paints as `b`;
/// the tail layer grows from it). A leading `-` marks a droppable row.
#[cfg(test)]
pub(crate) const LEGEND: &str = ".hbtarilenm*";

/// Sitting, faces right, face turned toward the viewer. Four empty columns
/// behind `*` hold the tail layer, so every tail fits the same box.
const SIT: [&str; 10] = [
    ".......r...r..",
    "......rir.rir.",
    "......hhhhhhhh",
    "......hehhhheh",
    "......hhhlnlhh",
    "-.......hhllhh.",
    "-......bbbllb..",
    ".....bbbbbllb.",
    "....*bbbbbbbb.",
    "....bbbbbama..",
];

/// Walking, legs apart: front-left and back-right forward.
const WALK1: [&str; 10] = [
    ".......r...r..",
    "......rir.rir.",
    "t.....hhhhhhhh",
    "t.....hehhhheh",
    ".tbbbbhhhlnlhh",
    "-..bbbbbhhllhh.",
    "-..bbbbbbbllb..",
    "..bbbbbbbbbbb.",
    "..b.b....b.b..",
    ".a...a..a...a.",
];

/// Walking, legs gathered under the body. Head and tail match `WALK1`.
const WALK2: [&str; 10] = [
    ".......r...r..",
    "......rir.rir.",
    "t.....hhhhhhhh",
    "t.....hehhhheh",
    ".tbbbbhhhlnlhh",
    "-..bbbbbhhllhh.",
    "-..bbbbbbbllb..",
    "..bbbbbbbbbbb.",
    "...bb....bb...",
    "...aa....aa...",
];

/// Sit with the paw raised to the mouth (2 `a` px on the nose row).
const GROOM_PAW: [&str; 10] = [
    ".......r...r..",
    "......rir.rir.",
    "......hhhhhhhh",
    "......hehhhheh",
    "......hhhlnaah",
    "-.......hhllah.",
    "-......bbbllb..",
    ".....bbbbbllb.",
    "....*bbbbbbbb.",
    "....bbbbbam...",
];

/// Head dipped one row (ear tips gone), eyes shut, tongue out.
const GROOM_LICK: [&str; 9] = [
    "......rir.rir.",
    "......hhhhhhhh",
    "......hmhhhhmh",
    "......hhhlnnhh",
    "-.......hhllhh.",
    "-......bbbllb..",
    ".....bbbbbllb.",
    "....*bbbbbbbb.",
    "....bbbbbama..",
];

/// Loaf: flat oval, paws tucked away. The tail socket sits at the rear.
const LOAF: [&str; 9] = [
    ".......r...r..",
    "......rir.rir.",
    "......hhhhhhhh",
    "....bbhehhhheh",
    "....bbhhhlnlhh",
    "....bbbbhhllhh",
    "-....bbbbbbllbb",
    "-....bbbbbbbbbb",
    "....*bbbbbbbbb",
];

/// Loaf breathing in: the back rises one px.
const LOAF_BREATH: [&str; 9] = [
    ".......r...r..",
    "......rir.rir.",
    "......hhhhhhhh",
    "...bbbhehhhheh",
    "....bbhhhlnlhh",
    "....bbbbhhllhh",
    "-....bbbbbbllbb",
    "-....bbbbbbbbbb",
    "....*bbbbbbbbb",
];

/// Sleeping donut, head tucked at the right, eyes shut, tail along the front.
const CURL: [&str; 6] = [
    "....bbbb.r...r",
    "...bbbbbbrihir",
    "..bbbbbbmhmhmh",
    ".bbbbbbbmhlnlh",
    "-.bbbbbbbbmhhhh",
    "..tttttttttt..",
];

/// Curl breathing in: the top of the back rises one px.
const CURL_BREATH: [&str; 6] = [
    "...bbbbb.r...r",
    "...bbbbbbrihir",
    "..bbbbbbmhmhmh",
    ".bbbbbbbmhlnlh",
    "-.bbbbbbbbmhhhh",
    "..tttttttttt..",
];

/// Downward-dog: rear up, chest low, front legs reaching along the ground.
const STRETCH: [&str; 7] = [
    "t.............",
    "-tbbb..........",
    ".bbbbbb.r....r",
    ".bbbbbbbrihhir",
    "bbbbbbbbhehheh",
    ".b.bbbbbhhlnlh",
    ".aa..bbaaaaaaa",
];

/// Front view, body foreshortened to 8 wide; one tick between facings.
const TURN: [&str; 10] = [
    "....r....r....",
    "...rir..rir...",
    "...hhhhhhhh...",
    "...hehhhheh...",
    "...hhlnnlhh...",
    "-....hhhhhh....",
    "-...bbbllbbb...",
    "...bbbllbbb...",
    "...bbbbbbbb...",
    "....aa..aa....",
];

/// Sit with the head raised a row and the ears tall.
const LOOK: [&str; 10] = [
    ".......r...r..",
    ".......r...r..",
    "......rir.rir.",
    "......hhhhhhhh",
    "......hehhhheh",
    "......hhhlnlhh",
    "-.......hhllhh.",
    "-......bbbllb..",
    "....*bbbbbbbb.",
    "....bbbbbama..",
];

/// `LOOK` with the mouth open (m px either side of the nose).
const MEOW: [&str; 10] = [
    ".......r...r..",
    ".......r...r..",
    "......rir.rir.",
    "......hhhhhhhh",
    "......hehhhheh",
    "......hhhmnmhh",
    "-.......hhllhh.",
    "-......bbbllb..",
    "....*bbbbbbbb.",
    "....bbbbbama..",
];

/// Running, legs fully extended front and back. Head and body match `RUN2`.
const RUN1: [&str; 8] = [
    ".......r...r..",
    "......rir.rir.",
    "......hhhhhhhh",
    "t.....hehhhheh",
    "ttbbbbhhhlnlhh",
    "-..bbbbbhhllhh.",
    "-..bbbbbbbbbbb.",
    "aaa........aaa",
];

/// Running, legs gathered under the body.
const RUN2: [&str; 8] = [
    ".......r...r..",
    "......rir.rir.",
    "......hhhhhhhh",
    "t.....hehhhheh",
    "ttbbbbhhhlnlhh",
    "-..bbbbbhhllhh.",
    "-..bbbbbbbbbbb.",
    "....aa....aa..",
];

/// Anticipation: low body, rear up, wide eyes (two px tall).
const CROUCH: [&str; 8] = [
    ".......r...r..",
    "......rir.rir.",
    "..bb..hhhhhhhh",
    ".bbbb.hehhhheh",
    "tbbbbbhehlnleh",
    "-tbbbbbbbbhhhh.",
    "-.bbbbbbbbbllb.",
    "..aa.....aaa..",
];

/// Mid-air: rear low-left, head high-right, paws reaching, `> <` squint.
const POUNCE: [&str; 9] = [
    ".......r...r..",
    "......rir.rir.",
    "......hhhhhhhh",
    "......hmhhhhmh",
    "....bbhhmlnmhh",
    "-t.bbbbbbhhllhh",
    "-tbbbbbbbbbllaa",
    "tbbbbbbb....aa",
    "aabbbb........",
];

/// On its back, four paws up.
const ROLL1: [&str; 6] = [
    ".a.a..a.a.r..r",
    ".a.a..a.arihir",
    ".bbbbbbbbheheh",
    "bllllllllhhnhh",
    "-bbllllllbhhhhh",
    "tbbbbbbbb.....",
];

/// On its side, belly toward the viewer, paws reaching out.
const ROLL2: [&str; 6] = [
    "..bbbbbbb.r..r",
    ".bbbbbbbbrihir",
    "bllllllllheheh",
    "bbllllllbhhnhh",
    "-tbbbbbbbbhhhhh",
    ".aa.aa.aa.aa..",
];

/// Sit with `^^` eyes, one paw up on each side. Tail grows from `*` (caller
/// picks `Tail::Up`).
const CHEER: [&str; 10] = [
    ".......r...r..",
    "......rir.rir.",
    "......hhhhhhhh",
    ".....ahmhhhhmh",
    ".....amhmlnmhm",
    "-......ahhllhha",
    "-......bbbllb..",
    ".....bbbbbllb.",
    "....*bbbbbbbb.",
    "....bbbbbama..",
];

/// Sit with `^^` eyes and one front paw waving beside the head.
const WAVE: [&str; 10] = [
    "...aa..r...r..",
    "...aa.rir.rir.",
    "....a.hhhhhhhh",
    "....a.hmhhhhmh",
    "....a.mhmlnmhm",
    ".....aahhllhh.",
    "-......bbbllb..",
    "-.....bbbbbllb.",
    "....*bbbbbbbb.",
    "....bbbbbama..",
];

/// Interior slot map for a pose, facing right.
pub(crate) fn pose_map(p: Pose, h: PxHeight) -> &'static [&'static str] {
    if h == PxHeight::H10 {
        return super::art10::pose_map10(p);
    }
    match p {
        Pose::Sit => &SIT,
        Pose::Walk1 => &WALK1,
        Pose::Walk2 => &WALK2,
        Pose::Loaf => &LOAF,
        Pose::LoafBreath => &LOAF_BREATH,
        Pose::Curl => &CURL,
        Pose::CurlBreath => &CURL_BREATH,
        Pose::GroomPaw => &GROOM_PAW,
        Pose::GroomLick => &GROOM_LICK,
        Pose::Stretch => &STRETCH,
        Pose::Turn => &TURN,
        Pose::Look => &LOOK,
        Pose::Meow => &MEOW,
        Pose::Run1 => &RUN1,
        Pose::Run2 => &RUN2,
        Pose::Crouch => &CROUCH,
        Pose::Pounce => &POUNCE,
        Pose::Roll1 => &ROLL1,
        Pose::Roll2 => &ROLL2,
        Pose::Cheer => &CHEER,
        Pose::Wave => &WAVE,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Prop {
    Ball,
    Butterfly,
    Yarn,
    Coin,
    Heart,
    Hat,
    Sparkle,
    Confetti,
    Flame,
    Z,
}

pub(crate) const ALL_PROPS: [Prop; 10] = [
    Prop::Ball,
    Prop::Butterfly,
    Prop::Yarn,
    Prop::Coin,
    Prop::Heart,
    Prop::Hat,
    Prop::Sparkle,
    Prop::Confetti,
    Prop::Flame,
    Prop::Z,
];

pub(super) const ORANGE: Rgb = Rgb(255, 140, 40);

/// Rows of palette letters (`.` transparent) to a sprite. Unknown letters panic:
/// the grids are static art.
pub(super) fn grid(rows: &[&str], pal: &[(char, Rgb)]) -> PxSprite {
    let (w, h) = (rows[0].len(), rows.len());
    let px = rows
        .iter()
        .flat_map(|r| r.chars())
        .map(|ch| (ch != '.').then(|| pal.iter().find(|(k, _)| *k == ch).expect("prop palette letter").1))
        .collect();
    PxSprite { w, h, px }
}

/// A prop, already coloured. `frame` toggles the 2-frame flaps and flickers
/// (odd = second frame); for `Confetti` it picks the `colors::RAINBOW` hue.
/// No outline: props are drawn over the cat's ground and sky. Every height
/// uses these.
pub(crate) fn prop(p: Prop, frame: u8) -> PxSprite {
    let second = frame % 2 == 1;
    let (yellow, wing, pink, gold, flame, z) = (colors::YELLOW, colors::PINK, pixel::NOSE, pixel::EPIC, ORANGE, pixel::NEON_CYAN);
    match (p, second) {
        (Prop::Ball, _) => grid(&[".w.", "RRR", ".R."], &[('w', pixel::WHITE), ('R', colors::RED)]),
        (Prop::Butterfly, false) => grid(&["MM.MM", "MMdMM", ".M.M."], &[('M', wing), ('d', OUTLINE)]),
        (Prop::Butterfly, true) => grid(&[".M.", "MdM", ".M."], &[('M', wing), ('d', OUTLINE)]),
        (Prop::Yarn, _) => grid(&[".PP.", "PPdP", "PdPP", ".PP."], &[('P', pink), ('d', OUTLINE)]),
        (Prop::Coin, false) => grid(&[".GG.", "GGGG", "GGGG", ".GG."], &[('G', gold)]),
        (Prop::Coin, true) => grid(&[".wG.", "GGGG", "GGGG", ".GG."], &[('G', gold), ('w', pixel::WHITE)]),
        (Prop::Heart, _) => grid(&[".H.H.", "HHHHH", ".HHH.", "..H.."], &[('H', pink)]),
        (Prop::Hat, _) => grid(&[".Y.", ".M.", "MWM", "CCC"], &[('Y', yellow), ('M', wing), ('W', pixel::WHITE), ('C', z)]),
        (Prop::Sparkle, false) => grid(&[".W.", "WWW", ".W."], &[('W', pixel::WHITE)]),
        (Prop::Sparkle, true) => grid(&["...", ".W.", "..."], &[('W', pixel::WHITE)]),
        (Prop::Confetti, _) => PxSprite { w: 1, h: 1, px: vec![Some(colors::RAINBOW[frame as usize % colors::RAINBOW.len()])] },
        (Prop::Flame, false) => grid(&["..O", ".OO", "OOO", "OYO", "OOO"], &[('O', flame), ('Y', yellow)]),
        (Prop::Flame, true) => grid(&[".O.", "OO.", "OOO", "OYO", "OOO"], &[('O', flame), ('Y', yellow)]),
        (Prop::Z, false) => grid(&["ZZ.", ".Z.", ".ZZ"], &[('Z', z)]),
        (Prop::Z, true) => grid(&["ZZ", ".Z"], &[('Z', pixel::mix(z, OUTLINE, 0.5))]),
    }
}

/// The rows a stage shows: adult keeps every row, young drops the first
/// droppable row, kitten drops all of them (unknown stage → adult). The `-`
/// marker is stripped. The head rows are never droppable, so kittens read chibi.
pub(crate) fn stage_rows<'a>(map: &'a [&'a str], stage: &str) -> Vec<&'a str> {
    let drop = match stage {
        "kitten" => usize::MAX,
        "young" => 1,
        _ => 0,
    };
    let mut dropped = 0;
    let mut out = Vec::with_capacity(map.len());
    for row in map {
        match row.strip_prefix('-') {
            Some(_) if dropped < drop => dropped += 1,
            Some(r) => out.push(r),
            None => out.push(row),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    const H12: PxHeight = PxHeight::H12;

    const TAIL_LAYER_POSES: [Pose; 5] = [Pose::Sit, Pose::Loaf, Pose::Look, Pose::Cheer, Pose::Wave];

    #[test]
    fn maps_are_rectangular_and_use_the_legend() {
        for p in ALL_POSES {
            let rows = stage_rows(pose_map(p, H12), "adult");
            let w = rows[0].len();
            for r in &rows {
                assert_eq!(r.len(), w, "{p:?}: ragged row {r:?}");
                assert!(r.chars().all(|c| LEGEND.contains(c)), "{p:?}: stray char in {r:?}");
            }
            assert!(w <= 14 && rows.len() <= 10, "{p:?}: interior {w}x{}", rows.len());
        }
    }

    #[test]
    fn one_tail_socket_where_the_tail_layer_grows() {
        for p in TAIL_LAYER_POSES {
            let n = pose_map(p, H12).iter().map(|r| r.matches('*').count()).sum::<usize>();
            assert_eq!(n, 1, "{p:?}");
        }
    }

    #[test]
    fn props_fit_their_size_and_are_deterministic() {
        // (w, h) of the first frame; the second frame never grows.
        let size = |p| match p {
            Prop::Ball | Prop::Sparkle | Prop::Z => (3, 3),
            Prop::Butterfly => (5, 3),
            Prop::Yarn | Prop::Coin => (4, 4),
            Prop::Heart => (5, 4),
            Prop::Hat => (3, 4),
            Prop::Confetti => (1, 1),
            Prop::Flame => (3, 5),
        };
        for p in ALL_PROPS {
            for f in 0..2 {
                let s = prop(p, f);
                let (w, h) = size(p);
                assert!(s.w <= w && s.h <= h && s.px.len() == s.w * s.h, "{p:?} f{f}: {}x{}", s.w, s.h);
                assert!(s.px.iter().any(Option::is_some), "{p:?} f{f}: empty");
                assert_eq!(s.px, prop(p, f).px, "{p:?} f{f}: not deterministic");
            }
        }
    }

    #[test]
    fn prop_flaps_differ_and_confetti_follows_the_caller() {
        for p in ALL_PROPS {
            if matches!(p, Prop::Ball | Prop::Yarn | Prop::Heart | Prop::Hat | Prop::Confetti) {
                continue;
            }
            assert_ne!(prop(p, 0).px, prop(p, 1).px, "{p:?}");
        }
        let hues: std::collections::HashSet<_> = (0u8..6).map(|f| prop(Prop::Confetti, f).px[0].map(|c| c.hex())).collect();
        assert_eq!(hues.len(), 6);
    }

    #[test]
    fn stage_drop_rule() {
        let m = pose_map(Pose::Sit, H12);
        assert_eq!(stage_rows(m, "adult").len(), 10);
        assert_eq!(stage_rows(m, "young").len(), 9);
        assert_eq!(stage_rows(m, "kitten").len(), 8);
        assert_eq!(stage_rows(m, "elder"), stage_rows(m, "adult"));
        assert!(stage_rows(m, "kitten").iter().all(|r| !r.starts_with('-')));
    }

    #[test]
    fn kittens_are_smaller_than_adults() {
        for p in ALL_POSES {
            let m = pose_map(p, H12);
            assert!(stage_rows(m, "kitten").len() < stage_rows(m, "adult").len(), "{p:?}");
        }
    }

    #[test]
    fn every_pose_is_drawn() {
        for p in ALL_POSES {
            if p != Pose::Sit {
                assert_ne!(pose_map(p, H12), pose_map(Pose::Sit, H12), "{p:?}");
            }
        }
    }

    /// Frame pairs that only move legs: head, back and tail rows are shared.
    #[test]
    fn gait_frames_share_head_and_tail() {
        for (a, b, rows) in [(Pose::Walk1, Pose::Walk2, 6), (Pose::Run1, Pose::Run2, 7)] {
            let (a, b) = (pose_map(a, H12), pose_map(b, H12));
            assert_eq!(a[..rows], b[..rows]);
        }
    }
}
