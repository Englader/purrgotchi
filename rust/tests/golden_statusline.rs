//! Byte-exact goldens of the classic statusline (Task 2 of the v4.0.0 Phase 0
//! plan). Recorded from the pre-refactor binary; every later task must keep
//! them identical.
#![cfg(unix)]

mod common;
use common::{assert_golden, base_payload, base_state, merge, strip_ansi, Sandbox, DAY, NOW};
use serde_json::{json, Value};

const PINK: &str = "\x1b[38;2;234;0;217m";

struct Case {
    dir: &'static str,         // golden subdirectory
    name: &'static str,
    now: u64,
    state: Value,               // patch over base_state()
    payload: Option<Value>,     // patch over base_payload(); None = empty stdin
    branch: Option<&'static str>,
    cwd_outside_home: bool,     // cwd = <root>/outside/a-very-long-directory-name-for-truncation
    anchor: Option<&'static str>,
    quip: Option<Value>,
    env: &'static [(&'static str, &'static str)],
    must_contain: &'static [&'static str], // checked on the ANSI-stripped output
    raw_contain: &'static [&'static str],  // raw-byte colour escapes, checked on the classic (PURR_PIXELS=0) output
}

fn case(name: &'static str) -> Case {
    Case {
        dir: "statusline", name, now: NOW, state: json!({}), payload: Some(json!({})), branch: Some("main"),
        cwd_outside_home: false, anchor: None, quip: None, env: &[], must_contain: &[], raw_contain: &[],
    }
}

fn cases() -> Vec<Case> {
    let scene = json!({ "scene_kind": "BallChase", "scene_started_at": NOW - 5,
                        "scene_duration_secs": 30, "scene_seed": 7 });
    let quip = json!({ "quip": "the cat watches", "generated_at": NOW - 10,
                       "renders_remaining": 6, "last_render_at": 0 });
    vec![
        Case { anchor: Some("1.25\n1000.000000\n"),
               must_contain: &["OFF PEAK", "Cost: $2.17", "Total: $3.42", "GIT: main", "DIR: ~/purrgotchi", "MOOD: calm"],
               ..case("calm_common_offpeak") },
        Case { now: NOW + 7 * 3600 + 1,
               state: json!({ "rarity": "rare", "level": 20, "total_tool_calls": 960, "last_milestone_celebrated": 500 }),
               payload: Some(json!({ "context_window": { "used_percentage": 55.0 } })),
               branch: Some("feat/v4.0.0-claude-code-mod-json-surface"),
               must_contain: &["╡ PEAK 2PM-8PM ╞", "GIT: ...ode-mod-json-surface", "MOOD: alert"],
               ..case("alert_rare_young_peak_frame1") },
        Case { state: json!({ "rarity": "legendary", "shiny": true, "level": 60, "total_tool_calls": 2950,
                              "last_milestone_celebrated": 2000 }),
               payload: Some(json!({ "context_window": { "used_percentage": 92.0 } })),
               must_contain: &["MOOD: panic", "✦★★★★★"],
               ..case("panic_legendary_shiny_hue0_even") },
        Case { now: NOW + 3,
               state: json!({ "rarity": "legendary", "shiny": true, "level": 60, "total_tool_calls": 2950,
                              "last_milestone_celebrated": 2000 }),
               ..case("legendary_shiny_hue3_odd") },
        Case { state: json!({ "rarity": "uncommon", "last_fed_at": NOW - 4000 }),
               must_contain: &["MOOD: hungry", "FED: 1h ago"], ..case("hungry_uncommon") },
        Case { state: json!({ "rarity": "epic", "last_fed_at": NOW - 30_000 }),
               must_contain: &["MOOD: sleep"], ..case("sleep_epic") },
        Case { state: json!({ "last_pet_at": NOW - 60, "bond": 80 }),
               must_contain: &["MOOD: happy", "80"], ..case("happy_petted_bond80") },
        Case { state: json!({ "celebration_kind": "LevelUp", "levelup_renders_remaining": 2,
                              "last_celebration_tick_at": NOW }),
               ..case("celebration_levelup") },
        Case { now: NOW + 1,
               state: json!({ "celebration_kind": "EvolvingYoung", "levelup_renders_remaining": 6,
                              "level": 10, "total_tool_calls": 450 }),
               ..case("celebration_evolving_young_frame1") },
        Case { state: json!({ "birth_time": NOW - 365 * DAY }), ..case("birthday") },
        Case { state: scene.clone(), env: &[("COLUMNS", "160")], ..case("scene_ballchase_mid") },
        Case { state: { let mut s = scene.clone(); merge(&mut s, json!({ "scene_started_at": NOW - 27 })); s },
               env: &[("COLUMNS", "160")], must_contain: &["gotcha!!"], ..case("scene_ballchase_finale") },
        Case { env: &[("PURR_FORCE_SCENE", "run"), ("COLUMNS", "200")], ..case("scene_forced_run") },
        Case { state: scene.clone(), env: &[("COLUMNS", "120")], ..case("scene_too_narrow_shows_sprite") },
        Case { state: scene.clone(), env: &[("COLUMNS", "160"), ("PURR_PLAYGROUND", "0")],
               ..case("scene_disabled_shows_sprite") },
        Case { quip: Some(quip.clone()), must_contain: &["\"the cat watches\""], ..case("quip_overlay") },
        Case { quip: Some(quip.clone()),
               state: json!({ "celebration_kind": "LevelUp", "levelup_renders_remaining": 2,
                              "last_celebration_tick_at": NOW }),
               ..case("quip_hidden_behind_celebration") },
        Case { payload: None, branch: None, must_contain: &["MODEL: Claude", "Total: $0.00"],
               ..case("empty_payload") },
        Case { cwd_outside_home: true, must_contain: &["DIR: ...ry-name-for-truncation"],
               ..case("long_cwd_outside_home") },
        Case { state: json!({ "name": "Princess Thalia Bartholomew Fluffington-Whiskerby III" }),
               ..case("long_name_overflows_rail") },
    ]
    .into_iter()
    .chain(extra_cases())
    .collect()
}

