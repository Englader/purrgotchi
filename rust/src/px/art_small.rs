//! The 8-px (band 4 rows) and 6-px (band 3 rows) cat. Slot maps started
//! from `purrgotchi pixels downscale --height 8|6` (slot-priority voting over
//! the 12-px maps) and were then hand-fixed for readability. Same legend and
//! `-` droppable-row rule as `art.rs`; the outline is added by `paint.rs`.

use super::art::{self, grid, Pose, Prop, ALL_POSES, ORANGE};
use super::paint::OUTLINE;
use super::{PxHeight, PxSprite};
use crate::{colors, pixel};

/// Slot priority for the downscale vote, strongest first.
const PRIORITY: &str = "enmirla*hbt";

/// Interior target (rows, cols) for a height.
fn target(h: PxHeight) -> (usize, usize) {
    match h {
        PxHeight::H8 => (6, 9),
        PxHeight::H6 => (4, 6),
        PxHeight::H12 => (10, 14),
    }
}

/// Source `[lo, hi)` of destination cell `i` of `n`, over `len` source cells
/// (never empty).
fn block(i: usize, n: usize, len: usize) -> (usize, usize) {
    (i * len / n, ((i + 1) * len / n).max(i * len / n + 1))
}

/// One pose map scaled by slot-priority voting: a feature slot (`e n m i r`)
/// wins its block if present; otherwise the block is filled when at least
/// half of it is filled, with its strongest coat slot.
fn downscale(map: &[&str], h: PxHeight) -> Vec<String> {
    let rows: Vec<(bool, &[u8])> = map.iter().map(|r| r.strip_prefix('-').map_or((false, r.as_bytes()), |s| (true, s.as_bytes()))).collect();
    let (max_r, cols) = target(h);
    let src_w = rows[0].1.len();
    let scale = cols as f64 / src_w as f64;
    let n_rows = ((rows.len() as f64 * scale).round() as usize).clamp(2, max_r);
    let mut out = Vec::with_capacity(n_rows);
    let mut droppable = 0;
    for j in 0..n_rows {
        let (y0, y1) = block(j, n_rows, rows.len());
        let mut line = String::new();
        let drop = rows[y0..y1].iter().any(|r| r.0) && (h != PxHeight::H6 || droppable == 0);
        if drop {
            droppable += 1;
            line.push('-');
        }
        for i in 0..cols {
            let (x0, x1) = block(i, cols, src_w);
            let cells: Vec<u8> = rows[y0..y1].iter().flat_map(|r| r.1[x0..x1].iter().copied()).collect();
            let has = |c: u8| cells.contains(&c);
            let filled = cells.iter().filter(|&&c| c != b'.').count();
            let slot = PRIORITY.bytes().take(5).find(|&c| has(c)).or_else(|| {
                (2 * filled >= cells.len()).then(|| PRIORITY.bytes().skip(5).find(|&c| has(c))).flatten()
            });
            line.push(slot.map_or('.', char::from));
        }
        out.push(line);
    }
    out
}

/// Dev verb `pixels downscale --height 8|6`: every pose map, downscaled, as
/// Rust consts (the starting point of this file).
pub(crate) fn downscale_source(h: PxHeight) -> String {
    let mut s = format!("// purrgotchi pixels downscale --height {}\n", h.px());
    for p in ALL_POSES {
        let rows = downscale(art::pose_map(p, PxHeight::H12), h);
        let name = const_name(p, h);
        s.push_str(&format!("const {name}: [&str; {}] = [\n", rows.len()));
        for r in rows {
            s.push_str(&format!("    {r:?},\n"));
        }
        s.push_str("];\n");
    }
    s
}

/// `GroomPaw` at 8 px → `GROOM_PAW8`; `Walk1` → `WALK1_8`.
fn const_name(p: Pose, h: PxHeight) -> String {
    let mut out = String::new();
    for (i, ch) in format!("{p:?}").chars().enumerate() {
        if ch.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_uppercase());
    }
    if out.ends_with(|c: char| c.is_ascii_digit()) {
        out.push('_');
    }
    out + &h.px().to_string()
}

// ---- 8 px (band 4 rows) ----

/// Sitting, face toward the viewer; two columns behind `*` hold the tail.
const SIT8: [&str; 6] = [
    "...ri..ir",
    "...hehheh",
    "...hhlnlh",
    "-....bbllb",
    "..*bbbllb",
    "..bbbbama",
];

/// Walking, legs apart.
const WALK1_8: [&str; 6] = [
    "...ri..ir",
    "t..hehheh",
    "t.bhhlnlh",
    "-.bbbbbllb",
    ".b.b..b.b",
    "a..a.a..a",
];

