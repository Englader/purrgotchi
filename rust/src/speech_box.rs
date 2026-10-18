//! Option B speech compartment: while the band is live, the HUD's right border
//! becomes a T-junction and the frame continues right into a box that holds
//! the cat's speech, word-wrapped over the HUD's 5 inner rows.

//!
//! Geometry (plan gap 3): the HUD box is 96 columns and v4.0.0 rows keep a
//! right margin of 2, so the compartment is `cw = min(50, COLUMNS − 98)`
//! columns including its right border. It shares the HUD's right border
//! column, so every composed row is `95 + cw` cells. Text is left-aligned
//! with 1 space of padding each side (`cw − 4` cells) and centred vertically.

use crate::colors::{Attr, PINK};
use crate::span::{cell_width, fg, plain, Span};

pub(crate) const MIN_CW: usize = 20;
pub(crate) const MAX_CW: usize = 50;

/// HUD box width plus the 2-column right margin.
const HUD_AND_MARGIN: usize = 98;
const INNER_ROWS: usize = 5;

/// `min(50, COLUMNS − 98)`, or `None` when COLUMNS is unknown, garbage, or
/// leaves less than `MIN_CW`. Never guesses a width.
pub(crate) fn compartment_width(columns: Option<&str>) -> Option<usize> {
    let cols = columns?.trim().parse::<usize>().ok()?;
    let cw = cols.checked_sub(HUD_AND_MARGIN)?.min(MAX_CW);
    (cw >= MIN_CW).then_some(cw)
}

fn str_cells(s: &str) -> usize {
    s.chars().map(cell_width).sum()
}

/// Splits `word` into unbreakable pieces with their cell widths: a char plus
/// any zero-width chars after it, and a ZWJ glues the next char on too.
fn clusters(word: &str) -> Vec<(&str, usize)> {
    let mut out = Vec::new();
    let (mut start, mut w, mut glue) = (0, 0, false);
    for (i, c) in word.char_indices() {
        let cw = cell_width(c);
        if i > start && cw > 0 && !glue {
            out.push((&word[start..i], w));
            (start, w) = (i, 0);
        }
        w += cw;
        glue = c == '\u{200D}';
    }
    if start < word.len() {
        out.push((&word[start..], w));
    }
    out
}

/// Greedy word wrap by terminal cells. Whitespace runs collapse to one space
/// and control chars are dropped. A word wider than `width` starts a fresh
/// line and is hard-split there (never mid-cluster, so a wide glyph never
/// straddles the edge). If the text needs more than `max_lines`, the last
/// kept line is cut so that it plus `…` fits in `width`.
pub(crate) fn wrap(text: &str, width: usize, max_lines: usize) -> Vec<String> {
    if width == 0 || max_lines == 0 {
        return Vec::new();
    }
    let mut lines: Vec<String> = Vec::new();
    let (mut cur, mut cur_w) = (String::new(), 0);
    let mut flush = |cur: &mut String, cur_w: &mut usize| {
        lines.push(std::mem::take(cur));
        *cur_w = 0;
    };
    for word in text.split_whitespace() {
        let word: String = word.chars().filter(|c| !c.is_control()).collect();
        if word.is_empty() {
            continue;
        }
        let ww = str_cells(&word);
        if !cur.is_empty() && cur_w + 1 + ww <= width {
            cur.push(' ');
            cur.push_str(&word);
            cur_w += 1 + ww;
            continue;
        }
        if !cur.is_empty() {
            flush(&mut cur, &mut cur_w);
        }
        if ww <= width {
            (cur, cur_w) = (word, ww);
            continue;
        }
        for (piece, pw) in clusters(&word) {
            // A pathological glued cluster wider than the line splits per char.
            let parts = if pw > width { clusters_per_char(piece) } else { vec![(piece, pw)] };
            for (p, w) in parts {
                if !cur.is_empty() && cur_w + w > width {
                    flush(&mut cur, &mut cur_w);
                }
                cur.push_str(p);
                cur_w += w;
            }
        }
    }
    if !cur.is_empty() {
        flush(&mut cur, &mut cur_w);
    }
    if lines.len() > max_lines {
        lines.truncate(max_lines);
        let last = lines.last_mut().expect("max_lines > 0");
        *last = with_ellipsis(last, width);
    }
    lines
}

fn clusters_per_char(s: &str) -> Vec<(&str, usize)> {
    s.char_indices().map(|(i, c)| (&s[i..i + c.len_utf8()], cell_width(c))).collect()
}

