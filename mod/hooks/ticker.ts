// The frame clock: frames into the `frame` atom, plus the status entry. The
// pane, band and status line only read what it writes. In dock / bottom the
// frames stream from `purrgotchi serve` (serve.ts); the 1 s clock is then its
// watchdog: it hands serve the wanted argv (pane width, context bucket) and
// polls `frame --json` only while serve is broken, dead or silent for 3 s.
// `classic` polls every 5 s.
//
// Single-flight: a tick (or poke) while one is in flight shares it, so a slow
// or hung binary never piles up processes. It does not go through `rt.serial`:
// `frame` is read-mostly and `--advance` takes the state lock itself. A failed
// tick keeps the last frame and never toasts. The engine's module scan needs
// every `$` passed by name to a plain function, so `FrameTicker` is state
// only and the functions below take `($, rt)`.
import type { EngineInterface, StateDollar, Timer } from 'claude-code'

import { writeFrame } from './atoms'
import { errorCode, errText, isError, okValue, outcomeOf, runJson, TIMEOUT } from './binary'
import type { RunOutcome } from './binary'
import { kittenOf } from './celebrate'
import type { FeedHost } from './feed'
import { finiteOr0, statusText } from './hud'
import type { PurrOptions } from './options'
import { debugLog, ready } from './runtime'
import type { Runtime } from './runtime'
import { servePolls, serveWants, startServe } from './serve'
import type { FrameJson } from '../types'

/** The slice of `$` a tick needs (any hook's or timer's `$`). */
type TickHost = FeedHost & StateDollar

/** Consecutive failed ticks before one debug line. */
const FAILURES_LOGGED_AT = 5
const STRIP_MIN = 24
const STRIP_MAX = 120

