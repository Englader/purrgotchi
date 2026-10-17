//! `purrgotchi band`: a long-lived NDJSON stream of cat-band frames.
//!
//! The mod spawns this with stdin closed and blits the latest line into one
//! `Raster`. The stream paces itself, samples the same deterministic model
//! as the statusline and never mutates state (no `advance`, no quip
//! consumption, no `load_or_hatch`).

use crate::json::{error_json, read_fail_parts};
use crate::px::band::{base64, pack_cells};
use crate::px::tempo::Tempo;
use crate::px::PxHeight;
use crate::state::{self, State};
use crate::statusline::{kitten_view, ViewInput};
use serde::Serialize;
use std::io::Write;
use std::time::{Duration, Instant, SystemTime};

const MAX_COLS: usize = 512;
const DEFAULT_FPS: u32 = 30;
/// Resend an unchanged frame at least this often (keepalive / error rate).
const KEEPALIVE_MS: u64 = 1000;
const STALE_ALIVE: Duration = Duration::from_secs(24 * 3600);

pub const USAGE: &str =
    "usage: purrgotchi band --cols N --rows R [--fps F] [--context-pct P] [--now-ms T --frames K]";

pub struct BandOpts {
    pub cols: usize,
    pub height: PxHeight,
    pub fps: u32,
    pub context_pct: f64,
    /// Dev/test: virtual start time and frame count (no sleeping).
    pub now_ms: Option<u64>,
    pub frames: Option<u64>,
}

pub fn parse_opts(args: &[String]) -> Result<BandOpts, String> {
    use crate::cli::opt_value as value;
    fn num<T: std::str::FromStr>(flag: &str, v: &str) -> Result<T, String> {
        v.trim().parse().map_err(|_| format!("{flag} must be a non-negative integer, got {v:?}"))
    }
    let cols: usize = num("--cols", value(args, "--cols")?.ok_or("--cols is required")?)?;
    if !(1..=MAX_COLS).contains(&cols) {
        return Err(format!("--cols must be in 1..={MAX_COLS}, got {cols}"));
    }
    let rows: usize = num("--rows", value(args, "--rows")?.ok_or("--rows is required")?)?;
    let height = PxHeight::from_rows(rows).ok_or_else(|| format!("--rows must be 5 or 6, got {rows}"))?;
    let fps = value(args, "--fps")?.map_or(Ok(DEFAULT_FPS), |v| num::<u32>("--fps", v))?.clamp(15, 60);
    let context_pct = match value(args, "--context-pct")? {
        None => 0.0,
        Some(v) => {
            let p: f64 = v.trim().parse().map_err(|_| format!("--context-pct must be a number, got {v:?}"))?;
            if !p.is_finite() {
                return Err(format!("--context-pct must be finite, got {v:?}"));
            }
            p.clamp(0.0, 100.0)
        }
    };
    let now_ms = value(args, "--now-ms")?.map(|v| num::<u64>("--now-ms", v)).transpose()?;
    let frames = value(args, "--frames")?.map(|v| num::<u64>("--frames", v)).transpose()?;
    if now_ms.is_some() != frames.is_some() {
        return Err("--now-ms and --frames go together".to_string());
    }
    Ok(BandOpts { cols, height, fps, context_pct, now_ms, frames })
}

#[derive(Serialize)]
struct Hello<'a> {
    v: u32,
    hello: bool,
    cols: usize,
    rows: usize,
    fps: u32,
    state_dir: &'a str,
}

#[derive(Serialize)]
struct Frame<'a> {
    v: u32,
    f: u64,
    t: u64,
    c: &'a str,
}

