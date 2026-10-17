//! Breed table and palette for the pixel cat (`px/`): coat colours, markings
//! and the rarity colours. A port of the look of the prototype
//! `docs/superpowers/specs/assets/2026-10-17-sprites/sprite.mjs`; `hash` uses
//! exact i32 wrapping math. The classic ASCII sprites (`sprites.rs`) are separate.

use crate::colors::{self, Rgb};

// ── palette: colors.rs plus the prototype's art constants ──────────────────

const fn rgb(hex: u32) -> Rgb {
    Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub(crate) const WHITE: Rgb = rgb(0xffffff);
const BLACK: Rgb = rgb(0x000000);
pub(crate) const NEON_CYAN: Rgb = colors::CYAN; // #0abdc6, rim light left
pub(crate) const NEON_MAGENTA: Rgb = colors::PINK; // #ea00d9, rim light right
pub(crate) const NOSE: Rgb = rgb(0xff7aa8);
pub(crate) const INNER_EAR: Rgb = rgb(0xf39bb8);
pub(crate) const VOID_INNER_EAR: Rgb = rgb(0x3a2a5e);
pub(crate) const LID_ON_DARK: Rgb = rgb(0xc9c3dd);
const RARE: Rgb = rgb(0xb45cff);
pub(crate) const EPIC: Rgb = rgb(0xffc93c);
const RAINBOW: [Rgb; 7] = [colors::RED, EPIC, colors::YELLOW, colors::GREEN, colors::CYAN, RARE, colors::PINK];

// ── tables ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pattern {
    Tabby,
    Tuxedo,
    Calico,
    Solid,
    Points,
    Spots,
    Van,
    Wrinkles,
    Void,
    Rosettes,
    Split,
}

pub(crate) struct Breed {
    pub(crate) name: &'static str,
    pub(crate) base: Rgb,
    pub(crate) mark: Rgb,
    pub(crate) mark2: Option<Rgb>,
    pub(crate) light: Option<Rgb>,
    pub(crate) pattern: Pattern,
    pub(crate) bushy: bool,
    pub(crate) hetero: bool,
}

const PLAIN: Breed = Breed {
    name: "",
    base: BLACK,
    mark: BLACK,
    mark2: None,
    light: None,
    pattern: Pattern::Solid,
    bushy: false,
    hetero: false,
};

/// The prototype's `BREEDS`, verbatim. Names match `identity::BREEDS_*`.
const BREEDS: [Breed; 16] = [
    Breed { name: "Tabby", base: rgb(0x9a7b5f), mark: rgb(0x5c4332), light: Some(rgb(0xe8dccb)), pattern: Pattern::Tabby, ..PLAIN },
    Breed { name: "Tuxedo", base: rgb(0x24212d), mark: rgb(0x24212d), light: Some(rgb(0xeeeaf2)), pattern: Pattern::Tuxedo, ..PLAIN },
    Breed { name: "Calico", base: rgb(0xf1ece4), mark: rgb(0xe5893a), mark2: Some(rgb(0x2a2530)), pattern: Pattern::Calico, ..PLAIN },
    Breed { name: "Ginger", base: rgb(0xe8913a), mark: rgb(0xc26a22), light: Some(rgb(0xf7d9b5)), pattern: Pattern::Tabby, ..PLAIN },
    Breed { name: "Grey Shorthair", base: rgb(0x8d94a5), mark: rgb(0x8d94a5), light: Some(rgb(0xc9ced9)), pattern: Pattern::Solid, ..PLAIN },
    Breed { name: "Siamese", base: rgb(0xefe2cc), mark: rgb(0x4a3328), pattern: Pattern::Points, ..PLAIN },
    Breed { name: "Russian Blue", base: rgb(0x6e8199), mark: rgb(0x6e8199), pattern: Pattern::Solid, ..PLAIN },
    Breed { name: "Bengal", base: rgb(0xd9a54c), mark: rgb(0x5a3a1e), light: Some(rgb(0xf3e2bd)), pattern: Pattern::Spots, ..PLAIN },
    Breed { name: "Maine Coon", base: rgb(0x8a6a4c), mark: rgb(0x4f3a29), light: Some(rgb(0xeadfcf)), pattern: Pattern::Tabby, bushy: true, ..PLAIN },
    Breed { name: "Norwegian Forest", base: rgb(0x9aa1ad), mark: rgb(0x646b78), light: Some(rgb(0xf0f0f3)), pattern: Pattern::Tabby, bushy: true, ..PLAIN },
    Breed { name: "Turkish Van", base: rgb(0xf3efe8), mark: rgb(0xb5552a), pattern: Pattern::Van, ..PLAIN },
    Breed { name: "Sphinx", base: rgb(0xdcab95), mark: rgb(0xb98270), pattern: Pattern::Wrinkles, ..PLAIN },
    Breed { name: "Scottish Fold", base: rgb(0xb9bdc9), mark: rgb(0x8c91a0), light: Some(rgb(0xe3e5ec)), pattern: Pattern::Tabby, ..PLAIN },
    Breed { name: "Void Cat", base: rgb(0x16121f), mark: rgb(0x2c2145), pattern: Pattern::Void, ..PLAIN },
    Breed { name: "Snow Leopard Cat", base: rgb(0xe2e5ec), mark: rgb(0x7b8292), pattern: Pattern::Rosettes, bushy: true, ..PLAIN },
    Breed { name: "Chimera", base: rgb(0x221d2b), mark: rgb(0xe08a3c), pattern: Pattern::Split, hetero: true, ..PLAIN },
];

