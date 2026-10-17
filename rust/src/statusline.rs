use std::fs;
use std::io::{self, Read, Write};
use std::time::SystemTime;

use crate::colors::{Attr, CYAN, GREEN, MAGENTA, PINK, RED, YELLOW};
use crate::identity;
use crate::cost::compute_turn_cost;
use crate::debug;
use crate::mood::compute_mood;
use crate::paths;
use crate::payload::Payload;
use crate::render::{
    anchor_spans, close_row_spans, format_tokens, human_duration, human_elapsed, pad_spans,
    pad_sprite, progress_bar, short_cwd, time_until_reset, truncate_tail,
};
use crate::span::{bold, dim, fg, plain, to_ansi, width as spans_width, Span};
use crate::speech::{compute_celebration_speech, compute_speech};
use crate::sprites::{get_celebration_sprite, get_sprite_lines};
use crate::state::{self, CelebrationKind, PlaygroundScene};

const W1: usize = 28;
const W2: usize = 30;
const W3: usize = 28;
const BOX_WIDTH: usize = 94;
const SW: usize = 16;

/// Minimum seconds between celebration-countdown decrements. With statusLine
/// refreshInterval=1 renders arrive every second; ungated, a 3-render
/// celebration would flash by in 3 seconds.
pub(crate) const CELEBRATION_TICK_SECS: u64 = 2;

pub(crate) struct ViewInput<'a> {
    pub st: &'a crate::state::State,
    pub now: i64,
    /// Sprite animation parity. statusline: now % 2; frame: tick % 2.
    pub frame: u8,
    pub ctx_pct: f64,
    pub forced_scene: Option<PlaygroundScene>,
    pub playground_enabled: bool,
    pub strip_width: Option<usize>,
}

pub(crate) struct SceneView {
    pub kind: PlaygroundScene,
    pub elapsed: u64,
    pub duration: u64,
    pub seed: u64,
    pub width: usize,
    pub finale: bool,
}

pub(crate) struct KittenView {
    pub mood: &'static str,
    pub frame: u8,
    pub is_birthday: bool,
    pub last_fed_delta: i64,
    pub speech_source: SpeechSource,
    pub speech: String,
    pub speech_attrs: Vec<Attr>,
    pub sprite_lines: Vec<String>,
    pub sprite_attrs: Vec<Attr>,
    /// The cat's rarity/shiny tint — used for scene cells even under a quip.
    pub rarity_attrs: Vec<Attr>,
    pub scene: Option<SceneView>,
}

