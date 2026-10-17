//! Versioned machine-readable output (spec §3.1). One JSON object per line on
//! stdout. Adding fields keeps `v: 1`; renaming/removing/retyping bumps it.
//! Consumers must ignore unknown keys.

use serde::{Deserialize, Serialize};

use crate::span::JsonSpan;
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

/// Read-only commands (frame/stats/card/cost/quip-prompt) when the kitten
/// could not be loaded. Missing keeps the classic `no_kitten` output; a
/// transient or corrupt state gets its own code so the mod does not mistake
/// a lock error for "not hatched". Caller exits 1.
pub fn state_read_error(fail: crate::state::ReadFail, json: bool) {
    use crate::state::ReadFail;
    let (code, msg) = match fail {
        ReadFail::Missing => return no_kitten(json),
        ReadFail::Unreadable => ("state_unavailable", "state.json is temporarily unreadable; try again"),
        ReadFail::Corrupt => ("state_corrupt", "state.json is corrupt (not valid v2 state)"),
    };
    eprintln!("{msg}");
    if json {
        print_error(code, msg);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BadArgsJson {
    pub v: u32,
    pub ok: bool,
    pub error: String,
    pub detail: String,
}

/// Bad command-line arguments. The stderr line stays (the caller prints it);
/// under `--json` stdout also gets `{"v":1,"ok":false,"error":"bad_args",...}`.
/// Caller exits 2.
pub fn print_bad_args(detail: &str) {
    print(&BadArgsJson { v: V, ok: false, error: "bad_args".to_string(), detail: detail.to_string() });
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

/// `frame --json` (spec §3.1/§4.3). Phase 3b adds `pixels` (32×32 of
/// "#rrggbb" | null) as an additive field — `v` stays 1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrameJson {
    pub v: u32,
    pub ok: bool,
    pub now: u64,
    /// Animation tick used for sprite parity (`--tick`, default = now).
    pub tick: u64,
    pub name: String,
    pub breed: String,
    pub rarity: String,
    pub shiny: bool,
    pub level: u64,
    pub stage: String,
    pub coins: u64,
    /// Effective (decayed) bond, as the HUD shows it.
    pub bond: u8,
    pub mood: String,
    /// Exactly 6 rows, one span each, coloured like the statusline sprite
    /// column. Unpadded (the statusline pads to 16).
    pub sprite: Vec<Vec<JsonSpan>>,
    /// Speech text WITHOUT the quotes the statusline wraps it in.
    pub speech: Vec<JsonSpan>,
    /// "celebration" | "birthday" | "scene" | "quip" | "mood"
    pub speech_source: String,
    pub scene: Option<SceneJson>,
    pub celebration: Option<CelebrationJson>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneJson {
    pub kind: PlaygroundScene,
    pub elapsed: u64,
    pub duration: u64,
    pub finale: bool,
    pub width: usize,
    /// 6 rows; plain width of each == `width`.
    pub rows: Vec<Vec<JsonSpan>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CelebrationJson {
    pub kind: CelebrationKind,
    /// Countdown ticks left; one tick per `tick_secs` of wall time while some
    /// renderer advances (statusline or `frame --advance`).
    pub remaining_ticks: u8,
    pub tick_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostJson {
    pub v: u32,
    pub ok: bool,
    /// USD credited to the lifetime tally by this call (0 when nothing new).
    pub credit: f64,
    pub lifetime_cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuipPromptJson {
    pub v: u32,
    pub ok: bool,
    pub mood: String,
    pub system: String,
    pub user: String,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuipStoreJson {
    pub v: u32,
    pub ok: bool,
    /// The sanitized text actually written to the quip cache.
    pub stored: String,
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

    #[test]
    fn frame_json_round_trips_with_null_scene_and_celebration() {
        use crate::span::JsonSpan;
        let span = |t: &str| JsonSpan { text: t.into(), fg: Some("#0abdc6".into()), bold: false, dim: false };
        let j = FrameJson {
            v: V, ok: true, now: 1, tick: 1, name: "Thalia".into(), breed: "Maine Coon".into(),
            rarity: "common".into(), shiny: false, level: 5, stage: "kitten".into(), coins: 340,
            bond: 50, mood: "calm".into(), sprite: vec![vec![span(" /\\_/\\ ")]; 6],
            speech: vec![span("mrow")], speech_source: "mood".into(), scene: None, celebration: None,
        };
        let s = serde_json::to_string(&j).unwrap();
        assert!(s.contains(r#""scene":null"#) && s.contains(r#""celebration":null"#), "{s}");
        assert_eq!(serde_json::from_str::<FrameJson>(&s).unwrap(), j);
    }

    #[test]
    fn scene_and_celebration_json_round_trip() {
        let sc = SceneJson { kind: crate::state::PlaygroundScene::Roll, elapsed: 3, duration: 30, finale: false, width: 40, rows: vec![vec![]; 6] };
        let s = serde_json::to_string(&sc).unwrap();
        assert!(s.contains(r#""kind":"Roll""#));
        assert_eq!(serde_json::from_str::<SceneJson>(&s).unwrap(), sc);
        let c = CelebrationJson { kind: CelebrationKind::LevelUp, remaining_ticks: 2, tick_secs: 2 };
        assert_eq!(serde_json::from_str::<CelebrationJson>(&serde_json::to_string(&c).unwrap()).unwrap(), c);
    }

    #[test]
    fn small_command_structs_round_trip() {
        let c = CostJson { v: V, ok: true, credit: 0.5, lifetime_cost_usd: 3.92 };
        let s = serde_json::to_string(&c).unwrap();
        assert_eq!(s, r#"{"v":1,"ok":true,"credit":0.5,"lifetime_cost_usd":3.92}"#);
        assert_eq!(serde_json::from_str::<CostJson>(&s).unwrap(), c);

        let p = QuipPromptJson { v: V, ok: true, mood: "calm".into(), system: "sys".into(), user: "u".into(), max_tokens: 50 };
        assert_eq!(serde_json::from_str::<QuipPromptJson>(&serde_json::to_string(&p).unwrap()).unwrap(), p);

        let q = QuipStoreJson { v: V, ok: true, stored: "mrow".into() };
        assert_eq!(serde_json::to_string(&q).unwrap(), r#"{"v":1,"ok":true,"stored":"mrow"}"#);
    }
}
