use std::fs;
use std::io::{self, Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::colors::{
    C_BOLD, C_CYAN, C_DIM, C_GREEN, C_MAGENTA, C_PINK, C_RED, C_RESET, C_YELLOW,
};
use crate::cost::compute_turn_cost;
use crate::mood::compute_mood;
use crate::paths;
use crate::payload::Payload;
use crate::render::{
    anchor_col, close_row, human_duration, human_elapsed, pad_col, pad_sprite,
    progress_bar, short_cwd, time_until_reset,
};
use crate::speech::compute_speech;
use crate::sprites::get_sprite_lines;
use crate::state;

const W1: usize = 28;
const W2: usize = 30;
const W3: usize = 28;
const BOX_WIDTH: usize = 94;
const SW: usize = 16;

pub fn run() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    if std::env::var("PURR_DEBUG").is_ok() {
        let _ = fs::create_dir_all(paths::state_dir());
        let _ = fs::write(paths::state_dir().join("last-input.json"), &input);
    }

    let payload: Payload = if input.trim().is_empty() {
        Payload::default()
    } else {
        serde_json::from_str(&input).unwrap_or_default()
    };

    let model = if payload.model.display_name.is_empty() {
        "Claude".to_string()
    } else {
        payload.model.display_name
    };

    let cwd_raw = if !payload.workspace.current_dir.is_empty() {
        payload.workspace.current_dir
    } else {
        payload.cwd
    };
    let cwd = normalize_windows_path(&cwd_raw);

    let state_snapshot = state::load();
    let (total_tc, coins, level, last_fed_at, kitten_name) = match state_snapshot {
        Some(s) => (s.total_tool_calls, s.coins, s.level, s.last_fed_at, s.name),
        None => (0, 0, 1, 0, "Purrgotchi".to_string()),
    };
    let _ = total_tc; // not displayed; referenced for future features

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let last_fed_delta: i64 = if last_fed_at > 0 {
        now - last_fed_at as i64
    } else {
        999_999
    };

    let mood = compute_mood(payload.context_window.used_percentage, last_fed_delta);
    let frame = ((now / 2) % 2) as u8;

    let reset_5h_str = time_until_reset(payload.rate_limits.five_hour.resets_at, now);
    let reset_7d_str = time_until_reset(payload.rate_limits.seven_day.resets_at, now);
    let speech = compute_speech(mood, now);

    let git_branch = read_git_branch(&cwd);

    let ctx_int = payload.context_window.used_percentage.round() as i64;
    let rate_int = payload.rate_limits.five_hour.used_percentage.round() as i64;
    let rate_7d_int = payload.rate_limits.seven_day.used_percentage.round() as i64;
    let cost_fmt = format!("{:.2}", payload.cost.total_cost_usd);

    let turn = compute_turn_cost(payload.cost.total_cost_usd);
    let turn_cost_fmt = format!("{}.{:02}", turn.whole, turn.cents);

    let ctx_bar = progress_bar(payload.context_window.used_percentage, 12);
    let r5h_bar = progress_bar(payload.rate_limits.five_hour.used_percentage, 5);
    let r7d_bar = progress_bar(payload.rate_limits.seven_day.used_percentage, 5);
    let duration = human_duration(payload.cost.total_duration_ms);
    let fed_ago = human_elapsed(last_fed_delta);

    let home_raw = paths::home_dir().to_string_lossy().to_string();
    let home = normalize_windows_path(&home_raw);
    let short_path = if cwd.is_empty() {
        String::new()
    } else {
        short_cwd(&cwd, &home)
    };

    let sprite_lines = get_sprite_lines(mood, frame);
    let k_color: String = match mood {
        "panic" => C_RED.to_string(),
        "alert" => C_YELLOW.to_string(),
        "happy" => C_GREEN.to_string(),
        "sleep" => C_MAGENTA.to_string(),
        "hungry" => format!("{C_DIM}{C_CYAN}"),
        _ => C_CYAN.to_string(),
    };

    // --- Build the 7 HUD rows ---
    let sep_p = " │ ";
    let sep_a = format!(" {C_PINK}│{C_RESET} ");

    // H1 — top rail.
    let title_plain = format!("═╡ NETRUNNER // {kitten_name} ╞");
    let title_ansi = format!(
        "{C_PINK}═╡{C_RESET} {C_BOLD}{C_CYAN}NETRUNNER{C_RESET} {C_MAGENTA}//{C_RESET} {C_PINK}{kitten_name}{C_RESET} {C_PINK}╞{C_RESET}"
    );
    let rail_count = BOX_WIDTH.saturating_sub(title_plain.chars().count());
    let rail: String = std::iter::repeat('═').take(rail_count).collect();
    let h1 = format!("{C_PINK}╔{C_RESET}{title_ansi}{C_PINK}{rail}╗{C_RESET}");

    // H2 — MODEL | DIR | GIT
    let (c1p, c1a) = pad_col(
        &format!("MODEL: {model}"),
        &format!("{C_DIM}MODEL:{C_RESET} {C_CYAN}{model}{C_RESET}"),
        W1,
    );
    let (c2p, c2a) = if cwd.is_empty() {
        pad_col("", "", W2)
    } else {
        pad_col(
            &format!("DIR: {short_path}"),
            &format!("{C_DIM}DIR:{C_RESET} {C_CYAN}{short_path}{C_RESET}"),
            W2,
        )
    };
    let (c3p, c3a) = if git_branch.is_empty() {
        pad_col("", "", W3)
    } else {
        pad_col(
            &format!("GIT: {git_branch}"),
            &format!("{C_DIM}GIT:{C_RESET} {C_CYAN}{git_branch}{C_RESET}"),
            W3,
        )
    };
    let p2 = format!(" {c1p}{sep_p}{c2p}{sep_p}{c3p}");
    let a2 = format!(" {c1a}{sep_a}{c2a}{sep_a}{c3a}");
    let h2 = close_row(&p2, &a2, BOX_WIDTH);

    // H3 — USAGE: 5h | 7d | CTX
    let (c1p, c1a) = pad_col(
        &format!("USAGE: 5h: {r5h_bar} {rate_int}%"),
        &format!("{C_DIM}USAGE:{C_RESET} {C_DIM}5h:{C_RESET} {C_YELLOW}{r5h_bar}{C_RESET} {C_BOLD}{rate_int}%{C_RESET}"),
        W1,
    );
    let (c2p, c2a) = pad_col(
        &format!("7d: {r7d_bar} {rate_7d_int}%"),
        &format!("{C_DIM}7d:{C_RESET} {C_YELLOW}{r7d_bar}{C_RESET} {C_BOLD}{rate_7d_int}%{C_RESET}"),
        W2,
    );
    let (c3p, c3a) = pad_col(
        &format!("CTX: {ctx_bar} {ctx_int}%"),
        &format!("{C_DIM}CTX:{C_RESET} {C_CYAN}{ctx_bar}{C_RESET} {C_BOLD}{ctx_int}%{C_RESET}"),
        W3,
    );
    let p3 = format!(" {c1p}{sep_p}{c2p}{sep_p}{c3p}");
    let a3 = format!(" {c1a}{sep_a}{c2a}{sep_a}{c3a}");
    let h3 = close_row(&p3, &a3, BOX_WIDTH);

    // H4 — RESET 5h | 7d | Cost
    let (c1p, c1a) = pad_col(
        &format!("RESET: 5h {reset_5h_str}"),
        &format!("{C_DIM}RESET:{C_RESET} {C_DIM}5h{C_RESET} {C_YELLOW}{reset_5h_str}{C_RESET}"),
        W1,
    );
    let (c2p, c2a) = pad_col(
        &format!("7d {reset_7d_str}"),
        &format!("{C_DIM}7d{C_RESET} {C_YELLOW}{reset_7d_str}{C_RESET}"),
        W2,
    );
    let (c3p, c3a) = pad_col(
        &format!("Cost: ${turn_cost_fmt}"),
        &format!("{C_DIM}Cost:{C_RESET} {C_YELLOW}${turn_cost_fmt}{C_RESET}"),
        W3,
    );
    let p4 = format!(" {c1p}{sep_p}{c2p}{sep_p}{c3p}");
    let a4 = format!(" {c1a}{sep_a}{c2a}{sep_a}{c3a}");
    let h4 = close_row(&p4, &a4, BOX_WIDTH);

    // H5 — LINES | TIME | Total
    let la = payload.cost.total_lines_added;
    let lr = payload.cost.total_lines_removed;
    let (c1p, c1a) = pad_col(
        &format!("LINES: +{la}/-{lr}"),
        &format!("{C_DIM}LINES:{C_RESET} {C_GREEN}+{la}{C_RESET}/{C_RED}-{lr}{C_RESET}"),
        W1,
    );
    let (c2p, c2a) = pad_col(
        &format!("TIME: {duration}"),
        &format!("{C_DIM}TIME:{C_RESET} {duration}"),
        W2,
    );
    let (c3p, c3a) = pad_col(
        &format!("Total: ${cost_fmt}"),
        &format!("{C_DIM}Total:{C_RESET} {C_YELLOW}${cost_fmt}{C_RESET}"),
        W3,
    );
    let p5 = format!(" {c1p}{sep_p}{c2p}{sep_p}{c3p}");
    let a5 = format!(" {c1a}{sep_a}{c2a}{sep_a}{c3a}");
    let h5 = close_row(&p5, &a5, BOX_WIDTH);

    // H6 — LV | COINS (anchored) | MOOD | FED (anchored) | empty
    let (c1p, c1a) = anchor_col(
        &format!("LV: {level}"),
        &format!("{C_DIM}LV:{C_RESET} {C_PINK}{C_BOLD}{level}{C_RESET}"),
        &format!("COINS: {coins}"),
        &format!("{C_DIM}COINS:{C_RESET} {C_YELLOW}{coins}{C_RESET}"),
        W1,
    );
    let (c2p, c2a) = anchor_col(
        &format!("MOOD: {mood}"),
        &format!("{C_DIM}MOOD:{C_RESET} {C_MAGENTA}{mood}{C_RESET}"),
        &format!("FED: {fed_ago}"),
        &format!("{C_DIM}FED:{C_RESET} {fed_ago}"),
        W2,
    );
    let (c3p, c3a) = pad_col("", "", W3);
    let p6 = format!(" {c1p}{sep_p}{c2p}{sep_p}{c3p}");
    let a6 = format!(" {c1a}{sep_a}{c2a}{sep_a}{c3a}");
    let h6 = close_row(&p6, &a6, BOX_WIDTH);

    // H7 — bottom rail.
    let bottom_rail: String = std::iter::repeat('═').take(BOX_WIDTH).collect();
    let h7 = format!("{C_PINK}╚{C_RESET}{C_PINK}{bottom_rail}╝{C_RESET}");

    // --- Compose final output with sprite column + speech bubble ---
    let rows = [h1, h2, h3, h4, h5, h6, h7];
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut buf = String::with_capacity(4096);

    for (i, row) in rows.iter().enumerate() {
        if i == 6 {
            buf.push_str(&format!(
                "{row}   {k_color}\"{speech}\"{C_RESET}\n"
            ));
        } else {
            let line = sprite_lines.get(i).map(String::as_str).unwrap_or("");
            let padded = pad_sprite(line, SW);
            buf.push_str(&format!("{row}   {k_color}{padded}{C_RESET}\n"));
        }
    }

    let _ = out.write_all(buf.as_bytes());
}

fn normalize_windows_path(cwd: &str) -> String {
    let bytes = cwd.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && bytes[2] == b'\\'
    {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        let rest: String = cwd[3..].chars().map(|c| if c == '\\' { '/' } else { c }).collect();
        format!("/{drive}/{rest}")
    } else {
        cwd.to_string()
    }
}

fn read_git_branch(cwd: &str) -> String {
    if cwd.is_empty() {
        return String::new();
    }
    let head_path = std::path::Path::new(cwd).join(".git").join("HEAD");
    match fs::read_to_string(&head_path) {
        Ok(contents) => {
            let line = contents.lines().next().unwrap_or("").trim();
            if let Some(branch) = line.strip_prefix("ref: refs/heads/") {
                branch.to_string()
            } else {
                String::new()
            }
        }
        Err(_) => String::new(),
    }
}