/// Pure "what does the kitten look like right now". `quip` is only called when
/// a quip could actually be shown (celebration, birthday and a visible scene
/// outrank it in select_speech_source — consuming behind them burned the
/// quip's renders without it ever appearing).
pub(crate) fn kitten_view(inp: &ViewInput, quip: impl FnOnce() -> Option<String>) -> KittenView {
    let st = inp.st;
    let now = inp.now;
    let celebration_kind = st.celebration_kind;
    let last_fed_delta: i64 = if st.last_fed_at > 0 { now - st.last_fed_at as i64 } else { 999_999 };
    let since = |t: u64| -> i64 { if t > 0 { now - t as i64 } else { -1 } };
    let mood = compute_mood(
        inp.ctx_pct,
        last_fed_delta,
        since(st.last_pet_at),
        since(st.last_played_at),
        since(st.last_treated_at),
    );
    let is_birthday = crate::events::is_birthday(st.birth_time, now as u64);

    // Playground scene visibility — resolved before the quip guard so quips
    // are not consumed (burned) behind a running scene.
    // PURR_FORCE_SCENE loops a scene on a 30 s cycle without touching state.
    let (scene_kind, scene_started_at, scene_duration_secs, scene_seed) = match inp.forced_scene {
        Some(k) => (k, (now as u64) - (now as u64) % 30, 30, 42),
        None => (st.scene_kind, st.scene_started_at, st.scene_duration_secs, st.scene_seed),
    };
    let scene_eligible = scene_kind != PlaygroundScene::None
        && celebration_kind == CelebrationKind::None
        && !is_birthday
        && inp.playground_enabled;
    // Caption context: only when the scene is actually visible. `finale` pins
    // the catch-beat line for scenes that have one.
    let scene = if scene_eligible { inp.strip_width } else { None }.map(|w| {
        let elapsed = (now as u64).saturating_sub(scene_started_at);
        let finale = scene_duration_secs > 0
            && elapsed >= scene_duration_secs.saturating_sub(crate::playground::FINALE_SECS);
        SceneView { kind: scene_kind, elapsed, duration: scene_duration_secs, seed: scene_seed, width: w, finale }
    });

    let quip = if celebration_kind == CelebrationKind::None && !is_birthday && scene.is_none() {
        quip()
    } else {
        None
    };
    let (speech_source, speech) = select_speech_source(
        celebration_kind,
        is_birthday,
        scene.as_ref().map(|s| (s.kind, s.finale)),
        quip,
        mood,
        now,
    );

    let sprite_lines = if celebration_kind != CelebrationKind::None {
        get_celebration_sprite(celebration_kind, inp.frame)
    } else if is_birthday {
        // Birthday day-long mode reuses the levelup celebration sprite; no new art.
        get_celebration_sprite(CelebrationKind::LevelUp, inp.frame)
    } else {
        get_sprite_lines(state::life_stage(st.level), &st.rarity, mood, inp.frame)
    };
    let speech_attrs: Vec<Attr> = if matches!(
        speech_source,
        SpeechSource::Celebration | SpeechSource::Birthday | SpeechSource::Quip | SpeechSource::Scene
    ) {
        // Celebration, birthday, scene and LLM-quip speech: pink for warmth.
        vec![Attr::Fg(PINK)]
    } else {
        match mood {
            "panic" => vec![Attr::Fg(RED)],
            "alert" => vec![Attr::Fg(YELLOW)],
            "happy" => vec![Attr::Fg(GREEN)],
            "sleep" => vec![Attr::Fg(MAGENTA)],
            "hungry" => vec![Attr::Dim, Attr::Fg(CYAN)],
            _ => vec![Attr::Fg(CYAN)],
        }
    };

    // Sprite colour is rarity-driven outside celebrations / overlays; during
    // celebration, birthday or quip it mirrors the speech colour.
    let rarity_attrs = crate::colors::rarity_attrs(&st.rarity, st.shiny, now);
    let sprite_attrs: Vec<Attr> = if matches!(
        speech_source,
        SpeechSource::Celebration | SpeechSource::Birthday | SpeechSource::Quip
    ) {
        speech_attrs.clone()
    } else {
        rarity_attrs.clone()
    };

    KittenView {
        mood,
        frame: inp.frame,
        is_birthday,
        last_fed_delta,
        speech_source,
        speech,
        speech_attrs,
        sprite_lines,
        sprite_attrs,
        rarity_attrs,
        scene,
    }
}

/// Timers that move on wall-clock time: the celebration countdown (at most one
/// decrement per CELEBRATION_TICK_SECS, gated on the SHARED
/// last_celebration_tick_at so the statusline and `frame --advance` together
/// still tick once) and the playground scheduler. Both re-check on a fresh
/// copy inside locked_update. Moved verbatim from run().
pub(crate) fn advance(
    st: &crate::state::State,
    now_u: u64,
    is_birthday: bool,
    forced_scene: Option<PlaygroundScene>,
    playground_enabled: bool,
) {
    // Celebration counter: decrement at most once per CELEBRATION_TICK_SECS,
    // clear kind when done. The write goes through locked_update on a FRESH
    // copy of state — saving the render-start snapshot here used to silently
    // revert any feed/pet/play increment that landed mid-render.
    if st.levelup_renders_remaining > 0
        && now_u >= st.last_celebration_tick_at.saturating_add(CELEBRATION_TICK_SECS)
    {
        let _ = state::locked_update(|fresh| {
            if fresh.levelup_renders_remaining > 0
                && now_u >= fresh.last_celebration_tick_at.saturating_add(CELEBRATION_TICK_SECS)
            {
                fresh.levelup_renders_remaining -= 1;
                fresh.last_celebration_tick_at = now_u;
                if fresh.levelup_renders_remaining == 0 {
                    fresh.celebration_kind = CelebrationKind::None;
                }
            }
        });
    }

    // Playground scheduler — same locked_update-on-fresh-state discipline as
    // the celebration tick above. Forced scenes bypass state entirely.
    if forced_scene.is_none() && playground_enabled {
        let planned = crate::playground::plan_transition(
            st.scene_kind,
            st.scene_started_at,
            st.scene_duration_secs,
            st.scene_next_at,
            st.scene_last_kind,
            st.celebration_kind != CelebrationKind::None || is_birthday,
            now_u,
        );
        if planned != crate::playground::SceneTransition::None {
            let _ = state::locked_update(|fresh| {
                // Re-plan on the FRESH copy — the snapshot may be stale.
                match crate::playground::plan_transition(
                    fresh.scene_kind,
                    fresh.scene_started_at,
                    fresh.scene_duration_secs,
                    fresh.scene_next_at,
                    fresh.scene_last_kind,
                    fresh.celebration_kind != CelebrationKind::None || is_birthday,
                    now_u,
                ) {
                    crate::playground::SceneTransition::None => {}
                    crate::playground::SceneTransition::Bootstrap { next_at } => {
                        fresh.scene_next_at = next_at;
                    }
                    crate::playground::SceneTransition::End { last_kind, next_at } => {
                        fresh.scene_last_kind = last_kind;
                        fresh.scene_kind = PlaygroundScene::None;
                        fresh.scene_next_at = next_at;
                    }
                    crate::playground::SceneTransition::Start { kind, duration_secs, seed } => {
                        fresh.scene_kind = kind;
                        fresh.scene_started_at = now_u;
                        fresh.scene_duration_secs = duration_secs;
                        fresh.scene_seed = seed;
                    }
                }
            });
        }
    }
}

