//! The 10-px cat (band 5 rows). The slot maps below started from
//! `purrgotchi pixels downscale --height 10` (slot-priority voting over
//! fractional blocks of the 12-px maps) and were then redrawn by hand: the
//! vote doubles eyes and noses where blocks overlap, so every map keeps the
//! 12-px head nearly whole (a 7-px face under notched ears) on a shorter body.
//! Same legend and `-` droppable-row rule as `art.rs`; the outline is added
//! by `paint.rs`.

use super::art::{self, Pose, ALL_POSES};
use super::PxHeight;

/// Slot priority for the downscale vote, strongest first. The first five are
/// features: they win their block outright.
const PRIORITY: &str = "enmirla*hbt";
const FEATURES: usize = 5;

/// Interior target (rows, cols) for a height.
fn target(h: PxHeight) -> (usize, usize) {
    match h {
        PxHeight::H10 => (8, 12),
        PxHeight::H12 => (10, 14),
    }
}

/// Source `[lo, hi)` of destination cell `i` of `n` over `len` source cells:
/// `[1.2 i, 1.2 (i + 1))` rounded outwards, the last block running to the end
/// (so no source row or column is lost).
fn block(i: usize, n: usize, len: usize) -> (usize, usize) {
    let lo = (i * 6 / 5).min(len - 1);
    let hi = if i + 1 == n { len } else { ((i + 1) * 6).div_ceil(5).min(len) };
    (lo, hi.max(lo + 1))
}

/// Maps with more than this many `-` rows lose the rest of the marks.
const MAX_DROPPABLE: usize = 2;

/// One pose map scaled by slot-priority voting: a feature slot (`e n m i r`)
/// wins its block if present; otherwise the block is filled when at least
/// half of it is filled, with its strongest coat slot. The tail socket `*` is
/// placed once, over the cell holding the source socket, never by the vote.
fn downscale(map: &[&str], h: PxHeight) -> Vec<String> {
    let rows: Vec<(bool, &[u8])> = map.iter().map(|r| r.strip_prefix('-').map_or((false, r.as_bytes()), |s| (true, s.as_bytes()))).collect();
    let (max_r, max_c) = target(h);
    let src_w = rows[0].1.len();
    let cols = (src_w * 5).div_ceil(6).min(max_c);
    let n_rows = (rows.len() * 5).div_ceil(6).clamp(2, max_r);
    let mut grid = vec![vec![b'.'; cols]; n_rows];
    let mut drop_row = vec![false; n_rows];
    let mut droppable = 0;
    for (j, line) in grid.iter_mut().enumerate() {
        let (y0, y1) = block(j, n_rows, rows.len());
        if rows[y0..y1].iter().any(|r| r.0) && droppable < MAX_DROPPABLE {
            droppable += 1;
            drop_row[j] = true;
        }
        for (i, out) in line.iter_mut().enumerate() {
            let (x0, x1) = block(i, cols, src_w);
            let cells: Vec<u8> = rows[y0..y1].iter().flat_map(|r| r.1[x0..x1].iter().copied()).collect();
            let has = |c: u8| cells.contains(&c);
            let filled = cells.iter().filter(|&&c| c != b'.').count();
            let pick = |slots: &str| slots.bytes().find(|&c| c != b'*' && has(c));
            let slot = pick(&PRIORITY[..FEATURES]).or_else(|| (2 * filled >= cells.len()).then(|| pick(&PRIORITY[FEATURES..])).flatten());
            *out = slot.unwrap_or(b'.');
        }
    }
    // The single socket: the cell the source socket falls in; a feature there
    // gives way to the cell behind it.
    if let Some((sy, sx)) = rows.iter().enumerate().find_map(|(y, r)| r.1.iter().position(|&c| c == b'*').map(|x| (y, x))) {
        let j = (sy * 5 / 6).min(n_rows - 1);
        let mut i = (sx * 5 / 6).min(cols - 1);
        while i > 0 && PRIORITY.as_bytes()[..FEATURES].contains(&grid[j][i]) {
            i -= 1;
        }
        grid[j][i] = b'*';
    }
    grid.into_iter().zip(drop_row).map(|(line, d)| format!("{}{}", if d { "-" } else { "" }, String::from_utf8(line).expect("ascii slots"))).collect()
}