/// Opt-in pixel-cat cases (goldens under `statusline_px/`).
fn px_cases() -> Vec<Case> {
    let px = |name| Case { dir: "statusline_px", env: &[("PURR_PIXELS", "1")], ..case(name) };
    vec![
        Case { env: &[("PURR_PIXELS", "test"), ("COLUMNS", "160")], ..px("pixel_test_card") },
        // Idle life. Times are picked to show distinct poses: walking (block start),
        // running, curled with a Z, grooming, loafing.
        Case { must_contain: &["MOOD: calm"], ..px("px_life_calm") },
        Case { now: NOW + 12, state: json!({ "rarity": "epic", "last_fed_at": NOW - 30_000 }),
               must_contain: &["MOOD: sleep"], ..px("px_life_sleep_epic") },
        Case { now: NOW + 9, payload: Some(json!({ "context_window": { "used_percentage": 92.0 } })),
               must_contain: &["MOOD: panic"], ..px("px_life_panic") },
        Case { now: NOW + 7 * 3600 + 1,
               state: json!({ "rarity": "rare", "level": 20, "total_tool_calls": 960, "last_milestone_celebrated": 500 }),
               payload: Some(json!({ "context_window": { "used_percentage": 55.0 } })),
               branch: Some("feat/v4.0.0-claude-code-mod-json-surface"),
               must_contain: &["MOOD: alert"], ..px("px_life_young_rare_frame1") },
        Case { now: NOW + 20, env: &[("PURR_PIXELS", "fg")], ..px("px_life_fg_mode") },
        Case { now: NOW + 9, env: &[("PURR_PIXELS", "1"), ("PURR_FORCE_POSE", "groom")], ..px("px_life_forced_groom") },
        // Too narrow for a stage: the classic sprite, byte for byte (checked below).
        Case { anchor: Some("1.25\n1000.000000\n"), env: &[("PURR_PIXELS", "1"), ("COLUMNS", "120")],
               ..px("px_life_narrow_is_ascii") },
    ]
    .into_iter()
    .chain(px_scene_cases())
    .chain(px_celebrate_cases())
    .collect()
}

/// Pixel celebration and birthday overlays: the states of the classic
/// `celebration_*` cases (EvolvingYoung on its odd, white-flash second).
fn px_celebrate_cases() -> Vec<Case> {
    let px = |name, state| Case { dir: "statusline_px", state, env: &[("PURR_PIXELS", "1"), ("COLUMNS", "160")], ..case(name) };
    let quip = json!({ "quip": "the cat watches", "generated_at": NOW - 10, "renders_remaining": 6, "last_render_at": 0 });
    vec![
        px("px_celebrate_levelup", celebration("LevelUp")),
        Case { now: NOW + 1, ..px("px_celebrate_evolving_young_flash", { let mut s = celebration("EvolvingYoung"); merge(&mut s, json!({ "level": 10, "total_tool_calls": 450 })); s }) },
        px("px_celebrate_evolving_adult", { let mut s = celebration("EvolvingAdult"); merge(&mut s, json!({ "level": 30, "total_tool_calls": 1500 })); s }),
        px("px_celebrate_streak", celebration("Streak")),
        px("px_celebrate_milestone_centurion", celebration("MilestoneCenturion")),
        px("px_celebrate_milestone_veteran", celebration("MilestoneVeteran")),
        px("px_celebrate_milestone_master", celebration("MilestoneMaster")),
        px("px_celebrate_milestone_legendary", celebration("MilestoneLegendary")),
        px("px_celebrate_greeting", celebration("Greeting")),
        Case { now: NOW + 1, ..px("px_celebrate_greeting_wave", celebration("Greeting")) },
        px("px_celebrate_random_drop", celebration("RandomDrop")),
        px("px_birthday", json!({ "birth_time": NOW - 365 * DAY })),
        // A quip shows in the speech row (pink); the pixel cat is the idle one.
        Case { quip: Some(quip), must_contain: &["\"the cat watches\""], ..px("px_quip_keeps_pixel_cat", json!({})) },
    ]
}

