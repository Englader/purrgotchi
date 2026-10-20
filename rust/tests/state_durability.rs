//! End-to-end: a corrupt state.json must be restored from state.json.bak
//! (never replaced by a fresh kitten when a good backup exists), and every
//! corrupt file is kept aside under its own name.
mod common;
use common::{base_state, Sandbox};
use serde_json::json;

fn feed(sb: &Sandbox) {
    let out = sb.run(&["feed"], "", &[]);
    assert!(out.status.success(), "feed failed: {}", String::from_utf8_lossy(&out.stderr));
}

fn corpses(sb: &Sandbox) -> Vec<String> {
    sb.snapshot().into_keys().filter(|n| n.starts_with("state.json.corrupt-")).collect()
}

fn write_bak(sb: &Sandbox, name: &str, tool_calls: u64) {
    let mut st = base_state();
    st["name"] = json!(name);
    st["total_tool_calls"] = json!(tool_calls);
    std::fs::write(sb.state.join("state.json.bak"), serde_json::to_string(&st).unwrap()).unwrap();
}

#[test]
fn first_save_creates_backup_and_fresh_backup_is_kept() {
    let sb = Sandbox::with_state("bak-create", json!({}));
    assert!(!sb.state.join("state.json.bak").exists());
    feed(&sb);
    let first = std::fs::read(sb.state.join("state.json.bak")).expect("backup created by save");
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&first).unwrap()["name"], "Thalia");
    feed(&sb);
    assert_eq!(std::fs::read(sb.state.join("state.json.bak")).unwrap(), first, "<1h old backup must not be rewritten");
}

#[test]
fn nul_filled_state_recovers_from_backup() {
    let sb = Sandbox::new("nul-recover");
    std::fs::write(sb.state.join("state.json"), vec![0u8; 953]).unwrap();
    write_bak(&sb, "Marmalade", 99_999);
    feed(&sb);
    let st = sb.read_state();
    assert_eq!(st["name"], "Marmalade");
    // feed recomputes level; the high tool-call count proves it is the backup's data.
    assert!(st["total_tool_calls"].as_u64().unwrap() >= 99_999);
    assert_eq!(corpses(&sb).len(), 1);
}

#[test]
fn corrupt_without_backup_hatches_fresh() {
    let sb = Sandbox::corrupt("no-bak");
    feed(&sb);
    assert_ne!(sb.read_state()["name"], "Marmalade");
    assert_eq!(sb.read_state()["version"], 2);
    assert_eq!(corpses(&sb).len(), 1);
}

#[test]
fn corrupt_with_corrupt_backup_hatches_fresh() {
    let sb = Sandbox::corrupt("bad-bak");
    std::fs::write(sb.state.join("state.json.bak"), vec![0u8; 953]).unwrap();
    feed(&sb);
    assert_eq!(sb.read_state()["version"], 2);
    assert_eq!(corpses(&sb).len(), 1);
}

#[test]
fn missing_state_hatches_fresh_and_ignores_backup() {
    let sb = Sandbox::new("missing");
    write_bak(&sb, "Marmalade", 99_999);
    feed(&sb);
    assert_ne!(sb.read_state()["name"], "Marmalade", "first install must not read .bak");
    assert!(corpses(&sb).is_empty());
}

#[test]
fn two_corruptions_leave_two_corpses() {
    let sb = Sandbox::corrupt("two-corpses");
    feed(&sb);
    std::fs::write(sb.state.join("state.json"), "{also not json").unwrap();
    feed(&sb);
    assert_eq!(corpses(&sb).len(), 2, "{:?}", corpses(&sb));
}
