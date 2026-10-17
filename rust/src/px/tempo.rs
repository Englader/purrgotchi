//! Speeds and periods of the pixel cat's motion.
//!
//! Life, scenes and celebrations are sampled at `now_ms` under a `Tempo`.
//! `STATUSLINE` holds the 1-fps constants: sampled at whole seconds it
//! reproduces the statusline exactly (`motion_hashes.txt` pins that). `BAND`
//! is what the 30-fps cat band uses.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Tempo {
    /// Walking and zoomies speed, px per second.
    pub walk_px_s: i32,
    pub run_px_s: i32,
    /// Zoomies reach: `run_reach_max`, or `min(max, span / div)` when `div > 0`.
    pub run_reach_max: i32,
    pub run_reach_span_div: i32,
    /// Triangle-wave steps per 30 s block of the two anchor waves.
    pub anchor_steps: (u64, u64),
    /// Gait frame period (also the 2-frame period of scene props).
    pub gait_ms: u64,
    /// How long a blink / tail flick beat shows within its second.
    pub blink_ms: u64,
    pub flick_ms: u64,
    /// Multiplier on the playground scene speeds.
    pub scene_speed_mul: u64,
    /// Step of the chase re-simulation.
    pub sim_step_ms: u64,
    /// Period of the evolving flash (and the fx 2-frame props).
    pub flash_ms: u64,
    /// Confetti fall speed, px per second.
    pub fall_px_s: i32,
}

impl Tempo {
    pub const STATUSLINE: Tempo = Tempo {
        walk_px_s: 2,
        run_px_s: 4,
        run_reach_max: 16,
        run_reach_span_div: 0,
        anchor_steps: (8, 14),
        gait_ms: 1000,
        blink_ms: 1000,
        flick_ms: 1000,
        scene_speed_mul: 1,
        sim_step_ms: 1000,
        flash_ms: 1000,
        fall_px_s: 2,
    };

    /// Same as the statusline until Tasks 2.2 and 2.3 tune it.
    pub const BAND: Tempo = Tempo::STATUSLINE;
}