/// Dev verb `pixels downscale --height 10`: every pose map, downscaled, as
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

/// `GroomPaw` at 10 px → `GROOM_PAW10`; `Walk1` → `WALK1_10`.
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

// ---- maps (hand-drawn) ----

/// Sitting, faces right, face toward the viewer. Four columns behind `*` hold the tail.
const SIT10: [&str; 8] = [
    "......r...r.",
    ".....rir.rir",
    ".....hehhheh",
    ".....hhlnlhh",
    "-......hhllh.",
    "-.....bbbllb.",
    "....*bbbbbbb",
    "....bbbbbama",
];

/// Walking, legs apart: front-left and back-right forward.
const WALK1_10: [&str; 8] = [
    "......r...r.",
    "t....rir.rir",
    "t....hehhheh",
    ".tbbbhhlnlhh",
    "-..bbbbbhllh.",
    "..bbbbbbbbb.",
    "..b.b...b.b.",
    ".a...a.a...a",
];

/// Walking, legs gathered under the body. Head and tail match `WALK1_10`.
const WALK2_10: [&str; 8] = [
    "......r...r.",
    "t....rir.rir",
    "t....hehhheh",
    ".tbbbhhlnlhh",
    "-..bbbbbhllh.",
    "..bbbbbbbbb.",
    "...bb...bb..",
    "...aa...aa..",
];

/// Loaf: flat oval, paws tucked away. The tail socket sits at the rear.
const LOAF10: [&str; 7] = [
    "......r...r.",
    ".....rir.rir",
    "....bhehhheh",
    "...bbhhlnlhh",
    "-...bbbbbhllh",
    "-...bbbbbbbbb",
    "....*bbbbbbb",
];

/// Loaf breathing in: the back rises one px.
const LOAF_BREATH10: [&str; 7] = [
    "......r...r.",
    ".....rir.rir",
    "...bbhehhheh",
    "...bbhhlnlhh",
    "-...bbbbbhllh",
    "-...bbbbbbbbb",
    "....*bbbbbbb",
];

/// Sleeping donut, head tucked at the right, eyes shut, tail along the front (a kitten drops the tail row).
const CURL10: [&str; 5] = [
    "..bbbb.r...r",
    ".bbbbbbrihir",
    "bbbbbbmhmhmh",
    "bbbbbbmhlnlh",
    "-.ttttttttt..",
];

/// Curl breathing in: the top of the back rises one px.
const CURL_BREATH10: [&str; 5] = [
    ".bbbbb.r...r",
    ".bbbbbbrihir",
    "bbbbbbmhmhmh",
    "bbbbbbmhlnlh",
    "-.ttttttttt..",
];

/// Sit with the paw raised to the mouth.
const GROOM_PAW10: [&str; 8] = [
    "......r...r.",
    ".....rir.rir",
    ".....hehhheh",
    ".....hhlnaah",
    "-......hhlah.",
    "-.....bbbllb.",
    "....*bbbbbbb",
    "....bbbbbam.",
];

/// Head dipped onto the chest, eyes shut, tongue out.
const GROOM_LICK10: [&str; 7] = [
    "......r...r.",
    ".....rir.rir",
    ".....hmhhhmh",
    ".....hhlnnhh",
    "-.....bbbllb.",
    "....*bbbbbbb",
    "....bbbbbama",
];

/// Downward-dog: rear up, chest low, front legs reaching along the ground.
const STRETCH10: [&str; 7] = [
    "t...........",
    "-tbbb........",
    ".bbbb.r....r",
    ".bbbbbrihhir",
    "bbbbbbhehheh",
    ".b.bbbhhlnlh",
    ".aa..baaaaaa",
];

/// Front view, between facings.
const TURN10: [&str; 8] = [
    "....r...r...",
    "...rir.rir..",
    "...hehhheh..",
    "...hhlnlhh..",
    "-....hhhhh...",
    "-...bblllbb..",
    "...bbbbbbb..",
    "....aa.aa...",
];

