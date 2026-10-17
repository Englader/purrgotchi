mod common;
use common::{run_json, strip_ansi, Sandbox, NOW};
use serde_json::{json, Value};

fn line_text(line: &Value) -> String {
    line.as_array().unwrap().iter().map(|s| s["text"].as_str().unwrap()).collect()
}

fn classic(sb: &Sandbox, args: &[&str]) -> String {
    let now = NOW.to_string();
    String::from_utf8(sb.run(args, "", &[("PURR_NOW", &now)]).stdout).unwrap()
}

#[test]
fn stats_json_lines_match_the_text_card_and_are_unstyled() {
    let sb = Sandbox::with_state("stats", json!({}));
    let text = classic(&sb, &["stats"]);
    let (v, _) = run_json(&sb, &["stats", "--json"], "");
    let lines = v["lines"].as_array().unwrap();
    assert_eq!(lines.len(), text.lines().count());
    for (l, t) in lines.iter().zip(text.lines()) {
        assert_eq!(line_text(l), t);
        for s in l.as_array().unwrap() {
            assert!(s.get("fg").is_none() && s.get("bold").is_none() && s.get("dim").is_none());
        }
    }
    assert_eq!(classic(&sb, &["stats", "--plain"]), text);
}

#[test]
fn card_json_colours_the_name_by_rarity() {
    let sb = Sandbox::with_state("card", json!({}));
    let text = classic(&sb, &["card"]);
    let (v, _) = run_json(&sb, &["card", "--json"], "");
    let lines = v["lines"].as_array().unwrap();
    let plain = strip_ansi(&text);
    assert_eq!(lines.len(), plain.lines().count());
    for (l, t) in lines.iter().zip(plain.lines()) {
        assert_eq!(line_text(l), t);
    }
    let name = lines[1].as_array().unwrap().iter().find(|s| s["text"] == "Thalia").expect("name span");
    assert_eq!(name["fg"], "#0abdc6", "common rarity is cyan");
}

#[test]
fn card_plain_is_the_card_without_escapes() {
    let sb = Sandbox::with_state("card_plain", json!({}));
    let text = classic(&sb, &["card"]);
    let plain = classic(&sb, &["card", "--plain"]);
    assert!(!plain.contains('\x1b'));
    assert_eq!(plain, strip_ansi(&text));
}

#[test]
fn stats_and_card_json_without_kitten_exit_1_with_error() {
    for cmd in ["stats", "card"] {
        let sb = Sandbox::new("nokitten");
        let (v, out) = run_json(&sb, &[cmd, "--json"], "");
        assert_eq!(out.status.code(), Some(1), "{cmd}");
        assert_eq!(v["error"], "no_kitten", "{cmd}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("No kitten hatched yet"));
        assert!(!sb.state.join("state.json").exists(), "{cmd} must not hatch");
    }
}

#[test]
fn hostile_name_survives_stats_json() {
    let sb = Sandbox::with_state("stats_hostile", json!({ "name": "Zoë \"the\" \\ ✦ \u{1b}[31m" }));
    let (v, _) = run_json(&sb, &["stats", "--json"], "");
    assert!(line_text(&v["lines"][0]).contains("Zoë \"the\" \\ ✦"));
}