pub fn run() {
    let t_start = SystemTime::now();
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    if debug::enabled() {
        let _ = fs::create_dir_all(paths::state_dir());
        let _ = fs::write(paths::state_dir().join("last-input.json"), &input);
    }

    let payload: Payload = if input.trim().is_empty() {
        Payload::default()
    } else {
        serde_json::from_str(&input).unwrap_or_default()
    };

    // v3.4.7 — credit any new cost on this render to the kitten's lifetime tally.
    // Runs BEFORE load_or_hatch so the in-memory `st` below picks up our save.
    crate::cost_acc::accumulate_from_payload(
        &payload.session_id,
        payload.cost.total_cost_usd,
    );

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

    let st = crate::hatch::load_or_hatch();
    let (coins, level, kitten_name, rarity, shiny) =
        (st.coins, st.level, st.name.clone(), st.rarity.clone(), st.shiny);
    let now = state::now_secs() as i64;

    let reset_5h_str = time_until_reset(payload.rate_limits.five_hour.resets_at, now);
    let reset_7d_str = time_until_reset(payload.rate_limits.seven_day.resets_at, now);

    let forced_scene = crate::playground::force_scene();
    let playground_enabled = crate::playground::enabled();
    let view = kitten_view(
        &ViewInput {
            st: &st,
            now,
            // One frame flip per second — visible as animation now that the
            // host re-renders on a 1s refreshInterval instead of only on events.
            frame: (now % 2) as u8,
            ctx_pct: payload.context_window.used_percentage,
            forced_scene,
            playground_enabled,
            strip_width: crate::playground::strip_width(),
        },
        crate::quip_cache::consume,
    );
    let mood = view.mood;
    let last_fed_delta = view.last_fed_delta;

    // Truncate the branch so `GIT: {branch}` fits its W3 column exactly, the
    // same way short_cwd shortens the DIR path. `pad_spans` only pads — it never
    // trims — so a long branch name (e.g. feat/v3.8.0-playground-scenes) would
    // otherwise overflow the column and push the card's right border out of
    // alignment, shattering the box. Budget = W3 minus the "GIT: " prefix.
    let git_branch = truncate_tail(&read_git_branch(&cwd), W3 - 5);

    let ctx_int = payload.context_window.used_percentage.round() as i64;
    let rate_int = payload.rate_limits.five_hour.used_percentage.round() as i64;
    let rate_7d_int = payload.rate_limits.seven_day.used_percentage.round() as i64;
    let cost_fmt = format!("{:.2}", payload.cost.total_cost_usd);

    let turn = compute_turn_cost(payload.cost.total_cost_usd, &payload.session_id);
    let turn_cost_fmt = format!("{}.{:02}", turn.whole, turn.cents);

    let ctx_bar = progress_bar(payload.context_window.used_percentage, 11);
    let usage = &payload.context_window.current_usage;
    let ctx_tokens = usage.input_tokens
        + usage.output_tokens
        + usage.cache_creation_input_tokens
        + usage.cache_read_input_tokens;
    let ctx_tok_fmt = format_tokens(ctx_tokens);
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

    // During a scene the cat keeps her rarity/shiny tint even if a quip is
    // active (quips normally re-tint the home sprite pink; there is no home
    // sprite on screen while she plays). attrs_ansi(rarity_attrs) prints
    // exactly like rarity_color (pinned in colors.rs).
    let scene_frame = view.scene.as_ref().map(|sv| {
        crate::playground::render_scene(
            sv.kind,
            sv.elapsed,
            sv.duration,
            sv.seed,
            sv.width,
            view.frame,
            &crate::span::attrs_ansi(&view.rarity_attrs),
        )
    });

    // --- Build the 7 HUD rows ---
    let sep = || [plain(" "), fg("│", PINK), plain(" ")];
    let row3 = |c1: Vec<Span>, c2: Vec<Span>, c3: Vec<Span>| -> Vec<Span> {
        let mut r = vec![plain(" ")];
        r.extend(c1);
        r.extend(sep());
        r.extend(c2);
        r.extend(sep());
        r.extend(c3);
        close_row_spans(r, BOX_WIDTH)
    };

    // H1 — top rail: title, rarity stars, PEAK label centred over column 3.
    let title = vec![
        fg("═╡", PINK),
        plain(" "),
        Span::new("NETRUNNER", &[Attr::Bold, Attr::Fg(CYAN)]),
        plain(" "),
        fg("//", MAGENTA),
        plain(" "),
        fg(kitten_name.as_str(), PINK),
        plain(" "),
        fg("╞", PINK),
    ];
    let peak = peak_spans(local_hour());
    // Stars stay yellow (HUD accent); rarity colour lives on the sprite only.
    let stars = identity::rarity_stars(&rarity, shiny);
    let stars_spans = vec![plain(" "), fg(stars, YELLOW), plain(" ")];
    let (pre_stars, mid_stars, post_peak) =
        compute_h1_layout(spans_width(&title), spans_width(&stars_spans), spans_width(&peak));
    let mut h1 = vec![fg("╔", PINK)];
    h1.extend(title);
    h1.push(fg("═".repeat(pre_stars), PINK));
    h1.extend(stars_spans);
    h1.push(fg("═".repeat(mid_stars), PINK));
    h1.extend(peak);
    h1.push(fg(format!("{}╗", "═".repeat(post_peak)), PINK));

    // H2 — MODEL | DIR | GIT
    let h2 = row3(
        pad_spans(vec![dim("MODEL:"), plain(" "), fg(model.as_str(), CYAN)], W1),
        if cwd.is_empty() {
            pad_spans(vec![], W2)
        } else {
            pad_spans(vec![dim("DIR:"), plain(" "), fg(short_path.as_str(), CYAN)], W2)
        },
        if git_branch.is_empty() {
            pad_spans(vec![], W3)
        } else {
            pad_spans(vec![dim("GIT:"), plain(" "), fg(git_branch.as_str(), CYAN)], W3)
        },
    );

    // H3 — USAGE: 5h | 7d | CTX
    let h3 = row3(
        pad_spans(
            vec![dim("USAGE:"), plain(" "), dim("5h:"), plain(" "), fg(r5h_bar.as_str(), YELLOW),
                 plain(" "), bold(format!("{rate_int}%"))],
            W1,
        ),
        pad_spans(
            vec![dim("7d:"), plain(" "), fg(r7d_bar.as_str(), YELLOW), plain(" "),
                 bold(format!("{rate_7d_int}%"))],
            W2,
        ),
        pad_spans(
            vec![dim("CTX:"), plain(" "), fg(ctx_bar.as_str(), CYAN), plain(" "),
                 bold(ctx_tok_fmt.as_str()), plain(" "), dim(format!("({ctx_int}%)"))],
            W3,
        ),
    );

    // H4 — RESET 5h | 7d | Cost
    let h4 = row3(
        pad_spans(
            vec![dim("RESET:"), plain(" "), dim("5h"), plain(" "), fg(reset_5h_str.as_str(), YELLOW)],
            W1,
        ),
        pad_spans(vec![dim("7d"), plain(" "), fg(reset_7d_str.as_str(), YELLOW)], W2),
        pad_spans(vec![dim("Cost:"), plain(" "), fg(format!("${turn_cost_fmt}"), YELLOW)], W3),
    );

    // H5 — LINES | TIME | Total
    let la = payload.cost.total_lines_added;
    let lr = payload.cost.total_lines_removed;
    let h5 = row3(
        pad_spans(
            vec![dim("LINES:"), plain(" "), fg(format!("+{la}"), GREEN), plain("/"),
                 fg(format!("-{lr}"), RED)],
            W1,
        ),
        pad_spans(vec![dim("TIME:"), plain(format!(" {duration}"))], W2),
        pad_spans(vec![dim("Total:"), plain(" "), fg(format!("${cost_fmt}"), YELLOW)], W3),
    );

    // H6 — LV | COINS (anchored) | MOOD | FED (anchored) | BOND
    let effective_bond = crate::bond::effective_bond(
        st.bond,
        crate::bond::last_interaction_at(&st),
        now as u64,
    );
    let bond_bar = crate::render::progress_bar(effective_bond as f64, 10);
    let h6 = row3(
        anchor_spans(
            vec![dim("LV:"), plain(" "), Span::new(level.to_string(), &[Attr::Fg(PINK), Attr::Bold])],
            vec![dim("COINS:"), plain(" "), fg(coins.to_string(), YELLOW)],
            W1,
        ),
        anchor_spans(
            vec![dim("MOOD:"), plain(" "), fg(mood, MAGENTA)],
            vec![dim("FED:"), plain(format!(" {fed_ago}"))],
            W2,
        ),
        pad_spans(
            vec![dim("BOND:"), plain(" "), fg("\u{2665}", PINK), plain(" "), fg(bond_bar, PINK),
                 plain(" "), bold(effective_bond.to_string())],
            W3,
        ),
    );

    // H7 — bottom rail (two pink spans, as before).
    let h7 = vec![fg("╚", PINK), fg(format!("{}╝", "═".repeat(BOX_WIDTH)), PINK)];

    // --- Compose final output with sprite column + speech bubble ---
    let rows = [h1, h2, h3, h4, h5, h6, h7];
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut buf = String::with_capacity(4096);

    for (i, row) in rows.iter().enumerate() {
        let row = to_ansi(row);
        if i == 6 {
            let bubble = to_ansi(&[Span::new(format!("\"{}\"", view.speech), &view.speech_attrs)]);
            buf.push_str(&format!("{row}   {bubble}\n"));
        } else if let Some(sf) = &scene_frame {
            // Scene rows carry their own colours + trailing reset (Canvas::emit).
            buf.push_str(&format!("{row}   {}\n", sf.rows[i]));
        } else {
            let line = view.sprite_lines.get(i).map(String::as_str).unwrap_or("");
            let sprite = to_ansi(&[Span::new(pad_sprite(line, SW), &view.sprite_attrs)]);
            buf.push_str(&format!("{row}   {sprite}\n"));
        }
    }

    let _ = out.write_all(buf.as_bytes());

    advance(&st, now as u64, view.is_birthday, forced_scene, playground_enabled);

    if debug::enabled() {
        let elapsed_us = SystemTime::now()
            .duration_since(t_start)
            .map(|d| d.as_micros())
            .unwrap_or(0);
        debug::log(&format!(
            "render mood={mood} fed_delta={last_fed_delta} ctx={ctx_int} render_us={elapsed_us}"
        ));
    }
}

