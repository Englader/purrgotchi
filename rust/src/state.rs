use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CelebrationKind {
    #[default]
    None,
    LevelUp,
    EvolvingYoung,
    EvolvingAdult,
    // D-group (v3.4.8)
    Streak,
    MilestoneCenturion,
    MilestoneVeteran,
    MilestoneMaster,
    MilestoneLegendary,
    Greeting,
    RandomDrop,
}

/// Playground scene kinds (v3.8.0). `None` = idle. See
/// docs/superpowers/specs/2026-07-11-v3.8.0-playground-scenes-design.md.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaygroundScene {
    #[default]
    None,
    Run,
    Zoomies,
    BallChase,
    ButterflyChase,
    Roll,
    YarnPush,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct State {
    pub version: u32,
    pub name: String,
    pub birth_time: u64,
    pub last_fed_at: u64,
    pub total_tool_calls: u64,
    pub coins: u64,
    pub level: u64,
    pub lifetime_cost_usd: f64,
    // identity (v2)
    pub breed: String,
    pub rarity: String,
    pub shiny: bool,
    pub stats: [u8; 5],       // [CURIOSITY, MISCHIEF, LAZINESS, AFFECTION, WISDOM]
    pub personality: String,  // blurb with literal `{name}` placeholder; substituted at render
    // celebration overlay (v3.4.3, additive — serde(default) for back-compat)
    #[serde(default)]
    pub levelup_renders_remaining: u8,
    #[serde(default)]
    pub celebration_kind: CelebrationKind,
    // bond (v3.4.5, additive — serde(default) for back-compat)
    #[serde(default = "default_bond")]
    pub bond: u8,
    #[serde(default)]
    pub last_pet_at: u64,
    // interactions (v3.4.6, additive — serde(default) for back-compat)
    #[serde(default)]
    pub last_played_at: u64,
    #[serde(default)]
    pub last_treated_at: u64,
    // D-group events (v3.4.8, additive — serde(default) for back-compat)
    #[serde(default)]
    pub streak_count: u8,
    #[serde(default)]
    pub last_milestone_celebrated: u64,
    #[serde(default)]
    pub last_turn_at: u64,
    // celebration pacing (v3.7.0, additive — serde(default) for back-compat).
    // With statusLine refreshInterval=1 the statusline renders every second;
    // this timestamp gates the per-render celebration countdown so a 3-render
    // celebration lasts seconds of wall time, not 3 consecutive refreshes.
    #[serde(default)]
    pub last_celebration_tick_at: u64,
    // playground scenes (v3.8.0, additive — serde(default) for back-compat).
    // A scene is pure data; every frame renders as a function of elapsed time,
    // so nothing here changes per-frame — only at scene start/end.
    #[serde(default)]
    pub scene_kind: PlaygroundScene,
    #[serde(default)]
    pub scene_started_at: u64,
    #[serde(default)]
    pub scene_duration_secs: u64,
    #[serde(default)]
    pub scene_seed: u64,
    #[serde(default)]
    pub scene_next_at: u64,
    #[serde(default)]
    pub scene_last_kind: PlaygroundScene,
}

fn default_bond() -> u8 { crate::bond::BOND_START }

impl State {
    pub fn base(now: u64) -> Self {
        State {
            version: 2,
            name: "Purrgotchi".to_string(),
            birth_time: now,
            last_fed_at: now,
            total_tool_calls: 0,
            coins: 0,
            level: 1,
            lifetime_cost_usd: 0.0,
            breed: String::new(),
            rarity: String::new(),
            shiny: false,
            stats: [0u8; 5],
            personality: String::new(),
            levelup_renders_remaining: 0,
            celebration_kind: CelebrationKind::None,
            bond: crate::bond::BOND_START,
            last_pet_at: 0,
            last_played_at: 0,
            last_treated_at: 0,
            streak_count: 0,
            last_milestone_celebrated: 0,
            last_turn_at: 0,
            last_celebration_tick_at: 0,
            scene_kind: PlaygroundScene::None,
            scene_started_at: 0,
            scene_duration_secs: 0,
            scene_seed: 0,
            scene_next_at: 0,
            scene_last_kind: PlaygroundScene::None,
        }
    }
}

pub fn life_stage(level: u64) -> &'static str {
    match level {
        1..=9 => "kitten",
        10..=49 => "young",
        _ => "adult",
    }
}

/// Dev/test clock override (keep out of README): PURR_NOW=<unix seconds>.
/// Unset → the real clock; behaviour is unchanged.
pub fn now_override() -> Option<u64> {
    now_override_from(std::env::var("PURR_NOW").ok().as_deref())
}

