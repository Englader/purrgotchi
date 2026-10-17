//! `frame --json` black-box tests (Task 7): contract, strict read-only,
//! `--advance` timers, and coexistence with the classic statusline.
#![cfg(unix)]
mod common;
use common::{base_payload, base_state, keys, merge, run_json, strip_ansi, Sandbox, NOW};
use serde_json::{json, Value};

fn sandbox(tag: &str, patch: Value) -> Sandbox {
    let sb = Sandbox::new(tag);
    let mut st = base_state();
    merge(&mut st, patch);
    sb.write_state(&st);
    sb
}

fn at(sb: &Sandbox, args: &[&str], now: u64) -> std::process::Output {
    let n = now.to_string();
    let out = sb.run(args, "", &[("PURR_NOW", n.as_str())]);
    assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
    out
}

fn statusline_at(sb: &Sandbox, now: u64) {
    let n = now.to_string();
    let out = sb.run(&["statusline"], "{}", &[("PURR_NOW", n.as_str())]);
    assert!(out.status.success());
}

fn levelup() -> Value {
    json!({ "celebration_kind": "LevelUp", "levelup_renders_remaining": 3, "last_celebration_tick_at": 0 })
}

fn write_quip(sb: &Sandbox) {
    std::fs::write(sb.state.join("llm-quip.json"),
        json!({ "quip": "hi", "generated_at": NOW - 10, "renders_remaining": 6, "last_render_at": 0 }).to_string()).unwrap();
}

#[test]
fn frame_json_contract() {
    let sb = sandbox("frame_contract", json!({}));
    let (v, out) = run_json(&sb, &["frame", "--json", "--width", "60"], "");
    assert!(out.status.success());
    assert_eq!(keys(&v), ["bond", "breed", "celebration", "coins", "level", "mood", "name", "now", "ok",
                          "rarity", "scene", "shiny", "speech", "speech_source", "sprite", "stage", "tick", "v"]);
    assert_eq!((v["v"].as_u64(), v["ok"].as_bool()), (Some(1), Some(true)));
    assert_eq!((v["now"].as_u64(), v["tick"].as_u64()), (Some(NOW), Some(NOW)));
    assert_eq!((v["name"].as_str(), v["stage"].as_str(), v["level"].as_u64()), (Some("Thalia"), Some("kitten"), Some(5)));
    assert_eq!(v["sprite"].as_array().unwrap().len(), 6);
    let speech = v["speech"][0]["text"].as_str().unwrap();
    assert!(!speech.is_empty() && !speech.starts_with('"'));
    assert_eq!(v["speech_source"], "mood");
    assert_eq!((v["scene"].clone(), v["celebration"].clone()), (Value::Null, Value::Null));
    assert_eq!(v["mood"], "calm");
}

#[test]
fn frame_writes_nothing_and_never_locks() {
    // Everything an advancer would touch is due: a celebration tick, the scene
    // scheduler, and a fresh quip.
    let mut patch = levelup();
    merge(&mut patch, json!({ "scene_next_at": NOW - 1 }));
    let sb = sandbox("frame_ro", patch);
    write_quip(&sb);
    let state_before = std::fs::read(sb.state.join("state.json")).unwrap();
    let before = sb.snapshot();
    for now in [NOW, NOW + 2, NOW + 60] {
        let out = at(&sb, &["frame", "--json"], now);
        assert_eq!(String::from_utf8(out.stdout).unwrap().matches('\n').count(), 1);
    }
    assert_eq!(std::fs::read(sb.state.join("state.json")).unwrap(), state_before, "state.json bytes changed");
    assert_eq!(sb.snapshot(), before, "frame must not write, create a lock, or consume the quip");
}

#[test]
fn frame_without_kitten_does_not_hatch() {
    let sb = Sandbox::new("frame_nokitten");
    let (v, out) = run_json(&sb, &["frame", "--json"], "");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!((v["ok"].as_bool(), v["error"].as_str()), (Some(false), Some("no_kitten")));
    assert!(sb.snapshot().is_empty());
}