/// Walking, legs gathered under the body. Head and tail match `WALK1_8`.
const WALK2_8: [&str; 6] = [
    "...ri..ir",
    "t..hehheh",
    "t.bhhlnlh",
    "-.bbbbbllb",
    "..bb..bb.",
    "..aa..aa.",
];

/// Loaf: flat oval, paws tucked away.
const LOAF8: [&str; 5] = [
    "...ri..ir",
    "...hehheh",
    ".bbhhlnlh",
    "-bbbbbbllb",
    "tt*bbbbbb",
];

/// Loaf breathing in: the back rises one px.
const LOAF_BREATH8: [&str; 5] = [
    "...ri..ir",
    ".bbhehheh",
    ".bbhhlnlh",
    "-bbbbbbllb",
    "tt*bbbbbb",
];

/// Sleeping donut, head tucked at the right, eyes shut.
const CURL8: [&str; 5] = [
    "..bbb.r.r",
    ".bbbbrihi",
    "-bbbbbmhmh",
    ".bbbbbhnh",
    "ttttttt..",
];

/// Curl breathing in.
const CURL_BREATH8: [&str; 5] = [
    ".bbbb.r.r",
    ".bbbbrihi",
    "-bbbbbmhmh",
    ".bbbbbhnh",
    "ttttttt..",
];

/// Sit with the paw raised to the mouth.
const GROOM_PAW8: [&str; 6] = [
    "...ri..ir",
    "...hehheh",
    "...hhlnaa",
    "-....bbla.",
    "..*bbbllb",
    "..bbbbam.",
];

/// Eyes shut, tongue out.
const GROOM_LICK8: [&str; 6] = [
    "...ri..ir",
    "...hmhhmh",
    "...hhlnnh",
    "-....bbllb",
    "..*bbbllb",
    "..bbbbama",
];

/// Downward-dog: rear up, chest low, front legs along the ground.
const STRETCH8: [&str; 6] = [
    "t........",
    "-tbb......",
    "bbbb.r..r",
    "bbbbbhehh",
    "bb.bbhhln",
    ".a..aaaaa",
];

/// Front view, between facings.
const TURN8: [&str; 6] = [
    "..ri..ir.",
    "..hehheh.",
    "..hlnnlh.",
    "-...hhhh..",
    "..bbllbb.",
    "..aa..aa.",
];

/// Sit with the head raised and the ears tall.
const LOOK8: [&str; 6] = [
    "...r....r",
    "...ri..ir",
    "...hehheh",
    "...hhlnlh",
    "-....bbllb",
    "..*bbbama",
];

/// `LOOK8` with the mouth open.
const MEOW8: [&str; 6] = [
    "...r....r",
    "...ri..ir",
    "...hehheh",
    "...hhmnmh",
    "-....bbllb",
    "..*bbbama",
];

/// Running, legs fully extended.
const RUN1_8: [&str; 5] = [
    "...ri..ir",
    "t..hehheh",
    "ttbhhlnlh",
    "-.bbbbbbbb",
    "aa.....aa",
];

/// Running, legs gathered.
const RUN2_8: [&str; 5] = [
    "...ri..ir",
    "t..hehheh",
    "ttbhhlnlh",
    "-.bbbbbbbb",
    "...aa.aa.",
];

/// Anticipation: low body, rear up, wide eyes.
const CROUCH8: [&str; 5] = [
    "...ri..ir",
    ".bbhehheh",
    "tbbhelneh",
    "-tbbbbbbll",
    ".aa...aaa",
];

/// Mid-air: rear low-left, head high-right, paws reaching.
const POUNCE8: [&str; 6] = [
    "...ri..ir",
    "...hmhhmh",
    ".bbhhlnlh",
    "-tbbbbbbaa",
    "tbbbb....",
    "aa.......",
];

/// On its back, four paws up.
const ROLL1_8: [&str; 4] = [
    "a.a.a.r.r",
    "bbbbbbheh",
    "llllllhnh",
    "-tbbbbbhhh",
];

/// On its side, belly toward the viewer, paws reaching out.
const ROLL2_8: [&str; 4] = [
    ".bbbbbr.r",
    "bllllbheh",
    "-bbllllhnh",
    ".a.a.a.a.",
];

/// Sit with `^^` eyes, paws up on each side.
const CHEER8: [&str; 6] = [
    "...ri..ir",
    "..ahmhhmh",
    "..ahhlnlh",
    "-...abllba",
    "..*bbbllb",
    "..bbbbama",
];

