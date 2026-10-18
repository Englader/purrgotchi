//! Styled text spans: the single representation behind the classic ANSI
//! statusline and the JSON surface (spec §3.1).

use serde::{Deserialize, Serialize};

use crate::colors::{Attr, Rgb, C_RESET};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    /// Ordered. The ANSI printer emits them in this order — byte-identity with
    /// the pre-v4 format strings depends on it (shiny = BOLD then colour,
    /// `LV:` = colour then BOLD).
    pub attrs: Vec<Attr>,
}

impl Span {
    pub fn new(text: impl Into<String>, attrs: &[Attr]) -> Self {
        Span { text: text.into(), attrs: attrs.to_vec() }
    }
    pub fn width(&self) -> usize {
        self.text.chars().count()
    }
}

pub fn plain(t: impl Into<String>) -> Span {
    Span::new(t, &[])
}
pub fn dim(t: impl Into<String>) -> Span {
    Span::new(t, &[Attr::Dim])
}
pub fn bold(t: impl Into<String>) -> Span {
    Span::new(t, &[Attr::Bold])
}
pub fn fg(t: impl Into<String>, c: Rgb) -> Span {
    Span::new(t, &[Attr::Fg(c)])
}

/// Display width (chars) — the repo-wide plain-width convention.
pub fn width(spans: &[Span]) -> usize {
    spans.iter().map(Span::width).sum()
}

/// Terminal cells for one char, for the speech compartment only (everything
/// else keeps the chars-count convention above, which the goldens pin).
/// 0: combining marks, zero-width joiners/spaces, variation selectors, emoji
/// skin-tone modifiers and tag chars. 2: East Asian Wide/Fullwidth, which
/// since Unicode 9 includes every default-emoji-presentation char. 1 otherwise.
/// A compact wcwidth: ZWJ sequences sum their parts (over-estimates where the
/// terminal fuses them, which can only narrow the line, never wrap it), and a
/// text-default symbol made emoji by VS16 (`❤️`) stays 1, as in wcwidth.
pub fn cell_width(c: char) -> usize {
    const ZERO: &[(u32, u32)] = &[
        (0x0300, 0x036F), (0x0483, 0x0489), (0x0591, 0x05BD), (0x0610, 0x061A),
        (0x064B, 0x065F), (0x0E31, 0x0E31), (0x0E34, 0x0E3A), (0x0E47, 0x0E4E),
        (0x1AB0, 0x1AFF), (0x1DC0, 0x1DFF), (0x200B, 0x200D), (0x20D0, 0x20FF),
        (0xFE00, 0xFE0F), (0xFE20, 0xFE2F), (0x1F3FB, 0x1F3FF), (0xE0000, 0xE01EF),
    ];
    const WIDE: &[(u32, u32)] = &[
        (0x1100, 0x115F), (0x231A, 0x231B), (0x2329, 0x232A), (0x23E9, 0x23EC),
        (0x23F0, 0x23F0), (0x23F3, 0x23F3), (0x25FD, 0x25FE), (0x2614, 0x2615),
        (0x2648, 0x2653), (0x267F, 0x267F), (0x2693, 0x2693), (0x26A1, 0x26A1),
        (0x26AA, 0x26AB), (0x26BD, 0x26BE), (0x26C4, 0x26C5), (0x26CE, 0x26CE),
        (0x26D4, 0x26D4), (0x26EA, 0x26EA), (0x26F2, 0x26F3), (0x26F5, 0x26F5),
        (0x26FA, 0x26FA), (0x26FD, 0x26FD), (0x2705, 0x2705), (0x270A, 0x270B),
        (0x2728, 0x2728), (0x274C, 0x274C), (0x274E, 0x274E), (0x2753, 0x2755),
        (0x2757, 0x2757), (0x2795, 0x2797), (0x27B0, 0x27B0), (0x27BF, 0x27BF),
        (0x2B1B, 0x2B1C), (0x2B50, 0x2B50), (0x2B55, 0x2B55), (0x2E80, 0x303E),
        (0x3041, 0x33FF), (0x3400, 0x4DBF), (0x4E00, 0x9FFF), (0xA000, 0xA4CF),
        (0xA960, 0xA97F), (0xAC00, 0xD7A3), (0xF900, 0xFAFF), (0xFE10, 0xFE19),
        (0xFE30, 0xFE6F), (0xFF00, 0xFF60), (0xFFE0, 0xFFE6), (0x16FE0, 0x16FE4),
        (0x17000, 0x18AFF), (0x1B000, 0x1B2FF), (0x1F004, 0x1F004), (0x1F0CF, 0x1F0CF),
        (0x1F18E, 0x1F18E), (0x1F191, 0x1F19A), (0x1F200, 0x1F251), (0x1F260, 0x1F265),
        (0x1F300, 0x1F64F), (0x1F680, 0x1F6FF), (0x1F7E0, 0x1F7EB), (0x1F90C, 0x1F9FF),
        (0x1FA70, 0x1FAFF), (0x20000, 0x3FFFD),
    ];
    let cp = c as u32;
    let hit = |t: &[(u32, u32)]| t.iter().any(|&(lo, hi)| (lo..=hi).contains(&cp));
    if cp < 0x0300 {
        1
    } else if hit(ZERO) {
        0
    } else if hit(WIDE) {
        2
    } else {
        1
    }
}