/// Sit with the head raised and the ears tall.
const LOOK10: [&str; 8] = [
    "......r...r.",
    "......r...r.",
    ".....rir.rir",
    ".....hehhheh",
    ".....hhlnlhh",
    "-.....bbhllh.",
    "....*bbbbbbb",
    "....bbbbbama",
];

/// `LOOK10` with the mouth open under the nose, tongue showing.
const MEOW10: [&str; 8] = [
    "......r...r.",
    "......r...r.",
    ".....rir.rir",
    ".....hehhheh",
    ".....hhlnlhh",
    "-.....bhmnmh.",
    "....*bbbbbbb",
    "....bbbbbama",
];

/// Running, legs fully extended front and back.
const RUN1_10: [&str; 7] = [
    "......r...r.",
    ".....rir.rir",
    "t....hehhheh",
    "ttbbbhhlnlhh",
    "-..bbbbbbhlh.",
    "..bbbbbbbbb.",
    "aaa......aaa",
];

/// Running, legs gathered under the body.
const RUN2_10: [&str; 7] = [
    "......r...r.",
    ".....rir.rir",
    "t....hehhheh",
    "ttbbbhhlnlhh",
    "-..bbbbbbhlh.",
    "..bbbbbbbbb.",
    "....aa..aa..",
];

/// Anticipation: low body, rear up, wide eyes (two px tall).
const CROUCH10: [&str; 7] = [
    "......r...r.",
    "..b..rir.rir",
    ".bbb.hehhheh",
    "tbbbbhelnleh",
    "-tbbbbbbbhhh.",
    ".bbbbbbbbllb",
    ".aa.....aaa.",
];

/// Mid-air: rear low-left, head high-right, paws reaching, `> <` squint.
const POUNCE10: [&str; 7] = [
    "......r...r.",
    ".....rir.rir",
    ".....hmhhhmh",
    "...bbhhmnmhh",
    "t.bbbbbbllaa",
    "-tbbbbbb...aa",
    "aabbb.......",
];

/// On its back, four paws up.
const ROLL1_10: [&str; 5] = [
    "a.a.a.a.r..r",
    "a.a.a.arihir",
    "bbbbbbbheheh",
    "blllllbhhnhh",
    "tbbbbbbbhhh.",
];

/// On its side, belly toward the viewer, paws reaching out.
const ROLL2_10: [&str; 5] = [
    ".bbbbbb.r..r",
    "bbbbbbbrihir",
    "blllllbheheh",
    "tbbbbbbhhnhh",
    ".aa.aa.aa.a.",
];

/// Sit with `^^` eyes and both paws up, one row shorter than `SIT10` so the birthday hat sits on the ears. Tail grows from `*` (caller picks `Tail::Up`).
const CHEER10: [&str; 7] = [
    ".....r...r..",
    "....rir.rir.",
    "...ahmhhhmh.",
    "...amhmnmhma",
    "-....ahhllha.",
    "...*bbbbbbb.",
    "...bbbbbama.",
];

/// Sit with `^^` eyes and one front paw waving beside the head.
const WAVE10: [&str; 8] = [
    "..aa..r...r.",
    "..aa.rir.rir",
    "...a.hmhhhmh",
    "...a.mhmnmhm",
    "-....aahhllh.",
    "-.....bbbllb.",
    "....*bbbbbbb",
    "....bbbbbama",
];