pub(crate) fn now_override_from(v: Option<&str>) -> Option<u64> {
    // 0 means "no clock" and anything past this bound overflows the i64
    // arithmetic (seconds → µs, age deltas) downstream; both fall back to the
    // real clock instead of poisoning time-dependent state.
    const MAX_NOW_SECS: u64 = (i64::MAX / 1_000_000) as u64;
    v?.trim().parse::<u64>().ok().filter(|n| (1..=MAX_NOW_SECS).contains(n))
}

pub fn now_secs() -> u64 {
    if let Some(n) = now_override() {
        return n;
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Microsecond twin of [`now_secs`] for code that needs sub-second precision
/// (the turn-cost grace window). PURR_NOW → whole seconds × 1e6.
pub fn now_micros() -> u64 {
    now_micros_from(now_override())
}

pub(crate) fn now_micros_from(override_secs: Option<u64>) -> u64 {
    if let Some(n) = override_secs {
        return n.saturating_mul(1_000_000);
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}

fn parse_state(bytes: &[u8]) -> Option<State> {
    let s: State = serde_json::from_slice(bytes).ok()?;
    if s.version != 2 { return None; }
    Some(s)
}

/// Why a state load failed. `load()` collapses everything to Option for
/// read-only display callers; anything that might *write* state.json must go
/// through this (or `locked_update`) so a transient read failure or a corrupt
/// file can never be answered by overwriting the user's permanent kitten.
#[allow(clippy::large_enum_variant)] // Loaded(State) is the hot path; boxing it buys nothing
pub enum LoadOutcome {
    Loaded(State),
    /// state.json does not exist — hatching a fresh kitten is safe.
    Missing,
    /// Transient read failure (e.g. a Windows sharing violation while an AV
    /// scanner or another process holds the file). The kitten still exists.
    Unreadable,
    /// The file exists but no longer parses as v2 state. Preserve it aside
    /// before hatching so the data stays recoverable.
    Corrupt,
}

pub fn load_outcome() -> LoadOutcome {
    match fs::read(paths::state_file()) {
        Ok(bytes) => match parse_state(&bytes) {
            Some(s) => LoadOutcome::Loaded(s),
            None => LoadOutcome::Corrupt,
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => LoadOutcome::Missing,
        Err(_) => LoadOutcome::Unreadable,
    }
}

/// Why a read-only command could not show the kitten.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadFail {
    /// No state.json — genuinely nothing hatched yet.
    Missing,
    /// Still unreadable after the retries (e.g. a Windows sharing violation).
    Unreadable,
    /// state.json exists but is not valid v2 state.
    Corrupt,
}

/// `load_outcome()` with a short retry for transient `Unreadable` results
/// (2 x 15ms), shared by `hatch::load_or_hatch` and the read-only commands.
pub fn load_outcome_retry() -> LoadOutcome {
    let mut outcome = load_outcome();
    for _ in 0..2 {
        if !matches!(outcome, LoadOutcome::Unreadable) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(15));
        outcome = load_outcome();
    }
    outcome
}

/// For read-only commands: the kitten, or exactly why it could not be shown.
/// Never writes. A transient lock error must not look like "no kitten".
pub fn load_for_read() -> Result<State, ReadFail> {
    match load_outcome_retry() {
        LoadOutcome::Loaded(s) => Ok(s),
        LoadOutcome::Missing => Err(ReadFail::Missing),
        LoadOutcome::Unreadable => Err(ReadFail::Unreadable),
        LoadOutcome::Corrupt => Err(ReadFail::Corrupt),
    }
}

pub fn load() -> Option<State> {
    match load_outcome() {
        LoadOutcome::Loaded(s) => Some(s),
        _ => None,
    }
}

/// Exclusive cross-process lock serializing every state.json (and sidecar)
/// read-modify-write cycle. OS-backed (LockFileEx/flock via fs2), so a killed
/// process can never leave a stale lock behind.
pub struct StateLock {
    file: fs::File,
}

impl Drop for StateLock {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.file);
    }
}

const LOCK_ATTEMPTS: u32 = 50;
const LOCK_RETRY_MS: u64 = 10;

