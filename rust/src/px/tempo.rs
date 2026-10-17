//! Speeds and periods of the pixel cat's motion.
//!
//! Life, scenes and celebrations are sampled at `now_ms` under a `Tempo`.
//! `STATUSLINE` holds the 1-fps constants: sampled at whole seconds it
//! reproduces the statusline exactly (`motion_hashes.txt` pins that). `BAND`
//! is what the 30-fps cat band uses.

use crate::playground::{BALL_SPEED, BUTTERFLY_SPEED, ROLL_SPEED, RUN_SPEED, YARN_SPEED, ZOOMIES_SPEED};

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
    /// Scenes and celebrations too: a chase step's beat shows on the way to
    /// it, she stops at walls to turn, leaps arc, the ball hops in arcs, the
    /// finale props run in from where they were, confetti falls piece by
    /// piece and the coin drops with a bounce.
    pub smooth: bool,
    /// How long a blink / tail flick beat shows within its second.
    pub blink_ms: u64,
    pub flick_ms: u64,
    /// Playground scene speeds.
    pub scene: ScenePace,
    /// Step of the chase re-simulation. A chasing cat covers at most
    /// `run_px_s` and strolls at up to `walk_px_s`, so chases match life.
    pub sim_step_ms: u64,
    /// How long a scene `Turn` (or roll's dizzy sit) shows at a wall.
    pub turn_ms: u64,
    /// The 2-frame period of scene props: butterfly wings, the ball's hop.
    pub prop_ms: u64,
    /// Period of the evolving flash, the greeting wave and the fx 2-frame
    /// props.
    pub flash_ms: u64,
    /// Confetti fall and sparkle rise speed, px per second.
    pub fall_px_s: i32,
    /// The sleep Z drifts 1 px every `z_ms` (3 steps a cycle).
    pub z_ms: u64,
    /// A shiny cat's twinkle: one every `twinkle_ms`, showing for the first half.
    pub twinkle_ms: u64,
}

/// Playground scene speeds, px per second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScenePace {
    pub run: u64,
    pub zoomies: u64,
    pub roll: u64,
    pub yarn: u64,
    pub ball: u64,
    pub butterfly: u64,
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
        scene: ScenePace { run: RUN_SPEED, zoomies: ZOOMIES_SPEED, roll: ROLL_SPEED, yarn: YARN_SPEED, ball: BALL_SPEED, butterfly: BUTTERFLY_SPEED },
        sim_step_ms: 1000,
        turn_ms: 1000,
        prop_ms: 1000,
        flash_ms: 1000,
        fall_px_s: 2,
        z_ms: 1000,
        twinkle_ms: 2000,
    };

    /// The 30-fps band. Life (Task 2.2): she strolls at 10 px/s, 2 px per
    /// gait frame (about the walk art's paw travel, so feet don't skate), and
    /// zoomies run at 30 px/s, 3 px per frame, over up to half the band. At full
    /// speed both step a px every whole number of frames at 30 and 60 fps,
    /// and the 200 / 100 ms gait frames are whole frames, so motion is even. Anchors move up
    /// to 44 px per block and excursions go 16-40 px, so she roams the whole
    /// band within minutes.
    ///
    /// Scenes (Task 2.3) move like life: laps run at her 30 px/s and the yarn
    /// is pushed at her 10 px/s walk, on the life gaits; zoomies go flat out
    /// at 45, a roll at 15, the ball rolls at 24 (so she can catch it) and the
    /// butterfly drifts at 8 (so she stalks it at a walk). Chases re-simulate
    /// every 200 ms, one walk gait frame: a 6 px run step, a 2 px stroll. She
    /// stops 250 ms at a wall to turn. Wings flap every 150 ms. The flash,
    /// wave, flames and twinkles flip every 250 ms (2 flashes a second, under
    /// the 3 a second photosensitivity line), confetti falls and sparkles rise
    /// at 6 px/s, the sleep Z drifts a px every 700 ms and a shiny cat
    /// twinkles every 1.4 s.
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
        scene: ScenePace { run: 30, zoomies: 45, roll: 15, yarn: 10, ball: 24, butterfly: 8 },
        sim_step_ms: 200,
        turn_ms: 250,
        prop_ms: 150,
        flash_ms: 250,
        fall_px_s: 6,
        z_ms: 700,
        twinkle_ms: 1400,
    };
}
