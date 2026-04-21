use crate::colors::{C_PINK, C_RESET};

fn display_width(s: &str) -> usize {
    s.chars().count()
}

pub fn progress_bar(pct: f64, width: usize) -> String {
    let pct_int = pct.round().clamp(0.0, 100.0) as usize;
    let filled = pct_int * width / 100;
    let empty = width - filled;
    let mut out = String::with_capacity((filled + empty) * 3);
    for _ in 0..filled {
        out.push('█');
    }
    for _ in 0..empty {
        out.push('░');
    }
    out
}

pub fn human_duration(ms: i64) -> String {
    let ms = ms.max(0);
    let s = ms / 1000;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let rs = s % 60;
    if h > 0 {
        format!("{h}h {m}m {rs}s")
    } else if m > 0 {
        format!("{m}m {rs}s")
    } else {
        format!("{rs}s")
    }
}

pub fn time_until_reset(reset_at: i64, now: i64) -> String {
    if reset_at <= 0 || reset_at <= now {
        return "---".to_string();
    }
    let delta = reset_at - now;
    let d = delta / 86400;
    let h = (delta % 86400) / 3600;
    let m = (delta % 3600) / 60;
    if d > 0 {
        format!("in {d}d {h:02}h {m:02}m")
    } else if h > 0 {
        format!("in {h}h {m:02}m")
    } else {
        format!("in {m}m")
    }
}

pub fn human_elapsed(d: i64) -> String {
    let d = d.max(0);
    if d < 60 {
        format!("{d}s ago")
    } else if d < 3600 {
        format!("{}m ago", d / 60)
    } else if d < 86400 {
        format!("{}h ago", d / 3600)
    } else {
        format!("{}d ago", d / 86400)
    }
}

pub fn short_cwd(full_path: &str, home: &str) -> String {
    let result = if full_path == home {
        "~".to_string()
    } else if !home.is_empty() && full_path.starts_with(&format!("{home}/")) {
        format!("~{}", &full_path[home.len()..])
    } else if full_path.is_empty() {
        "?".to_string()
    } else {
        full_path.to_string()
    };
    let chars: Vec<char> = result.chars().collect();
    if chars.len() > 25 {
        let tail: String = chars[chars.len() - 22..].iter().collect();
        format!("...{tail}")
    } else {
        result
    }
}

pub fn pad_col(plain: &str, ansi: &str, width: usize) -> (String, String) {
    let len = display_width(plain);
    let pad = width.saturating_sub(len);
    let spaces = " ".repeat(pad);
    (format!("{plain}{spaces}"), format!("{ansi}{spaces}"))
}

pub fn anchor_col(
    lp: &str,
    la: &str,
    rp: &str,
    ra: &str,
    width: usize,
) -> (String, String) {
    let ll = display_width(lp);
    let rl = display_width(rp);
    let half_l = width.saturating_sub(3) / 2;
    let half_r = width.saturating_sub(3) - half_l;
    let l_pad = half_l.saturating_sub(ll);
    let r_pad = half_r.saturating_sub(rl);
    let lp_spaces = " ".repeat(l_pad);
    let rp_spaces = " ".repeat(r_pad);
    (
        format!("{lp}{lp_spaces} │ {rp}{rp_spaces}"),
        format!("{la}{lp_spaces} {C_PINK}│{C_RESET} {ra}{rp_spaces}"),
    )
}

pub fn close_row(plain: &str, ansi: &str, box_width: usize) -> String {
    let len = display_width(plain);
    let pad = box_width.saturating_sub(len);
    let spaces = " ".repeat(pad);
    format!("{C_PINK}║{C_RESET}{ansi}{spaces}{C_PINK}║{C_RESET}")
}

pub fn pad_sprite(s: &str, sw: usize) -> String {
    let len = display_width(s);
    let pad = sw.saturating_sub(len);
    let spaces = " ".repeat(pad);
    format!("{s}{spaces}")
}