const GINGER: usize = 3;

pub(crate) fn breed(name: &str) -> &'static Breed {
    BREEDS.iter().find(|b| b.name == name).unwrap_or(&BREEDS[GINGER])
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    pub(crate) fn parse(s: &str) -> Rarity {
        match s {
            "uncommon" => Rarity::Uncommon,
            "rare" => Rarity::Rare,
            "epic" => Rarity::Epic,
            "legendary" => Rarity::Legendary,
            _ => Rarity::Common,
        }
    }

    /// Eye/collar colour; legendary cycles the rainbow by `tick % 7`.
    pub(crate) fn colour(self, tick: u64) -> Rgb {
        match self {
            Rarity::Common => colors::CYAN,
            Rarity::Uncommon => colors::GREEN,
            Rarity::Rare => RARE,
            Rarity::Epic => EPIC,
            Rarity::Legendary => RAINBOW[(tick % 7) as usize],
        }
    }

}

// ── helpers ─────────────────────────────────────────────────────────────────

/// `Math.round` for the non-negative values the prototype rounds.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// FNV-style integer hash in [0, 1), exact i32/u32 wrapping math.
pub(crate) fn hash(x: i32, y: i32, s: i32) -> f64 {
    let h = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263))
        .wrapping_add(s.wrapping_mul(2147483647)) as u32;
    let h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    (h ^ (h >> 16)) as f64 / 4294967296.0
}

/// Linear blend, quantised to u8 per channel like the prototype's `toHex`.
pub(crate) fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let ch = |u: u8, v: u8| {
        let (u, v) = (u as f64, v as f64);
        js_round(u + (v - u) * t).clamp(0.0, 255.0) as u8
    };
    Rgb(ch(a.0, b.0), ch(a.1, b.1), ch(a.2, b.2))
}

pub(crate) fn lum(c: Rgb) -> f64 {
    (0.299 * c.0 as f64 + 0.587 * c.1 as f64 + 0.114 * c.2 as f64) / 255.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity;

    #[test]
    fn breed_table_covers_identity() {
        let all: Vec<&str> = identity::BREEDS_COMMON
            .iter()
            .chain(identity::BREEDS_UNCOMMON.iter())
            .chain(identity::BREEDS_RARE.iter())
            .chain(identity::BREEDS_EPIC.iter())
            .chain(identity::BREEDS_LEGENDARY.iter())
            .copied()
            .collect();
        for b in &all {
            assert!(BREEDS.iter().any(|e| e.name == *b), "no BREEDS entry for {b}");
        }
        assert_eq!(BREEDS.len(), all.len());
    }
}