/// Sit with `^^` eyes and one front paw waving beside the head.
const WAVE8: [&str; 6] = [
    ".a.ri..ir",
    "..ahmhhmh",
    "..ahhlnlh",
    "-.....bllb",
    "..*bbbllb",
    "..bbbbama",
];

// ---- 6 px (band 3 rows) ----

/// Sitting, face toward the viewer.
const SIT6: [&str; 4] = [
    "..r..r",
    "..hehe",
    "..*bnh",
    ".bbbaa",
];

/// Walking, legs apart.
const WALK1_6: [&str; 4] = [
    "..r..r",
    "t.hehe",
    "tbbbnh",
    "a.aa.a",
];

/// Walking, legs gathered.
const WALK2_6: [&str; 4] = [
    "..r..r",
    "t.hehe",
    "tbbbnh",
    ".a..a.",
];

/// Loaf.
const LOAF6: [&str; 4] = [
    "..r..r",
    "..hehe",
    "bbbbnh",
    "t*bbbb",
];

/// Loaf breathing in.
const LOAF_BREATH6: [&str; 4] = [
    "..r..r",
    ".bhehe",
    "bbbbnh",
    "t*bbbb",
];

/// Sleeping donut.
const CURL6: [&str; 3] = [
    ".bbr.r",
    "bbbmhm",
    "tttthn",
];

/// Curl breathing in.
const CURL_BREATH6: [&str; 3] = [
    "bbbr.r",
    "bbbmhm",
    "tttthn",
];

/// Paw raised to the mouth.
const GROOM_PAW6: [&str; 4] = [
    "..r..r",
    "..heha",
    "..*bna",
    ".bbba.",
];

/// Eyes shut, tongue out.
const GROOM_LICK6: [&str; 4] = [
    "..r..r",
    "..hmhm",
    "..*bnn",
    ".bbbaa",
];

/// Downward-dog.
const STRETCH6: [&str; 4] = [
    "t.....",
    ".bbr.r",
    "bbbehe",
    "a.aaaa",
];

/// Front view.
const TURN6: [&str; 4] = [
    ".r..r.",
    ".ehhe.",
    ".bnnb.",
    ".a..a.",
];

/// Head raised, ears tall.
const LOOK6: [&str; 4] = [
    "..r..r",
    "..r..r",
    "..hehe",
    "..*baa",
];

/// `LOOK6` with the mouth open.
const MEOW6: [&str; 4] = [
    "..r..r",
    "..r..r",
    "..hehe",
    "..*bmn",
];

/// Running, legs extended.
const RUN1_6: [&str; 4] = [
    "..r..r",
    "t.hehe",
    "tbbbbn",
    "a....a",
];

/// Running, legs gathered.
const RUN2_6: [&str; 4] = [
    "..r..r",
    "t.hehe",
    "tbbbbn",
    "..aa..",
];

/// Anticipation, low.
const CROUCH6: [&str; 3] = [
    "..r..r",
    "bbhehe",
    "aabbna",
];

/// Mid-air.
const POUNCE6: [&str; 4] = [
    "...r.r",
    "..bmhm",
    "tbbbna",
    "a.....",
];

/// On its back, paws up.
const ROLL1_6: [&str; 3] = [
    "a.a.rr",
    "bbbbhe",
    "tllhnh",
];

/// On its side, belly out.
const ROLL2_6: [&str; 3] = [
    ".bbbrr",
    "bllleh",
    "a.a.a.",
];

/// Paws up; open eyes (`^^` is unreadable at 6 px).
const CHEER6: [&str; 4] = [
    ".ar..r",
    ".ahehe",
    "..*bna",
    ".bbbaa",
];

/// One paw waving; open eyes.
const WAVE6: [&str; 4] = [
    "a.r..r",
    ".ahehe",
    "..*bnh",
    ".bbbaa",
];

