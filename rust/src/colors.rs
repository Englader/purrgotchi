pub const C_CYAN: &str = "\x1b[38;2;10;189;198m";
pub const C_PINK: &str = "\x1b[38;2;234;0;217m";
pub const C_MAGENTA: &str = "\x1b[38;2;113;28;145m";
pub const C_YELLOW: &str = "\x1b[38;2;243;243;27m";
pub const C_GREEN: &str = "\x1b[38;2;87;236;166m";
pub const C_RED: &str = "\x1b[38;2;255;84;104m";
pub const C_DIM: &str = "\x1b[2m";
pub const C_BOLD: &str = "\x1b[1m";
pub const C_RESET: &str = "\x1b[0m";

#[allow(dead_code)]
pub const C_PURPLE: &str = "\x1b[38;2;170;100;255m";
#[allow(dead_code)]
pub const C_GOLD:   &str = "\x1b[38;2;255;200;60m";
#[allow(dead_code)]
pub const C_BLUE:   &str = "\x1b[38;2;80;150;255m";

#[allow(dead_code)]
pub const RAINBOW_HUES: [&str; 6] = [
    C_RED,
    C_YELLOW,
    C_GREEN,
    C_CYAN,
    C_BLUE,
    C_MAGENTA,
];

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
}