/// Interior slot map for a pose at 10 px, facing right.
pub(crate) fn pose_map10(p: Pose) -> &'static [&'static str] {
    match p {
        Pose::Sit => &SIT10,
        Pose::Walk1 => &WALK1_10,
        Pose::Walk2 => &WALK2_10,
        Pose::Loaf => &LOAF10,
        Pose::LoafBreath => &LOAF_BREATH10,
        Pose::Curl => &CURL10,
        Pose::CurlBreath => &CURL_BREATH10,
        Pose::GroomPaw => &GROOM_PAW10,
        Pose::GroomLick => &GROOM_LICK10,
        Pose::Stretch => &STRETCH10,
        Pose::Turn => &TURN10,
        Pose::Look => &LOOK10,
        Pose::Meow => &MEOW10,
        Pose::Run1 => &RUN1_10,
        Pose::Run2 => &RUN2_10,
        Pose::Crouch => &CROUCH10,
        Pose::Pounce => &POUNCE10,
        Pose::Roll1 => &ROLL1_10,
        Pose::Roll2 => &ROLL2_10,
        Pose::Cheer => &CHEER10,
        Pose::Wave => &WAVE10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::px::art::{stage_rows, ALL_TAILS, LEGEND};
    use crate::px::paint::{paint, CatLook};

    const H10: PxHeight = PxHeight::H10;

    fn count(map: &[&str], c: char) -> usize {
        map.iter().map(|r| r.matches(c).count()).sum()
    }

    #[test]
    fn h10_maps_are_rectangular_and_use_the_legend() {
        let (max_r, max_c) = target(H10);
        for p in ALL_POSES {
            let rows = stage_rows(pose_map10(p), "adult");
            let w = rows[0].len();
            for r in &rows {
                assert_eq!(r.len(), w, "{p:?}: ragged row {r:?}");
                assert!(r.chars().all(|c| LEGEND.contains(c)), "{p:?}: stray char in {r:?}");
            }
            assert!(w <= max_c && rows.len() <= max_r, "{p:?}: interior {w}x{}", rows.len());
        }
    }

    #[test]
    fn h10_maps_fit_their_height() {
        for p in ALL_POSES {
            for stage in ["kitten", "young", "adult"] {
                for t in ALL_TAILS {
                    for right in [true, false] {
                        let look = CatLook { breed: "Maine Coon", rarity: "rare", stage, tick: 0 };
                        let s = paint(p, &look, right, t, false, H10);
                        assert!(s.h <= H10.px() && s.w <= H10.cat_max_w(), "{p:?} {stage} {t:?}: {}x{}", s.w, s.h);
                    }
                }
            }
        }
    }

    /// A pose keeps its tail socket (and its eyes) at every size.
    #[test]
    fn one_tail_socket_per_h10_map() {
        for p in ALL_POSES {
            let (big, small) = (art::pose_map(p, PxHeight::H12), pose_map10(p));
            assert_eq!(count(small, '*'), count(big, '*'), "{p:?}");
            assert!(count(small, '*') <= 1, "{p:?}");
            if count(big, 'e') > 0 {
                assert!(count(small, 'e') > 0, "{p:?}: lost its eyes");
            }
        }
    }

    #[test]
    fn h10_stages_drop_rows() {
        for p in ALL_POSES {
            let m = pose_map10(p);
            assert!(stage_rows(m, "kitten").len() <= stage_rows(m, "adult").len(), "{p:?}");
            assert!(count(m, '-') <= MAX_DROPPABLE, "{p:?}");
        }
        let sit = pose_map10(Pose::Sit);
        assert!(stage_rows(sit, "kitten").len() < stage_rows(sit, "adult").len());
    }

    #[test]
    fn h10_gait_frames_differ() {
        for (a, b) in [(Pose::Walk1, Pose::Walk2), (Pose::Run1, Pose::Run2), (Pose::Loaf, Pose::LoafBreath), (Pose::Curl, Pose::CurlBreath)] {
            assert_ne!(pose_map10(a), pose_map10(b), "{a:?}");
        }
        for p in ALL_POSES {
            if p != Pose::Sit {
                assert_ne!(pose_map10(p), pose_map10(Pose::Sit), "{p:?}");
            }
        }
    }

    #[test]
    fn downscale_fits_the_targets() {
        let (max_r, max_c) = target(H10);
        for p in ALL_POSES {
            let rows = downscale(art::pose_map(p, PxHeight::H12), H10);
            assert!(rows.len() <= max_r, "{p:?}");
            assert!(rows.iter().all(|r| r.trim_start_matches('-').len() <= max_c), "{p:?}");
            assert_eq!(rows.iter().map(|r| r.matches('*').count()).sum::<usize>(), count(art::pose_map(p, PxHeight::H12), '*'), "{p:?}");
        }
        let src = downscale_source(H10);
        assert!(src.contains("const SIT10: [&str; ") && src.contains("const WALK1_10"));
    }
}
