//! Byte-exact goldens of the classic statusline (Task 2 of the v4.0.0 Phase 0
//! plan). Recorded from the pre-refactor binary; every later task must keep
//! them identical.
#![cfg(unix)]

mod common;
use common::{assert_golden, base_payload, base_state, merge, strip_ansi, Sandbox, DAY, NOW};
use serde_json::{json, Value};

const PINK: &str = "\x1b[38;2;234;0;217m";

struct Case {
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
}

fn case(name: &'static str) -> Case {
    Case {
        name, now: NOW, state: json!({}), payload: Some(json!({})), branch: Some("main"),
        cwd_outside_home: false, anchor: None, quip: None, env: &[], must_contain: &[],
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
}

fn run_case(c: &Case) -> (Vec<u8>, Sandbox) {
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
    let out = sb.run(&["statusline"], &stdin, &env);
    assert!(out.status.success(), "{}: exit {:?} stderr {}", c.name, out.status, String::from_utf8_lossy(&out.stderr));
    (out.stdout, sb)
}

#[test]
fn statusline_goldens() {
    for c in cases() {
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
        assert_golden(&format!("statusline/{}.ansi", c.name), &bytes);
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