/// Pixel playground scenes: the states of the classic `scene_*` cases.
fn px_scene_cases() -> Vec<Case> {
    let scene = |kind: &str, ago: u64| json!({ "scene_kind": kind, "scene_started_at": NOW - ago,
                                              "scene_duration_secs": 30, "scene_seed": 7 });
    let px = |name, state| Case { dir: "statusline_px", state, env: &[("PURR_PIXELS", "1"), ("COLUMNS", "160")], ..case(name) };
    vec![
        px("px_scene_run_mid", scene("Run", 9)),
        px("px_scene_zoomies_mid", scene("Zoomies", 9)),
        px("px_scene_ballchase_mid", scene("BallChase", 5)),
        Case { must_contain: &["gotcha!!"], ..px("px_scene_ballchase_finale", scene("BallChase", 27)) },
        px("px_scene_butterfly_mid", scene("ButterflyChase", 9)),
        px("px_scene_roll_mid", scene("Roll", 9)),
        px("px_scene_yarn_mid", scene("YarnPush", 9)),
        Case { env: &[("PURR_PIXELS", "1"), ("COLUMNS", "160"), ("PURR_PLAYGROUND", "0")],
               ..px("px_scene_disabled_still_shows_life", scene("BallChase", 5)) },
    ]
}

fn legendary() -> Value {
    json!({ "rarity": "legendary", "shiny": true, "level": 60,
            "total_tool_calls": 2950, "last_milestone_celebrated": 2000 })
}

fn celebration(kind: &str) -> Value {
    json!({ "celebration_kind": kind, "levelup_renders_remaining": 2, "last_celebration_tick_at": NOW })
}

