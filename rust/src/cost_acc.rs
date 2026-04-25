//! Lifetime cost accumulator.
//!
//! `statusline::run` calls `accumulate_from_payload(session_id, cur)` on every
//! render. We track per-session monotonic deltas in a sidecar file
//! (`paths::cost_sessions_file()`) and credit each delta to
//! `state.lifetime_cost_usd`. See
//! `docs/superpowers/specs/2026-04-25-v3.4.7-cost-accumulator-design.md`.
