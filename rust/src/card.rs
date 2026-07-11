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

    let now: i64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let name_color = crate::colors::rarity_color(&s.rarity, s.shiny, now);

    let stars = identity::rarity_stars(&s.rarity, s.shiny);
    let breed_upper = s.breed.to_uppercase();

    // Top border
    writeln!(out, "╔{}╗", "═".repeat(CARD_WIDTH)).unwrap();

    // Title: BREED // Name // STARS — name is colored by rarity; breed and stars stay plain.
    let inner_content = inner - 2;
    let title_raw = format!("{} // {} // {}", breed_upper, s.name, stars);
    let title: String = title_raw.chars().take(inner_content).collect();
    // Pad plain first (width-correct), then splice color around the name.
    let padded_plain = format!("{:<width$}", title, width = inner_content);
    let colored = if padded_plain.contains(&s.name) {
        padded_plain.replacen(
            s.name.as_str(),
            &format!("{name_color}{}{}", s.name, crate::colors::C_RESET),
            1,
        )
    } else {
        // Name was truncated out of the visible title — no coloring possible.
        padded_plain
    };
    writeln!(out, "║  {colored}║").unwrap();

    // Shiny / born row
    let born = crate::render::format_ymd(s.birth_time);
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

    // Sprite (6 lines, each padded to fit inside the box, then wrapped in rarity color)
    let sprite_inner = inner - 2;
    let stage = crate::state::life_stage(s.level);
    let sprite = crate::sprites::get_sprite_lines(stage, &s.rarity, "calm", 0);
    for line in sprite.iter().take(6) {
        let padded = format!("{:<width$}", line, width = sprite_inner);
        writeln!(out, "║  {name_color}{padded}{}║", crate::colors::C_RESET).unwrap();
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

    // Bond row (dynamic stat, visually separated from the 5 identity stats above
    // by its own divider block). Value reflects lazy-decay, not raw state.
    // Decay anchors on the most recent of pet/play/treat — the same anchor the
    // statusline and stats card use — so all three views agree.
    // Cast the i64 `now` computed above for rarity_color — safe because
    // SystemTime::now().as_secs() is always non-negative and well below i64::MAX.
    let effective = crate::bond::effective_bond(s.bond, crate::bond::last_interaction_at(s), now as u64);
    let bond_bar = stat_bar(effective);
    let pink = crate::colors::C_PINK;
    let reset = crate::colors::C_RESET;
    let bond_content = format!(
        "  {:<11}{pink}\u{2665}{reset} {bond_bar}  {:>3}",
        "BOND", effective,
    );
    // `bond_content` embeds ANSI color escapes for the pink heart. The `{:<width$}`
    // pattern used by the identity stat rows above counts escape bytes as visible chars
    // and would over-pad this line by dozens of characters. Pad manually against the
    // ANSI-stripped plain width instead: 2 + 11 + 1 + 1 + 10 + 2 + 3 = 30.
    let plain_bond_len = 2 + 11 + 1 + 1 + 10 + 2 + 3;
    let pad = inner.saturating_sub(plain_bond_len);
    let bond_line = format!("{bond_content}{}", " ".repeat(pad));
    writeln!(out, "║{bond_line}║").unwrap();

    // Divider
    writeln!(out, "╠{}╣", "═".repeat(CARD_WIDTH)).unwrap();

    // Personality (word-wrapped, quoted), indented by 2 spaces.
    // Render-time {name} substitution: stored blurbs use the literal placeholder
    // so renames flow through automatically. No-op for old kittens whose stored
    // personality has the name already inlined.
    let pers_wrap_width = inner_content - 2;
    let rendered_personality = s.personality.replace("{name}", &s.name);
    let wrapped = wrap(&rendered_personality, pers_wrap_width);
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
    // Clamp like hooks::nine_cell_bar — a hand-edited/corrupt state.json can
    // carry any u8, and `10 - filled` must not underflow.
    let val = val.min(100) as usize;
    let filled = ((val * 10 + 50) / 100).min(10);
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
            bond: 50,
            last_pet_at: 0,
            last_played_at: 0,
            last_treated_at: 0,
            streak_count: 0,
            last_milestone_celebrated: 0,
            last_turn_at: 0,
            last_celebration_tick_at: 0,
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
        let expected = CARD_WIDTH + 2; // e.g. 44
        for (i, line) in output.lines().enumerate() {
            let stripped = strip_ansi(line);
            let len = stripped.chars().count();
            assert_eq!(len, expected,
                "line {i} stripped width {len} != {expected}: {:?}", line);
        }
    }

    /// Strip CSI (ESC [ ... letter) sequences so char counts reflect visible width.
    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' && chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // consume until a letter (inclusive) — CSI final byte is 0x40..=0x7E
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn card_title_contains_rarity_color_for_name() {
        // test_state is legendary shiny → rarity_color will include a RAINBOW hue
        // and either C_BOLD or C_DIM.
        let output = render(&test_state());
        let has_any_hue = crate::colors::RAINBOW_HUES
            .iter()
            .any(|h| output.contains(h));
        assert!(has_any_hue, "expected a rainbow hue in legendary card output:\n{output}");
    }

    #[test]
    fn card_title_contains_rarity_color_for_common() {
        let mut s = test_state();
        s.rarity = "common".to_string();
        s.shiny = false;
        let output = render(&s);
        assert!(output.contains(crate::colors::C_CYAN),
            "expected C_CYAN in common card output:\n{output}");
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
        assert_eq!(crate::render::format_ymd(1_776_729_600), "2026-04-21");
        // 2024-02-29 00:00:00 UTC (leap day) = 1_709_164_800
        assert_eq!(crate::render::format_ymd(1_709_164_800), "2024-02-29");
        // Epoch
        assert_eq!(crate::render::format_ymd(0), "1970-01-01");
    }

    #[test]
    fn card_sprite_rows_wrapped_in_rarity_color() {
        let mut s = test_state();
        s.rarity = "common".to_string();
        s.shiny = false;
        let output = render(&s);
        // Expect at least one line where a plain sprite glyph is preceded by C_CYAN in
        // the same line. Simpler: confirm C_CYAN occurs more than once in the output
        // (title + at least one sprite row).
        let count = output.matches(crate::colors::C_CYAN).count();
        assert!(count >= 2, "expected C_CYAN in title + sprite rows, saw {count} in:\n{output}");
    }

    #[test]
    fn card_contains_bond_label() {
        let output = render(&test_state());
        assert!(output.contains("BOND"), "card missing BOND label:\n{output}");
    }

    #[test]
    fn card_shows_bond_value() {
        let mut s = test_state();
        s.bond = 73;
        s.last_pet_at = 0; // never petted → effective == raw
        let output = render(&s);
        assert!(output.contains("73"), "card missing bond value 73:\n{output}");
    }

    #[test]
    fn card_bond_has_heart_glyph() {
        let output = render(&test_state());
        assert!(output.contains('\u{2665}'), "card missing ♥ glyph:\n{output}");
    }

    #[test]
    fn card_bond_row_reflects_decay() {
        // Raw bond=80, petted exactly 2 days ago → effective = 80 - 10 = 70.
        // Both now_secs() and SystemTime::now().as_secs() truncate to whole seconds,
        // so at most a 1-second delta elapses between them — not enough to shift decay
        // off of 10 (requires 86400s per extra point). Result is deterministically 70.
        let mut s = test_state();
        s.bond = 80;
        s.last_pet_at = crate::state::now_secs() - 2 * 86_400;
        let output = render(&s);
        assert!(output.contains("70"), "card bond value should be 70 after 2-day decay, got:\n{output}");
    }

    #[test]
    fn card_substitutes_name_placeholder_in_personality() {
        let mut s = test_state();
        s.name = "Whiskers".into();
        // Keep the personality short enough to render on a single wrapped line
        // (wrap width = 38) so the substring assertion below sees the substituted
        // name contiguously with the surrounding text.
        s.personality = "{name} stares.".into();
        let output = render(&s);
        assert!(output.contains("Whiskers stares."),
            "expected name substitution in personality, got:\n{output}");
        assert!(!output.contains("{name}"),
            "card should not show the literal `{{name}}` placeholder");
    }

    #[test]
    fn card_personality_without_placeholder_renders_unchanged() {
        // Old kittens stored personality with the name pre-substituted. The render-time
        // .replace must be a no-op for them.
        let mut s = test_state();
        s.name = "Whiskers".into();
        s.personality = "An old, dignified cat.".into();
        let output = render(&s);
        assert!(output.contains("An old, dignified cat."));
    }
}
