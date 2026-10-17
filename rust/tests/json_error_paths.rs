//! Error paths the mod relies on: unreadable/corrupt state is never reported
//! as "no kitten", bad args and bad stdin are machine-readable under --json,
//! and `--json` ok:false always exits 1 (classic exit codes are unchanged).
#![cfg(unix)]
mod common;
use common::{base_state, run_json, Sandbox};
use serde_json::{json, Value};

/// state.json as a directory: read() fails with something other than NotFound.
fn unreadable(tag: &str) -> Sandbox {
    let sb = Sandbox::new(tag);
    std::fs::create_dir_all(sb.state.join("state.json")).unwrap();
    sb
}

fn corrupt(tag: &str) -> Sandbox {
    let sb = Sandbox::new(tag);
    std::fs::write(sb.state.join("state.json"), "{not json").unwrap();
    sb
}

const READ_ONLY: [&[&str]; 5] = [
    &["frame", "--json"],
    &["stats", "--json"],
    &["card", "--json"],
    &["cost", "--session", "s1", "--total", "1", "--json"],
    &["quip-prompt", "--json"],
];

#[test]
fn read_only_commands_report_unreadable_state_as_state_unavailable() {
    for args in READ_ONLY {
        let sb = unreadable("ro_unreadable");
        let (v, out) = run_json(&sb, args, "");
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert_eq!((v["ok"].as_bool(), v["error"].as_str()), (Some(false), Some("state_unavailable")), "{args:?}");
        assert!(!String::from_utf8_lossy(&out.stderr).contains("No kitten"), "{args:?}");
    }
}

#[test]
fn read_only_commands_report_corrupt_state_as_state_corrupt() {
    for args in READ_ONLY {
        let sb = corrupt("ro_corrupt");
        let (v, out) = run_json(&sb, args, "");
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert_eq!(v["error"], "state_corrupt", "{args:?}");
        // never rewritten or moved aside by a read-only command
        assert_eq!(std::fs::read_to_string(sb.state.join("state.json")).unwrap(), "{not json", "{args:?}");
    }
}

#[test]
fn read_only_commands_keep_no_kitten_for_genuinely_missing_state() {
    for args in READ_ONLY {
        let sb = Sandbox::new("ro_missing");
        let (v, out) = run_json(&sb, args, "");
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert_eq!(v["error"], "no_kitten", "{args:?}");
    }
}

#[test]
fn classic_stats_and_card_still_exit_1_on_unreadable_state() {
    for cmd in ["stats", "card"] {
        let sb = unreadable("classic_unreadable");
        let out = sb.run(&[cmd], "", &[]);
        assert_eq!(out.status.code(), Some(1), "{cmd}");
        assert!(out.stdout.is_empty(), "{cmd}");
        assert!(!out.stderr.is_empty(), "{cmd}");
    }
}

#[test]
fn bad_args_under_json_also_print_a_bad_args_object() {
    let sb = Sandbox::new("bad_args");
    sb.write_state(&base_state());
    for args in [
        vec!["feed", "--json", "--count", "0"],
        vec!["frame", "--json", "--width", "abc"],
        vec!["cost", "--json", "--session", "s1"],
    ] {
        let out = sb.run(&args, "", &[]);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("{args:?}: {e}"));
        assert_eq!(v["v"], 1);
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"], "bad_args");
        assert!(v["detail"].as_str().is_some_and(|d| !d.is_empty()), "{args:?}");
        assert!(!out.stderr.is_empty(), "{args:?}");
    }
}

#[test]
fn bad_args_without_json_print_nothing_on_stdout() {
    let sb = Sandbox::new("bad_args_classic");
    for args in [vec!["feed", "--count", "0"], vec!["frame", "--width", "x"], vec!["cost", "--session", "s1"]] {
        let out = sb.run(&args, "", &[]);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
    }
}

#[test]
fn quip_prompt_rejects_invalid_utf8_stdin() {
    let sb = Sandbox::new("qp_utf8");
    sb.write_state(&base_state());
    let mut c = sb.cmd(&["quip-prompt", "--json"]);
    use std::io::Write;
    use std::process::Stdio;
    c.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = c.spawn().unwrap();
    child.stdin.take().unwrap().write_all(&[b'{', 0xff, 0xfe, b'}']).unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!((v["ok"].as_bool(), v["error"].as_str()), (Some(false), Some("bad_input")));
}

#[test]
fn json_error_results_exit_1_for_every_command_but_classic_hooks_stay_0() {
    for cmd in ["feed", "turn", "pet", "play", "treat"] {
        let sb = unreadable("exit_json");
        let (v, out) = run_json(&sb, &[cmd, "--json"], "{}");
        assert_eq!(v["ok"], false, "{cmd}");
        assert_eq!(out.status.code(), Some(1), "{cmd} --json");

        let sb = unreadable("exit_classic");
        let out = sb.run(&[cmd], "{}", &[]);
        assert_eq!(out.status.code(), Some(0), "{cmd} classic keeps exit 0");
    }
}

#[test]
fn successful_json_commands_still_exit_0() {
    for cmd in ["feed", "turn", "pet", "play", "treat"] {
        let sb = Sandbox::new("exit_ok");
        sb.write_state(&base_state());
        let (v, out) = run_json(&sb, &[cmd, "--json"], "{}");
        assert_eq!(v["ok"], true, "{cmd}");
        assert_eq!(out.status.code(), Some(0), "{cmd}");
        let _ = json!(null);
    }
}
