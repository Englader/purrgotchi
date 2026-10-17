//! `purrgotchi frame --json` — one render of the kitten for the mod's pane
//! (spec §3.1, §4.3). Read-only unless `--advance`.

use crate::colors::Attr;
use crate::json::{CelebrationJson, FrameJson, HudJson, PeakJson, SceneJson, V};
use crate::span::{to_json, Span};
use crate::state::{self, CelebrationKind, State};
use crate::statusline::{advance, kitten_view, KittenView, ViewInput, CELEBRATION_TICK_SECS};

pub struct FrameOpts {
    /// Strip/pane width in cells (the mod passes the pane's `bodyColumns`).
    /// None → the statusline's COLUMNS / PURR_PLAYGROUND_WIDTH logic.
    pub width: Option<usize>,
    /// Animation tick; sprite parity is `tick % 2`. None → now.
    pub tick: Option<u64>,
    /// Context-window fill 0..=100 (drives panic/alert moods). Clamped.
    pub context_pct: f64,
    /// Also run the wall-clock timers (celebration countdown, scene
    /// scheduler) and consume the quip — for pane-only setups.
    pub advance: bool,
    /// Also emit the generated 32x32 pixel sprite as `pixels`.
    pub pixels: bool,
}

pub fn parse_opts(args: &[String]) -> Result<FrameOpts, String> {
    use crate::cli::opt_value as value;
    let width = value(args, "--width")?
        .map(|v| v.trim().parse::<usize>().map_err(|_| format!("--width must be a non-negative integer, got {v:?}")))
        .transpose()?;
    let tick = value(args, "--tick")?
        .map(|v| v.trim().parse::<u64>().map_err(|_| format!("--tick must be a non-negative integer, got {v:?}")))
        .transpose()?;
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
    Ok(FrameOpts {
        width, tick, context_pct,
        advance: crate::cli::has_flag(args, "--advance"),
        pixels: crate::cli::has_flag(args, "--pixels"),
    })
}

pub fn run(opts: &FrameOpts) -> i32 {
    // Read-only: never load_or_hatch (it would create state.json).
    let Some(st) = crate::json::load_for_read(true) else { return 1 };
    let now = state::now_secs();
    let tick = opts.tick.unwrap_or(now);
    let forced_scene = crate::playground::force_scene();
    let playground_enabled = crate::playground::enabled();
    let strip_width = match opts.width {
        Some(w) => crate::playground::clamp_strip(w),
        None => crate::playground::strip_width(),
    };
    let view = kitten_view(
        &ViewInput {
            st: &st,
            now: now as i64,
            frame: (tick % 2) as u8,
            ctx_pct: opts.context_pct,
            forced_scene,
            playground_enabled,
            strip_width,
        },
        || if opts.advance { crate::quip_cache::consume() } else { crate::quip_cache::peek() },
    );
    crate::json::print(&build(&st, now, tick, &view, opts.pixels));
    if opts.advance {
        advance(&st, now, view.is_birthday, forced_scene, playground_enabled);
    }
    0
}

pub(crate) fn build(st: &State, now: u64, tick: u64, view: &KittenView, pixels: bool) -> FrameJson {
    let sprite = (0..crate::playground::ROWS)
        .map(|i| {
            let line = view.sprite_lines.get(i).cloned().unwrap_or_default();
            to_json(&[Span::new(line, &view.sprite_attrs)])
        })
        .collect();
    let scene = view.scene.as_ref().map(|sv| SceneJson {
        kind: sv.kind,
        elapsed: sv.elapsed,
        duration: sv.duration,
        finale: sv.finale,
        width: sv.width,
        rows: crate::playground::render_scene_spans(
            sv.kind, sv.elapsed, sv.duration, sv.seed, sv.width, view.frame, &view.rarity_attrs,
        )
        .iter()
        .map(|r| to_json(r))
        .collect(),
    });
    let celebration = (st.celebration_kind != CelebrationKind::None).then(|| CelebrationJson {
        kind: st.celebration_kind,
        remaining_ticks: st.levelup_renders_remaining,
        tick_secs: CELEBRATION_TICK_SECS,
    });
    FrameJson {
        v: V,
        ok: true,
        now,
        tick,
        name: st.name.clone(),
        breed: st.breed.clone(),
        rarity: st.rarity.clone(),
        shiny: st.shiny,
        level: st.level,
        stage: state::life_stage(st.level).to_string(),
        coins: st.coins,
        bond: crate::bond::effective_bond(st.bond, crate::bond::last_interaction_at(st), now),
        mood: view.mood.to_string(),
        sprite,
        speech: to_json(&[Span::new(view.speech.clone(), &view.speech_attrs)]),
        speech_source: view.speech_source.as_str().to_string(),
        scene,
        celebration,
        hud: hud(st, now),
        pixels: pixels.then(|| {
            crate::pixel::grid_hex(&crate::pixel::render(&crate::pixel::PixelOpts {
                breed: &st.breed,
                rarity: &st.rarity,
                stage: state::life_stage(st.level),
                mood: view.mood,
                frame: view.frame,
                shiny: st.shiny,
                tick,
            }))
        }),
    }
}