pub fn to_plain(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

pub fn attrs_ansi(attrs: &[Attr]) -> String {
    let mut out = String::new();
    attrs.iter().for_each(|a| a.write_ansi(&mut out));
    out
}

/// ANSI printer. Unstyled → raw text. Styled → attrs (in order) + text +
/// RESET, even for empty text. Never merges spans.
pub fn to_ansi(spans: &[Span]) -> String {
    let mut out = String::new();
    for s in spans {
        if s.attrs.is_empty() {
            out.push_str(&s.text);
            continue;
        }
        s.attrs.iter().for_each(|a| a.write_ansi(&mut out));
        out.push_str(&s.text);
        out.push_str(C_RESET);
    }
    out
}

/// JSON wire form (spec §3.1): `{ "text", "fg"?: "#rrggbb", "bold"?: true, "dim"?: true }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonSpan {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fg: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub dim: bool,
}

impl From<&Span> for JsonSpan {
    fn from(s: &Span) -> Self {
        JsonSpan {
            text: s.text.clone(),
            fg: s.attrs.iter().rev().find_map(|a| match a {
                Attr::Fg(c) => Some(c.hex()),
                Attr::Bold | Attr::Dim => None,
            }),
            bold: s.attrs.contains(&Attr::Bold),
            dim: s.attrs.contains(&Attr::Dim),
        }
    }
}

/// JSON printer.
pub fn to_json(spans: &[Span]) -> Vec<JsonSpan> {
    spans.iter().map(JsonSpan::from).collect()
}

/// Inverse of the ANSI printer for the SGR subset this codebase emits
/// (0 reset, 1 bold, 2 dim, 38;2;r;g;b). Lets string renderers that were not
/// migrated to spans (card, pet/play/treat prose) feed the JSON surface.
/// One `Vec<Span>` per `\n`-separated line; empty-text spans are not emitted;
/// unknown SGR params and non-SGR CSI sequences are dropped.
pub fn parse_ansi(s: &str) -> Vec<Vec<Span>> {
    fn flush(lines: &mut [Vec<Span>], text: &mut String, attrs: &[Attr]) {
        if !text.is_empty() {
            lines.last_mut().expect("at least one line").push(Span { text: std::mem::take(text), attrs: attrs.to_vec() });
        }
    }
    let mut lines: Vec<Vec<Span>> = vec![Vec::new()];
    let mut attrs: Vec<Attr> = Vec::new();
    let mut text = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' if chars.peek() == Some(&'[') => {
                chars.next();
                let mut params = String::new();
                let mut final_byte = None;
                for p in chars.by_ref() {
                    if p.is_ascii_digit() || p == ';' {
                        params.push(p);
                    } else {
                        final_byte = Some(p);
                        break;
                    }
                }
                if final_byte != Some('m') {
                    continue;
                }
                flush(&mut lines, &mut text, &attrs);
                apply_sgr(&mut attrs, &params);
            }
            '\n' => {
                flush(&mut lines, &mut text, &attrs);
                lines.push(Vec::new());
            }
            _ => text.push(c),
        }
    }
    flush(&mut lines, &mut text, &attrs);
    lines
}

fn apply_sgr(attrs: &mut Vec<Attr>, params: &str) {
    let n: Vec<u16> = params.split(';').map(|p| p.parse().unwrap_or(0)).collect();
    let mut i = 0;
    while i < n.len() {
        match n[i] {
            0 => attrs.clear(),
            1 => attrs.push(Attr::Bold),
            2 => attrs.push(Attr::Dim),
            38 if n.get(i + 1) == Some(&2) && i + 4 < n.len() => {
                attrs.retain(|a| !matches!(a, Attr::Fg(_)));
                attrs.push(Attr::Fg(Rgb(n[i + 2] as u8, n[i + 3] as u8, n[i + 4] as u8)));
                i += 4;
            }
            _ => {}
        }
        i += 1;
    }
}

/// `text` -> JSON lines for `stats`/`card`. A trailing `\n` does not produce a
/// final empty line; interior blank lines stay as `[]`.
pub fn lines_json(text: &str) -> Vec<Vec<JsonSpan>> {
    let mut lines = parse_ansi(text);
    if text.ends_with('\n') && lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.iter().map(|l| to_json(l)).collect()
}

