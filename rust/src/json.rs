//! Versioned machine-readable output (spec §3.1). One JSON object per line on
//! stdout. Adding fields keeps `v: 1`; renaming/removing/retyping bumps it.
//! Consumers must ignore unknown keys.

use serde::{Deserialize, Serialize};

use crate::state::{CelebrationKind, PlaygroundScene};

pub const V: u32 = 1;

pub fn print<T: Serialize>(value: &T) {
    match serde_json::to_string(value) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("json: serialize failed: {e}"),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorJson {
    pub v: u32,
    pub ok: bool,
    pub error: String,
    pub message: String,
}

pub fn print_error(code: &str, message: &str) {
    print(&ErrorJson { v: V, ok: false, error: code.to_string(), message: message.to_string() });
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinesJson {
    pub v: u32,
    pub ok: bool,
    pub lines: Vec<Vec<crate::span::JsonSpan>>,
}

pub const NO_KITTEN: &str = "No kitten hatched yet. Run: purrgotchi hatch";

/// Classic stderr line, plus the error JSON in --json mode. Caller exits 1.
pub fn no_kitten(json: bool) {
    eprintln!("{NO_KITTEN}");
    if json {
        print_error("no_kitten", NO_KITTEN);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedJson {
    pub v: u32,
    pub ok: bool,
    /// Highest-priority celebration detected by THIS invocation; null if none.
    /// Not the state's currently-running celebration.
    pub celebration: Option<CelebrationKind>,
    pub level: u64,
    pub stage: String,
    pub coins: u64,
    pub total_tool_calls: u64,
    /// A random drop was rolled in this invocation (coins already credited).
    pub drop: bool,
    /// Tool calls credited by this invocation (`--count`, default 1).
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnJson {
    pub v: u32,
    pub ok: bool,
    pub celebration: Option<CelebrationKind>,
    /// A detached `react` worker spawn was attempted this turn.
    pub react_spawned: bool,
}

/// pet / play / treat / rename result.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActionJson {
    pub v: u32,
    /// The command ran without an I/O error. A cooldown is ok:true.
    pub ok: bool,
    /// "applied" | "cooldown" | "insufficient_coins" | "renamed" | "error"
    pub result: String,
    /// The classic stdout line with ANSI stripped — short and in-character;
    /// the mod returns it as slash-command text (which the model also reads).
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bond_old: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bond_new: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capped: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_secs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub need: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub have: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub was_hungry: Option<bool>,
    /// play only: the playground scene that started (play always starts one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<PlaygroundScene>,
    /// rename only: the new name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ActionJson {
    pub fn new(ok: bool, result: &str, message: &str) -> Self {
        ActionJson { v: V, ok, result: result.to_string(), message: message.to_string(), ..Default::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CelebrationKind;

    #[test]
    fn feed_json_round_trips_with_null_celebration() {
        let j = FeedJson {
            v: V, ok: true, celebration: None, level: 2, stage: "kitten".into(),
            coins: 51, total_tool_calls: 50, drop: false, count: 1,
        };
        let s = serde_json::to_string(&j).unwrap();
        assert!(s.contains(r#""celebration":null"#), "{s}");
        assert!(s.starts_with(r#"{"v":1,"ok":true"#), "{s}");
        assert_eq!(serde_json::from_str::<FeedJson>(&s).unwrap(), j);
    }

    #[test]
    fn celebration_serializes_as_variant_name() {
        let j = TurnJson { v: V, ok: true, celebration: Some(CelebrationKind::Greeting), react_spawned: false };
        let s = serde_json::to_string(&j).unwrap();
        assert!(s.contains(r#""celebration":"Greeting""#), "{s}");
        assert_eq!(serde_json::from_str::<TurnJson>(&s).unwrap(), j);
    }

    #[test]
    fn lines_json_struct_round_trips() {
        let j = LinesJson {
            v: V, ok: true,
            lines: vec![vec![crate::span::JsonSpan { text: "a".into(), fg: Some("#0abdc6".into()), bold: false, dim: false }], vec![]],
        };
        let s = serde_json::to_string(&j).unwrap();
        assert_eq!(s, r##"{"v":1,"ok":true,"lines":[[{"text":"a","fg":"#0abdc6"}],[]]}"##);
        assert_eq!(serde_json::from_str::<LinesJson>(&s).unwrap(), j);
    }

    #[test]
    fn error_json_round_trips() {
        let j = ErrorJson { v: V, ok: false, error: "no_kitten".into(), message: "m".into() };
        let s = serde_json::to_string(&j).unwrap();
        assert_eq!(s, r#"{"v":1,"ok":false,"error":"no_kitten","message":"m"}"#);
        assert_eq!(serde_json::from_str::<ErrorJson>(&s).unwrap(), j);
    }

    #[test]
    fn action_json_omits_absent_fields_and_round_trips() {
        let mut j = ActionJson::new(true, "cooldown", "Thalia is already purring at you.");
        j.wait_secs = Some(600);
        let s = serde_json::to_string(&j).unwrap();
        assert_eq!(
            s,
            r#"{"v":1,"ok":true,"result":"cooldown","message":"Thalia is already purring at you.","wait_secs":600}"#
        );
        assert_eq!(serde_json::from_str::<ActionJson>(&s).unwrap(), j);
    }
}