/// `line` cut at a cluster boundary so it plus `…` fits in `width`, with
/// trailing spaces dropped before the `…`.
fn with_ellipsis(line: &str, width: usize) -> String {
    let budget = width.saturating_sub(1);
    let mut out = String::new();
    let mut used = 0;
    for (piece, w) in clusters(line) {
        if used + w > budget {
            break;
        }
        out.push_str(piece);
        used += w;
    }
    let mut out = out.trim_end().to_string();
    out.push('…');
    out
}

/// The 7 row suffixes that turn the HUD into Option B, or `None` when there
/// is nothing to say (or `cw < MIN_CW`; the caller then keeps the loose line).
/// Rows 0 and 6 replace the HUD's `╗`/`╝`: the caller drops that last char
/// and appends, so they start with the `╦`/`╩` junction and are `cw` cells.
/// Rows 1–5 follow the HUD's `║` and are `cw − 1` cells.
pub(crate) fn compartment(text: &str, attrs: &[Attr], cw: usize) -> Option<[Vec<Span>; 7]> {
    if cw < MIN_CW {
        return None;
    }
    let tw = cw - 4;
    let lines = wrap(text, tw, INNER_ROWS);
    if lines.is_empty() {
        return None;
    }
    let top = (INNER_ROWS - lines.len()) / 2;
    let rail = "═".repeat(cw - 2);
    Some(std::array::from_fn(|i| match i {
        0 => vec![fg(format!("╦{rail}╗"), PINK)],
        6 => vec![fg(format!("╩{rail}╝"), PINK)],
        _ => match (i - 1).checked_sub(top).and_then(|k| lines.get(k)) {
            Some(line) => vec![
                plain(" "),
                Span::new(line.as_str(), attrs),
                plain(" ".repeat(tw - str_cells(line) + 1)),
                fg("║", PINK),
            ],
            None => vec![plain(" ".repeat(cw - 2)), fg("║", PINK)],
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::PINK;
    use crate::span::{cell_width, to_plain};

    fn cells(s: &str) -> usize {
        s.chars().map(cell_width).sum()
    }

    #[test]
    fn width_rules() {
        assert_eq!(compartment_width(Some("130")), Some(32));
        assert_eq!(compartment_width(Some("148")), Some(50));
        assert_eq!(compartment_width(Some("200")), Some(50));
        assert_eq!(compartment_width(Some("118")), Some(20));
        assert_eq!(compartment_width(Some("117")), None);
        assert_eq!(compartment_width(None), None);
        assert_eq!(compartment_width(Some("abc")), None);
        assert_eq!(compartment_width(Some("")), None);
        assert_eq!(compartment_width(Some("-130")), None);
        assert_eq!(compartment_width(Some("0")), None);
    }

    #[test]
    fn wrap_by_words() {
        let t = "you've been staring at that diff for 20 minutes. maybe the bug is you? (kidding. mostly.)";
        let lines = wrap(t, 28, 5);
        assert!(!lines.is_empty() && lines.len() <= 5, "{lines:?}");
        assert!(lines.iter().all(|l| cells(l) <= 28), "{lines:?}");
        // Nothing lost or reordered: the words come back joined by single spaces.
        assert_eq!(lines.join(" "), t);
        assert!(lines.iter().all(|l| !l.starts_with(' ') && !l.ends_with(' ')));
    }

    #[test]
    fn wrap_truncates_with_ellipsis() {
        let t = "meow purr ".repeat(40);
        assert_eq!(t.chars().count(), 400);
        let lines = wrap(&t, 16, 5);
        assert_eq!(lines.len(), 5);
        assert!(lines[4].ends_with('…'), "{lines:?}");
        assert!(!lines[4].ends_with(" …"), "{lines:?}");
        assert!(lines.iter().all(|l| cells(l) <= 16), "{lines:?}");
    }

    #[test]
    fn wrap_ellipsis_cuts_a_full_last_line() {
        // Every line is exactly full, so the last one must lose cells for the `…`.
        let lines = wrap("abcd efgh ijkl mnop", 4, 2);
        assert_eq!(lines, vec!["abcd", "efg…"]);
    }

    #[test]
    fn wrap_cjk_counts_double() {
        let lines = wrap("猫猫猫猫猫", 6, 5);
        assert_eq!(lines, vec!["猫猫猫", "猫猫"]);
        // A wide glyph never straddles the edge: at width 5 a line holds 2.
        assert_eq!(wrap("猫猫猫猫猫", 5, 5), vec!["猫猫", "猫猫", "猫"]);
    }

    #[test]
    fn wrap_hard_splits_long_words() {
        let lines = wrap("see https://example.com/a/very/long/path ok", 10, 5);
        assert_eq!(lines, vec!["see", "https://ex", "ample.com/", "a/very/lon", "g/path ok"]);
    }

    #[test]
    fn wrap_keeps_zero_width_marks_on_their_base() {
        // "e" + combining acute must not be split across lines.
        let lines = wrap("abce\u{301}fg", 4, 5);
        assert_eq!(lines, vec!["abce\u{301}", "fg"]);
        // A ZWJ emoji sequence (2+2 cells) moves as a whole.
        let fam = "\u{1F468}\u{200D}\u{1F469}";
        assert_eq!(wrap(&format!("ab{fam}"), 5, 5), vec!["ab".to_string(), fam.to_string()]);
    }

    #[test]
    fn wrap_degenerate_inputs() {
        assert!(wrap("", 10, 5).is_empty());
        assert!(wrap("   \t\n ", 10, 5).is_empty());
        assert!(wrap("hello", 0, 5).is_empty());
        assert!(wrap("hello", 10, 0).is_empty());
        assert_eq!(wrap("a\tb\nc", 10, 5), vec!["a b c"]);
        assert_eq!(wrap("x\u{7}y", 10, 5), vec!["xy"]);
    }

    #[test]
    fn compartment_widths() {
        let quip = "you've been staring at that diff for 20 minutes. maybe the bug is you? (kidding. mostly.)";
        for cw in [MIN_CW, 32, MAX_CW] {
            for text in [quip, "hi", "猫猫猫 cats \u{1F431}\u{1F431} everywhere", &"z".repeat(400)] {
                let rows = compartment(text, &[Attr::Fg(PINK)], cw).expect("text present");
                for (i, r) in rows.iter().enumerate() {
                    let want = if i == 0 || i == 6 { cw } else { cw - 1 };
                    assert_eq!(cells(&to_plain(r)), want, "cw={cw} row={i} {:?}", to_plain(r));
                }
            }
        }
    }

    #[test]
    fn compartment_borders() {
        let rows = compartment("hi", &[], 20).unwrap();
        assert_eq!(to_plain(&rows[0]), format!("╦{}╗", "═".repeat(18)));
        assert_eq!(to_plain(&rows[6]), format!("╩{}╝", "═".repeat(18)));
        for r in &rows[1..6] {
            assert!(to_plain(r).ends_with('║'));
        }
        for r in &rows {
            assert!(r.iter().filter(|s| s.text.contains(['╦', '╗', '║', '╩', '╝', '═'])).all(|s| s.attrs == vec![Attr::Fg(PINK)]));
        }
    }

    #[test]
    fn compartment_centres_text() {
        let rows = compartment("hello there", &[Attr::Dim], 32).unwrap();
        let plain: Vec<String> = rows.iter().map(|r| to_plain(r)).collect();
        assert_eq!(plain[3], format!(" hello there{} ║", " ".repeat(28 - 11)));
        for i in [1, 2, 4, 5] {
            assert_eq!(plain[i], format!("{}║", " ".repeat(30)), "row {i}");
        }
        // The text span carries the speech attrs; padding stays unstyled.
        assert!(rows[3].iter().any(|s| s.text == "hello there" && s.attrs == vec![Attr::Dim]));
        assert!(rows[3].iter().filter(|s| s.text.trim().is_empty()).all(|s| s.attrs.is_empty()));
        // Two lines: top = (5 - 2) / 2 = 1, so inner rows 2 and 3.
        let two = compartment(&"word ".repeat(6), &[], 20).unwrap();
        let p: Vec<String> = two.iter().map(|r| to_plain(r)).collect();
        assert!(p[1].trim_end_matches('║').trim().is_empty());
        assert!(p[2].contains("word") && p[3].contains("word"));
        assert!(p[4].trim_end_matches('║').trim().is_empty());
    }

    #[test]
    fn empty_text_has_no_compartment() {
        assert!(compartment("", &[], 32).is_none());
        assert!(compartment("   ", &[], 32).is_none());
        // Below the minimum the caller falls back; never draw a broken box.
        assert!(compartment("hi", &[], MIN_CW - 1).is_none());
    }
}
