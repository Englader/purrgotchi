#![cfg(unix)]
mod common;
use common::Sandbox;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn have_jq() -> bool {
    Command::new("jq").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn install_sh() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("install.sh")
}

fn run_install(sb: &Sandbox, arg: &str) -> Output {
    Command::new("bash")
        .arg(install_sh())
        .arg(arg)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", &sb.home)
        .output()
        .unwrap()
}

fn settings(sb: &Sandbox) -> PathBuf {
    sb.home.join(".claude").join("settings.json")
}

fn write_settings(sb: &Sandbox, v: &str) {
    std::fs::create_dir_all(sb.home.join(".claude")).unwrap();
    std::fs::write(settings(sb), v).unwrap();
}

fn backups(sb: &Sandbox) -> usize {
    std::fs::read_dir(sb.home.join(".claude")).unwrap().flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("settings.json.bak."))
        .count()
}

fn fixture() -> Value {
    json!({
        "theme": "dark",
        "statusLine": { "type": "command", "command": "\"/x/purrgotchi\" statusline", "padding": 1, "refreshInterval": 1 },
        "hooks": {
            "PostToolUse": [
                { "matcher": "*", "hooks": [ { "type": "command", "command": "\"/x/purrgotchi\" feed" } ] },
                { "matcher": "Bash", "hooks": [ { "type": "command", "command": "echo audit" } ] },
                { "matcher": "*", "hooks": [ { "type": "command", "command": "fmt.sh" },
                                            { "type": "command", "command": "/x/purrgotchi feed" } ] }
            ],
            "UserPromptSubmit": [ { "hooks": [ { "type": "command", "command": "\"/x/purrgotchi\" turn" } ] } ],
            "Stop": [ { "hooks": [ { "type": "command", "command": "say done" } ] } ]
        }
    })
}

#[test]
fn removes_only_purrgotchi_feed_and_turn_and_keeps_statusline() {
    if !have_jq() { eprintln!("skipping: jq not installed"); return; }
    let sb = Sandbox::new("uninstall");
    write_settings(&sb, &serde_json::to_string_pretty(&fixture()).unwrap());
    let out = run_install(&sb, "--uninstall-hooks");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
    let s: Value = serde_json::from_slice(&std::fs::read(settings(&sb)).unwrap()).unwrap();
    assert_eq!(s["statusLine"], fixture()["statusLine"]);
    assert_eq!(s["theme"], "dark");
    assert_eq!(s["hooks"]["PostToolUse"], json!([
        { "matcher": "Bash", "hooks": [ { "type": "command", "command": "echo audit" } ] },
        { "matcher": "*", "hooks": [ { "type": "command", "command": "fmt.sh" } ] }
    ]));
    assert!(s["hooks"].get("UserPromptSubmit").is_none());
    assert_eq!(s["hooks"]["Stop"], fixture()["hooks"]["Stop"]);
    assert_eq!(backups(&sb), 1);
}

#[test]
fn second_run_is_a_noop_without_a_new_backup() {
    if !have_jq() { return; }
    let sb = Sandbox::new("uninstall_twice");
    write_settings(&sb, &fixture().to_string());
    assert!(run_install(&sb, "--uninstall-hooks").status.success());
    let out = run_install(&sb, "--uninstall-hooks");
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("No purrgotchi hooks found"));
    assert_eq!(backups(&sb), 1);
}

#[test]
fn drops_hooks_object_when_only_purrgotchi_was_there() {
    if !have_jq() { return; }
    let sb = Sandbox::new("uninstall_only");
    write_settings(&sb, &json!({ "hooks": {
        "PostToolUse": [ { "matcher": "*", "hooks": [ { "type": "command", "command": "purrgotchi feed" } ] } ],
        "UserPromptSubmit": [ { "hooks": [ { "type": "command", "command": "purrgotchi turn" } ] } ]
    } }).to_string());
    assert!(run_install(&sb, "--uninstall-hooks").status.success());
    let s: Value = serde_json::from_slice(&std::fs::read(settings(&sb)).unwrap()).unwrap();
    assert_eq!(s, json!({}));
}

#[test]
fn refuses_invalid_json_and_leaves_it_untouched() {
    if !have_jq() { return; }
    let sb = Sandbox::new("uninstall_bad");
    write_settings(&sb, "{not json");
    let out = run_install(&sb, "--uninstall-hooks");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(std::fs::read_to_string(settings(&sb)).unwrap(), "{not json");
    assert!(!sb.home.join(".claude").join("settings.json.new").exists());
    assert_eq!(backups(&sb), 0);
}

