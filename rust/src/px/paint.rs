//! Slot map → coloured `PxSprite`. Pipeline order is load-bearing:
//! slots → colour, breed pattern, tail layer (before the pattern), flip,
//! cyan/magenta rim, auto-outline, blink. Coats are flat (Gate B option D).

use super::{PxHeight, PxSprite};
use super::art::{self, Pose, Tail};
use crate::colors::Rgb;
use crate::pixel::{self, Breed, Pattern, Rarity};

pub(crate) struct CatLook<'a> {
    pub breed: &'a str,
    pub rarity: &'a str,
    pub stage: &'a str,
    pub tick: u64,
}

/// Outline and dark-line colour (Gate B option D). Documented fallback if it
/// is too loud next to the HUD box: `#4a3475`.
pub(crate) const OUTLINE: Rgb = Rgb(0x5a, 0x3f, 0x8a);

/// Rim light pixels, flat on every coat (blending turned light and orange coats
/// olive). These are the cyan / magenta neons at 45% over the Tuxedo coat.
const RIM_CYAN: Rgb = Rgb(0x18, 0x67, 0x72);
const RIM_MAGENTA: Rgb = Rgb(0x7d, 0x12, 0x7a);

/// Tail layer offsets from `*`, facing right (−x is behind the cat).
fn tail_offsets(t: Tail) -> &'static [(i32, i32)] {
    match t {
        Tail::Curl => &[(-1, 0), (-2, 1), (-3, 1), (-4, 0)],
        Tail::Up => &[(-1, 0), (-2, -1), (-2, -2), (-2, -3), (-1, -4)],
        Tail::Flick => &[(-1, 0), (-2, -1), (-2, -2), (-2, -3), (-3, -4)],
    }
}

/// Slots that take coat colour, lighting and rim.
fn is_coat(s: u8) -> bool {
    matches!(s, b'h' | b'b' | b't' | b'a' | b'r' | b'l' | b'*')
}

/// Slot grid (map coords, facing right) with the tail layer grown from `*`.
fn slot_grid(p: Pose, stage: &str, tail: Tail, bushy: bool, h: PxHeight) -> (usize, usize, Vec<u8>) {
    let rows = art::stage_rows(art::pose_map(p, h), stage);
    let (w, h) = (rows[0].len(), rows.len());
    let mut g: Vec<u8> = rows.iter().flat_map(|r| r.bytes()).collect();
    if let Some(at) = g.iter().position(|&c| c == b'*') {
        let (sx, sy) = ((at % w) as i32, (at / w) as i32);
        let mut put = |x: i32, y: i32| {
            if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && g[y as usize * w + x as usize] == b'.' {
                g[y as usize * w + x as usize] = b't';
            }
        };
        for &(dx, dy) in tail_offsets(tail) {
            put(sx + dx, sy + dy);
            if bushy {
                match tail {
                    Tail::Curl => put(sx + dx, sy + dy - 1),
                    Tail::Up | Tail::Flick => put(sx + dx - 1, sy + dy),
                }
            }
        }
    }
    (w, h, g)
}

/// Step 1 + 2: slot colour, then the breed pattern (map coordinates).
/// Map-space context for one slot: the hetero (rightmost) eye column, the top
/// head row, and whether the row is a head row (`l` there is muzzle, not chest).
#[derive(Clone, Copy)]
struct Ctx {
    w: usize,
    eye_x: Option<i32>,
    top_head: Option<i32>,
}