/// Try for ~500ms, then give up (returns None). Callers treat a missed lock
/// as "skip this mutation" — the next hook/render retries, which is always
/// safer than an unlocked write.
pub fn acquire_lock() -> Option<StateLock> {
    use fs2::FileExt;
    let dir = paths::state_dir();
    fs::create_dir_all(&dir).ok()?;
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(paths::lock_file())
        .ok()?;
    for _ in 0..LOCK_ATTEMPTS {
        if file.try_lock_exclusive().is_ok() {
            return Some(StateLock { file });
        }
        std::thread::sleep(std::time::Duration::from_millis(LOCK_RETRY_MS));
    }
    None
}

/// Run a read-modify-write cycle on state.json under the exclusive lock.
/// Missing state hatches fresh inside the lock; corrupt state is preserved
/// aside first, then restored from state.json.bak when that is valid; a transient read failure aborts with Err — the next event
/// retries — rather than mutating a stale or fabricated snapshot.
pub fn locked_update<R>(f: impl FnOnce(&mut State) -> R) -> io::Result<R> {
    let _lock = acquire_lock()
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "state lock busy"))?;
    let mut s = match load_outcome() {
        LoadOutcome::Loaded(s) => s,
        LoadOutcome::Missing => crate::hatch::generate(&crate::hatch::HatchOpts::default()),
        LoadOutcome::Corrupt => {
            preserve_corrupt_aside();
            // Prefer the last-good backup over a fresh kitten; only an absent
            // or equally unparseable .bak falls through to hatching.
            match load_backup() {
                Some(b) => {
                    crate::debug::log("state.json corrupt — recovered from state.json.bak");
                    b
                }
                None => crate::hatch::generate(&crate::hatch::HatchOpts::default()),
            }
        }
        LoadOutcome::Unreadable => {
            return Err(io::Error::other("state.json unreadable"));
        }
    };
    let r = f(&mut s);
    save_atomic(&s)?;
    Ok(r)
}

/// Move an unparseable state.json aside (state.json.corrupt-<unix_secs>) so a
/// fresh hatch can't destroy recoverable data. Best-effort. Every corpse is
/// kept under its own name: a second corruption must never overwrite the
/// first, which may be the only copy of a long-lived kitten.
pub fn preserve_corrupt_aside() {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    preserve_corrupt_aside_in(&paths::state_dir(), secs);
}

fn preserve_corrupt_aside_in(dir: &Path, secs: u64) {
    let base = format!("state.json.corrupt-{secs}");
    let mut dst = dir.join(&base);
    if dst.exists() {
        let pid = process::id();
        dst = dir.join(format!("{base}-{pid}"));
        let mut n = 1;
        while dst.exists() {
            dst = dir.join(format!("{base}-{pid}-{n}"));
            n += 1;
        }
    }
    let _ = fs::rename(dir.join("state.json"), &dst);
    crate::debug::log(&format!(
        "state.json unparseable — preserved as {}",
        dst.file_name().unwrap_or_default().to_string_lossy()
    ));
}

/// The last-good backup, if it exists and parses as valid v2 state.
fn load_backup() -> Option<State> {
    load_backup_at(&paths::state_backup_file())
}

fn load_backup_at(bak: &Path) -> Option<State> {
    parse_state(&fs::read(bak).ok()?)
}

/// A backup older than this is refreshed on the next save.
const BACKUP_MAX_AGE_SECS: u64 = 3600;

/// Refresh state.json.bak from a just-saved state when it is missing or older
/// than an hour. Best-effort: never fails the save. Written atomically (tmp +
/// fsync + rename) so the backup itself can't be half-written.
fn refresh_backup_at(bak: &Path, state: &State) {
    let fresh = fs::metadata(bak)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|m| SystemTime::now().duration_since(m).ok())
        .is_some_and(|age| age.as_secs() < BACKUP_MAX_AGE_SECS);
    if fresh {
        return;
    }
    // ".state.*.tmp" so sweep_stale_files reaps a leaked one.
    let tmp = bak.with_file_name(format!(".state.bak.{}.tmp", process::id()));
    if write_json(&tmp, state).is_err() || fs::rename(&tmp, bak).is_err() {
        let _ = fs::remove_file(&tmp);
    }
}

/// Remove orphaned atomic-write temp files (>24h) and stale per-session turn
/// anchors (>12h). PID-named temps leak when a process dies between create
/// and rename; anchors accumulate one file per session. Called from
/// `hooks::turn` — once per user prompt is plenty.
pub fn sweep_stale_files() {
    let Ok(entries) = fs::read_dir(paths::state_dir()) else { return };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy().into_owned();
        let is_tmp = name.starts_with('.') && name.ends_with(".tmp");
        let is_anchor = name.starts_with("turn-start-cost-") && name.ends_with(".txt");
        if !is_tmp && !is_anchor {
            continue;
        }
        let max_age_secs = if is_tmp { 24 * 3600 } else { 12 * 3600 };
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|m| now.duration_since(m).ok())
            .is_some_and(|age| age.as_secs() > max_age_secs);
        if stale {
            let _ = fs::remove_file(entry.path());
        }
    }
}

