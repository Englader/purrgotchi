//! Pixel cat art: pose enums, interior slot maps and the stage row-drop rule.
//! Colours, outline and lighting are added by `paint.rs`.

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

/// Interior slot map for a pose, facing right. Poses not drawn yet alias Sit
/// (Task 5 draws them).
pub(crate) fn pose_map(p: Pose) -> &'static [&'static str] {
    match p {
        Pose::Walk1 => &WALK1,
        Pose::Walk2 => &WALK2,
        _ => &SIT,
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

    const TAIL_LAYER_POSES: [Pose; 5] = [Pose::Sit, Pose::Loaf, Pose::Look, Pose::Cheer, Pose::Wave];

    #[test]
    fn maps_are_rectangular_and_use_the_legend() {
        for p in ALL_POSES {
            let rows = stage_rows(pose_map(p), "adult");
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
            let n = pose_map(p).iter().map(|r| r.matches('*').count()).sum::<usize>();
            assert_eq!(n, 1, "{p:?}");
        }
    }

    #[test]
    fn stage_drop_rule() {
        let m = pose_map(Pose::Sit);
        assert_eq!(stage_rows(m, "adult").len(), 10);
        assert_eq!(stage_rows(m, "young").len(), 9);
        assert_eq!(stage_rows(m, "kitten").len(), 8);
        assert_eq!(stage_rows(m, "elder"), stage_rows(m, "adult"));
        assert!(stage_rows(m, "kitten").iter().all(|r| !r.starts_with('-')));
    }

    #[test]
    fn undrawn_poses_alias_sit_until_task_5() {
        for p in ALL_POSES {
            let drawn = matches!(p, Pose::Sit | Pose::Walk1 | Pose::Walk2);
            assert_eq!(std::ptr::eq(pose_map(p), pose_map(Pose::Sit)), p == Pose::Sit || !drawn, "{p:?}");
        }
    }

    #[test]
    fn walk_frames_share_head_and_tail() {
        let (a, b) = (pose_map(Pose::Walk1), pose_map(Pose::Walk2));
        for y in 0..6 {
            assert_eq!(a[y], b[y], "row {y}");
        }
    }
}