/// Remove `band-*.alive` heartbeat files untouched for 24 h.
fn prune_alive(dir: &std::path::Path) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let name = e.file_name();
        let name = name.to_string_lossy();
        if !(name.starts_with("band-") && name.ends_with(".alive")) {
            continue;
        }
        let old = e.metadata().and_then(|m| m.modified()).ok().and_then(|m| SystemTime::now().duration_since(m).ok());
        if old.is_some_and(|d| d > STALE_ALIVE) {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// Run the stream; returns the process exit code.
pub fn run(o: &BandOpts) -> i32 {
    let dir = crate::paths::state_dir();
    prune_alive(&dir);
    let mut out = std::io::stdout().lock();
    let mut emit = |line: &str| -> bool { writeln!(out, "{line}").and_then(|()| out.flush()).is_ok() };

    let state_dir = dir.to_string_lossy();
    let hello = Hello { v: 1, hello: true, cols: o.cols, rows: o.height.rows(), fps: o.fps, state_dir: &state_dir };
    if !emit(&serde_json::to_string(&hello).unwrap_or_default()) {
        return 0;
    }

    let forced_scene = crate::playground::force_scene();
    let playground_enabled = crate::playground::enabled();
    let forced = crate::px::life::forced();
    let period_us = 1_000_000 / u64::from(o.fps);

    let start = Instant::now();
    let base_ms = o.now_ms.unwrap_or_else(|| match state::now_override() {
        Some(n) => n * 1000,
        None => SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64),
    });
    let mut next = start;

    let mut st: Option<State> = None;
    let mut last_load: Option<u64> = None;
    let mut last_err: Option<u64> = None;
    let mut cells: Vec<u8> = Vec::new();
    let mut cells_key: Option<(u64, u64)> = None; // (second, state generation)
    let mut generation = 0u64;
    // `cells` differ from the last frame sent (or none was sent yet).
    let mut dirty = false;
    let mut last_sent_at: Option<u64> = None;

    let mut i = 0u64;
    loop {
        if o.frames.is_some_and(|k| i >= k) {
            return 0;
        }
        let t = match o.now_ms {
            Some(t0) => t0 + i * 1000 / u64::from(o.fps),
            None => base_ms + start.elapsed().as_millis() as u64,
        };

        if last_load.is_none_or(|l| t.saturating_sub(l) >= 1000) {
            last_load = Some(t);
            generation += 1;
            match state::load_for_read() {
                Ok(s) => st = Some(s),
                Err(fail) => {
                    st = None;
                    if last_err.is_none_or(|e| t.saturating_sub(e) >= 1000) {
                        last_err = Some(t);
                        let (code, msg) = read_fail_parts(fail);
                        let line = serde_json::to_string(&error_json(code, msg)).unwrap_or_default();
                        if !emit(&line) {
                            return 0;
                        }
                    }
                }
            }
        }

        if let Some(s) = &st {
            let sec = t / 1000;
            if cells_key != Some((sec, generation)) {
                cells_key = Some((sec, generation));
                let view = kitten_view(
                    &ViewInput {
                        st: s,
                        now: sec as i64,
                        frame: (sec % 2) as u8,
                        ctx_pct: o.context_pct,
                        forced_scene,
                        playground_enabled,
                        // Unclamped: clamp_strip caps at 120 and would hide scenes on wide bands.
                        strip_width: Some(o.cols),
                    },
                    || None,
                );
                let canvas = crate::px::band_canvas(&view, s, sec * 1000, o.cols, forced, o.height, &Tempo::BAND);
                let packed = pack_cells(&canvas);
                if packed != cells {
                    cells = packed;
                    dirty = true;
                }
            }
            if dirty || last_sent_at.is_none_or(|at| t.saturating_sub(at) >= KEEPALIVE_MS) {
                let c = base64(&cells);
                let line = serde_json::to_string(&Frame { v: 1, f: i, t, c: &c }).unwrap_or_default();
                if !emit(&line) {
                    return 0;
                }
                dirty = false;
                last_sent_at = Some(t);
            }
        }

        i += 1;
        if o.now_ms.is_none() {
            next += Duration::from_micros(period_us);
            let now = Instant::now();
            if next > now {
                std::thread::sleep(next - now);
            } else {
                next = now;
            }
        }
    }
}