/// Added after the first 20, still recorded from the pre-refactor binary.
fn extra_cases() -> Vec<Case> {
    // Mid-scene state for the scenes not covered by the BallChase cases.
    let scene = |kind: &str, ago: u64| json!({ "scene_kind": kind, "scene_started_at": NOW - ago,
                                              "scene_duration_secs": 30, "scene_seed": 7 });
    let wide: &'static [(&'static str, &'static str)] = &[("COLUMNS", "160")];
    vec![
        // Legendary rainbow hues 1, 2, 4, 5 (0 and 3 are covered above). RAINBOW_HUES order:
        // red, yellow, green, cyan, blue, magenta.
        Case { now: NOW + 1, state: legendary(),
               raw_contain: &["\x1b[38;2;243;243;27m"], ..case("legendary_shiny_hue1_yellow") },
        Case { now: NOW + 2, state: legendary(),
               raw_contain: &["\x1b[38;2;87;236;166m"], ..case("legendary_shiny_hue2_green") },
        Case { now: NOW + 4, state: legendary(),
               raw_contain: &["\x1b[38;2;80;150;255m"], ..case("legendary_shiny_hue4_blue") },
        Case { now: NOW + 5, state: legendary(),
               raw_contain: &["\x1b[38;2;113;28;145m"], ..case("legendary_shiny_hue5_magenta") },
        // Celebration kinds (LevelUp and EvolvingYoung are covered above).
        Case { state: { let mut s = celebration("EvolvingAdult"); merge(&mut s, json!({ "level": 30, "total_tool_calls": 1500 })); s },
               ..case("celebration_evolving_adult") },
        Case { state: celebration("Streak"), ..case("celebration_streak") },
        Case { state: celebration("MilestoneCenturion"), ..case("celebration_milestone_centurion") },
        Case { state: celebration("MilestoneVeteran"), ..case("celebration_milestone_veteran") },
        Case { state: celebration("MilestoneMaster"), ..case("celebration_milestone_master") },
        Case { state: celebration("MilestoneLegendary"), ..case("celebration_milestone_legendary") },
        Case { state: celebration("Greeting"), ..case("celebration_greeting") },
        Case { state: celebration("RandomDrop"), ..case("celebration_random_drop") },
        // Playground scenes mid-flight (BallChase is covered above) + the butterfly finale.
        Case { state: scene("Zoomies", 9), env: wide, ..case("scene_zoomies_mid") },
        Case { state: scene("ButterflyChase", 9), env: wide, ..case("scene_butterfly_mid") },
        Case { state: scene("ButterflyChase", 27), env: wide, ..case("scene_butterfly_finale") },
        Case { state: scene("Roll", 9), env: wide, ..case("scene_roll_mid") },
        Case { state: scene("YarnPush", 9), env: wide, ..case("scene_yarn_mid") },
        // Non-ASCII names: pins the current chars().count()-based padding, whatever it is.
        Case { state: json!({ "name": "Zoë Élan" }), must_contain: &["Zoë Élan"], ..case("name_accented_latin") },
        Case { state: json!({ "name": "猫ちゃん🐱" }), must_contain: &["猫ちゃん🐱"], ..case("name_cjk_emoji") },
        // Speech longer than the bubble, ctx >= 100 %, M-range tokens, >= 1 h duration.
        Case { quip: Some(json!({ "quip": "the cat has a great many thoughts about this refactor and she intends to share every single one of them with you right now, at length, whether or not the bubble can hold them all",
                                  "generated_at": NOW - 10, "renders_remaining": 6, "last_render_at": 0 })),
               ..case("quip_longer_than_bubble") },
        Case { payload: Some(json!({ "context_window": { "used_percentage": 104.0 } })),
               must_contain: &["MOOD: panic"], ..case("ctx_over_100") },
        Case { payload: Some(json!({ "context_window": { "used_percentage": 100.0,
                   "current_usage": { "input_tokens": 1_150_000, "output_tokens": 62_000,
                                      "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0 } } })),
               ..case("ctx_100_tokens_millions") },
        Case { payload: Some(json!({ "cost": { "total_duration_ms": 5_025_000 } })),
               must_contain: &["1h 23m 45s"], ..case("duration_over_one_hour") },
    ]
}

fn run_case(c: &Case) -> (Vec<u8>, Sandbox) {
    run_case_with(c, &[])
}

/// `extra` is appended after `c.env`, so it wins.
fn run_case_with(c: &Case, extra: &[(&str, &str)]) -> (Vec<u8>, Sandbox) {
    let sb = Sandbox::new(c.name);
    let mut st = base_state();
    merge(&mut st, c.state.clone());
    sb.write_state(&st);

    let cwd = if c.cwd_outside_home {
        sb.root.join("outside").join("a-very-long-directory-name-for-truncation")
    } else {
        sb.home.join("purrgotchi")
    };
    std::fs::create_dir_all(cwd.join(".git")).unwrap();
    if let Some(b) = c.branch {
        std::fs::write(cwd.join(".git").join("HEAD"), format!("ref: refs/heads/{b}\n")).unwrap();
    }
    if let Some(a) = c.anchor {
        std::fs::write(sb.state.join("turn-start-cost.txt"), a).unwrap();
    }
    if let Some(q) = &c.quip {
        std::fs::write(sb.state.join("llm-quip.json"), serde_json::to_string(q).unwrap()).unwrap();
    }
    let stdin = match &c.payload {
        None => String::new(),
        Some(p) => {
            let mut full = base_payload(cwd.to_str().unwrap());
            merge(&mut full, p.clone());
            serde_json::to_string(&full).unwrap()
        }
    };
    let now = c.now.to_string();
    let mut env: Vec<(&str, &str)> = vec![("PURR_NOW", now.as_str())];
    env.extend_from_slice(c.env);
    env.extend_from_slice(extra);
    let out = sb.run(&["statusline"], &stdin, &env);
    assert!(out.status.success(), "{}: exit {:?} stderr {}", c.name, out.status, String::from_utf8_lossy(&out.stderr));
    (out.stdout, sb)
}

#[test]
fn statusline_goldens() {
    for c in cases().into_iter().chain(px_cases()) {
        let (bytes, sb) = run_case(&c);
        let text = String::from_utf8(bytes.clone()).expect("utf8");
        // Structural sanity, independent of the recorded bytes.
        assert_eq!(text.matches('\n').count(), 7, "{}: expected 7 rows", c.name);
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].starts_with(&format!("{PINK}╔")), "{}: row 0", c.name);
        for (i, l) in lines.iter().enumerate().take(6).skip(1) {
            assert!(l.starts_with(&format!("{PINK}║")), "{}: row {i}", c.name);
        }
        assert!(lines[6].starts_with(&format!("{PINK}╚")), "{}: row 6", c.name);
        let plain = strip_ansi(&text);
        for needle in c.must_contain {
            assert!(plain.contains(needle), "{}: missing {needle:?} in\n{plain}", c.name);
        }
        if c.name == "quip_hidden_behind_celebration" {
            let q: Value = serde_json::from_slice(&std::fs::read(sb.state.join("llm-quip.json")).unwrap()).unwrap();
            assert_eq!(q["renders_remaining"], 6, "quip must not burn behind a celebration");
        }
        if c.name == "px_life_narrow_is_ascii" {
            assert_golden(&format!("{CLASSIC_DIR}/calm_common_offpeak.ansi"), &bytes);
        }
        assert_golden(&format!("{}/{}.ansi", c.dir, c.name), &bytes);
    }
}

/// One-off recorder for NEW statusline goldens. Never overwrites an existing
/// file, so it cannot paper over a regression. A re-record = git mv/delete the
/// directory, run this, then eyeball the new files. Usage:
///   cargo test --test golden_statusline -- --ignored record_missing_statusline_goldens
#[test]
#[ignore = "recorder: writes golden files that do not exist yet; run by hand"]
fn record_missing_statusline_goldens() {
    for c in cases().into_iter().chain(px_cases()) {
        let (bytes, _sb) = run_case(&c);
        common::record_missing_golden(&format!("{}/{}.ansi", c.dir, c.name), &bytes);
    }
}

#[test]
fn statusline_is_tz_independent_under_purr_now() {
    // 10:00 UTC is OFF PEAK; in Asia/Kolkata it is 15:30 (PEAK). Identical
    // bytes prove local_hour() no longer consults `date` when PURR_NOW is set.
    // Sandbox temp paths never reach the output (cwd is under HOME → "~/purrgotchi").
    let (a, _sa) = run_case(&Case { now: NOW + 2 * 3600, ..case("tz_default") });
    let (b, _sb) = run_case(&Case { now: NOW + 2 * 3600, env: &[("TZ", "Asia/Kolkata")], ..case("tz_kolkata") });
    assert!(strip_ansi(&String::from_utf8_lossy(&a)).contains("OFF PEAK"));
    assert_eq!(a, b);
}

#[test]
fn card_goldens() {
    for (name, patch) in [
        ("card_common", json!({})),
        ("card_legendary_shiny", json!({ "rarity": "legendary", "shiny": true, "breed": "Void Cat" })),
    ] {
        let sb = Sandbox::new(name);
        let mut st = base_state();
        merge(&mut st, patch);
        sb.write_state(&st);
        let now = NOW.to_string();
        let out = sb.run(&["card"], "", &[("PURR_NOW", &now)]);
        assert!(out.status.success());
        assert_golden(&format!("{name}.ansi"), &out.stdout);
    }
}

/// Where the classic (pixel cat off) goldens live.
const CLASSIC_DIR: &str = "statusline_ascii";

/// Rows 0-5 cut after the box's right border, row 6 whole; every line `\n`-terminated.
fn box_rows(text: &str) -> String {
    text.lines().enumerate().map(|(i, l)| common::box_prefix(l, i)).collect::<Vec<_>>().join("\n") + "\n"
}

/// The HUD box is frozen: whatever the sprite column draws, the box bytes of
/// every case stay as recorded.
#[test]
fn statusline_box_goldens() {
    for c in cases() {
        for extra in [&[][..], &[("PURR_PIXELS", "1")][..], &[("PURR_PIXELS", "fg")][..]] {
            let (bytes, _sb) = run_case_with(&c, extra);
            assert_golden(&format!("statusline_box/{}.ansi", c.name), box_rows(&String::from_utf8(bytes).unwrap()).as_bytes());
        }
    }
}

/// Opting out (`PURR_PIXELS=0`, `NO_COLOR`) reproduces the classic bytes exactly.
#[test]
fn statusline_pixels_off_is_classic() {
    for c in cases() {
        for extra in [&[("PURR_PIXELS", "0")][..], &[("NO_COLOR", "1"), ("PURR_PIXELS", "1")][..]] {
            let (bytes, _sb) = run_case_with(&c, extra);
            // The ASCII sprite's rainbow hue is a classic-only colour.
            let text = String::from_utf8_lossy(&bytes);
            for needle in c.raw_contain {
                assert!(text.contains(needle), "{}: missing raw {needle:?}", c.name);
            }
            assert_golden(&format!("{CLASSIC_DIR}/{}.ansi", c.name), &bytes);
        }
    }
}
