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

/** Known codes: no_kitten, state_unavailable, bad_input, empty_quip, write_failed. */
export type ErrorJson = { v: 1; ok: false; error: string; message: string }

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

declare module 'claude-code' {
  interface PluginState {
    purrgotchi: {
      binary: BinaryStatus
    }
  }
}