/// Progress to the next level (level = 1 + total / 50), as a percentage.
pub(crate) fn xp_pct(total: u64) -> u8 {
    ((total % 50) * 2) as u8
}

fn hud(st: &State, now: u64) -> HudJson {
    let name_fg = crate::colors::rarity_attrs(&st.rarity, st.shiny, now as i64)
        .into_iter()
        .find_map(|a| match a {
            Attr::Fg(rgb) => Some(rgb.hex()),
            Attr::Bold | Attr::Dim => None,
        })
        .unwrap_or_else(|| "#0abdc6".to_string());
    let (label, range) = crate::statusline::peak_period(crate::statusline::local_hour());
    HudJson {
        total_tool_calls: st.total_tool_calls,
        xp_pct: xp_pct(st.total_tool_calls),
        last_fed_at: st.last_fed_at,
        stars: crate::identity::rarity_stars(&st.rarity, st.shiny),
        name_fg,
        peak: PeakJson { label: label.to_string(), range: range.to_string(), is_peak: label == "PEAK" },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn xp_pct_is_progress_to_next_level() {
        for (total, want) in [(0, 0), (49, 98), (50, 0), (125, 50)] {
            assert_eq!(xp_pct(total), want, "{total}");
        }
    }

    #[test]
    fn name_fg_is_hex_for_every_rarity() {
        let mut st = crate::state::State::base(1_700_000_000);
        for r in ["common", "uncommon", "rare", "epic", "legendary"] {
            st.rarity = r.to_string();
            for shiny in [false, true] {
                st.shiny = shiny;
                let fg = hud(&st, 1_700_000_000).name_fg;
                assert!(fg.len() == 7 && fg.starts_with('#') && fg[1..].chars().all(|c| c.is_ascii_hexdigit()), "{r}: {fg}");
            }
        }
    }

    #[test]
    fn parse_opts_pixels_flag() {
        assert!(!parse_opts(&a(&["p", "frame", "--json"])).unwrap().pixels);
        assert!(parse_opts(&a(&["p", "frame", "--json", "--pixels"])).unwrap().pixels);
    }

    #[test]
    fn parse_opts_defaults() {
        let o = parse_opts(&a(&["p", "frame", "--json"])).unwrap();
        assert_eq!((o.width, o.tick, o.context_pct, o.advance), (None, None, 0.0, false));
    }

    #[test]
    fn parse_opts_values_and_clamping() {
        let o = parse_opts(&a(&["p", "frame", "--width", "56", "--tick", "9", "--context-pct", "250", "--advance"])).unwrap();
        assert_eq!((o.width, o.tick, o.context_pct, o.advance), (Some(56), Some(9), 100.0, true));
        assert_eq!(parse_opts(&a(&["p", "frame", "--context-pct", "-5"])).unwrap().context_pct, 0.0);
    }

    #[test]
    fn parse_opts_rejects_garbage() {
        for bad in [
            vec!["--width", "abc"], vec!["--width"], vec!["--tick", "-1"],
            vec!["--context-pct", "nan"], vec!["--context-pct", "inf"], vec!["--context-pct", "x"],
        ] {
            let mut v = vec!["p", "frame"];
            v.extend(bad.iter().copied());
            assert!(parse_opts(&a(&v)).is_err(), "{bad:?}");
        }
    }
}