pub fn strip_ansi(s: &str) -> String {
    parse_ansi(s).iter().map(|l| to_plain(l)).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::{Attr, CYAN, PINK};

    #[test]
    fn lines_json_drops_only_the_trailing_newline_line() {
        let l = lines_json("a\n\n\x1b[2mb\x1b[0m\n");
        assert_eq!(l.len(), 3);
        assert_eq!(l[0][0].text, "a");
        assert!(l[1].is_empty());
        assert!(l[2][0].dim);
        assert_eq!(lines_json("x").len(), 1);
    }

    #[test]
    fn unstyled_span_prints_raw() {
        assert_eq!(to_ansi(&[plain("a b")]), "a b");
    }

    #[test]
    fn styled_span_prints_attrs_text_reset_in_order() {
        let s = Span::new("x", &[Attr::Bold, Attr::Fg(CYAN)]);
        assert_eq!(to_ansi(&[s]), "\x1b[1m\x1b[38;2;10;189;198mx\x1b[0m");
        let s = Span::new("7", &[Attr::Fg(PINK), Attr::Bold]);
        assert_eq!(to_ansi(&[s]), "\x1b[38;2;234;0;217m\x1b[1m7\x1b[0m");
    }

    #[test]
    fn empty_styled_span_still_prints_prefix_and_reset() {
        assert_eq!(to_ansi(&[fg("", PINK)]), "\x1b[38;2;234;0;217m\x1b[0m");
    }

    #[test]
    fn adjacent_same_style_spans_are_not_merged() {
        assert_eq!(
            to_ansi(&[fg("╚", PINK), fg("═╝", PINK)]),
            "\x1b[38;2;234;0;217m╚\x1b[0m\x1b[38;2;234;0;217m═╝\x1b[0m"
        );
    }

    #[test]
    fn width_counts_chars_not_bytes() {
        assert_eq!(width(&[plain("═╡"), fg("★", PINK)]), 3);
        assert_eq!(to_plain(&[plain("a"), dim("b")]), "ab");
    }

    #[test]
    fn cell_width_classes() {
        for c in ['a', ' ', '…', '★', '☆', '│', '═', '♥', '❤', 'é', '\u{1F1FA}'] {
            assert_eq!(cell_width(c), 1, "{c:?}");
        }
        for c in ['\u{301}', '\u{200D}', '\u{200B}', '\u{FE0F}', '\u{1F3FD}', '\u{E0067}'] {
            assert_eq!(cell_width(c), 0, "{c:?}");
        }
        for c in ['猫', 'ね', 'カ', '한', '，', 'Ａ', '\u{1F431}', '\u{1F600}', '\u{1F680}', '\u{1F97A}',
                  '\u{1FAF6}', '\u{2B50}', '\u{26A1}', '\u{2728}', '\u{20000}'] {
            assert_eq!(cell_width(c), 2, "{c:?}");
        }
    }

    #[test]
    fn json_printer_emits_spec_shape() {
        let spans = [Span::new("x", &[Attr::Bold, Attr::Fg(CYAN)]), plain(" "), dim("y")];
        let s = serde_json::to_string(&to_json(&spans)).unwrap();
        assert_eq!(
            s,
            r##"[{"text":"x","fg":"#0abdc6","bold":true},{"text":" "},{"text":"y","dim":true}]"##
        );
        let back: Vec<JsonSpan> = serde_json::from_str(&s).unwrap();
        assert_eq!(back, to_json(&spans));
    }

    #[test]
    fn parse_ansi_inverts_the_printer() {
        let spans = vec![
            fg("╔", PINK), plain(" x "), Span::new("7", &[Attr::Fg(PINK), Attr::Bold]), dim("d"),
        ];
        let parsed = parse_ansi(&to_ansi(&spans));
        assert_eq!(parsed.len(), 1);
        assert_eq!(to_json(&parsed[0]), to_json(&spans));
    }

    #[test]
    fn parse_ansi_splits_lines_and_resets_clear_attrs() {
        let p = parse_ansi("a\x1b[1mb\x1b[0m\nc");
        assert_eq!(p, vec![vec![plain("a"), bold("b")], vec![plain("c")]]);
    }

    #[test]
    fn parse_ansi_drops_unknown_and_non_sgr_sequences() {
        let p = parse_ansi("\x1b[5mx\x1b[2Ky");
        assert_eq!(p, vec![vec![plain("xy")]]);
    }

    #[test]
    fn strip_ansi_keeps_newlines() {
        assert_eq!(strip_ansi("\x1b[38;2;1;2;3mhi\x1b[0m\nyo\n"), "hi\nyo\n");
    }
}
