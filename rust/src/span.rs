//! Styled text spans: the single representation behind the classic ANSI
//! statusline and the mod's JSON (spec §3.1).

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

// Consumed by the --json printers (Task 4+); test-only until then.
#[allow(dead_code)]
pub fn to_plain(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

pub fn attrs_ansi(attrs: &[Attr]) -> String {
    attrs.iter().map(|a| a.ansi()).collect()
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
        out.push_str(&attrs_ansi(&s.attrs));
        out.push_str(&s.text);
        out.push_str(C_RESET);
    }
    out
}

/// JSON wire form (spec §3.1): `{ "text", "fg"?: "#rrggbb", "bold"?: true, "dim"?: true }`.
/// Consumed by the --json printers (Task 4+); test-only until then.
#[allow(dead_code)]
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
#[allow(dead_code)]
pub fn to_json(spans: &[Span]) -> Vec<JsonSpan> {
    spans.iter().map(JsonSpan::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::{Attr, CYAN, PINK};

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
}