fn normalize_windows_path(cwd: &str) -> String {
    // C:\foo\bar -> C:/foo/bar. Forward slashes so Rust's fs handles it on
    // Windows, and the literal drive-letter prefix stays intact for the
    // HOME-starts-with check in short_cwd. Avoids MSYS `/c/...` form since
    // Windows APIs don't resolve that.
    let bytes = cwd.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && bytes[2] == b'\\'
    {
        cwd.replace('\\', "/")
    } else {
        cwd.to_string()
    }
}

fn read_git_branch(cwd: &str) -> String {
    if cwd.is_empty() {
        return String::new();
    }
    let mut dir = std::path::PathBuf::from(cwd);
    loop {
        let head_path = dir.join(".git").join("HEAD");
        if let Ok(contents) = fs::read_to_string(&head_path) {
            let line = contents.lines().next().unwrap_or("").trim();
            if let Some(branch) = line.strip_prefix("ref: refs/heads/") {
                return branch.to_string();
            }
            return String::new();
        }
        if !dir.pop() {
            return String::new();
        }
    }
}

fn local_hour() -> u32 {
    match state::now_override() {
        // Deterministic PEAK label under PURR_NOW (tests): UTC hour, no `date` spawn.
        Some(n) => ((n / 3600) % 24) as u32,
        None => os_local_hour(),
    }
}