/// Interior slot map for a pose at 8 or 6 px, facing right.
pub(crate) fn pose_map_small(p: Pose, h: PxHeight) -> &'static [&'static str] {
    match (p, h) {
        (Pose::Sit, PxHeight::H8) => &SIT8,
        (Pose::Walk1, PxHeight::H8) => &WALK1_8,
        (Pose::Walk2, PxHeight::H8) => &WALK2_8,
        (Pose::Loaf, PxHeight::H8) => &LOAF8,
        (Pose::LoafBreath, PxHeight::H8) => &LOAF_BREATH8,
        (Pose::Curl, PxHeight::H8) => &CURL8,
        (Pose::CurlBreath, PxHeight::H8) => &CURL_BREATH8,
        (Pose::GroomPaw, PxHeight::H8) => &GROOM_PAW8,
        (Pose::GroomLick, PxHeight::H8) => &GROOM_LICK8,
        (Pose::Stretch, PxHeight::H8) => &STRETCH8,
        (Pose::Turn, PxHeight::H8) => &TURN8,
        (Pose::Look, PxHeight::H8) => &LOOK8,
        (Pose::Meow, PxHeight::H8) => &MEOW8,
        (Pose::Run1, PxHeight::H8) => &RUN1_8,
        (Pose::Run2, PxHeight::H8) => &RUN2_8,
        (Pose::Crouch, PxHeight::H8) => &CROUCH8,
        (Pose::Pounce, PxHeight::H8) => &POUNCE8,
        (Pose::Roll1, PxHeight::H8) => &ROLL1_8,
        (Pose::Roll2, PxHeight::H8) => &ROLL2_8,
        (Pose::Cheer, PxHeight::H8) => &CHEER8,
        (Pose::Wave, PxHeight::H8) => &WAVE8,
        (Pose::Sit, _) => &SIT6,
        (Pose::Walk1, _) => &WALK1_6,
        (Pose::Walk2, _) => &WALK2_6,
        (Pose::Loaf, _) => &LOAF6,
        (Pose::LoafBreath, _) => &LOAF_BREATH6,
        (Pose::Curl, _) => &CURL6,
        (Pose::CurlBreath, _) => &CURL_BREATH6,
        (Pose::GroomPaw, _) => &GROOM_PAW6,
        (Pose::GroomLick, _) => &GROOM_LICK6,
        (Pose::Stretch, _) => &STRETCH6,
        (Pose::Turn, _) => &TURN6,
        (Pose::Look, _) => &LOOK6,
        (Pose::Meow, _) => &MEOW6,
        (Pose::Run1, _) => &RUN1_6,
        (Pose::Run2, _) => &RUN2_6,
        (Pose::Crouch, _) => &CROUCH6,
        (Pose::Pounce, _) => &POUNCE6,
        (Pose::Roll1, _) => &ROLL1_6,
        (Pose::Roll2, _) => &ROLL2_6,
        (Pose::Cheer, _) => &CHEER6,
        (Pose::Wave, _) => &WAVE6,
    }
}