export type FrameTicker = {
  timer: Timer | null
  /** The tick in progress, shared by every caller until it settles. */
  inflight: Promise<void> | null
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

type FrameOpts = { width: number; ctxPct: number | null; advance: boolean; pixels: boolean }

/** The flags `frame` and `serve` share. Width `0` is "no scene", never the inherited `COLUMNS`. */
function frameFlags(o: FrameOpts): string[] {
  const argv = ['--width', String(Math.max(0, Math.floor(finiteOr0(o.width)))), '--context-pct', finiteOr0(o.ctxPct).toFixed(1)]
  if (o.advance) argv.push('--advance')
  if (o.pixels) argv.push('--pixels')
  return argv
}

function frameArgv(o: FrameOpts): string[] {
  return ['frame', '--json', ...frameFlags(o)]
}

function serveArgv(o: FrameOpts): string[] {
  return ['serve', '--interval-ms', '1000', ...frameFlags(o)]
}

/**
 * The context fill `serve` is started with: 5-point buckets keep the mood
 * thresholds (40, 80) exact and respawn at most about once per turn.
 */
function ctxBucket(p: number | null): number {
  return p == null || !Number.isFinite(p) ? 0 : Math.floor(p / 5) * 5
}

/** Pixel art is only drawn by the pane and the band, and only until an older binary rejects the flag. */
/** dock / bottom: the 1 s layouts, streamed from `serve` and drawn with pixels. */
const isLive = (layout: PurrOptions['layout']) => layout === 'dock' || layout === 'bottom'

function wantsPixels(rt: Runtime): boolean {
  return !rt.noPixels && isLive(rt.options.layout)
}

/** An older binary refusing `--pixels`: exit 2 with no JSON, or a `bad_args` answer. */
function pixelsRefused(out: RunOutcome<FrameJson>): boolean {
  if (out.kind === 'failed') return out.reason === 'not-json' && out.exitCode === 2
  return errorCode(out) === 'bad_args'
}

/**
 * The scene strip's width: only the pane draws a scene (the band never does),
 * so it follows the pane's body in any layout, and is 0 (no scene) until the
 * pane is drawn or below 24, at most 120.
 */
function stripWidth(rt: Runtime): number {
  const w = rt.paneColumns === null ? 0 : Math.floor(finiteOr0(rt.paneColumns - 4))
  return w < STRIP_MIN ? 0 : Math.min(STRIP_MAX, w)
}

/** The `serve` argv for the runtime as it is now. */
function wantedServeArgv(rt: Runtime): string[] {
  return serveArgv({ width: stripWidth(rt), ctxPct: ctxBucket(rt.ctxPct), advance: !rt.scan.classicStatusline, pixels: wantsPixels(rt) })
}

/** Shows `text` as the status entry (undefined clears it), only when it changed. Never throws. */
function setStatus($: Pick<EngineInterface, 'ui'>, rt: Runtime, text: string | undefined): void {
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

type ClockHost = TickHost & Pick<EngineInterface, 'clock' | 'process'>

/**
 * Starts the clock (and, at 1 s, `serve`) once per runtime (a later session
 * start only ticks), for any layout with an interval. Call it with the
 * `session.start` `$`: the serve child lives on it. Never throws.
 */
export function startTicker($: ClockHost, rt: Runtime): void {
  const ms = tickIntervalMs(rt.options.layout)
  if (ms === null) return
  const t = (rt.ticker ??= { timer: null, inflight: null })
  if (isLive(rt.options.layout)) {
    startServe($, rt, wantedServeArgv(rt), line => applyLine($, rt, line))
  }
  try {
    t.timer ??= $.clock.every(ms, () => {
      onClock($, rt)
    })
  } catch (e) {
    debugLog($, `frame clock not started: ${errText(e)}`)
  }
  void tickFrame($, rt)
}

/** A clock tick: poll when serve can't feed frames, else keep its argv current. Never throws. */
function onClock($: ClockHost, rt: Runtime): void {
  try {
    if (servePolls(rt)) void tickFrame($, rt)
    if (rt.serve !== undefined) serveWants($, rt, wantedServeArgv(rt))
  } catch (e) {
    debugLog($, `clock tick failed: ${errText(e)}`)
  }
}

/** One `serve` line, applied as a tick's answer. True when it was a v1 answer. Never throws. */
async function applyLine($: TickHost, rt: Runtime, line: string): Promise<boolean> {
  const out = outcomeOf<FrameJson>(line, '', 0)
  // Stray output, another version, or no answer shape: skipped.
  if (out.kind !== 'json' || (out.value.ok !== true && !isError(out.value))) return false
  await applyFrame($, rt, out)
  return true
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
    const pixels = wantsPixels(rt)
    const argv = frameArgv({ width: stripWidth(rt), ctxPct: rt.ctxPct, advance: !rt.scan.classicStatusline, pixels })
    const out = await runJson<FrameJson>($, rt.bin, argv, { timeoutMs: TIMEOUT.frame })
    if (pixels && pixelsRefused(out)) {
      rt.noPixels = true
      debugLog($, 'binary rejected --pixels: drawing the ASCII sprite')
    }
    await applyFrame($, rt, out)
  } catch (e) {
    debugLog($, `frame tick failed: ${errText(e)}`)
  }
}

/**
 * A `frame` answer (a run's, or one `serve` line): the frame, `no_kitten`, or
 * a failure that keeps the last frame. Never throws.
 */
async function applyFrame($: TickHost, rt: Runtime, out: RunOutcome<FrameJson>): Promise<void> {
  const f = okValue(out)
  const code = errorCode(out)
  if (f !== null || code === 'no_kitten') {
    rt.frameFailures = 0
    if (f !== null) {
      rt.kitten = kittenOf(f)
    } else {
      // As the handshake's `no_kitten`: the next feed hatches one, announced.
      // Never cleared here: a tick may see the new kitten before its feed result.
      rt.kitten = null
      rt.hadNoKitten = true
    }
    await showFrame($, rt, f)
    return
  }
  // No answer, or an error about the state (lock busy, corrupt file): the
  // last frame stays; a run of them is one debug line.
  const why = code ?? (out.kind === 'failed' ? `${out.reason}: ${out.detail}` : 'not ok')
  if (++rt.frameFailures === FAILURES_LOGGED_AT) debugLog($, `frame failed ${FAILURES_LOGGED_AT} times in a row: ${why}`)
}
