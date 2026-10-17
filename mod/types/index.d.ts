// Purrgotchi mod types: the binary's Phase 0 `--json` contract (mirrors
// rust/src/json.rs field for field) and the `PluginState` keys every
// `atom(...)` in hooks/atoms.ts uses.
//
// Contract rules: one JSON object per line; accept only `v === 1`; ignore
// unknown keys. Optional fields here are the ones the binary omits when unset.

/** One coloured run of text. `bold` / `dim` are absent when false. */
export type Span = { text: string; fg?: string; bold?: boolean; dim?: boolean }

export type CelebrationKind =
  | 'LevelUp'
  | 'EvolvingYoung'
  | 'EvolvingAdult'
  | 'Streak'
  | 'MilestoneCenturion'
  | 'MilestoneVeteran'
  | 'MilestoneMaster'
  | 'MilestoneLegendary'
  | 'Greeting'
  | 'RandomDrop'

export type PlaygroundScene = 'Run' | 'Zoomies' | 'BallChase' | 'ButterflyChase' | 'Roll' | 'YarnPush'

/** Error codes the binary emits; any other string is a generic failure. */
export type ErrorCode =
  | 'no_kitten'
  | 'state_unavailable'
  | 'state_corrupt'
  | 'bad_input'
  | 'bad_args'
  | 'empty_quip'
  | 'write_failed'

/**
 * Any `ok: false` object with an `error` code. Under `--json` every one exits
 * non-zero (1, or 2 for `bad_args`), so stdout is parsed whatever the exit
 * code. `bad_args` carries `detail` instead of `message`.
 */
export type ErrorJson = { v: 1; ok: false; error: ErrorCode | (string & {}); message?: string; detail?: string }

export type FeedJson = {
  v: 1
  ok: true
  celebration: CelebrationKind | null
  level: number
  stage: string
  coins: number
  total_tool_calls: number
  drop: boolean
  count: number
}

export type TurnJson = { v: 1; ok: true; celebration: CelebrationKind | null; react_spawned: boolean }

export type ActionJson = {
  v: 1
  ok: boolean
  result: 'applied' | 'cooldown' | 'insufficient_coins' | 'renamed' | 'error'
  message: string
  bond_old?: number
  bond_new?: number
  capped?: boolean
  cost?: number
  wait_secs?: number
  need?: number
  have?: number
  was_hungry?: boolean
  scene?: PlaygroundScene
  name?: string
}

export type LinesJson = { v: 1; ok: true; lines: Span[][] }

export type SceneJson = {
  kind: PlaygroundScene
  elapsed: number
  duration: number
  finale: boolean
  width: number
  rows: Span[][]
}

export type CelebrationJson = { kind: CelebrationKind; remaining_ticks: number; tick_secs: number }

export type PeakJson = { label: 'PEAK' | 'OFF PEAK' | (string & {}); range: string; is_peak: boolean }

/** Every HUD datum the pane shows, computed by the binary (no game rules in TypeScript). */
export type HudJson = {
  total_tool_calls: number
  /** Progress to the next level, 0..=98. */
  xp_pct: number
  last_fed_at: number
  stars: string
  /** `#rrggbb` the statusline colours the name with (legendary cycles). */
  name_fg: string
  peak: PeakJson
}

export type FrameJson = {
  v: 1
  ok: true
  now: number
  tick: number
  name: string
  breed: string
  rarity: string
  shiny: boolean
  level: number
  stage: string
  coins: number
  bond: number
  mood: string
  sprite: Span[][]
  speech: Span[]
  speech_source: 'celebration' | 'birthday' | 'scene' | 'quip' | 'mood'
  scene: SceneJson | null
  celebration: CelebrationJson | null
  /** Absent from binaries before Phase 3 (additive, `v` stays 1). */
  hud?: HudJson
}

export type CostJson = { v: 1; ok: true; credit: number; lifetime_cost_usd: number }

export type QuipPromptJson = { v: 1; ok: true; mood: string; system: string; user: string; max_tokens: number }

export type QuipStoreJson = { v: 1; ok: true; stored: string }

/** What session start learned about the purrgotchi binary. */
export type BinaryStatus =
  | { kind: 'unknown' }
  | { kind: 'ready'; version: string }
  | { kind: 'missing'; detail: string }
  | { kind: 'outdated'; version: string }

/** One rate-limit window as the engine measured it. */
export type RateLimitView = { kind: string; percentUsed: number; resetsAt?: string }

/**
 * The session figures the pane shows: `session.measure` pushes the first five,
 * session start seeds all of them. Null is "not known", never zero.
 */
export type MeasureView = {
  contextPct: number | null
  contextTokens: number | null
  contextWindow: number | null
  rateLimits: RateLimitView[]
  costUsd: number | null
  model: string | null
  cwd: string | null
  /** Epoch ms. */
  startedAt: number | null
}

/** One printed card/stats row: the coloured lines behind a command's summary text. */
export type OutputEntry = { key: string; lines: Span[][] }

declare module 'claude-code' {
  interface PluginState {
    purrgotchi: {
      binary: BinaryStatus
      outputs: OutputEntry[]
      frame: FrameJson | null
      measure: MeasureView
    }
  }
}
