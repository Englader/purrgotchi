// The frame clock: one `frame --json` per tick into the `frame` atom, plus the
// status entry. The pane, band and status line only read what it writes.
//
// Single-flight: a tick (or poke) while one is in flight shares it, so a slow
// or hung binary never piles up processes. It does not go through `rt.serial`:
// `frame` is read-mostly and `--advance` takes the state lock itself. A failed
// tick keeps the last frame and never toasts. The engine's module scan needs
// every `$` passed by name to a plain function, so `FrameTicker` holds state
// only and the functions below take `($, rt)`.
import type { EngineInterface, StateDollar, Timer } from 'claude-code'

import { writeFrame } from './atoms'
import { errorCode, errText, okValue, runJson, TIMEOUT } from './binary'
import { kittenOf } from './celebrate'
import type { FeedHost } from './feed'
import { statusText } from './hud'
import type { PurrOptions } from './options'
import { debugLog, ready } from './runtime'
import type { Runtime } from './runtime'
import type { FrameJson } from '../types'

/** The slice of `$` a tick needs (any hook's or timer's `$`). */
export type TickHost = FeedHost & StateDollar

/** Consecutive failed ticks before one debug line. */
const FAILURES_LOGGED_AT = 5
const STRIP_MIN = 24
const STRIP_MAX = 120

export class FrameTicker {
  timer: Timer | null = null
  /** The tick in progress, shared by every caller until it settles. */
  inflight: Promise<void> | null = null
}

/** `dock` / `bottom` animate every second; `classic` only feeds the status entry. */
export function tickIntervalMs(layout: PurrOptions['layout']): number | null {
  switch (layout) {
    case 'dock':
    case 'bottom':
      return 1000
    case 'classic':
      return 5000
    default:
      return null
  }
}

const finite = (n: number | null): number => (n !== null && Number.isFinite(n) ? n : 0)

/** Width `0` is "no scene", never the inherited `COLUMNS`. */
export function frameArgv(o: { width: number; ctxPct: number | null; advance: boolean }): string[] {
  const argv = [
    'frame',
    '--json',
    '--width',
    String(Math.max(0, Math.floor(finite(o.width)))),
    '--context-pct',
    finite(o.ctxPct).toFixed(1),
  ]
  if (o.advance) argv.push('--advance')
  return argv
}

/** The scene strip's width for the current layout: 0 (no scene) below 24, at most 120. */
export function stripWidth(rt: Runtime): number {
  const { paneColumns, bandColumns } = rt.view
  let w = 0
  if (rt.options.layout === 'dock' && paneColumns !== null) w = paneColumns - 4
  else if (rt.options.layout === 'bottom' && bandColumns !== null) w = bandColumns - 40
  w = Math.floor(finite(w))
  return w < STRIP_MIN ? 0 : Math.min(STRIP_MAX, w)
}

/** Shows `text` as the status entry (undefined clears it), only when it changed. Never throws. */
export function setStatus($: Pick<EngineInterface, 'ui'>, rt: Runtime, text: string | undefined): void {
  if (text === rt.lastStatus) return
  try {
    $.ui.status(text)
    rt.lastStatus = text
  } catch (e) {
    debugLog($, `status entry failed: ${errText(e)}`)
  }
}

/** Stores a frame (null: no kitten) and its status entry. Never throws. */
export async function showFrame($: TickHost, rt: Runtime, frame: FrameJson | null): Promise<void> {
  try {
    await writeFrame($, frame)
  } catch (e) {
    debugLog($, `frame not stored: ${errText(e)}`)
  }
  setStatus($, rt, statusText(frame))
}

/**
 * Starts the clock once per runtime (a later session start only ticks), for
 * any layout with an interval. Never throws.
 */
export function startTicker($: TickHost & Pick<EngineInterface, 'clock'>, rt: Runtime): void {
  const ms = tickIntervalMs(rt.options.layout)
  if (ms === null) return
  const t = (rt.ticker ??= new FrameTicker())
  try {
    t.timer ??= $.clock.every(ms, () => {
      void tickFrame($, rt)
    })
  } catch (e) {
    debugLog($, `frame clock not started: ${errText(e)}`)
  }
  void tickFrame($, rt)
}

/** An immediate tick (still single-flight), once the ticker has started. */
export function pokeTicker($: TickHost, rt: Runtime): void {
  if (rt.ticker !== undefined) void tickFrame($, rt)
}

/** One tick, or the one in flight. Never throws. */
export function tickFrame($: TickHost, rt: Runtime): Promise<void> {
  const t = rt.ticker
  if (t === undefined || !ready(rt)) return Promise.resolve()
  if (t.inflight !== null) return t.inflight
  t.inflight = runTick($, rt).finally(() => {
    t.inflight = null
  })
  return t.inflight
}

async function runTick($: TickHost, rt: Runtime): Promise<void> {
  try {
    const argv = frameArgv({ width: stripWidth(rt), ctxPct: rt.ctxPct, advance: !rt.scan.classicStatusline })
    const out = await runJson<FrameJson>($, rt.bin, argv, { timeoutMs: TIMEOUT.frame })
    const f = okValue(out)
    const code = errorCode(out)
    if (f !== null || code === 'no_kitten') {
      rt.frameFailures = 0
      if (f !== null) rt.kitten = kittenOf(f)
      await showFrame($, rt, f)
      return
    }
    // No answer, or an error about the state (lock busy, corrupt file): the
    // last frame stays; a run of them is one debug line.
    const why = code ?? (out.kind === 'failed' ? `${out.reason}: ${out.detail}` : 'not ok')
    if (++rt.frameFailures === FAILURES_LOGGED_AT) debugLog($, `frame failed ${FAILURES_LOGGED_AT} times in a row: ${why}`)
  } catch (e) {
    debugLog($, `frame tick failed: ${errText(e)}`)
  }
}
