pub const C_CYAN: &str = "\x1b[38;2;10;189;198m";
pub const C_PINK: &str = "\x1b[38;2;234;0;217m";
pub const C_MAGENTA: &str = "\x1b[38;2;113;28;145m";
pub const C_YELLOW: &str = "\x1b[38;2;243;243;27m";
pub const C_GREEN: &str = "\x1b[38;2;87;236;166m";
pub const C_RED: &str = "\x1b[38;2;255;84;104m";
pub const C_DIM: &str = "\x1b[2m";
pub const C_BOLD: &str = "\x1b[1m";
pub const C_RESET: &str = "\x1b[0m";

pub const C_PURPLE: &str = "\x1b[38;2;170;100;255m";
pub const C_GOLD:   &str = "\x1b[38;2;255;200;60m";
pub const C_BLUE:   &str = "\x1b[38;2;80;150;255m";

pub const RAINBOW_HUES: [&str; 6] = [
    C_RED,
    C_YELLOW,
    C_GREEN,
    C_CYAN,
    C_BLUE,
    C_MAGENTA,
];

/// 24-bit colour. `ansi()` must equal the matching `C_*` constant (tested).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    // Consumed by the --json printers (Task 4+); test-only until then.
    #[allow(dead_code)]
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
    pub fn ansi(self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.0, self.1, self.2)
    }
}

pub const CYAN: Rgb = Rgb(10, 189, 198);
pub const PINK: Rgb = Rgb(234, 0, 217);
pub const MAGENTA: Rgb = Rgb(113, 28, 145);
pub const YELLOW: Rgb = Rgb(243, 243, 27);
pub const GREEN: Rgb = Rgb(87, 236, 166);
pub const RED: Rgb = Rgb(255, 84, 104);
pub const PURPLE: Rgb = Rgb(170, 100, 255);
pub const GOLD: Rgb = Rgb(255, 200, 60);
pub const BLUE: Rgb = Rgb(80, 150, 255);
pub const RAINBOW: [Rgb; 6] = [RED, YELLOW, GREEN, CYAN, BLUE, MAGENTA];

/// One SGR attribute. Order inside a span is significant (see span.rs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attr {
    Fg(Rgb),
    Bold,
    Dim,
}

impl Attr {
    pub fn ansi(self) -> String {
        match self {
            Attr::Fg(c) => c.ansi(),
            Attr::Bold => C_BOLD.to_string(),
            Attr::Dim => C_DIM.to_string(),
        }
    }
}

pub fn rarity_rgb(rarity: &str, now: i64) -> Rgb {
    match rarity {
        "common" => CYAN,
        "uncommon" => GREEN,
        "rare" => PURPLE,
        "epic" => GOLD,
        "legendary" => RAINBOW[now.rem_euclid(6) as usize],
        _ => CYAN,
    }
}

/// Span-attribute twin of [`rarity_color`]; prints byte-identically (tested).
pub fn rarity_attrs(rarity: &str, shiny: bool, now: i64) -> Vec<Attr> {
    let base = Attr::Fg(rarity_rgb(rarity, now));
    if shiny {
        let shimmer = if now.rem_euclid(2) == 0 { Attr::Bold } else { Attr::Dim };
        vec![shimmer, base]
    } else {
        vec![base]
    }
}

