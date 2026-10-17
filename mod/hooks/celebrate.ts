// Celebrations: what the binary reports in a `feed` / `turn` result becomes a
// toast, with a native notification for the rare ones. The engine's module
// scan needs `$` passed only to plain functions, so `feed.ts` and `turn.ts`
// call these by name.
import type { EngineInterface } from 'claude-code'

import { errText, okValue, runJson, TIMEOUT } from './binary'
import { debugLog } from './runtime'
import type { Runtime } from './runtime'
import type { FeedJson, FrameJson, TurnJson } from '../types'

/** The slice of `$` celebrations need (any hook's or timer's `$`). */
export type CelebrateHost = Pick<EngineInterface, 'process' | 'ui'>

/** What the mod knows of the kitten between binary results. */
export type KittenId = { name: string; level: number; stage: string; rarity: string; breed: string }

export type CelebrationCtx = { name: string; level: number; total: number; toastStreaks: boolean }

export function kittenOf(f: FrameJson): KittenId {
  return { name: f.name, level: f.level, stage: f.stage, rarity: f.rarity, breed: f.breed }
}

/**
 * Pure: the toast for a celebration kind, or null. The numbers mirror
 * `events.rs` (STREAK_TRIGGER 5, RANDOM_DROP_REWARD 10, milestones 100 / 500 /
 * 1000 / every 1000 from 2000); change them together.
 */
export function celebrationToast(kind: string | null, ctx: CelebrationCtx): { text: string; notify: boolean } | null {
  const name = ctx.name === '' ? 'Your kitten' : ctx.name
  switch (kind) {
    case 'LevelUp':
      return { text: `${name} reached level ${ctx.level} ✦`, notify: false }
    case 'EvolvingYoung':
      return { text: `${name} evolved into a young cat!`, notify: true }
    case 'EvolvingAdult':
      return { text: `${name} evolved into an adult cat!`, notify: true }
    case 'MilestoneCenturion':
      return { text: '100 tool calls together — Centurion', notify: false }
    case 'MilestoneVeteran':
      return { text: '500 tool calls together — Veteran', notify: false }
    case 'MilestoneMaster':
      return { text: '1000 tool calls together — Master', notify: false }
    case 'MilestoneLegendary':
      return { text: `${Math.floor(ctx.total / 1000) * 1000} tool calls together — Legendary`, notify: true }
    case 'Streak':
      return ctx.toastStreaks ? { text: `5-call streak — ${name} is on fire`, notify: false } : null
    case 'RandomDrop':
      return { text: `${name} found +10¢`, notify: false }
    case 'Greeting':
      return { text: `Welcome back — ${name} missed you`, notify: false }
    default:
      return null
  }
}

/** Toasts (and maybe notifies) `text`. Never throws. */
function show($: Pick<EngineInterface, 'ui'>, text: string, notify: boolean): void {
  try {
    $.ui.toast(text)
    if (notify) void $.ui.notify(text, { title: 'Purrgotchi' }).catch(() => {})
  } catch {
    // observer: a failed toast is not worth surfacing
  }
}

/**
 * One toast per binary result. A null or unknown `kind` is silent. `total` is
 * the lifetime tool-call count (Legendary's round number); `level` is the
 * result's own (a feed's), else the cached one.
 */
export function announce(
  $: Pick<EngineInterface, 'ui'>,
  rt: Runtime,
  kind: string | null,
  total = 0,
  level = rt.kitten?.level ?? 0,
): void {
  const k = rt.kitten
  const t = celebrationToast(kind, {
    name: k?.name ?? '',
    level,
    total,
    toastStreaks: rt.options.toastStreaks,
  })
  if (t !== null) show($, t.text, t.notify)
}

/**
 * A feed result: refresh the cached level and stage, then toast. The first
 * success after a fresh machine's `no_kitten` names the new kitten instead
 * (one `frame`), replacing any celebration in the same result. Never throws.
 */
export async function onFeedResult($: CelebrateHost, rt: Runtime, r: FeedJson): Promise<void> {
  try {
    if (rt.kitten !== null) rt.kitten = { ...rt.kitten, level: r.level, stage: r.stage }
    if (rt.hadNoKitten) {
      rt.hadNoKitten = false
      const out = await rt.serial.run(() => runJson<FrameJson>($, rt.bin, ['frame', '--json'], { timeoutMs: TIMEOUT.frame }))
      const f = okValue(out)
      if (f !== null) {
        rt.kitten = kittenOf(f)
        show($, `${f.name} hatched — a ${f.rarity} ${f.breed}!`, false)
        return
      }
      debugLog($, 'hatch frame failed; announcing the feed result instead')
    }
    announce($, rt, r.celebration, r.total_tool_calls, r.level)
  } catch (e) {
    debugLog($, `celebration failed: ${errText(e)}`)
  }
}

/** A turn result: its celebration (Greeting, or a streak the turn ended). */
export function onTurnResult($: Pick<EngineInterface, 'ui'>, rt: Runtime, r: TurnJson): void {
  announce($, rt, r.celebration)
}
