//! `purrgotchi band` black-box tests: the NDJSON frame stream must keep
//! running with stdin closed (exactly how `$.process.spawn` starts it), stay
//! quiet about a missing kitten, never write state, and exit 0 (no abort)
//! when stdout closes. Every wait is bounded, so a regression fails instead
//! of hanging.
#![cfg(unix)]
mod common;
use common::{Sandbox, NOW};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// Spawn `band` with stdin closed; a reader thread forwards stdout lines.
fn spawn(sb: &Sandbox, args: &[&str]) -> (Child, Receiver<String>) {
    let mut argv = vec!["band"];
    argv.extend_from_slice(args);
    let now = NOW.to_string();
    let mut child = sb
        .cmd(&argv)
        .env("PURR_NOW", &now)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn band");
    let out = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines() {
            let Ok(l) = line else { break };
            if tx.send(l).is_err() {
                break;
            }
        }
    });
    (child, rx)
}

fn next(rx: &Receiver<String>, within: Duration) -> Value {
    let line = rx.recv_timeout(within).expect("a band line within the deadline");
    serde_json::from_str(&line).unwrap_or_else(|e| panic!("bad JSON {line:?}: {e}"))
}

fn b64_len(s: &str) -> usize {
    let pad = s.bytes().rev().take_while(|&b| b == b'=').count();
    s.len() / 4 * 3 - pad
}

fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3))
}

/// `--now-ms --frames` run: raw stdout.
fn test_clock(sb: &Sandbox, cols: &str, now_ms: u64, frames: u64) -> Vec<u8> {
    let (n, f) = (now_ms.to_string(), frames.to_string());
    let out = sb.run(&["band", "--cols", cols, "--rows", "6", "--now-ms", &n, "--frames", &f], "", &[]);
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    out.stdout
}