/// A 6-px prop (at most 3 px tall). Yarn, coin, heart and flame get 3×3
/// variants, the hat 3×2 and the Z 2×2; the rest keep their 12-px art.
pub(crate) fn prop_small(p: Prop, frame: u8) -> PxSprite {
    let second = frame % 2 == 1;
    let (pink, gold) = (pixel::NOSE, pixel::EPIC);
    match (p, second) {
        (Prop::Yarn, _) => grid(&[".PP", "PdP", "PP."], &[('P', pink), ('d', OUTLINE)]),
        (Prop::Coin, false) => grid(&[".G.", "GGG", ".G."], &[('G', gold)]),
        (Prop::Coin, true) => grid(&[".w.", "GGG", ".G."], &[('G', gold), ('w', pixel::WHITE)]),
        (Prop::Heart, _) => grid(&["H.H", "HHH", ".H."], &[('H', pink)]),
        (Prop::Hat, _) => grid(&[".Y.", "MWM"], &[('Y', colors::YELLOW), ('M', colors::PINK), ('W', pixel::WHITE)]),
        (Prop::Flame, false) => grid(&["..O", "OYO", "OOO"], &[('O', ORANGE), ('Y', colors::YELLOW)]),
        (Prop::Flame, true) => grid(&["O..", "OYO", "OOO"], &[('O', ORANGE), ('Y', colors::YELLOW)]),
        (Prop::Z, false) => grid(&["ZZ", ".Z"], &[('Z', pixel::NEON_CYAN)]),
        (Prop::Z, true) => grid(&["ZZ", ".Z"], &[('Z', pixel::mix(pixel::NEON_CYAN, OUTLINE, 0.5))]),
        _ => art::prop(p, frame, PxHeight::H8),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::px::art::{stage_rows, ALL_PROPS, ALL_TAILS, LEGEND};
    use crate::px::paint::{paint, CatLook};

    const SMALL: [PxHeight; 2] = [PxHeight::H8, PxHeight::H6];

    fn count(map: &[&str], c: char) -> usize {
        map.iter().map(|r| r.matches(c).count()).sum()
    }

    #[test]
    fn small_maps_are_rectangular_and_use_the_legend() {
        for h in SMALL {
            let (max_r, max_c) = target(h);
            for p in ALL_POSES {
                let rows = stage_rows(pose_map_small(p, h), "adult");
                let w = rows[0].len();
                for r in &rows {
                    assert_eq!(r.len(), w, "{h:?} {p:?}: ragged row {r:?}");
                    assert!(r.chars().all(|c| LEGEND.contains(c)), "{h:?} {p:?}: stray char in {r:?}");
                }
                assert!(w <= max_c && rows.len() <= max_r, "{h:?} {p:?}: interior {w}x{}", rows.len());
            }
        }
    }

    #[test]
    fn small_maps_fit_their_height() {
        for h in SMALL {
            for p in ALL_POSES {
                for stage in ["kitten", "young", "adult"] {
                    for t in ALL_TAILS {
                        for right in [true, false] {
                            let look = CatLook { breed: "Maine Coon", rarity: "rare", stage, tick: 0 };
                            let s = paint(p, &look, right, t, false, h);
                            assert!(s.h <= h.px() && s.w <= h.cat_max_w(), "{h:?} {p:?} {stage} {t:?}: {}x{}", s.w, s.h);
                        }
                    }
                }
            }
        }
    }

    /// A pose keeps its tail socket (and its eyes or eye lines) at every size.
    #[test]
    fn one_tail_socket_per_small_map() {
        for h in SMALL {
            for p in ALL_POSES {
                let (big, small) = (art::pose_map(p, PxHeight::H12), pose_map_small(p, h));
                assert_eq!(count(small, '*'), count(big, '*'), "{h:?} {p:?}");
                assert!(count(small, '*') <= 1, "{h:?} {p:?}");
                if count(big, 'e') > 0 {
                    assert!(count(small, 'e') > 0, "{h:?} {p:?}: lost its eyes");
                }
            }
        }
    }

    #[test]
    fn small_stages_drop_rows() {
        for p in ALL_POSES {
            let m8 = pose_map_small(p, PxHeight::H8);
            assert!(stage_rows(m8, "kitten").len() <= stage_rows(m8, "adult").len(), "{p:?}");
            assert!(count(pose_map_small(p, PxHeight::H6), '-') <= 1, "{p:?}");
        }
        let sit = pose_map_small(Pose::Sit, PxHeight::H8);
        assert!(stage_rows(sit, "kitten").len() < stage_rows(sit, "adult").len());
    }

    #[test]
    fn small_gait_frames_differ() {
        for h in SMALL {
            for (a, b) in [(Pose::Walk1, Pose::Walk2), (Pose::Run1, Pose::Run2), (Pose::Loaf, Pose::LoafBreath), (Pose::Curl, Pose::CurlBreath)] {
                assert_ne!(pose_map_small(a, h), pose_map_small(b, h), "{h:?} {a:?}");
            }
            for p in ALL_POSES {
                if p != Pose::Sit {
                    assert_ne!(pose_map_small(p, h), pose_map_small(Pose::Sit, h), "{h:?} {p:?}");
                }
            }
        }
    }

    #[test]
    fn small_props_fit_h6() {
        for p in ALL_PROPS {
            for f in 0..2 {
                let s = art::prop(p, f, PxHeight::H6);
                assert!(s.h <= 3 && s.px.len() == s.w * s.h, "{p:?} f{f}: {}x{}", s.w, s.h);
                assert!(s.px.iter().any(Option::is_some), "{p:?} f{f}: empty");
                // 8 px keeps the 12-px props.
                assert_eq!(art::prop(p, f, PxHeight::H8).px, art::prop(p, f, PxHeight::H12).px, "{p:?}");
            }
        }
        let size = |p| (art::prop(p, 0, PxHeight::H6).w, art::prop(p, 0, PxHeight::H6).h);
        assert_eq!(size(Prop::Hat), (3, 2));
        assert_eq!(size(Prop::Z), (2, 2));
        for p in [Prop::Yarn, Prop::Coin, Prop::Heart, Prop::Flame] {
            assert_eq!(size(p), (3, 3), "{p:?}");
        }
    }

    #[test]
    fn downscale_fits_the_targets() {
        for h in SMALL {
            let (max_r, max_c) = target(h);
            for p in ALL_POSES {
                let rows = downscale(art::pose_map(p, PxHeight::H12), h);
                assert!(rows.len() <= max_r, "{h:?} {p:?}");
                assert!(rows.iter().all(|r| r.trim_start_matches('-').len() == max_c), "{h:?} {p:?}");
            }
            let src = downscale_source(h);
            assert!(src.contains(&format!("const SIT{}: [&str; ", h.px())) && src.contains(&format!("const WALK1_{}", h.px())));
        }
    }
}
