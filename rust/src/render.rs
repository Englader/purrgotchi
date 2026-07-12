use crate::colors::{C_PINK, C_RESET};

fn display_width(s: &str) -> usize {
    s.chars().count()
}

pub fn progress_bar(pct: f64, width: usize) -> String {
    let pct_int = pct.round().clamp(0.0, 100.0) as usize;
    let filled = if pct_int == 0 {
        0
    } else {
        ((pct_int * width + 50) / 100).clamp(1, width)
    };
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

pub fn format_tokens(n: i64) -> String {
    let n = n.max(0);
    if n >= 1_000_000 {
        let tenths = (n + 50_000) / 100_000;
        let whole = tenths / 10;
        let frac = tenths % 10;
        if frac == 0 {
            format!("{whole}M")
        } else {
            format!("{whole}.{frac}M")
        }
    } else if n >= 10_000 {
        format!("{}k", (n + 500) / 1000)
    } else if n >= 1000 {
        let tenths = (n + 50) / 100;
        let whole = tenths / 10;
        let frac = tenths % 10;
        if frac == 0 {
            format!("{whole}k")
        } else {
            format!("{whole}.{frac}k")
        }
    } else {
        n.to_string()
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
    } else if !home.is_empty()
        && full_path.len() > home.len()
        && full_path.starts_with(home)
        && full_path.as_bytes()[home.len()] == b'/'
    {
        format!("~{}", &full_path[home.len()..])
    } else if full_path.is_empty() {
        "?".to_string()
    } else {
        full_path.to_string()
    };
    let count = display_width(&result);
    if count > 25 {
        let tail: String = result.chars().skip(count - 22).collect();
        format!("...{tail}")
    } else {
        result
    }
}

/// Truncate `s` to at most `max` display columns, keeping the TAIL behind a
/// leading `...` (the same shape short_cwd gives paths). For fields whose end
/// carries the meaning — paths, git branch names — so a long value can never
/// overflow its fixed card column and shove the box border out of alignment.
/// `pad_col` only pads short content; it never trims, so callers must shorten
/// oversized values here first.
pub fn truncate_tail(s: &str, max: usize) -> String {
    let count = display_width(s);
    if count <= max {
        return s.to_string();
    }
    if max <= 3 {
        return ".".repeat(max);
    }
    let keep = max - 3;
    let tail: String = s.chars().skip(count - keep).collect();
    format!("...{tail}")
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

fn is_leap(y: u32) -> bool {
    (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400)
}

/// Formats a Unix timestamp as a date string `YYYY-MM-DD` (UTC).
pub fn format_ymd(secs: u64) -> String {
    let mut days = (secs / 86400) as u32;
    let mut year = 1970u32;
    loop {
        let in_year = if is_leap(year) { 366 } else { 365 };
        if days < in_year { break; }
        days -= in_year;
        year += 1;
    }
    let month_days = [
        31u32,
        if is_leap(year) { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mut month = 0usize;
    for &md in &month_days {
        if days < md { break; }
        days -= md;
        month += 1;
    }
    format!("{year}-{:02}-{:02}", month + 1, days + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_tail_leaves_short_strings_untouched() {
        assert_eq!(truncate_tail("main", 23), "main");
        assert_eq!(truncate_tail("", 23), "");
        // exactly at the limit (23 chars) is left alone
        assert_eq!(truncate_tail("exactly-twenty-three-ch", 23), "exactly-twenty-three-ch");
    }

    #[test]
    fn truncate_tail_keeps_tail_behind_ellipsis() {
        // 29-char branch into a 23-col budget -> "..." + last 20, fits exactly
        let out = truncate_tail("feat/v3.8.0-playground-scenes", 23);
        assert_eq!(display_width(&out), 23);
        assert!(out.starts_with("..."));
        assert!(out.ends_with("playground-scenes"), "got {out:?}");
    }

    #[test]
    fn truncate_tail_degenerate_small_max() {
        assert_eq!(truncate_tail("anything", 3), "...");
        assert_eq!(truncate_tail("anything", 2), "..");
        assert_eq!(truncate_tail("anything", 0), "");
    }

    #[test]
    fn format_ymd_epoch_zero() {
        assert_eq!(format_ymd(0), "1970-01-01");
    }

    #[test]
    fn format_ymd_known_date() {
        // 2024-02-29 00:00:00 UTC (leap day)
        // 2024-01-01 UTC epoch = 1,704,067,200
        // + 59 days * 86400 = 5,097,600
        // = 1,709,164,800
        assert_eq!(format_ymd(1_709_164_800), "2024-02-29");
    }

    #[test]
    fn format_ymd_non_leap_feb_ends_28() {
        // 2023-03-01 UTC = 1,677,628,800
        assert_eq!(format_ymd(1_677_628_800), "2023-03-01");
        // 2023-02-28 UTC = 1,677,542,400
        assert_eq!(format_ymd(1_677_542_400), "2023-02-28");
    }
}
