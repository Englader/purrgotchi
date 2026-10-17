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
    /// Walking gait frame period (also the 2-frame period of scene props).
    pub gait_ms: u64,
    /// Running gait frame period.
    pub run_gait_ms: u64,
    /// How far a Wander excursion goes back the way she came, px (`lo..=hi`).
    pub wander_px: (u64, u64),
    /// Life is choreographed in ms: walks and zoomies glide from their start
    /// to their end, zoomies spin and skid round, the run bobs and each
    /// grooming lick is a dip and back up. Off, x shows the end of each
    /// whole-second step and slides toward the next (the 1-fps statusline).
    pub smooth: bool,
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
        run_gait_ms: 1000,
        wander_px: (4, 8),
        smooth: false,
        blink_ms: 1000,
        flick_ms: 1000,
        scene_speed_mul: 1,
        sim_step_ms: 1000,
        flash_ms: 1000,
        fall_px_s: 2,
    };

    /// The 30-fps band. Life (Task 2.2): she strolls at 10 px/s, 2 px per
    /// gait frame (about the walk art's paw travel, so feet don't skate), and
    /// zoomies run at 30 px/s, 3 px per frame, over up to half the band. At full
    /// speed both step a px every whole number of frames at 30 and 60 fps,
    /// and the 200 / 100 ms gait frames are whole frames, so motion is even. Anchors move up
    /// to 44 px per block and excursions go 16-40 px, so she roams the whole
    /// band within minutes. Scenes and fx keep the statusline values until
    /// Task 2.3.
    pub const BAND: Tempo = Tempo {
        walk_px_s: 10,
        run_px_s: 30,
        run_reach_max: 64,
        run_reach_span_div: 2,
        anchor_steps: (32, 56),
        gait_ms: 200,
        run_gait_ms: 100,
        wander_px: (16, 40),
        smooth: true,
        blink_ms: 150,
        flick_ms: 400,
        ..Tempo::STATUSLINE
    };
}