#[test]
fn frame_sprite_and_speech_match_the_statusline() {
    let sb = sandbox("frame_vs_statusline", json!({}));
    let payload = serde_json::to_string(&base_payload(sb.home.to_str().unwrap())).unwrap();
    let n = NOW.to_string();
    // statusline mutates nothing that matters here (no celebration, no quip, scene not due).
    let hud = strip_ansi(&String::from_utf8(sb.run(&["statusline"], &payload, &[("PURR_NOW", &n)]).stdout).unwrap());
    let (v, _) = run_json(&sb, &["frame", "--json", "--context-pct", "20"], "");
    let rows: Vec<&str> = hud.lines().collect();
    for (i, row) in rows.iter().enumerate().take(6) {
        let col: String = row.chars().skip(96 + 3).collect();
        assert_eq!(col.trim_end(), v["sprite"][i][0]["text"].as_str().unwrap().trim_end(), "row {i}");
    }
    let bubble: String = rows[6].chars().skip(96 + 3).collect();
    assert_eq!(bubble, format!("\"{}\"", v["speech"][0]["text"].as_str().unwrap()));
}

#[test]
fn frame_context_pct_drives_mood() {
    let sb = sandbox("frame_ctx", json!({}));
    for (pct, mood) in [("90", "panic"), ("50", "alert"), ("0", "calm"), ("250", "panic"), ("-5", "calm")] {
        let (v, _) = run_json(&sb, &["frame", "--json", "--context-pct", pct], "");
        assert_eq!(v["mood"], mood, "{pct}");
    }
}

#[test]
fn frame_tick_controls_sprite_parity() {
    let sb = sandbox("frame_tick", levelup());
    let (a, _) = run_json(&sb, &["frame", "--json", "--tick", "0"], "");
    let (b, _) = run_json(&sb, &["frame", "--json", "--tick", "1"], "");
    assert_eq!((a["tick"].as_u64(), b["tick"].as_u64()), (Some(0), Some(1)));
    assert_ne!(a["sprite"], b["sprite"], "levelup art has two frames");
    assert_eq!(a["celebration"]["kind"], "LevelUp");
    assert_eq!(a["celebration"]["remaining_ticks"], 3);
    assert_eq!(a["celebration"]["tick_secs"], 2);
    assert_eq!(a["speech_source"], "celebration");
}

#[test]
fn frame_reports_scene_only_when_it_fits() {
    let sb = sandbox("frame_scene", json!({ "scene_kind": "BallChase", "scene_started_at": NOW - 5,
                                            "scene_duration_secs": 30, "scene_seed": 7 }));
    let (v, _) = run_json(&sb, &["frame", "--json", "--width", "40"], "");
    let sc = &v["scene"];
    assert_eq!((sc["kind"].as_str(), sc["elapsed"].as_u64(), sc["finale"].as_bool(), sc["width"].as_u64()),
               (Some("BallChase"), Some(5), Some(false), Some(40)));
    assert_eq!(sc["rows"].as_array().unwrap().len(), 6);
    for row in sc["rows"].as_array().unwrap() {
        let w: usize = row.as_array().unwrap().iter().map(|s| s["text"].as_str().unwrap().chars().count()).sum();
        assert_eq!(w, 40);
    }
    assert_eq!(v["speech_source"], "scene");
    let (v, _) = run_json(&sb, &["frame", "--json", "--width", "10"], "");
    assert_eq!(v["scene"], Value::Null);
    assert_ne!(v["speech_source"], "scene");
    let (v, _) = run_json(&sb, &["frame", "--json", "--width", "500"], "");
    assert_eq!(v["scene"]["width"], 120, "clamped to MAX_STRIP");
}

#[test]
fn frame_rejects_bad_flags_with_exit_2() {
    let sb = sandbox("frame_bad", json!({}));
    for bad in [vec!["--width", "abc"], vec!["--width"], vec!["--width", "-3"], vec!["--tick", "-1"],
                vec!["--context-pct", "nan"], vec!["--context-pct", "inf"], vec!["--context-pct", "x"]] {
        let mut args = vec!["frame", "--json"];
        args.extend(bad.iter().copied());
        let out = sb.run(&args, "", &[]);
        assert_eq!(out.status.code(), Some(2), "{bad:?}");
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!((v["ok"].as_bool(), v["error"].as_str()), (Some(false), Some("bad_args")), "{bad:?}");
        assert!(v["detail"].as_str().unwrap().starts_with("frame: "), "{bad:?}");
        assert!(String::from_utf8_lossy(&out.stderr).starts_with("frame: "), "{bad:?}");
    }
}