fn slot_colour(b: &Breed, look: &CatLook, ctx: &Ctx, s: u8, x: i32, y: i32, muzzle: bool) -> Option<Rgb> {
    let Ctx { w, eye_x, top_head } = *ctx;
    let light = b.light.unwrap_or_else(|| pixel::mix(b.base, pixel::WHITE, 0.25));
    let mut c = match s {
        b'.' => return None,
        b'h' | b'b' | b't' | b'r' | b'*' => b.base,
        // Void keeps a plain muzzle: the nose pixel and eyes carry its face.
        b'l' if b.pattern == Pattern::Void => b.base,
        b'l' | b'a' => light,
        b'i' if b.pattern == Pattern::Void => return Some(pixel::VOID_INNER_EAR),
        b'i' => return Some(pixel::INNER_EAR),
        b'e' => {
            let eye = Rarity::parse(look.rarity).colour(look.tick);
            return Some(if b.hetero && eye_x == Some(x) { pixel::NEON_MAGENTA } else { eye });
        }
        b'n' => return Some(pixel::NOSE),
        _ => return Some(OUTLINE), // `m`
    };
    let body = matches!(s, b'b' | b't' | b'*');
    let fur = body || matches!(s, b'h' | b'r');
    let top = s == b'h' && top_head == Some(y);
    match b.pattern {
        Pattern::Tabby => {
            // body stripes only: head stripes turn to noise at 12 px
            if body && (x + y) % 3 == 0 {
                c = b.mark;
            }
        }
        Pattern::Points => {
            if matches!(s, b'r' | b't' | b'a') {
                c = b.mark;
            } else if s == b'l' && muzzle {
                c = pixel::mix(b.base, b.mark, 0.6);
            }
        }
        Pattern::Spots | Pattern::Rosettes => {
            if body && pixel::hash(x, y, 3) > 0.78 {
                c = b.mark;
            }
        }
        Pattern::Van => {
            // head cap only (ears + top head row)
            if s == b'r' || top {
                c = b.mark;
            }
        }
        Pattern::Calico => {
            let v = pixel::hash(x / 3, y / 3, 7);
            if fur && v > 0.85 {
                c = b.mark2.unwrap_or(b.mark);
            } else if fur && v > 0.6 {
                c = b.mark;
            }
        }
        Pattern::Split => {
            if (x as usize) >= w / 2 {
                c = if fur { b.mark } else { pixel::mix(b.mark, pixel::WHITE, 0.25) };
            }
        }
        Pattern::Tuxedo | Pattern::Solid | Pattern::Wrinkles | Pattern::Void => {}
    }
    Some(c)
}

pub(crate) fn paint(p: Pose, look: &CatLook, facing_right: bool, tail: Tail, blink: bool, ph: PxHeight) -> PxSprite {
    let b = pixel::breed(look.breed);
    let (w, h, mut slots) = slot_grid(p, look.stage, tail, b.bushy, ph);
    let at = |x: usize, y: usize| y * w + x;

    // 1–2. colour + pattern, in map coordinates.
    let ctx = Ctx {
        w,
        eye_x: slots.iter().enumerate().filter(|(_, c)| **c == b'e').map(|(i, _)| (i % w) as i32).max(),
        top_head: slots.iter().position(|&c| c == b'h').map(|i| (i / w) as i32),
    };
    let head_row = |y: usize| slots[y * w..(y + 1) * w].contains(&b'h');
    let mut col: Vec<Option<Rgb>> = (0..w * h)
        .map(|i| slot_colour(b, look, &ctx, slots[i], (i % w) as i32, (i / w) as i32, head_row(i / w)))
        .collect();

    // 3. flip
    if !facing_right {
        for y in 0..h {
            slots[y * w..(y + 1) * w].reverse();
            col[y * w..(y + 1) * w].reverse();
        }
    }

    // 4. rim light: cyan leftmost, magenta rightmost coat px per row.
    let mut out = col;
    for y in 0..h {
        let coat: Vec<usize> = (0..w).filter(|&x| is_coat(slots[at(x, y)])).collect();
        if coat.len() >= 3 {
            let (l, r) = (at(coat[0], y), at(coat[coat.len() - 1], y));
            out[l] = Some(RIM_CYAN);
            out[r] = Some(RIM_MAGENTA);
        }
    }

    // 5. auto-outline on a 1 px border.
    let (sw, sh) = (w + 2, h + 2);
    let mut px: Vec<Option<Rgb>> = vec![None; sw * sh];
    for y in 0..h {
        for x in 0..w {
            px[(y + 1) * sw + x + 1] = out[at(x, y)];
        }
    }
    let filled = px.iter().map(Option::is_some).collect::<Vec<_>>();
    let f = |x: i32, y: i32| x >= 0 && y >= 0 && (x as usize) < sw && (y as usize) < sh && filled[y as usize * sw + x as usize];
    for y in 0..sh as i32 {
        for x in 0..sw as i32 {
            if !f(x, y) && (f(x - 1, y) || f(x + 1, y) || f(x, y - 1) || f(x, y + 1)) {
                px[y as usize * sw + x as usize] = Some(OUTLINE);
            }
        }
    }

    // 6. blink: lids in the outline colour, or light lids on a dark coat.
    if blink {
        for y in 0..h {
            for x in 0..w {
                if slots[at(x, y)] != b'e' {
                    continue;
                }
                let under = [x.wrapping_sub(1), x + 1]
                    .into_iter()
                    .filter(|&n| n < w && is_coat(slots[at(n, y)]))
                    .find_map(|n| out[at(n, y)])
                    .unwrap_or(b.base);
                let lid = if pixel::lum(under) < 0.3 { pixel::LID_ON_DARK } else { OUTLINE };
                px[(y + 1) * sw + x + 1] = Some(lid);
            }
        }
    }

    PxSprite { w: sw, h: sh, px }
}

