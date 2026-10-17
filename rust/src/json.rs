//! Versioned machine-readable output (spec §3.1). One JSON object per line on
//! stdout. Adding fields keeps `v: 1`; renaming/removing/retyping bumps it.
//! Consumers must ignore unknown keys.

use serde::{Deserialize, Serialize};

use crate::state::CelebrationKind;

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
    fn error_json_round_trips() {
        let j = ErrorJson { v: V, ok: false, error: "no_kitten".into(), message: "m".into() };
        let s = serde_json::to_string(&j).unwrap();
        assert_eq!(s, r#"{"v":1,"ok":false,"error":"no_kitten","message":"m"}"#);
        assert_eq!(serde_json::from_str::<ErrorJson>(&s).unwrap(), j);
    }
}
