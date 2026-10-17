//! `serve` black-box tests (Task 4.1): the NDJSON frame stream must keep
//! running with stdin closed (exactly how `$.process.spawn` starts it), stay
//! quiet about a missing kitten, and exit 0 (no abort) when stdout closes.
//! Every wait is bounded, so a regression fails instead of hanging.
#![cfg(unix)]
mod common;
use common::{Sandbox, NOW};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// Spawn `serve` with stdin closed; a reader thread forwards stdout lines.
fn spawn(sb: &Sandbox, args: &[&str]) -> (Child, Receiver<String>) {
    let mut argv = vec!["serve"];
    argv.extend_from_slice(args);
    let now = NOW.to_string();
    let mut child = sb
        .cmd(&argv)
        .env("PURR_NOW", &now)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn serve");
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
    let line = rx.recv_timeout(within).expect("a serve line within the deadline");
    serde_json::from_str(&line).unwrap_or_else(|e| panic!("bad JSON {line:?}: {e}"))
}

#[test]
fn serve_streams_frames_with_stdin_closed() {
    let sb = Sandbox::with_state("serve_stream", json!({}));
    let (mut child, rx) = spawn(&sb, &["--interval-ms", "250", "--width", "60"]);
    let start = Instant::now();
    for _ in 0..3 {
        let v = next(&rx, Duration::from_secs(3));
        assert_eq!((v["v"].as_u64(), v["ok"].as_bool()), (Some(1), Some(true)), "{v}");
        assert!(v["hud"].is_object() && v["sprite"].as_array().unwrap().len() == 6, "{v}");
    }
    assert!(start.elapsed() < Duration::from_secs(3));
    assert!(child.try_wait().unwrap().is_none(), "serve must not exit on stdin EOF");
    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn serve_emits_no_kitten_then_recovers() {
    let sb = Sandbox::new("serve_recover");
    let (mut child, rx) = spawn(&sb, &["--interval-ms", "250"]);
    let v = next(&rx, Duration::from_secs(3));
    assert_eq!((v["ok"].as_bool(), v["error"].as_str()), (Some(false), Some("no_kitten")), "{v}");
    assert!(child.try_wait().unwrap().is_none(), "no_kitten must not end the stream");
    let now = NOW.to_string();
    let out = sb.run(&["feed"], "", &[("PURR_NOW", now.as_str())]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut ok = false;
    while Instant::now() < deadline {
        let Ok(line) = rx.recv_timeout(Duration::from_millis(500)) else { continue };
        let v: Value = serde_json::from_str(&line).unwrap();
        if v["ok"] == true {
            ok = true;
            break;
        }
    }
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(ok, "serve never produced an ok frame after feed");
}

#[test]
fn serve_exits_when_stdout_closes() {
    let sb = Sandbox::with_state("serve_epipe", json!({}));
    let (mut child, rx) = spawn(&sb, &["--interval-ms", "250"]);
    next(&rx, Duration::from_secs(3));
    // The reader thread owns the pipe; once it stops reading and drops it the
    // child's next write fails with EPIPE. Drain nothing more: close by
    // dropping the receiver (the thread exits on its next send) and wait.
    drop(rx);
    let deadline = Instant::now() + Duration::from_secs(3);
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("serve did not exit after stdout closed");
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    assert_eq!(status.signal(), None, "serve aborted instead of exiting quietly");
    assert_eq!(status.code(), Some(0));
}

#[test]
fn serve_rejects_bad_interval() {
    let sb = Sandbox::with_state("serve_bad", json!({}));
    for bad in [vec!["--interval-ms", "abc"], vec!["--interval-ms"]] {
        let mut argv = vec!["serve"];
        argv.extend(bad.iter().copied());
        let out = sb.run(&argv, "", &[]);
        assert_eq!(out.status.code(), Some(2), "{bad:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("usage: purrgotchi serve [--interval-ms N] [--width N] [--context-pct P] [--advance] [--pixels]"),
            "{err}"
        );
        assert!(out.stdout.is_empty(), "serve must not emit on bad args");
    }
}

#[test]
fn serve_clamps_interval() {
    // 1 ms is clamped to 250 ms: ~1 s yields at most 5 lines, never a flood.
    let sb = Sandbox::with_state("serve_clamp", json!({}));
    let (mut child, rx) = spawn(&sb, &["--interval-ms", "1"]);
    std::thread::sleep(Duration::from_millis(1000));
    child.kill().unwrap();
    child.wait().unwrap();
    let n = rx.try_iter().count();
    assert!((1..=5).contains(&n), "{n} lines in 1 s");
}

#[test]
fn serve_advance_respects_the_tick_gate() {
    let sb = Sandbox::with_state(
        "serve_advance",
        json!({ "celebration_kind": "LevelUp", "levelup_renders_remaining": 3, "last_celebration_tick_at": 0 }),
    );
    let (mut child, rx) = spawn(&sb, &["--advance", "--interval-ms", "250"]);
    std::thread::sleep(Duration::from_millis(1200));
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(rx.try_iter().count() >= 3);
    // PURR_NOW is frozen, so the 2 s CELEBRATION_TICK_SECS gate opens exactly once.
    let remaining = sb.read_state()["levelup_renders_remaining"].as_u64().unwrap();
    assert_eq!(remaining, 2, "decremented more than once (or never)");
}
