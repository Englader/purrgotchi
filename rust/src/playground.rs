//! Playground scenes — Thalia plays in the strip right of the HUD card.
//!
//! Deterministic engine: every frame is a pure function of
//! (kind, elapsed, duration, seed, strip_width, blink). State is written only
//! when a scene starts or ends (scheduler in statusline.rs; instant trigger in
//! hooks.rs `play`). Spec: docs/superpowers/specs/2026-07-11-v3.8.0-playground-scenes-design.md

// Items in this module gain their callers across Phases 1-3 of the build plan
// (engine -> statusline in Phase 1, choreography in Phase 2, play trigger in
// Phase 3). The bin-only crate flags not-yet-wired items as dead code, so this
// allow holds until Phase 3 Task 13 removes it once everything is live.
#![allow(dead_code)]

use crate::state::PlaygroundScene;

/// The playground is 6 rows tall — the sprite rows of the 7-row HUD (row 6 is speech).
pub const ROWS: usize = 6;
/// Below this many cells the strip can't stage a scene; render falls back to
/// the static home sprite.
pub const MIN_STRIP: usize = 24;
/// Wider than this stops adding charm and increases wrap risk on mis-reported COLUMNS.
pub const MAX_STRIP: usize = 120;
/// Used when COLUMNS is absent and no override is set.
pub const DEFAULT_STRIP: usize = 60;
/// Card renders 96 visible chars + 3-space gap + 2 safety cells. Lines longer
/// than the terminal wrap and shatter the 7-row layout, so always under-shoot.
pub const STRIP_OVERHEAD: usize = 101;
pub const SCENE_MIN_SECS: u64 = 20;
pub const SCENE_MAX_SECS: u64 = 40;
pub const IDLE_MIN_SECS: u64 = 120; // 2 min
pub const IDLE_MAX_SECS: u64 = 300; // 5 min
/// Last N seconds of a scene play its ending beat (catch/pounce).
pub const FINALE_SECS: u64 = 4;

/// Every startable scene. `PlaygroundScene::None` is deliberately absent.
pub const ACTIVE_SCENES: [PlaygroundScene; 6] = [
    PlaygroundScene::Run,
    PlaygroundScene::Zoomies,
    PlaygroundScene::BallChase,
    PlaygroundScene::ButterflyChase,
    PlaygroundScene::Roll,
    PlaygroundScene::YarnPush,
];

/// Kill switch: PURR_PLAYGROUND=0 disables scheduling and rendering.
pub fn enabled() -> bool {
    enabled_from(std::env::var("PURR_PLAYGROUND").ok().as_deref())
}

pub(crate) fn enabled_from(v: Option<&str>) -> bool {
    v != Some("0")
}

/// Usable playground width. Claude Code (>= v2.1.153) sets COLUMNS for the
/// statusline process. PURR_PLAYGROUND_WIDTH overrides everything (for hosts
/// where COLUMNS is absent or wrong). None = too narrow, don't render scenes.
pub fn strip_width() -> Option<usize> {
    strip_width_from(
        std::env::var("COLUMNS").ok().as_deref(),
        std::env::var("PURR_PLAYGROUND_WIDTH").ok().as_deref(),
    )
}

pub(crate) fn strip_width_from(
    columns: Option<&str>,
    override_w: Option<&str>,
) -> Option<usize> {
    let w = if let Some(o) = override_w.and_then(|v| v.trim().parse::<usize>().ok()) {
        o
    } else if let Some(c) = columns.and_then(|v| v.trim().parse::<usize>().ok()) {
        c.saturating_sub(STRIP_OVERHEAD)
    } else {
        DEFAULT_STRIP
    };
    let w = w.min(MAX_STRIP);
    if w < MIN_STRIP { None } else { Some(w) }
}

/// Dev backdoor (keep out of README): PURR_FORCE_SCENE=run|zoomies|ball|butterfly|roll|yarn
/// renders that scene on a repeating 30 s loop without touching state.
pub fn force_scene() -> Option<PlaygroundScene> {
    force_scene_from(std::env::var("PURR_FORCE_SCENE").ok().as_deref())
}

pub(crate) fn force_scene_from(v: Option<&str>) -> Option<PlaygroundScene> {
    match v? {
        "run" => Some(PlaygroundScene::Run),
        "zoomies" => Some(PlaygroundScene::Zoomies),
        "ball" => Some(PlaygroundScene::BallChase),
        "butterfly" => Some(PlaygroundScene::ButterflyChase),
        "roll" => Some(PlaygroundScene::Roll),
        "yarn" => Some(PlaygroundScene::YarnPush),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_only_zero_disables() {
        assert!(enabled_from(None));
        assert!(enabled_from(Some("1")));
        assert!(enabled_from(Some("")));
        assert!(!enabled_from(Some("0")));
    }

    #[test]
    fn strip_width_from_columns() {
        // 200 cols - 101 overhead = 99
        assert_eq!(strip_width_from(Some("200"), None), Some(99));
        // clamped to MAX_STRIP
        assert_eq!(strip_width_from(Some("500"), None), Some(MAX_STRIP));
        // 120 cols - 101 = 19 < MIN_STRIP -> None
        assert_eq!(strip_width_from(Some("120"), None), None);
        // garbage COLUMNS -> DEFAULT_STRIP
        assert_eq!(strip_width_from(Some("wat"), None), Some(DEFAULT_STRIP));
        // absent COLUMNS -> DEFAULT_STRIP
        assert_eq!(strip_width_from(None, None), Some(DEFAULT_STRIP));
    }

    #[test]
    fn strip_width_override_wins() {
        assert_eq!(strip_width_from(Some("200"), Some("40")), Some(40));
        assert_eq!(strip_width_from(None, Some("40")), Some(40));
        assert_eq!(strip_width_from(None, Some("10")), None); // below MIN
        assert_eq!(strip_width_from(None, Some("999")), Some(MAX_STRIP));
        assert_eq!(strip_width_from(Some("200"), Some("junk")), Some(99)); // bad override ignored
    }

    #[test]
    fn force_scene_parses_known_names_only() {
        assert_eq!(force_scene_from(Some("run")), Some(PlaygroundScene::Run));
        assert_eq!(force_scene_from(Some("ball")), Some(PlaygroundScene::BallChase));
        assert_eq!(force_scene_from(Some("yarn")), Some(PlaygroundScene::YarnPush));
        assert_eq!(force_scene_from(Some("nope")), None);
        assert_eq!(force_scene_from(None), None);
    }
}
