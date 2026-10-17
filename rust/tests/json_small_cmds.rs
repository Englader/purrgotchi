#![cfg(unix)]
mod common;
use common::{base_payload, keys, run_json, strip_ansi, Sandbox, NOW};
use serde_json::{json, Value};

fn approx(v: &Value, want: f64) {
    let got = v.as_f64().unwrap();
    assert!((got - want).abs() < 1e-9, "got {got}, want {want}");
}

#[test]
fn cost_credits_only_new_spend_per_session() {
    let sb = Sandbox::with_state("cost", json!({}));
    let (v, _) = run_json(&sb, &["cost", "--session", "s1", "--total", "0.5", "--json"], "");
    assert_eq!(keys(&v), ["credit", "lifetime_cost_usd", "ok", "v"]);
    approx(&v["credit"], 0.5);
    approx(&v["lifetime_cost_usd"], 3.92);
    let (v, _) = run_json(&sb, &["cost", "--session", "s1", "--total", "0.5", "--json"], "");
    approx(&v["credit"], 0.0);
    let (v, _) = run_json(&sb, &["cost", "--session", "s1", "--total", "0.8", "--json"], "");
    approx(&v["credit"], 0.3);
    let (v, _) = run_json(&sb, &["cost", "--session", "s2", "--total", "1.0", "--json"], "");
    approx(&v["credit"], 1.0);
    approx(&v["lifetime_cost_usd"], 5.22);
}

#[test]
fn cost_rejects_bad_arguments() {
    let sb = Sandbox::with_state("cost_bad", json!({}));
    for args in [
        vec!["cost", "--total", "1"], vec!["cost", "--session", "s1"],
        vec!["cost", "--session", "s1", "--total", "-1"], vec!["cost", "--session", "s1", "--total", "nan"],
        vec!["cost", "--session", "--total", "1"],
    ] {
        assert_eq!(sb.run(&args, "", &[]).status.code(), Some(2), "{args:?}");
    }
}

#[test]
fn cost_without_kitten_exits_1_and_writes_nothing() {
    let sb = Sandbox::new("cost_nokitten");
    let (v, out) = run_json(&sb, &["cost", "--session", "s1", "--total", "1", "--json"], "");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(v["error"], "no_kitten");
    assert!(sb.snapshot().is_empty());
}

#[test]
fn quip_prompt_builds_from_stdin_context() {
    let sb = Sandbox::with_state("qp", json!({}));
    let (v, _) = run_json(&sb, &["quip-prompt", "--json"], r#"{"last_user":"fix the build","last_assistant":"done"}"#);
    assert_eq!(keys(&v), ["max_tokens", "mood", "ok", "system", "user", "v"]);
    let system = v["system"].as_str().unwrap();
    assert!(system.contains("Thalia") && system.contains("Maine Coon"), "{system}");
    let user = v["user"].as_str().unwrap();
    assert!(user.contains("USER: fix the build") && user.contains("A: done") && user.ends_with("Mood signal: calm"), "{user}");
    assert_eq!(v["max_tokens"], 50);
}

#[test]
fn quip_prompt_with_empty_stdin_still_works() {
    let sb = Sandbox::with_state("qp_empty", json!({}));
    let (v, _) = run_json(&sb, &["quip-prompt", "--json"], "");
    assert_eq!(v["user"], "Here's what just happened:\n\nMood signal: calm");
}

#[test]
fn quip_prompt_bad_stdin_is_exit_2() {
    let sb = Sandbox::with_state("qp_bad", json!({}));
    let (v, out) = run_json(&sb, &["quip-prompt", "--json"], "{not json");
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(v["error"], "bad_input");
}

#[test]
fn quip_store_sanitizes_and_the_statusline_shows_it() {
    let sb = Sandbox::with_state("qs", json!({}));
    let (v, out) = run_json(&sb, &["quip-store", "--json", "\x1b]0;pwned\x07hi there"], "");
    assert!(out.status.success());
    let stored = v["stored"].as_str().unwrap();
    assert!(!stored.chars().any(char::is_control), "{stored:?}");
    let cache: Value = serde_json::from_slice(&std::fs::read(sb.state.join("llm-quip.json")).unwrap()).unwrap();
    assert_eq!(cache["generated_at"], NOW);
    let payload = serde_json::to_string(&base_payload(sb.home.to_str().unwrap())).unwrap();
    let n = NOW.to_string();
    let hud = strip_ansi(&String::from_utf8(sb.run(&["statusline"], &payload, &[("PURR_NOW", &n)]).stdout).unwrap());
    assert!(hud.contains(&format!("\"{stored}\"")), "{hud}");
}

#[test]
fn quip_store_rejects_empty_text() {
    let sb = Sandbox::with_state("qs_empty", json!({}));
    let out = sb.run(&["quip-store", "   "], "", &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(!sb.state.join("llm-quip.json").exists());
}
