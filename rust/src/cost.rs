use std::fs;
use std::io::Write;
use std::path::Path;

use crate::paths;

const GRACE_US: i64 = 500_000;

pub struct TurnCost {
    pub whole: i64,
    pub cents: i64,
}

pub fn compute_turn_cost(cost_usd: f64, session_id: &str) -> TurnCost {
    // PURR_NOW-aware (whole seconds under the override); real µs otherwise.
    let now_us = crate::state::now_micros() as i64;
    compute_turn_cost_at(cost_usd, &paths::turn_anchor_file_for(session_id), now_us)
}

/// Clock-injected core of [`compute_turn_cost`].
pub(crate) fn compute_turn_cost_at(cost_usd: f64, anchor_path: &Path, now_us: i64) -> TurnCost {
    let now_real = format!("{}.{:06}", now_us / 1_000_000, now_us % 1_000_000);

    let turn_anchor = match fs::read_to_string(anchor_path) {
        Ok(contents) => {
            let mut lines = contents.lines();
            let line1 = lines.next().unwrap_or("").trim();
            let line2 = lines.next().unwrap_or("").trim();
            let anchor_cost: f64 = line1.parse().unwrap_or(0.0);
            let anchor_ts_us = parse_micros(line2);

            let age_us = now_us.saturating_sub(anchor_ts_us);
            if (0..=GRACE_US).contains(&age_us) {
                // In grace window — refresh anchor, keep original creation ts.
                let _ = write_anchor(anchor_path, cost_usd, line2);
                cost_usd
            } else {
                // Past grace: freeze.
                anchor_cost
            }
        }
        Err(_) => {
            // Hook cleared it — this render starts the turn.
            let _ = write_anchor(anchor_path, cost_usd, &now_real);
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

fn write_anchor(path: &Path, cost: f64, ts: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::File::create(path)?;
    writeln!(f, "{cost}")?;
    writeln!(f, "{ts}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("purrgotchi-test-cost-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d.join("turn-start-cost.txt")
    }

    const T: i64 = 1_800_000_000_000_000; // µs

    #[test]
    fn missing_anchor_starts_the_turn_at_now() {
        let p = tmp("missing");
        let c = compute_turn_cost_at(3.5, &p, T + 250_000);
        assert_eq!((c.whole, c.cents), (0, 0));
        assert_eq!(fs::read_to_string(&p).unwrap(), "3.5\n1800000000.250000\n");
        let _ = fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn inside_grace_refreshes_cost_and_keeps_the_timestamp() {
        let p = tmp("grace");
        write_anchor(&p, 1.0, "1800000000.000000").unwrap();
        let c = compute_turn_cost_at(1.2, &p, T + GRACE_US);
        assert_eq!((c.whole, c.cents), (0, 0));
        assert_eq!(fs::read_to_string(&p).unwrap(), "1.2\n1800000000.000000\n");
        let _ = fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn past_grace_freezes_the_anchor() {
        let p = tmp("frozen");
        write_anchor(&p, 1.25, "1800000000.000000").unwrap();
        let c = compute_turn_cost_at(3.42, &p, T + GRACE_US + 1);
        assert_eq!((c.whole, c.cents), (2, 17));
        assert_eq!(fs::read_to_string(&p).unwrap(), "1.25\n1800000000.000000\n");
        let _ = fs::remove_dir_all(p.parent().unwrap());
    }
}
