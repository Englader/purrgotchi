//! Error paths scripts rely on: unreadable/corrupt state is never reported
//! as "no kitten", bad args and bad stdin are machine-readable under --json,
//! and `--json` ok:false always exits 1 (classic exit codes are unchanged).
#![cfg(unix)]
mod common;
use common::{base_state, run_json, Sandbox};
use serde_json::{json, Value};

/// state.json as a directory: read() fails with something other than NotFound.
const READ_ONLY: [&[&str]; 3] = [
    &["frame", "--json"],
    &["stats", "--json"],
    &["card", "--json"],
];

#[test]
fn read_only_commands_report_unreadable_state_as_state_unavailable() {
    for args in READ_ONLY {
        let sb = Sandbox::unreadable("ro_unreadable");
        let (v, out) = run_json(&sb, args, "");
        assert_eq!(out.status.code(), Some(1), "{args:?}");
        assert_eq!((v["ok"].as_bool(), v["error"].as_str()), (Some(false), Some("state_unavailable")), "{args:?}");
        assert!(!String::from_utf8_lossy(&out.stderr).contains("No kitten"), "{args:?}");
    }
}

#[test]
fn read_only_commands_report_corrupt_state_as_state_corrupt() {
    for args in READ_ONLY {
        let sb = Sandbox::corrupt("ro_corrupt");
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
        let sb = Sandbox::unreadable("classic_unreadable");
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
    for args in [vec!["feed", "--count", "0"], vec!["frame", "--width", "x"]] {
        let out = sb.run(&args, "", &[]);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
    }
}

#[test]
fn removed_mod_verbs_are_unknown() {
    let sb = Sandbox::with_state("removed", json!({}));
    for v in ["serve", "cost", "quip-prompt", "quip-store"] {
        let out = sb.run(&[v], "", &[]);
        assert_eq!(out.status.code(), Some(2), "{v}");
    }
}

#[test]
fn json_error_results_exit_1_for_every_command_but_classic_hooks_stay_0() {
    for cmd in ["feed", "turn", "pet", "play", "treat"] {
        let sb = Sandbox::unreadable("exit_json");
        let (v, out) = run_json(&sb, &[cmd, "--json"], "{}");
        assert_eq!(v["ok"], false, "{cmd}");
        assert_eq!(out.status.code(), Some(1), "{cmd} --json");

        let sb = Sandbox::unreadable("exit_classic");
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