#[cfg(windows)]
fn os_local_hour() -> u32 {
    // Win32 type name — intentional upper-case for FFI fidelity.
    #[allow(clippy::upper_case_acronyms)]
    #[repr(C)]
    struct SYSTEMTIME {
        year: u16, month: u16, dow: u16, day: u16,
        hour: u16, minute: u16, second: u16, ms: u16,
    }
    unsafe extern "system" {
        fn GetLocalTime(lp: *mut SYSTEMTIME);
    }
    let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut st); }
    st.hour as u32
}

#[cfg(not(windows))]
fn os_local_hour() -> u32 {
    // std has no timezone database; ask the system. Falls back to UTC (the
    // old code hardcoded UTC+1, which was wrong everywhere else).
    if let Ok(out) = std::process::Command::new("date").arg("+%H").output()
        && let Ok(s) = String::from_utf8(out.stdout)
        && let Ok(h) = s.trim().parse::<u32>()
    {
        return h % 24;
    }
    ((state::now_secs() / 3600) % 24) as u32
}

fn peak_spans(hour: u32) -> Vec<Span> {
    let (label, range) = peak_period(hour);
    vec![
        fg("╡", PINK), plain(" "), fg(label, CYAN), plain(" "), fg(range, YELLOW), plain(" "), fg("╞", PINK),
    ]
}