#[test]
fn missing_settings_is_a_noop() {
    if !have_jq() { return; }
    let sb = Sandbox::new("uninstall_missing");
    assert!(run_install(&sb, "--uninstall-hooks").status.success());
    assert!(!settings(&sb).exists());
}

#[test]
fn unknown_option_exits_2() {
    let sb = Sandbox::new("install_bogus");
    assert_eq!(run_install(&sb, "--bogus").status.code(), Some(2));
    assert!(!settings(&sb).exists());
}

fn read(sb: &Sandbox) -> Value {
    serde_json::from_slice(&std::fs::read(settings(sb)).unwrap()).unwrap()
}

fn out_text(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn keeps_user_hooks_that_merely_mention_purrgotchi() {
    if !have_jq() { return; }
    let sb = Sandbox::new("uninstall_lint");
    let keep = json!({ "hooks": {
        "PostToolUse": [
            { "matcher": "Bash", "hooks": [ { "type": "command", "command": "~/purrgotchi-tools/lint.sh" } ] },
            { "matcher": "*", "hooks": [ { "type": "command", "command": "~/purrgotchi-tools/lint.sh --all" },
                                         { "type": "command", "command": "echo purrgotchi feed" } ] },
            { "matcher": "Bash", "hooks": [ { "type": "command", "command": "\"/x/purrgotchi\" feed" } ] }
        ],
        "UserPromptSubmit": [ { "hooks": [ { "type": "command", "command": "/x/purrgotchi-notes turn" } ] } ]
    } });
    write_settings(&sb, &keep.to_string());
    let out = run_install(&sb, "--uninstall-hooks");
    assert!(out.status.success());
    assert_eq!(read(&sb), keep);
    assert_eq!(backups(&sb), 0);
    assert!(out_text(&out).contains("No purrgotchi hooks found"));
}

#[test]
fn removes_the_purr_wrapper_and_windows_exe_forms() {
    if !have_jq() { return; }
    let sb = Sandbox::new("uninstall_purr");
    write_settings(&sb, &json!({ "hooks": {
        "PostToolUse": [
            { "matcher": "*", "hooks": [ { "type": "command", "command": "purr feed" } ] },
            { "matcher": "*", "hooks": [ { "type": "command", "command": "\"C:\\Program Files\\p\\purrgotchi.exe\" feed" } ] },
            { "matcher": "*", "hooks": [ { "type": "command", "command": "~/bin/purr feed --json" } ] }
        ],
        "UserPromptSubmit": [ { "hooks": [ { "type": "command", "command": "purr turn" } ] } ]
    } }).to_string());
    assert!(run_install(&sb, "--uninstall-hooks").status.success());
    assert_eq!(read(&sb), json!({}));
}

#[test]
fn no_match_leaves_file_byte_identical_and_keeps_preexisting_empty_arrays() {
    if !have_jq() { return; }
    let sb = Sandbox::new("uninstall_nomatch");
    let raw = "{\"hooks\":{\"PostToolUse\":[]},\"x\":1}";
    write_settings(&sb, raw);
    let out = run_install(&sb, "--uninstall-hooks");
    assert!(out.status.success());
    assert!(out_text(&out).contains("No purrgotchi hooks found"));
    assert!(!out_text(&out).contains("Removed"));
    assert_eq!(std::fs::read_to_string(settings(&sb)).unwrap(), raw);
    assert_eq!(backups(&sb), 0);
}

#[test]
fn only_prunes_what_this_run_emptied() {
    if !have_jq() { return; }
    let sb = Sandbox::new("uninstall_prune");
    write_settings(&sb, &json!({ "hooks": {
        "PostToolUse": [ { "matcher": "*", "hooks": [ { "type": "command", "command": "purr feed" } ] } ],
        "Stop": [],
        "Notification": [ { "hooks": [] } ]
    } }).to_string());
    let out = run_install(&sb, "--uninstall-hooks");
    assert!(out.status.success());
    assert!(out_text(&out).contains("Removed 1"));
    assert_eq!(read(&sb), json!({ "hooks": { "Stop": [], "Notification": [ { "hooks": [] } ] } }));
    assert_eq!(backups(&sb), 1);
}
