//! The PURR_NOW / PURR_STATE_DIR dev backdoors must stay under the control of
//! the process environment: not the state-dir `.env`, not hostile values.
#![cfg(unix)]
mod common;
use common::{base_state, Sandbox};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn real_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

fn assert_real_clock(t: u64) {
    assert!(t.abs_diff(real_now()) < 60, "last_fed_at {t} is not the real clock {}", real_now());
}

/// Run a bare command in the sandbox HOME with NO PURR_STATE_DIR / PURR_NOW
/// unless `extra` supplies them.
fn run_bare(sb: &Sandbox, args: &[&str], extra: &[(&str, &str)], cwd: &std::path::Path) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_purrgotchi"));
    c.args(args)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", &sb.home)
        .env("USERPROFILE", &sb.home)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (k, v) in extra {
        c.env(k, v);
    }
    assert!(c.status().unwrap().success());
}

fn default_state_dir(sb: &Sandbox) -> std::path::PathBuf {
    let d = sb.home.join(".claude").join("purrgotchi");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("state.json"), serde_json::to_string(&base_state()).unwrap()).unwrap();
    d
}

fn last_fed(dir: &std::path::Path) -> u64 {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("state.json")).unwrap()).unwrap();
    v["last_fed_at"].as_u64().unwrap()
}

#[test]
fn env_file_cannot_set_purr_now_but_other_keys_still_load() {
    let sb = Sandbox::new("env_now");
    let dir = default_state_dir(&sb);
    std::fs::write(dir.join(".env"), "PURR_NOW=1900000000\n").unwrap();
    run_bare(&sb, &["feed"], &[], &sb.root);
    assert_real_clock(last_fed(&dir));

    // Control: the same .env mechanism still delivers ordinary keys.
    let sb2 = Sandbox::new("env_ctrl");
    let dir2 = sb2.home.join(".claude").join("purrgotchi");
    std::fs::create_dir_all(&dir2).unwrap();
    std::fs::write(dir2.join(".env"), "PURR_FORCE_RARITY=legendary\nPURR_NOW=1900000000\n").unwrap();
    run_bare(&sb2, &["hatch"], &[], &sb2.root);
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(dir2.join("state.json")).unwrap()).unwrap();
    assert_eq!(v["rarity"], "legendary");
    assert_real_clock(v["birth_time"].as_u64().unwrap());
}

#[test]
fn env_file_cannot_redirect_the_state_dir() {
    let sb = Sandbox::new("env_dir");
    let dir = default_state_dir(&sb);
    let evil = sb.root.join("evil");
    std::fs::write(dir.join(".env"), format!("PURR_STATE_DIR={}\n", evil.display())).unwrap();
    run_bare(&sb, &["feed"], &[], &sb.root);
    assert!(!evil.exists(), "state dir was redirected by .env");
    assert_real_clock(last_fed(&dir));
}

#[test]
fn process_env_still_wins_over_env_file_for_ordinary_keys_and_backdoors() {
    let sb = Sandbox::new("env_proc");
    let dir = default_state_dir(&sb);
    std::fs::write(dir.join(".env"), "PURR_NOW=1900000000\n").unwrap();
    run_bare(&sb, &["feed"], &[("PURR_NOW", "1800000000")], &sb.root);
    assert_eq!(last_fed(&dir), 1_800_000_000);
}

#[test]
fn relative_state_dir_is_ignored() {
    let sb = Sandbox::new("rel_dir");
    let dir = default_state_dir(&sb);
    let cwd = sb.root.join("cwd");
    std::fs::create_dir_all(&cwd).unwrap();
    run_bare(&sb, &["feed"], &[("PURR_STATE_DIR", "rel-state")], &cwd);
    assert!(!cwd.join("rel-state").exists());
    assert_real_clock(last_fed(&dir));
}

#[test]
fn hostile_purr_now_values_fall_back_to_the_real_clock() {
    for (i, bad) in ["0", "18446744073709551615", "9223372036854775807"].iter().enumerate() {
        let sb = Sandbox::new(&format!("bad_now{i}"));
        let dir = default_state_dir(&sb);
        run_bare(&sb, &["feed"], &[("PURR_NOW", bad)], &sb.root);
        assert_real_clock(last_fed(&dir));
    }
}