#[cfg(test)]
mod tests {
    use super::*;
    const H12: PxHeight = PxHeight::H12;
    use crate::px::art::ALL_POSES;

    fn look(b: &'static str, st: &'static str) -> CatLook<'static> {
        CatLook { breed: b, rarity: "common", stage: st, tick: 0 }
    }

    #[test]
    fn sizes_and_ground() {
        for p in ALL_POSES {
            for st in ["kitten", "young", "adult"] {
                for f in [true, false] {
                    for t in crate::px::art::ALL_TAILS {
                        let s = paint(p, &look("Ginger", st), f, t, false, H12);
                        assert!(s.w <= H12.cat_max_w() && s.h <= 12, "{p:?} {st}");
                        assert!((0..s.w).any(|x| s.px[(s.h - 1) * s.w + x].is_some()), "{p:?} {st}: nothing on the ground row");
                    }
                }
            }
        }
    }

    #[test]
    fn breath_is_subtle() {
        let px = |p| paint(p, &look("Ginger", "adult"), true, Tail::Curl, false, H12).px;
        for (a, b) in [(Pose::Loaf, Pose::LoafBreath), (Pose::Curl, Pose::CurlBreath)] {
            let d = px(a).iter().zip(px(b).iter()).filter(|(x, y)| x != y).count();
            assert!(d > 0 && d <= 6, "{a:?}: {d}");
        }
    }

    #[test]
    fn stages_scale() {
        let h = |st| paint(Pose::Sit, &look("Ginger", st), true, Tail::Curl, false, H12).h;
        assert!(h("kitten") < h("young") && h("young") < h("adult"));
    }

    #[test]
    fn flip_mirrors_silhouette() {
        let r = paint(Pose::Walk1, &look("Tabby", "adult"), true, Tail::Curl, false, H12);
        let l = paint(Pose::Walk1, &look("Tabby", "adult"), false, Tail::Curl, false, H12);
        for y in 0..r.h {
            for x in 0..r.w {
                assert_eq!(r.px[y * r.w + x].is_some(), l.px[y * l.w + (l.w - 1 - x)].is_some());
            }
        }
    }

    #[test]
    fn rim_signature() {
        let s = paint(Pose::Sit, &look("Ginger", "adult"), true, Tail::Curl, false, H12);
        let row = &s.px[4 * s.w..5 * s.w];
        let coat: Vec<_> = row.iter().flatten().filter(|c| **c != OUTLINE).collect();
        let (l, r) = (coat[0], coat[coat.len() - 1]);
        assert!(l.1 > l.0, "left not cyan {l:?}");
        assert!(r.0 > r.1, "right not magenta {r:?}");
    }

    #[test]
    fn rim_stays_cyan_left_when_facing_left() {
        let s = paint(Pose::Sit, &look("Ginger", "adult"), false, Tail::Curl, false, H12);
        let row = &s.px[4 * s.w..5 * s.w];
        let coat: Vec<_> = row.iter().flatten().filter(|c| **c != OUTLINE).collect();
        let (l, r) = (coat[0], coat[coat.len() - 1]);
        assert!(l.1 > l.0 && r.0 > r.1, "{l:?} {r:?}");
    }

    #[test]
    fn blink_hides_eyes() {
        let eye = crate::colors::CYAN;
        assert!(paint(Pose::Sit, &look("Tabby", "adult"), true, Tail::Curl, false, H12).px.contains(&Some(eye)));
        assert!(!paint(Pose::Sit, &look("Tabby", "adult"), true, Tail::Curl, true, H12).px.contains(&Some(eye)));
    }

    #[test]
    fn unknown_or_empty_breed_is_ginger() {
        let g = paint(Pose::Sit, &look("Ginger", "adult"), true, Tail::Up, false, H12).px;
        assert_eq!(paint(Pose::Sit, &look("", "adult"), true, Tail::Up, false, H12).px, g);
        assert_eq!(paint(Pose::Sit, &look("Dragon", "elder"), true, Tail::Up, false, H12).px, g);
    }

    #[test]
    fn tails_differ_and_hetero_eyes() {
        let at = |t| paint(Pose::Sit, &look("Ginger", "adult"), true, t, false, H12).px;
        assert_ne!(at(Tail::Curl), at(Tail::Up));
        assert_ne!(at(Tail::Up), at(Tail::Flick));
        let c = paint(Pose::Sit, &CatLook { rarity: "legendary", ..look("Chimera", "adult") }, true, Tail::Curl, false, H12);
        assert!(c.px.contains(&Some(crate::pixel::NEON_MAGENTA)));
    }

    #[test]
    fn legendary_eyes_cycle_with_tick() {
        let at = |tick| paint(Pose::Sit, &CatLook { rarity: "legendary", tick, ..look("Void Cat", "adult") }, true, Tail::Curl, false, H12).px;
        assert_ne!(at(0), at(1));
        assert_eq!(at(0), at(7));
    }

    fn all_breeds() -> Vec<&'static str> {
        use crate::identity::*;
        [&BREEDS_COMMON[..], &BREEDS_UNCOMMON[..], &BREEDS_RARE[..], &BREEDS_EPIC[..], &BREEDS_LEGENDARY[..]].concat()
    }

    fn canonical_rarity(breed: &str) -> &'static str {
        ["common", "uncommon", "rare", "epic", "legendary"]
            .into_iter()
            .find(|r| crate::identity::breeds_for_rarity(r).contains(&breed))
            .unwrap_or("common")
    }

    fn serialise(s: &PxSprite) -> String {
        let mut out = format!("{}x{}:", s.w, s.h);
        for p in &s.px {
            match p {
                Some(c) => out.push_str(&c.hex()),
                None => out.push('-'),
            }
        }
        out
    }

    fn pxcat_lines() -> Vec<String> {
        use crate::px::art::ALL_TAILS;
        let mut lines = Vec::new();
        let mut push = |p: Pose, st: &str, f: bool, t: Tail, blink: bool, breed: &str, tick: u64| {
            let lk = CatLook { breed, rarity: canonical_rarity(breed), stage: st, tick };
            let face = if f { "right" } else { "left" };
            let hash = crate::identity::fnv1a(&serialise(&paint(p, &lk, f, t, blink, H12)));
            lines.push(format!("{p:?}|{st}|{face}|{t:?}|{blink}|{breed}|{tick} {hash:016x}"));
        };
        for p in ALL_POSES {
            for st in ["kitten", "young", "adult"] {
                for f in [true, false] {
                    push(p, st, f, Tail::Curl, false, "Ginger", 0);
                    if p == Pose::Sit {
                        for t in ALL_TAILS {
                            if t != Tail::Curl {
                                push(p, st, f, t, false, "Ginger", 0);
                            }
                            push(p, st, f, t, true, "Ginger", 0);
                        }
                    }
                }
            }
        }
        for b in all_breeds() {
            push(Pose::Sit, "adult", true, Tail::Curl, false, b, 0);
        }
        for b in crate::identity::BREEDS_LEGENDARY {
            for tick in 0..7 {
                push(Pose::Sit, "adult", true, Tail::Curl, false, b, tick);
            }
        }
        lines
    }

    /// Art snapshot. Re-record with `UPDATE_GOLDEN=1 cargo test pxcat`.
    #[test]
    fn pxcat_snapshot() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/pxcat_hashes.txt");
        let lines = pxcat_lines();
        assert_eq!(lines.len(), 21 * 3 * 2 + 3 * 2 * 5 + 16 + 3 * 7);
        if std::env::var("UPDATE_GOLDEN").as_deref() == Ok("1") {
            std::fs::write(path, lines.join("\n") + "\n").unwrap();
            return;
        }
        let golden = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e} (record with UPDATE_GOLDEN=1)"));
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
            "pxcat snapshot drift ({} golden lines, {} rendered); first mismatches:\n{}",
            want.len(),
            lines.len(),
            mismatches.join("\n")
        );
    }
}
