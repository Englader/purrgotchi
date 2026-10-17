mod common;
use common::{keys, run_json, Sandbox, NOW};
use serde_json::{json, Value};

#[test]
fn feed_json_contract() {
    let sb = Sandbox::with_state("feed_contract", json!({}));
    let (v, out) = run_json(&sb, &["feed", "--json"], "");
    assert!(out.status.success());
    assert_eq!(keys(&v), ["celebration", "coins", "count", "drop", "level", "ok", "stage", "total_tool_calls", "v"]);
    assert_eq!(v["v"], 1);
    assert_eq!(v["ok"], true);
    assert_eq!(v["count"], 1);
    assert_eq!(v["total_tool_calls"], 231);
    assert_eq!(v["stage"], "kitten");
    let dropped = v["drop"].as_bool().unwrap();
    assert_eq!(v["celebration"], if dropped { json!("RandomDrop") } else { Value::Null });
    assert_eq!(v["coins"], 341 + if dropped { 10 } else { 0 });
    assert_eq!(sb.read_state()["last_fed_at"], NOW);
}

#[test]
fn feed_json_reports_level_up() {
    let sb = Sandbox::with_state("feed_levelup", json!({ "total_tool_calls": 49, "level": 1, "last_milestone_celebrated": 0 }));
    let (v, _) = run_json(&sb, &["feed", "--json"], "");
    assert_eq!(v["celebration"], "LevelUp");
    assert_eq!(v["level"], 2);
}

#[test]
fn feed_json_reports_milestone() {
    let sb = Sandbox::with_state("feed_milestone", json!({ "total_tool_calls": 99, "level": 2, "last_milestone_celebrated": 0 }));
    let (v, _) = run_json(&sb, &["feed", "--json"], "");
    assert_eq!(v["celebration"], "MilestoneCenturion");
}

#[test]
fn feed_count_batches_and_reports_streak() {
    let sb = Sandbox::with_state("feed_count", json!({}));
    let (v, _) = run_json(&sb, &["feed", "--json", "--count", "5"], "");
    assert_eq!(v["count"], 5);
    assert_eq!(v["total_tool_calls"], 235);
    assert_eq!(v["celebration"], "Streak");
    assert_eq!(sb.read_state()["streak_count"], 5);
}

#[test]
fn feed_count_rejects_bad_values_without_touching_state() {
    for bad in [vec!["--count", "0"], vec!["--count", "abc"], vec!["--count", "10001"], vec!["--count"]] {
        let sb = Sandbox::with_state("feed_badcount", json!({}));
        let before = sb.snapshot();
        let mut args = vec!["feed", "--json"];
        args.extend(bad.iter().copied());
        let out = sb.run(&args, "", &[]);
        assert_eq!(out.status.code(), Some(2), "{bad:?}");
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!((v["ok"].as_bool(), v["error"].as_str()), (Some(false), Some("bad_args")), "{bad:?}");
        assert!(v["detail"].as_str().unwrap().starts_with("feed: "), "{bad:?}");
        assert!(String::from_utf8_lossy(&out.stderr).starts_with("feed: "), "{bad:?}");
        assert_eq!(sb.snapshot(), before, "{bad:?}");
    }
}

#[test]
fn classic_feed_stays_silent() {
    let sb = Sandbox::with_state("feed_silent", json!({}));
    let out = sb.run(&["feed"], "", &[]);
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
}

#[test]
fn feed_json_on_unreadable_state_prints_one_error_line() {
    let sb = Sandbox::unreadable("feed_unreadable");
    let (v, out) = run_json(&sb, &["feed", "--json"], "");
    assert_eq!(out.status.code(), Some(1), "--json ok:false exits 1");
    assert_eq!(keys(&v), ["error", "message", "ok", "v"]);
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "state_unavailable");
}

#[test]
fn turn_json_greets_after_six_hours() {
    let sb = Sandbox::with_state("turn_greet", json!({ "last_turn_at": NOW - 7 * 3600 }));
    let (v, _) = run_json(&sb, &["turn", "--json"], "{}");
    assert_eq!(keys(&v), ["celebration", "ok", "react_spawned", "v"]);
    assert_eq!(v["celebration"], "Greeting");
    assert_eq!(v["react_spawned"], false, "no provider configured in the sandbox");
}

#[test]
fn turn_json_without_gap_is_null() {
    let sb = Sandbox::with_state("turn_nogreet", json!({}));
    let (v, _) = run_json(&sb, &["turn", "--json"], "{}");
    assert_eq!(v["celebration"], Value::Null);
}
