mod common;
use common::Sandbox;

#[test]
fn purr_state_dir_redirects_all_state_writes() {
    let sb = Sandbox::new("statedir");
    let out = sb.run(&["feed"], "", &[]);
    assert!(out.status.success(), "feed failed: {}", String::from_utf8_lossy(&out.stderr));
    assert!(sb.state.join("state.json").exists(), "state.json must land in PURR_STATE_DIR");
    assert!(sb.state.join(".purr.lock").exists(), "lock file must follow PURR_STATE_DIR");
    assert!(!sb.home.join(".claude").exists(), "nothing may be written under HOME");
}
