use std::fs;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::paths;

const GRACE_US: i64 = 500_000;

pub struct TurnCost {
    pub whole: i64,
    pub cents: i64,
}

pub fn compute_turn_cost(cost_usd: f64, session_id: &str) -> TurnCost {
    let anchor_path = paths::turn_anchor_file_for(session_id);
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let now_us = now.as_micros() as i64;
    let now_real = format!(
        "{}.{:06}",
        now.as_secs(),
        now.subsec_micros()
    );

    let turn_anchor = match fs::read_to_string(&anchor_path) {
        Ok(contents) => {
            let mut lines = contents.lines();
            let line1 = lines.next().unwrap_or("").trim();
            let line2 = lines.next().unwrap_or("").trim();
            let anchor_cost: f64 = line1.parse().unwrap_or(0.0);
            let anchor_ts_us = parse_micros(line2);

            let age_us = now_us.saturating_sub(anchor_ts_us);
            if (0..=GRACE_US).contains(&age_us) {
                // In grace window — refresh anchor, keep original creation ts.
                let _ = write_anchor(&anchor_path, cost_usd, line2);
                cost_usd
            } else {
                // Past grace: freeze.
                anchor_cost
            }
        }
        Err(_) => {
            // Hook cleared it — this render starts the turn.
            let _ = write_anchor(&anchor_path, cost_usd, &now_real);
            cost_usd
        }
    };

    let cur_cents = (cost_usd * 10_000.0).round() as i64;
    let anchor_cents = (turn_anchor * 10_000.0).round() as i64;
    let mut delta = cur_cents - anchor_cents;
    if delta < 0 {
        delta = 0;
    }
    let mut whole = delta / 10_000;
    let mut cents = (delta % 10_000 + 50) / 100;
    if cents >= 100 {
        whole += 1;
        cents = 0;
    }
    TurnCost { whole, cents }
}

fn parse_micros(s: &str) -> i64 {
    // Parse "1776778376.978200" as microseconds (strip the dot).
    let cleaned: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    cleaned.parse().unwrap_or(0)
}

fn write_anchor(path: &std::path::Path, cost: f64, ts: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::File::create(path)?;
    writeln!(f, "{cost}")?;
    writeln!(f, "{ts}")?;
    Ok(())
}