#[test]
fn band_hello_then_frames_with_stdin_closed() {
    let sb = Sandbox::with_state("band_stream", json!({}));
    let (mut child, rx) = spawn(&sb, &["--cols", "60", "--rows", "6"]);
    let h = next(&rx, Duration::from_secs(3));
    assert_eq!(h["hello"], true, "{h}");
    assert_eq!((h["cols"].as_u64(), h["rows"].as_u64(), h["fps"].as_u64()), (Some(60), Some(6), Some(30)));
    assert_eq!(h["state_dir"].as_str(), sb.state.to_str());
    let f = next(&rx, Duration::from_secs(3));
    assert_eq!(f["v"], 1);
    assert_eq!(b64_len(f["c"].as_str().expect("c")), 60 * 6 * 12, "{f}");
    assert!(child.try_wait().unwrap().is_none(), "band must not exit on stdin EOF");
    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn band_no_kitten_streams_errors_then_recovers() {
    let sb = Sandbox::new("band_recover");
    let (mut child, rx) = spawn(&sb, &["--cols", "60", "--rows", "6"]);
    assert_eq!(next(&rx, Duration::from_secs(3))["hello"], true);
    let v = next(&rx, Duration::from_secs(3));
    assert_eq!((v["ok"].as_bool(), v["error"].as_str()), (Some(false), Some("no_kitten")), "{v}");
    assert!(child.try_wait().unwrap().is_none(), "no_kitten must not end the stream");
    let now = NOW.to_string();
    let out = sb.run(&["feed"], "", &[("PURR_NOW", now.as_str())]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut ok = false;
    while Instant::now() < deadline {
        let Ok(line) = rx.recv_timeout(Duration::from_millis(500)) else { continue };
        let v: Value = serde_json::from_str(&line).unwrap();
        if v.get("c").is_some() {
            ok = true;
            break;
        }
    }
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(ok, "band never produced a frame after feed");
}

#[test]
fn band_exits_zero_when_stdout_closes() {
    let sb = Sandbox::with_state("band_epipe", json!({}));
    let (mut child, rx) = spawn(&sb, &["--cols", "60", "--rows", "6"]);
    next(&rx, Duration::from_secs(3));
    // Dropping the receiver ends the reader thread, which drops the pipe;
    // the child's next write fails with EPIPE.
    drop(rx);
    let deadline = Instant::now() + Duration::from_secs(2);
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("band did not exit after stdout closed");
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    assert_eq!(status.signal(), None, "band aborted instead of exiting quietly");
    assert_eq!(status.code(), Some(0));
}

#[test]
fn band_test_clock_is_deterministic() {
    let sb = Sandbox::with_state("band_det", json!({}));
    let t0 = NOW * 1000;
    let (a, b) = (test_clock(&sb, "60", t0, 30), test_clock(&sb, "60", t0, 30));
    assert_eq!(a, b);
    let text = String::from_utf8(a).unwrap();
    let lines: Vec<Value> = text.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(lines[0]["hello"], true);
    let frames = &lines[1..];
    assert!(!frames.is_empty() && frames[0]["f"] == 0);
    for fr in frames {
        let (f, t) = (fr["f"].as_u64().unwrap(), fr["t"].as_u64().unwrap());
        assert_eq!(t, t0 + f * 1000 / 30, "{fr}");
    }
    // Unchanged frames are skipped, so a 1 s run is far fewer than 30 lines.
    assert!(frames.len() < 30, "{} frames", frames.len());
}

#[test]
fn band_bad_args_exit_2() {
    let sb = Sandbox::with_state("band_bad", json!({}));
    for bad in [
        vec!["--cols", "60", "--rows", "5"],
        vec!["--cols", "0", "--rows", "6"],
        vec!["--cols", "513", "--rows", "6"],
        vec!["--rows", "6"],
        vec!["--cols", "60"],
        vec!["--cols", "abc", "--rows", "6"],
    ] {
        let mut argv = vec!["band"];
        argv.extend(bad.iter().copied());
        let out = sb.run(&argv, "", &[]);
        assert_eq!(out.status.code(), Some(2), "{bad:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.starts_with("band: ") && err.contains("usage: purrgotchi band"), "{bad:?}: {err}");
        assert!(out.stdout.is_empty(), "{bad:?}");
    }
}

#[test]
fn band_never_writes_state() {
    let sb = Sandbox::with_state("band_ro", json!({}));
    std::fs::write(
        sb.state.join("llm-quip.json"),
        json!({ "quip": "the cat watches", "generated_at": NOW - 10, "renders_remaining": 6, "last_render_at": 0 }).to_string(),
    )
    .unwrap();
    let before = sb.snapshot();
    test_clock(&sb, "60", NOW * 1000, 30);
    assert_eq!(sb.snapshot(), before);
}

#[test]
fn band_prunes_only_stale_alive_files() {
    let sb = Sandbox::with_state("band_prune", json!({}));
    let (old, fresh, other) = (sb.state.join("band-old.alive"), sb.state.join("band-new.alive"), sb.state.join("band-x.txt"));
    for p in [&old, &fresh, &other] {
        std::fs::write(p, "1").unwrap();
    }
    let f = std::fs::File::options().write(true).open(&old).unwrap();
    f.set_modified(std::time::SystemTime::now() - Duration::from_secs(25 * 3600)).unwrap();
    test_clock(&sb, "60", NOW * 1000, 2);
    assert!(!old.exists() && fresh.exists() && other.exists());
}

#[test]
fn band_hash_golden() {
    let scene = json!({ "scene_kind": "BallChase", "scene_started_at": NOW - 5, "scene_duration_secs": 30, "scene_seed": 7 });
    let cases: [(&str, Value); 3] = [
        ("idle_calm", json!({})),
        ("scene_ball", scene),
        ("celebration_levelup", json!({ "celebration_kind": "LevelUp", "levelup_renders_remaining": 2, "last_celebration_tick_at": NOW })),
    ];
    let mut lines = String::new();
    for (name, patch) in cases {
        let sb = Sandbox::with_state(&format!("band_gold_{name}"), patch);
        let text = String::from_utf8(test_clock(&sb, "120", NOW * 1000, 60)).unwrap();
        for l in text.lines().skip(1) {
            let v: Value = serde_json::from_str(l).unwrap();
            lines.push_str(&format!("{name}|{}|{}|{:016x}\n", v["f"], v["t"], fnv1a(v["c"].as_str().unwrap())));
        }
    }
    let path = common::golden_path("band_hashes.txt");
    if std::env::var("UPDATE_GOLDEN").as_deref() == Ok("1") {
        std::fs::write(&path, &lines).unwrap();
    }
    common::assert_golden("band_hashes.txt", lines.as_bytes());
}