/// Compute the three `═` rail spans that bracket the stars and peak label in H1.
/// Sum of (title_len, pre_stars, stars_len, mid_stars, peak_len, post_peak) equals
/// BOX_WIDTH for all realistic names. Pathologically long names (rare: requires
/// renaming past ~45 chars) can cause the rail to overflow — this is an accepted
/// limitation for now; the pre-v3.4.5 HUD had the same issue minus the stars.
fn compute_h1_layout(title_len: usize, stars_len: usize, peak_len: usize)
    -> (usize, usize, usize) {
    let w3_center = 1 + W1 + 3 + W2 + 3 + W3 / 2;
    let peak_start = w3_center.saturating_sub(peak_len / 2);

    let gap = peak_start.saturating_sub(title_len);
    let pre_stars = gap.saturating_sub(stars_len) / 2;
    let mid_stars = gap.saturating_sub(stars_len + pre_stars);
    let post_peak = BOX_WIDTH
        .saturating_sub(title_len + pre_stars + stars_len + mid_stars + peak_len);

    (pre_stars, mid_stars, post_peak)
}

fn peak_period(hour: u32) -> (&'static str, &'static str) {
    // 14..=19 == 2:00PM-7:59PM, matching the displayed "2PM-8PM" exactly.
    // (The old 14..=20 bound plus an "9PM-3PM" off-peak label claimed
    // 2PM-3PM for both ranges.)
    if (14..=19).contains(&hour) {
        ("PEAK", "2PM-8PM")
    } else {
        ("OFF PEAK", "8PM-2PM")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SpeechSource {
    Celebration,
    Birthday,
    Scene,
    Quip,
    Mood,
}

impl SpeechSource {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            SpeechSource::Celebration => "celebration",
            SpeechSource::Birthday => "birthday",
            SpeechSource::Scene => "scene",
            SpeechSource::Quip => "quip",
            SpeechSource::Mood => "mood",
        }
    }
}

