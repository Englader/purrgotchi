mod common;
use common::{base_state, keys, merge, run_json, Sandbox, NOW};
use serde_json::{json, Value};

fn sandbox(tag: &str, patch: Value) -> Sandbox {
    let sb = Sandbox::new(tag);
    let mut st = base_state();
    merge(&mut st, patch);
    sb.write_state(&st);
    sb
}

#[test]
fn pet_json_applied_then_cooldown() {
    let sb = sandbox("pet", json!({}));
    let (v, out) = run_json(&sb, &["pet", "--json"], "");
    assert!(out.status.success());
    assert_eq!(keys(&v), ["bond_new", "bond_old", "capped", "message", "ok", "result", "v"]);
    assert_eq!((v["result"].as_str(), v["bond_old"].as_u64(), v["bond_new"].as_u64()), (Some("applied"), Some(50), Some(65)));
    assert!(!v["message"].as_str().unwrap().contains('\x1b'));
    let (v, _) = run_json(&sb, &["pet", "--json"], "");
    assert_eq!(v["result"], "cooldown");
    assert_eq!(v["wait_secs"], 4 * 3600);
    assert_eq!(v["ok"], true);
}

#[test]
fn classic_pet_output_is_unchanged_ansi_prose() {
    let sb = sandbox("pet_classic", json!({}));
    let now = NOW.to_string();
    let out = sb.run(&["pet"], "", &[("PURR_NOW", &now)]);
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(s.contains("\x1b[") && s.contains("purrs contentedly"), "{s:?}");
}

#[test]
fn play_json_applied_and_insufficient_coins_both_start_a_scene() {
    let sb = sandbox("play_ok", json!({}));
    let (v, _) = run_json(&sb, &["play", "--json"], "");
    assert_eq!(v["result"], "applied");
    assert_eq!((v["cost"].as_u64(), v["bond_new"].as_u64()), (Some(50), Some(70)));
    assert!(v["scene"].is_string());

    let sb = sandbox("play_broke", json!({ "coins": 10 }));
    let (v, _) = run_json(&sb, &["play", "--json"], "");
    assert_eq!(v["result"], "insufficient_coins");
    assert_eq!((v["need"].as_u64(), v["have"].as_u64()), (Some(50), Some(10)));
    assert!(v["scene"].is_string());
    assert_ne!(sb.read_state()["scene_kind"], "None");
}

#[test]
fn treat_json_hungry() {
    let sb = sandbox("treat", json!({ "last_fed_at": NOW - 4000 }));
    let (v, _) = run_json(&sb, &["treat", "--json"], "");
    assert_eq!(v["result"], "applied");
    assert_eq!(v["was_hungry"], true);
    assert_eq!((v["bond_old"].as_u64(), v["bond_new"].as_u64(), v["cost"].as_u64()), (Some(50), Some(55), Some(25)));
}

#[test]
fn rename_json_round_trips_hostile_names() {
    for name in ["Nyx", "Zoë \"the\" \\ ✦ 🐈", "\x1b[31mRed"] {
        let sb = sandbox("rename", json!({}));
        let (v, out) = run_json(&sb, &["rename", name, "--json"], "");
        assert_eq!(out.status.code(), Some(0));
        assert_eq!(v["result"], "renamed");
        assert_eq!(v["name"], name);
        assert_eq!(sb.read_state()["name"], name);
    }
}

#[test]
fn rename_json_empty_name_keeps_exit_1() {
    let sb = sandbox("rename_empty", json!({}));
    let (v, out) = run_json(&sb, &["rename", "", "--json"], "");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!((v["ok"].as_bool(), v["result"].as_str()), (Some(false), Some("error")));
    assert!(String::from_utf8_lossy(&out.stderr).contains("rename: name cannot be empty"));
}

#[test]
fn rename_dash_name_is_still_a_name() {
    let sb = sandbox("rename_dash", json!({}));
    let out = sb.run(&["rename", "-Kitty"], "", &[]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(sb.read_state()["name"], "-Kitty");
}

#[test]
fn pet_json_on_unreadable_state_reports_error_with_exit_0() {
    let sb = Sandbox::new("pet_unreadable");
    std::fs::create_dir_all(sb.state.join("state.json")).unwrap();
    let (v, out) = run_json(&sb, &["pet", "--json"], "");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!((v["ok"].as_bool(), v["result"].as_str()), (Some(false), Some("error")));
}