/// Return an ANSI prefix for the kitten's name / sprite based on rarity.
///
/// Legendary cycles through `RAINBOW_HUES` by `now % 6`.
/// When `shiny`, the prefix is preceded by `C_BOLD` (even `now`) or `C_DIM` (odd `now`).
/// Unknown rarity falls back to `C_CYAN`.
pub fn rarity_color(rarity: &str, shiny: bool, now: i64) -> String {
    let base = match rarity {
        "common"    => C_CYAN,
        "uncommon"  => C_GREEN,
        "rare"      => C_PURPLE,
        "epic"      => C_GOLD,
        "legendary" => RAINBOW_HUES[now.rem_euclid(6) as usize],
        _           => C_CYAN,
    };
    if shiny {
        let shimmer = if now.rem_euclid(2) == 0 { C_BOLD } else { C_DIM };
        format!("{shimmer}{base}")
    } else {
        base.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_palette_constants_are_truecolor_escapes() {
        for c in [C_PURPLE, C_GOLD, C_BLUE] {
            assert!(c.starts_with("\x1b[38;2;"), "expected truecolor escape, got {c:?}");
            assert!(c.ends_with('m'), "expected ANSI escape terminator, got {c:?}");
        }
    }

    #[test]
    fn rainbow_hues_has_six_distinct_entries() {
        assert_eq!(RAINBOW_HUES.len(), 6);
        let mut seen = std::collections::HashSet::new();
        for c in RAINBOW_HUES {
            assert!(seen.insert(c), "duplicate hue in RAINBOW_HUES: {c:?}");
        }
    }

    #[test]
    fn rarity_color_common_is_cyan() {
        let s = rarity_color("common", false, 0);
        assert_eq!(s, C_CYAN.to_string());
    }

    #[test]
    fn rarity_color_uncommon_is_green() {
        let s = rarity_color("uncommon", false, 0);
        assert_eq!(s, C_GREEN.to_string());
    }

    #[test]
    fn rarity_color_rare_is_purple() {
        let s = rarity_color("rare", false, 0);
        assert_eq!(s, C_PURPLE.to_string());
    }

    #[test]
    fn rarity_color_epic_is_gold() {
        let s = rarity_color("epic", false, 0);
        assert_eq!(s, C_GOLD.to_string());
    }

    #[test]
    fn rarity_color_legendary_cycles_through_all_hues() {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for now in 0..6 {
            seen.insert(rarity_color("legendary", false, now));
        }
        assert_eq!(seen.len(), 6, "expected 6 distinct hues across now=0..6");
        for hue in RAINBOW_HUES {
            let hue_str = hue.to_string();
            assert!(seen.contains(&hue_str), "missing hue {hue:?}");
        }
    }

    #[test]
    fn rarity_color_legendary_is_deterministic_per_now() {
        for now in [-7_i64, 0, 1, 6, 7, 42, 1_000_000] {
            let a = rarity_color("legendary", false, now);
            let b = rarity_color("legendary", false, now);
            assert_eq!(a, b, "legendary color must be deterministic for now={now}");
        }
    }

    #[test]
    fn rarity_color_unknown_falls_back_to_cyan() {
        assert_eq!(rarity_color("", false, 0), C_CYAN.to_string());
        assert_eq!(rarity_color("garbage", false, 0), C_CYAN.to_string());
    }

    #[test]
    fn rarity_color_shiny_adds_bold_on_even_now() {
        let s = rarity_color("common", true, 0);
        assert!(s.starts_with(C_BOLD), "expected shiny to prefix bold on even now, got {s:?}");
        assert!(s.contains(C_CYAN), "expected shiny to still contain base hue");
    }

    #[test]
    fn rarity_color_shiny_adds_dim_on_odd_now() {
        let s = rarity_color("common", true, 1);
        assert!(s.starts_with(C_DIM), "expected shiny to prefix dim on odd now, got {s:?}");
        assert!(s.contains(C_CYAN), "expected shiny to still contain base hue");
    }

    #[test]
    fn rarity_color_shiny_legendary_combines_shimmer_and_cycle() {
        let s = rarity_color("legendary", true, 2);
        // now=2 is even → bold, RAINBOW_HUES[2] is C_GREEN
        assert!(s.starts_with(C_BOLD));
        assert!(s.contains(C_GREEN));
    }

    #[test]
    fn rgb_ansi_matches_legacy_constants() {
        for (rgb, c) in [
            (CYAN, C_CYAN), (PINK, C_PINK), (MAGENTA, C_MAGENTA), (YELLOW, C_YELLOW),
            (GREEN, C_GREEN), (RED, C_RED), (PURPLE, C_PURPLE), (GOLD, C_GOLD), (BLUE, C_BLUE),
        ] {
            assert_eq!(rgb.ansi(), c);
            assert_eq!(Attr::Fg(rgb).ansi(), c);
        }
        assert_eq!(Attr::Bold.ansi(), C_BOLD);
        assert_eq!(Attr::Dim.ansi(), C_DIM);
        assert_eq!(CYAN.hex(), "#0abdc6");
        assert_eq!(PINK.hex(), "#ea00d9");
    }

    #[test]
    fn rarity_attrs_print_exactly_like_rarity_color() {
        for r in ["common", "uncommon", "rare", "epic", "legendary", "", "garbage"] {
            for shiny in [false, true] {
                for now in -7i64..14 {
                    let printed: String = rarity_attrs(r, shiny, now).into_iter().map(Attr::ansi).collect();
                    assert_eq!(printed, rarity_color(r, shiny, now), "{r} shiny={shiny} now={now}");
                }
            }
        }
    }
}