pub(crate) fn select_speech_source(
    celebration_kind: crate::state::CelebrationKind,
    is_birthday: bool,
    scene: Option<(PlaygroundScene, bool)>,
    quip: Option<String>,
    mood: &str,
    now: i64,
) -> (SpeechSource, String) {
    if celebration_kind != crate::state::CelebrationKind::None {
        return (SpeechSource::Celebration, compute_celebration_speech(celebration_kind, now).to_string());
    }
    if is_birthday {
        return (SpeechSource::Birthday, crate::speech::compute_birthday_speech(now).to_string());
    }
    if let Some((kind, finale)) = scene {
        return (
            SpeechSource::Scene,
            crate::speech::compute_scene_speech(kind, now, finale).to_string(),
        );
    }
    if let Some(q) = quip {
        return (SpeechSource::Quip, q);
    }
    (SpeechSource::Mood, compute_speech(mood, now).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peak_period_peak_hours() {
        assert_eq!(peak_period(14), ("PEAK", "2PM-8PM"));
        assert_eq!(peak_period(17), ("PEAK", "2PM-8PM"));
        assert_eq!(peak_period(19), ("PEAK", "2PM-8PM"));
    }

    #[test]
    fn test_peak_period_off_peak_hours() {
        assert_eq!(peak_period(0),  ("OFF PEAK", "8PM-2PM"));
        assert_eq!(peak_period(13), ("OFF PEAK", "8PM-2PM"));
        assert_eq!(peak_period(20), ("OFF PEAK", "8PM-2PM"));
        assert_eq!(peak_period(23), ("OFF PEAK", "8PM-2PM"));
    }

    #[test]
    fn test_peak_period_ranges_do_not_overlap() {
        // The displayed ranges must partition the day: every hour belongs to
        // exactly the range its label claims.
        for hour in 0..24u32 {
            let (label, range) = peak_period(hour);
            if (14..=19).contains(&hour) {
                assert_eq!((label, range), ("PEAK", "2PM-8PM"), "hour {hour}");
            } else {
                assert_eq!((label, range), ("OFF PEAK", "8PM-2PM"), "hour {hour}");
            }
        }
    }

    #[test]
    fn test_peak_label_plain_content() {
        let plain = crate::span::to_plain(&peak_spans(17));
        assert_eq!(plain, "╡ PEAK 2PM-8PM ╞");
        let plain = crate::span::to_plain(&peak_spans(8));
        assert_eq!(plain, "╡ OFF PEAK 8PM-2PM ╞");
    }

    #[test]
    fn test_peak_label_plain_len() {
        let plain = crate::span::to_plain(&peak_spans(17));
        assert_eq!(plain.chars().count(), 16);
        let plain = crate::span::to_plain(&peak_spans(8));
        assert_eq!(plain.chars().count(), 20);
    }

    fn build_h1_parts(name: &str, hour: u32, rarity: &str, shiny: bool)
        -> (usize, usize, usize, usize, usize, usize) {
        let title_plain = format!("═╡ NETRUNNER // {} ╞", name);
        let peak_plain = crate::span::to_plain(&peak_spans(hour));
        let stars = crate::identity::rarity_stars(rarity, shiny);
        let stars_plain = format!(" {stars} ");

        let title_len = title_plain.chars().count();
        let peak_len = peak_plain.chars().count();
        let stars_len = stars_plain.chars().count();

        let (pre_stars, mid_stars, post_peak) =
            compute_h1_layout(title_len, stars_len, peak_len);

        (title_len, pre_stars, stars_len, mid_stars, peak_len, post_peak)
    }

    #[test]
    fn test_h1_rail_total_len_peak_common_non_shiny() {
        let (t, a, s, b, p, c) = build_h1_parts("Thalia", 17, "common", false);
        assert_eq!(t + a + s + b + p + c, BOX_WIDTH);
    }

    #[test]
    fn test_h1_rail_total_len_off_peak_common_non_shiny() {
        let (t, a, s, b, p, c) = build_h1_parts("Thalia", 8, "common", false);
        assert_eq!(t + a + s + b + p + c, BOX_WIDTH);
    }

    #[test]
    fn test_h1_rail_total_len_peak_legendary_shiny() {
        // shiny adds a ✦ prefix → stars_plain is 1 char longer.
        let (t, a, s, b, p, c) = build_h1_parts("Thalia", 17, "legendary", true);
        assert_eq!(t + a + s + b + p + c, BOX_WIDTH);
    }

    #[test]
    fn test_h1_rail_total_len_off_peak_legendary_shiny() {
        let (t, a, s, b, p, c) = build_h1_parts("Thalia", 8, "legendary", true);
        assert_eq!(t + a + s + b + p + c, BOX_WIDTH);
    }

    #[test]
    fn select_speech_celebration_beats_quip() {
        let got = super::select_speech_source(
            crate::state::CelebrationKind::LevelUp,
            /* is_birthday */ false,
            /* scene */ None,
            /* quip */ Some("ignored".into()),
            "calm",
            0,
        );
        assert_eq!(got.0, super::SpeechSource::Celebration);
    }

    #[test]
    fn select_speech_birthday_beats_quip() {
        let got = super::select_speech_source(
            crate::state::CelebrationKind::None,
            /* is_birthday */ true,
            /* scene */ None,
            Some("ignored".into()),
            "calm",
            0,
        );
        assert_eq!(got.0, super::SpeechSource::Birthday);
    }

    #[test]
    fn select_speech_quip_beats_mood() {
        let got = super::select_speech_source(
            crate::state::CelebrationKind::None,
            false,
            None,
            Some("the cat watches".into()),
            "calm",
            0,
        );
        assert_eq!(got.0, super::SpeechSource::Quip);
        assert_eq!(got.1, "the cat watches");
    }

    #[test]
    fn select_speech_falls_back_to_mood_when_no_quip() {
        let got = super::select_speech_source(
            crate::state::CelebrationKind::None,
            false,
            None,
            None,
            "calm",
            0,
        );
        assert_eq!(got.0, super::SpeechSource::Mood);
    }

    #[test]
    fn scene_outranks_quip_and_mood() {
        let (src, text) = select_speech_source(
            CelebrationKind::None,
            false,
            Some((PlaygroundScene::BallChase, false)),
            Some("cached quip".to_string()),
            "calm",
            0,
        );
        assert_eq!(src, SpeechSource::Scene);
        assert_eq!(text, "after it!");
    }

    #[test]
    fn celebration_outranks_scene() {
        let (src, _) = select_speech_source(
            CelebrationKind::LevelUp,
            false,
            Some((PlaygroundScene::Run, false)),
            None,
            "calm",
            0,
        );
        assert_eq!(src, SpeechSource::Celebration);
    }

    #[test]
    fn scene_finale_caption_reaches_the_speech_line() {
        let (_, text) = select_speech_source(
            CelebrationKind::None,
            false,
            Some((PlaygroundScene::BallChase, true)),
            None,
            "calm",
            0,
        );
        assert_eq!(text, "gotcha!!");
    }

    fn view_input(st: &crate::state::State, now: i64) -> ViewInput<'_> {
        ViewInput {
            st, now, frame: (now % 2) as u8, ctx_pct: 10.0,
            forced_scene: None, playground_enabled: true, strip_width: Some(60),
        }
    }

    #[test]
    fn kitten_view_never_calls_quip_behind_a_celebration() {
        let mut st = crate::state::State::base(1_000);
        st.celebration_kind = CelebrationKind::LevelUp;
        st.levelup_renders_remaining = 2;
        let called = std::cell::Cell::new(false);
        let v = kitten_view(&view_input(&st, 1_800_000_000), || { called.set(true); Some("q".into()) });
        assert!(!called.get());
        assert_eq!(v.speech_source, SpeechSource::Celebration);
    }

    #[test]
    fn kitten_view_uses_quip_when_idle_and_tints_sprite_pink() {
        let st = crate::state::State::base(1_000);
        let v = kitten_view(&view_input(&st, 1_800_000_000), || Some("the cat watches".into()));
        assert_eq!((v.speech_source, v.speech.as_str()), (SpeechSource::Quip, "the cat watches"));
        assert_eq!(v.sprite_attrs, vec![Attr::Fg(PINK)]);
    }

    #[test]
    fn kitten_view_scene_needs_a_strip() {
        let mut st = crate::state::State::base(1_000);
        st.scene_kind = PlaygroundScene::BallChase;
        st.scene_started_at = 1_800_000_000 - 5;
        st.scene_duration_secs = 30;
        let v = kitten_view(&ViewInput { strip_width: Some(40), ..view_input(&st, 1_800_000_000) }, || None);
        let sv = v.scene.expect("scene visible");
        assert_eq!((sv.elapsed, sv.width, sv.finale), (5, 40, false));
        let called = std::cell::Cell::new(false);
        let v = kitten_view(&ViewInput { strip_width: None, ..view_input(&st, 1_800_000_000) }, || { called.set(true); None });
        assert!(v.scene.is_none());
        assert!(called.get(), "no strip → no scene → quip may show");
    }

    #[test]
    fn kitten_view_forced_scene_loops_on_30s() {
        let st = crate::state::State::base(1_000);
        let v = kitten_view(&ViewInput { forced_scene: Some(PlaygroundScene::Run), ..view_input(&st, 1_800_000_017) }, || None);
        let sv = v.scene.unwrap();
        assert_eq!((sv.kind, sv.elapsed, sv.duration, sv.seed), (PlaygroundScene::Run, 17, 30, 42));
    }

    #[test]
    fn kitten_view_frame_comes_from_input() {
        let st = crate::state::State::base(1_000);
        let v = kitten_view(&ViewInput { frame: 1, ..view_input(&st, 1_800_000_000) }, || None);
        assert_eq!(v.frame, 1);
    }

    #[test]
    fn speech_source_names() {
        assert_eq!(SpeechSource::Celebration.as_str(), "celebration");
        assert_eq!(SpeechSource::Mood.as_str(), "mood");
    }
}
