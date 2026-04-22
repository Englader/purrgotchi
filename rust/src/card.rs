use crate::identity;
use crate::state::life_stage;
use crate::state::State;

// Total number of ═ chars between ╔ and ╗ (and the inner content width).
// With ║ on each side, total line width = CARD_WIDTH + 2 = 44 chars.
const CARD_WIDTH: usize = 42;

fn stage_label(level: u64) -> &'static str {
    match life_stage(level) {
        "kitten" => "kitten",
        "young"  => "young cat",
        _        => "adult cat",
    }
}

pub fn run() {
    // Deliberately uses `state::load()` rather than `hatch::load_or_hatch()`:
    // `card` is a read-only inspection command, so a missing state should error
    // loudly instead of silently auto-hatching (which would feel surprising
    // from a command whose contract is "show me my cat").
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
    // inner - 2: account for the 2 leading spaces before content, leaving 40 chars for content
    let inner_content = inner - 2;
    let title_raw = format!("{} // {} // {}", breed_upper, s.name, stars);
    let title: String = title_raw.chars().take(inner_content).collect();
    writeln!(out, "║  {:<width$}║", title, width = inner_content).unwrap();

    // Shiny / born row
    let born = format_born(s.birth_time);
    if s.shiny {
        let shiny_row: String = format!("[SHINY]   Born: {born}").chars().take(inner_content).collect();
        writeln!(out, "║  {:<width$}║", shiny_row, width = inner_content).unwrap();
    } else {
        let born_row: String = format!("Born: {born}").chars().take(inner_content).collect();
        writeln!(out, "║  {:<width$}║", born_row, width = inner_content).unwrap();
    }

    // Stage row — sits between the identity rows and the sprite divider.
    let stage_row: String = format!("Stage: {}", stage_label(s.level))
        .chars()
        .take(inner_content)
        .collect();
    writeln!(out, "║  {:<width$}║", stage_row, width = inner_content).unwrap();

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
    // Layout: ║  <name:11><bar:10>  <val:3>         ║
    // Fixed content width = 2 + 11 + 10 + 2 + 3 = 28
    // Padding to inner (42): 42 - 28 = 14 trailing spaces
    for (i, &val) in s.stats.iter().enumerate() {
        let bar = stat_bar(val);
        // Build content string, then pad to inner width
        let content = format!("  {:<11}{}  {:>3}", identity::STAT_NAMES[i], bar, val);
        writeln!(out, "║{:<width$}║", content, width = inner).unwrap();
    }

    // Divider
    writeln!(out, "╠{}╣", "═".repeat(CARD_WIDTH)).unwrap();

    // Personality (word-wrapped, quoted), indented by 2 spaces
    // inner_content = 40. Subtract 2 for opening/closing quote chars → wrap at 38.
    let pers_wrap_width = inner_content - 2;
    let wrapped = wrap(&s.personality, pers_wrap_width);
    let last = wrapped.len().saturating_sub(1);
    for (i, line) in wrapped.iter().enumerate() {
        let decorated = if wrapped.len() == 1 {
            format!("\"{line}\"")
        } else if i == 0 {
            format!("\"{line}")
        } else if i == last {
            format!("{line}\"")
        } else {
            line.clone()
        };
        writeln!(out, "║  {:<width$}║", decorated, width = inner_content).unwrap();
    }

    // Bottom border
    write!(out, "╚{}╝", "═".repeat(CARD_WIDTH)).unwrap();
    out
}

fn stat_bar(val: u8) -> String {
    let filled = (val as usize * 10 + 50) / 100;
    let empty = 10 - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word.chars().count() <= width {
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
    let mut days = (secs / 86400) as u32;
    let mut year = 1970u32;
    loop {
        let in_year = if is_leap(year) { 366 } else { 365 };
        if days < in_year { break; }
        days -= in_year;
        year += 1;
    }
    let month_days = [31u32, if is_leap(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 0usize;
    for &md in &month_days {
        if days < md { break; }
        days -= md;
        month += 1;
    }
    format!("{year}-{:02}-{:02}", month + 1, days + 1)
}

fn is_leap(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
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
            levelup_renders_remaining: 0,
            celebration_kind: state::CelebrationKind::None,
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

    fn make_state(level: u64) -> State {
        let mut s = State::base(1000);
        s.name = "Luna".to_string();
        s.breed = "Tabby".to_string();
        s.rarity = "common".to_string();
        s.level = level;
        s.stats = [50, 50, 50, 50, 50];
        s.personality = "A chill cat.".to_string();
        s
    }

    #[test]
    fn card_shows_kitten_stage_at_low_level() {
        let out = render(&make_state(3));
        assert!(out.contains("Stage: kitten"), "expected kitten stage in card, got:\n{out}");
    }

    #[test]
    fn card_shows_young_cat_stage_at_middle_level() {
        let out = render(&make_state(25));
        assert!(out.contains("Stage: young cat"), "expected young cat stage in card, got:\n{out}");
    }

    #[test]
    fn card_shows_adult_cat_stage_at_high_level() {
        let out = render(&make_state(100));
        assert!(out.contains("Stage: adult cat"), "expected adult cat stage in card, got:\n{out}");
    }

    #[test]
    fn format_born_correct_for_recent_date() {
        // 2026-04-21 00:00:00 UTC = 1_776_729_600 seconds
        assert_eq!(format_born(1_776_729_600), "2026-04-21");
        // 2024-02-29 00:00:00 UTC (leap day) = 1_709_164_800
        assert_eq!(format_born(1_709_164_800), "2024-02-29");
        // Epoch
        assert_eq!(format_born(0), "1970-01-01");
    }
}
