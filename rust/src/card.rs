use crate::identity;
use crate::state::State;

// Total number of ═ chars between ╔ and ╗ (and the inner content width).
// With ║ on each side, total line width = CARD_WIDTH + 2 = 44 chars.
const CARD_WIDTH: usize = 42;

pub fn run() {
    let s = match crate::state::load() {
        Some(s) => s,
        None => {
            eprintln!("No kitten hatched yet. Run: purrgotchi hatch");
            std::process::exit(1);
        }
    };
    print!("{}", render(&s));
}

pub fn render(s: &State) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let inner = CARD_WIDTH; // chars between ║ and ║

    let stars = identity::rarity_stars(&s.rarity, s.shiny);
    let breed_upper = s.breed.to_uppercase();

    // Top border
    writeln!(out, "╔{}╗", "═".repeat(CARD_WIDTH)).unwrap();

    // Title: BREED // Name // STARS
    let title = format!("{} // {} // {}", breed_upper, s.name, stars);
    writeln!(out, "║ {:<width$}║", title, width = inner - 1).unwrap();

    // Shiny / born row
    let born = format_born(s.birth_time);
    if s.shiny {
        writeln!(out, "║ {:<width$}║", format!("[SHINY]   Born: {born}"), width = inner - 1).unwrap();
    } else {
        writeln!(out, "║ {:<width$}║", format!("Born: {born}"), width = inner - 1).unwrap();
    }

    // Divider
    writeln!(out, "╠{}╣", "═".repeat(CARD_WIDTH)).unwrap();

    // Sprite (6 lines, each padded to fit inside the box)
    // Sprite line max width = inner - 2 (two leading spaces + content + trailing pad)
    let sprite_inner = inner - 2;
    let sprite = get_card_sprite(&s.rarity);
    for line in sprite.iter().take(6) {
        writeln!(out, "║  {:<width$}║", line, width = sprite_inner).unwrap();
    }

    // Divider
    writeln!(out, "╠{}╣", "═".repeat(CARD_WIDTH)).unwrap();

    // Stats — no peak/dump suffix per spec
    // Layout: ║  <name:11><bar:12> <val:3>          ║
    // Fixed content width = 2 + 11 + 12 + 1 + 3 = 29
    // Padding to inner (42): 42 - 29 = 13 trailing spaces
    for (i, &val) in s.stats.iter().enumerate() {
        let bar = stat_bar(val);
        // Build content string, then pad to inner width
        let content = format!("  {:<11}{} {:>3}", identity::STAT_NAMES[i], bar, val);
        writeln!(out, "║{:<width$}║", content, width = inner).unwrap();
    }

    // Divider
    writeln!(out, "╠{}╣", "═".repeat(CARD_WIDTH)).unwrap();

    // Personality (word-wrapped), each line indented by 1 space
    // Wrap width = inner - 1 (1 leading space + content + trailing pad to reach ║)
    let wrap_width = inner - 2; // 1 leading + content fits in inner-1, leave 1 for safety
    for line in wrap(&s.personality, wrap_width) {
        writeln!(out, "║ {:<width$}║", line, width = inner - 1).unwrap();
    }

    // Bottom border
    write!(out, "╚{}╝", "═".repeat(CARD_WIDTH)).unwrap();
    out
}

fn stat_bar(val: u8) -> String {
    let filled = (val as usize * 10 + 50) / 100;
    let empty = 10 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current.clone());
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn format_born(secs: u64) -> String {
    // Rough YYYY-MM-DD approximation without chrono dependency
    let days_since_epoch = secs / 86400;
    let year = 1970 + days_since_epoch / 365;
    let day_of_year = days_since_epoch % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;
    format!("{year}-{month:02}-{day:02}")
}

fn get_card_sprite(rarity: &str) -> Vec<&'static str> {
    match rarity {
        "legendary" => vec![
            " /########\\",
            "(# O    O #)",
            " #  ---   #",
            " ##########",
            " ##      ##",
            "_##_    _##_",
        ],
        "epic" => vec![
            "/\\        /\\",
            "(  .    .  )",
            " \\  ___  /",
            "  \\_____/",
            "   |   |",
            "  _|___|_",
        ],
        "rare" => vec![
            " _/\\  /\\_",
            "(  o    o  )",
            " \\ ----- /",
            "  \\_____/",
            "  (     )",
            " __|___|__",
        ],
        "uncommon" => vec![
            "  /\\  /\\",
            " ( .    . )",
            "  \\_---_/",
            "   |   |",
            "   |   |",
            "  _|___|_",
        ],
        _ => vec![  // common
            "  /\\_/\\",
            " ( o . o )",
            "  \\  v  /",
            "  /|   |\\",
            " (_|   |_)",
            "",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state;

    fn test_state() -> state::State {
        state::State {
            version: 2,
            name: "Shadowmere".to_string(),
            birth_time: 1000,
            last_fed_at: 1000,
            total_tool_calls: 42,
            coins: 42,
            level: 2,
            lifetime_cost_usd: 0.5,
            breed: "Void Cat".to_string(),
            rarity: "legendary".to_string(),
            shiny: true,
            stats: [85, 42, 8, 61, 93],
            personality: "Born from the space between stars, Shadowmere sees what others miss.".to_string(),
        }
    }

    #[test]
    fn card_contains_breed() {
        let output = render(&test_state());
        assert!(output.contains("VOID CAT"), "card missing uppercased breed");
    }

    #[test]
    fn card_contains_name() {
        let output = render(&test_state());
        assert!(output.contains("Shadowmere"), "card missing name");
    }

    #[test]
    fn card_contains_stars() {
        let output = render(&test_state());
        assert!(output.contains("★★★★★"), "card missing stars");
    }

    #[test]
    fn card_contains_stat_names() {
        let output = render(&test_state());
        for name in identity::STAT_NAMES {
            assert!(output.contains(name), "card missing stat {name}");
        }
    }

    #[test]
    fn card_contains_personality() {
        let output = render(&test_state());
        assert!(output.contains("space between stars"), "card missing personality");
    }

    #[test]
    fn card_shiny_tag() {
        let s = test_state();
        let output = render(&s);
        assert!(output.contains("[SHINY]"), "shiny tag missing");

        let mut ns = s.clone();
        ns.shiny = false;
        let output2 = render(&ns);
        assert!(!output2.contains("[SHINY]"), "shiny tag should be absent");
    }

    #[test]
    fn card_no_peak_dump_suffix() {
        let output = render(&test_state());
        assert!(!output.contains(" peak"), "card should not contain 'peak' suffix");
        assert!(!output.contains(" dump"), "card should not contain 'dump' suffix");
    }

    #[test]
    fn card_line_widths_consistent() {
        let output = render(&test_state());
        // Every line (except the final ╚╝ line which has no newline) should be CARD_WIDTH+2 chars wide
        // The box chars ║, ╠, ╔ etc. are single Unicode scalar values so .chars().count() gives visual width
        let expected = CARD_WIDTH + 2; // e.g. 44
        for (i, line) in output.lines().enumerate() {
            let len = line.chars().count();
            assert_eq!(len, expected,
                "line {i} width {len} != {expected}: {:?}", line);
        }
    }
}