#[test]
fn frame_advance_ticks_celebration_once_per_two_seconds() {
    let sb = sandbox("frame_adv", levelup());
    let remaining = |sb: &Sandbox| sb.read_state()["levelup_renders_remaining"].as_u64().unwrap();
    at(&sb, &["frame", "--json", "--advance"], NOW);
    assert_eq!(remaining(&sb), 2);
    at(&sb, &["frame", "--json", "--advance"], NOW);
    at(&sb, &["frame", "--json", "--advance"], NOW + 1);
    assert_eq!(remaining(&sb), 2, "gated: < 2 s since the last tick");
    at(&sb, &["frame", "--json", "--advance"], NOW + 2);
    assert_eq!(remaining(&sb), 1);
}

#[test]
fn statusline_and_frame_advance_in_the_same_second_tick_once() {
    let sb = sandbox("coexist", levelup());
    statusline_at(&sb, NOW);
    at(&sb, &["frame", "--json", "--advance"], NOW);
    assert_eq!(sb.read_state()["levelup_renders_remaining"], 2);
}

#[test]
fn statusline_and_frame_advance_interleaved_never_double_tick() {
    // Two renderers at 1 Hz each (the classic statusline's refreshInterval and
    // the mod's pane timer), interleaved in both orders. The countdown must
    // follow wall time (one tick per CELEBRATION_TICK_SECS), not render count.
    let sb = sandbox("coexist_interleaved", levelup());
    let remaining = |sb: &Sandbox| sb.read_state()["levelup_renders_remaining"].as_u64().unwrap();
    let mut seen = Vec::new();
    for dt in 0..6u64 {
        if dt % 2 == 0 {
            statusline_at(&sb, NOW + dt);
            at(&sb, &["frame", "--json", "--advance"], NOW + dt);
        } else {
            at(&sb, &["frame", "--json", "--advance"], NOW + dt);
            statusline_at(&sb, NOW + dt);
        }
        seen.push(remaining(&sb));
    }
    assert_eq!(seen, [2, 2, 1, 1, 0, 0]);
    let st = sb.read_state();
    assert_eq!(st["celebration_kind"], "None", "cleared exactly when the countdown hit 0");
    assert_eq!(st["last_celebration_tick_at"], NOW + 4);
    let (v, _) = run_json(&sb, &["frame", "--json"], "");
    assert_eq!(v["celebration"], Value::Null);
}

#[test]
fn only_frame_advance_consumes_quips_and_runs_the_scheduler() {
    let sb = sandbox("frame_quip", json!({ "scene_next_at": 0 }));
    write_quip(&sb);
    let (v, _) = run_json(&sb, &["frame", "--json"], "");
    assert_eq!((v["speech_source"].as_str(), v["speech"][0]["text"].as_str()), (Some("quip"), Some("hi")));
    let q = |sb: &Sandbox| -> Value { serde_json::from_slice(&std::fs::read(sb.state.join("llm-quip.json")).unwrap()).unwrap() };
    assert_eq!(q(&sb)["renders_remaining"], 6);
    assert_eq!(sb.read_state()["scene_next_at"], 0);
    let (v, _) = run_json(&sb, &["frame", "--json", "--advance"], "");
    assert_eq!(v["speech_source"], "quip");
    assert_eq!(q(&sb)["renders_remaining"], 5);
    assert!(sb.read_state()["scene_next_at"].as_u64().unwrap() > NOW, "scheduler bootstrapped");
}

#[test]
fn statusline_turn_anchor_follows_purr_now() {
    // cost.rs reads the PURR_NOW-aware clock, so renderer coexistence runs are
    // deterministic end to end (no real-clock timestamp in the turn anchor).
    let sb = sandbox("anchor_clock", json!({}));
    statusline_at(&sb, NOW);
    assert_eq!(std::fs::read_to_string(sb.state.join("turn-start-cost.txt")).unwrap(),
               format!("0\n{NOW}.000000\n"));
}