pub fn save_atomic(state: &State) -> io::Result<()> {
    let dir = paths::state_dir();
    fs::create_dir_all(&dir)?;

    let pid = process::id();
    let tmp = dir.join(format!(".state.{pid}.tmp"));
    let final_path = paths::state_file();

    if let Err(e) = write_json(&tmp, state) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    match fs::rename(&tmp, &final_path) {
        Ok(()) => {
            refresh_backup_at(&paths::state_backup_file(), state);
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

fn write_json(path: &Path, state: &State) -> io::Result<()> {
    // Atomicity comes from tempfile + rename; the fsync below is what makes it
    // crash-safe. Without it the rename can reach disk before the data does, so
    // a power loss between the two leaves a zero-filled state.json (observed
    // on Windows: 953 NUL bytes in place of the kitten).
    let mut f = fs::File::create(path)?;
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    f.write_all(json.as_bytes())?;
    f.write_all(b"\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_state_returns_none() {
        let v1_json = r#"{
            "version": 1,
            "name": "Thalia",
            "birth_time": 1000,
            "last_fed_at": 1000,
            "total_tool_calls": 5,
            "coins": 5,
            "level": 1,
            "lifetime_cost_usd": 0.1
        }"#;
        assert!(parse_state(v1_json.as_bytes()).is_none());
    }

    #[test]
    fn v2_state_loads_correctly() {
        let v2_json = r#"{
            "version": 2,
            "name": "Thalia",
            "birth_time": 1000,
            "last_fed_at": 1000,
            "total_tool_calls": 5,
            "coins": 5,
            "level": 1,
            "lifetime_cost_usd": 0.1,
            "breed": "Tabby",
            "rarity": "common",
            "shiny": false,
            "stats": [42, 80, 10, 55, 30],
            "personality": "A streetwise cat."
        }"#;
        let s = parse_state(v2_json.as_bytes()).expect("v2 JSON should load");
        assert_eq!(s.breed, "Tabby");
        assert_eq!(s.stats[1], 80);
    }

    #[test]
    fn life_stage_kitten_boundaries() {
        assert_eq!(life_stage(1), "kitten");
        assert_eq!(life_stage(9), "kitten");
    }

    #[test]
    fn life_stage_young_boundaries() {
        assert_eq!(life_stage(10), "young");
        assert_eq!(life_stage(49), "young");
    }

    #[test]
    fn life_stage_adult_boundaries() {
        assert_eq!(life_stage(50), "adult");
        assert_eq!(life_stage(9_999), "adult");
    }

    #[test]
    fn v2_state_without_celebration_fields_defaults_clean() {
        // A v3.4.2-shaped state.json has no levelup_renders_remaining or
        // celebration_kind fields. serde(default) must make them zero/None.
        let json = r#"{
            "version": 2,
            "name": "Thalia",
            "birth_time": 1000,
            "last_fed_at": 1000,
            "total_tool_calls": 5,
            "coins": 5,
            "level": 1,
            "lifetime_cost_usd": 0.1,
            "breed": "Tabby",
            "rarity": "common",
            "shiny": false,
            "stats": [42, 80, 10, 55, 30],
            "personality": "A streetwise cat."
        }"#;
        let s = parse_state(json.as_bytes()).expect("v3.4.2 shape should load");
        assert_eq!(s.levelup_renders_remaining, 0);
        assert_eq!(s.celebration_kind, CelebrationKind::None);
    }

    #[test]
    fn v3_4_4_state_without_bond_fields_defaults_to_fifty_and_zero() {
        // A v3.4.4-shaped state.json has no bond or last_pet_at fields.
        // serde(default) must make bond=50 (BOND_START), last_pet_at=0.
        let json = r#"{
            "version": 2,
            "name": "Thalia",
            "birth_time": 1000,
            "last_fed_at": 1000,
            "total_tool_calls": 5,
            "coins": 5,
            "level": 1,
            "lifetime_cost_usd": 0.1,
            "breed": "Tabby",
            "rarity": "common",
            "shiny": false,
            "stats": [42, 80, 10, 55, 30],
            "personality": "A streetwise cat."
        }"#;
        let s = parse_state(json.as_bytes()).expect("v3.4.4 shape should load");
        assert_eq!(s.bond, 50);
        assert_eq!(s.last_pet_at, 0);
    }

    #[test]
    fn state_base_initializes_bond_at_fifty() {
        let s = State::base(12345);
        assert_eq!(s.bond, 50);
        assert_eq!(s.last_pet_at, 0);
    }

    #[test]
    fn v3_4_5_state_without_play_treat_fields_defaults_to_zero() {
        // A v3.4.5-shaped state.json has `bond` and `last_pet_at` but no
        // `last_played_at` or `last_treated_at`. serde(default) must make
        // both default to 0 ("never").
        let json = r#"{
            "version": 2,
            "name": "Thalia",
            "birth_time": 1000,
            "last_fed_at": 1000,
            "total_tool_calls": 5,
            "coins": 5,
            "level": 1,
            "lifetime_cost_usd": 0.1,
            "breed": "Tabby",
            "rarity": "common",
            "shiny": false,
            "stats": [42, 80, 10, 55, 30],
            "personality": "A streetwise cat.",
            "bond": 72,
            "last_pet_at": 2000
        }"#;
        let s = parse_state(json.as_bytes()).expect("v3.4.5 shape should load");
        assert_eq!(s.bond, 72);
        assert_eq!(s.last_pet_at, 2000);
        assert_eq!(s.last_played_at, 0);
        assert_eq!(s.last_treated_at, 0);
    }

    #[test]
    fn state_base_initializes_play_treat_timestamps_to_zero() {
        let s = State::base(12345);
        assert_eq!(s.last_played_at, 0);
        assert_eq!(s.last_treated_at, 0);
    }

    #[test]
    fn v3_4_7_state_without_d_group_fields_defaults_to_zero() {
        let json = r#"{
            "version": 2,
            "name": "Thalia",
            "birth_time": 1000,
            "last_fed_at": 1000,
            "total_tool_calls": 5,
            "coins": 5,
            "level": 1,
            "lifetime_cost_usd": 0.1,
            "breed": "Tabby",
            "rarity": "common",
            "shiny": false,
            "stats": [42, 80, 10, 55, 30],
            "personality": "A streetwise cat.",
            "bond": 72,
            "last_pet_at": 2000,
            "last_played_at": 0,
            "last_treated_at": 0
        }"#;
        let s = parse_state(json.as_bytes()).expect("v3.4.7 shape should load");
        assert_eq!(s.streak_count, 0);
        assert_eq!(s.last_milestone_celebrated, 0);
        assert_eq!(s.last_turn_at, 0);
    }

    #[test]
    fn state_base_initializes_d_group_fields_to_zero() {
        let s = State::base(12345);
        assert_eq!(s.streak_count, 0);
        assert_eq!(s.last_milestone_celebrated, 0);
        assert_eq!(s.last_turn_at, 0);
    }

    #[test]
    fn v3_7_0_state_without_scene_fields_parses_with_defaults() {
        // Shape of a real v3.7.0 state.json — no scene_* keys at all.
        let json = r#"{
            "version": 2, "name": "Thalia", "birth_time": 100, "last_fed_at": 100,
            "total_tool_calls": 7, "coins": 3, "level": 1, "lifetime_cost_usd": 0.5,
            "breed": "tabby", "rarity": "common", "shiny": false,
            "stats": [1,2,3,4,5], "personality": "curious {name}"
        }"#;
        let s: State = serde_json::from_str(json).expect("old state must parse");
        assert_eq!(s.scene_kind, PlaygroundScene::None);
        assert_eq!(s.scene_started_at, 0);
        assert_eq!(s.scene_duration_secs, 0);
        assert_eq!(s.scene_seed, 0);
        assert_eq!(s.scene_next_at, 0);
        assert_eq!(s.scene_last_kind, PlaygroundScene::None);
    }

    #[test]
    fn scene_fields_roundtrip_through_json() {
        let mut s = State::base(1000);
        s.scene_kind = PlaygroundScene::BallChase;
        s.scene_started_at = 2000;
        s.scene_duration_secs = 30;
        s.scene_seed = 42;
        s.scene_next_at = 2500;
        s.scene_last_kind = PlaygroundScene::Roll;
        let json = serde_json::to_string(&s).unwrap();
        let back: State = serde_json::from_str(&json).unwrap();
        assert_eq!(back.scene_kind, PlaygroundScene::BallChase);
        assert_eq!(back.scene_last_kind, PlaygroundScene::Roll);
        assert_eq!(back.scene_next_at, 2500);
    }

    #[test]
    fn now_override_parses_unix_seconds() {
        assert_eq!(now_override_from(Some("1800000000")), Some(1_800_000_000));
        assert_eq!(now_override_from(Some(" 42 ")), Some(42));
    }

    #[test]
    fn now_override_ignores_absent_or_garbage() {
        assert_eq!(now_override_from(None), None);
        assert_eq!(now_override_from(Some("")), None);
        assert_eq!(now_override_from(Some("soon")), None);
        assert_eq!(now_override_from(Some("-5")), None);
    }

    #[test]
    fn now_override_rejects_zero_and_values_that_overflow_i64_micros() {
        assert_eq!(now_override_from(Some("0")), None);
        assert_eq!(now_override_from(Some("18446744073709551615")), None);
        assert_eq!(now_override_from(Some("9223372036854775807")), None);
        let max = (i64::MAX / 1_000_000) as u64;
        assert_eq!(now_override_from(Some(&max.to_string())), Some(max));
        assert_eq!(now_override_from(Some(&(max + 1).to_string())), None);
    }

    #[test]
    fn now_micros_follows_the_override_in_whole_seconds() {
        assert_eq!(now_micros_from(Some(1_800_000_000)), 1_800_000_000_000_000);
        assert_eq!(now_micros_from(Some(u64::MAX)), u64::MAX, "saturates");
    }

    #[test]
    fn now_micros_without_override_is_the_real_clock() {
        let real = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as u64;
        let got = now_micros_from(None);
        assert!(got >= real && got - real < 5_000_000, "{got} vs {real}");
    }

    fn scratch(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("purr-ut-{tag}-{}", process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn write_json_round_trips_after_fsync() {
        let d = scratch("roundtrip");
        let mut s = State::base(1000);
        s.name = "Thalia".into();
        write_json(&d.join("s.json"), &s).unwrap();
        let back = parse_state(&fs::read(d.join("s.json")).unwrap()).unwrap();
        assert_eq!(back.name, "Thalia");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn backup_created_when_missing_and_not_rewritten_when_fresh() {
        let d = scratch("bak-fresh");
        let bak = d.join("state.json.bak");
        let mut s = State::base(1000);
        s.name = "First".into();
        refresh_backup_at(&bak, &s);
        assert_eq!(load_backup_at(&bak).unwrap().name, "First");
        s.name = "Second".into();
        refresh_backup_at(&bak, &s);
        assert_eq!(load_backup_at(&bak).unwrap().name, "First", "fresh backup must be kept");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn backup_refreshed_when_older_than_an_hour() {
        let d = scratch("bak-stale");
        let bak = d.join("state.json.bak");
        let mut s = State::base(1000);
        s.name = "Old".into();
        refresh_backup_at(&bak, &s);
        let old = SystemTime::now() - std::time::Duration::from_secs(2 * 3600);
        fs::File::options().write(true).open(&bak).unwrap().set_modified(old).unwrap();
        s.name = "New".into();
        refresh_backup_at(&bak, &s);
        assert_eq!(load_backup_at(&bak).unwrap().name, "New");
        assert!(!d.join(format!(".state.bak.{}.tmp", process::id())).exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn load_backup_rejects_nul_garbage_and_missing() {
        let d = scratch("bak-bad");
        let bak = d.join("state.json.bak");
        assert!(load_backup_at(&bak).is_none());
        fs::write(&bak, vec![0u8; 953]).unwrap();
        assert!(load_backup_at(&bak).is_none());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn corpses_do_not_overwrite_each_other() {
        let d = scratch("corpses");
        fs::write(d.join("state.json"), vec![0u8; 953]).unwrap();
        preserve_corrupt_aside_in(&d, 42);
        fs::write(d.join("state.json"), b"{not json").unwrap();
        preserve_corrupt_aside_in(&d, 42);
        fs::write(d.join("state.json"), b"third").unwrap();
        preserve_corrupt_aside_in(&d, 42);
        let mut corpses: Vec<_> = fs::read_dir(&d)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("state.json.corrupt-"))
            .collect();
        corpses.sort();
        assert_eq!(corpses.len(), 3, "{corpses:?}");
        assert!(!d.join("state.json").exists());
        assert_eq!(fs::read(d.join("state.json.corrupt-42")).unwrap(), vec![0u8; 953]);
        let _ = fs::remove_dir_all(&d);
    }
}
