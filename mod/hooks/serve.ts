// The streaming frame source: one long-lived `purrgotchi serve` child per
// runtime (dock / bottom), whose NDJSON lines go to the ticker's line handler.
//
// Lifetime (probe-after-return.sh --spawn, 2.1.302): a spawn loop started from
// the `session.start` `$` keeps its child after the hook returns, and the child
// is gone when the session ends. Every spawn here runs on that `$` (the
// ticker's clock callbacks close over it). `session.end` (`/clear`) never stops
// it; a module unload or hot reload kills it (engine) and the next runtime
// spawns its own.
//
// Respawns: a deliberate restart when the wanted argv changes (debounced
// 500 ms), or after an unexpected exit with backoff 1 s, 2 s, 4 s, ... 30 s.
// Three exits within 60 s mark the source broken (an old binary without
// `serve` exits 2 at once): no more spawns, and the ticker polls. The module
// scan needs every `$` passed by name to a plain function, so `ServeSource`
// is state only.
import type { EngineInterface, Timer } from 'claude-code'

import { spawnLines } from './binary'
import type { SpawnedLines, SpawnHost } from './binary'
import { debugLog } from './runtime'
import type { Runtime } from './runtime'

/** The probe kept the child alive from `session.start`: stream frames. */
export const SERVE_ENABLED = true

const DEBOUNCE_MS = 500
const QUICK_EXITS = 3
const QUICK_WINDOW_MS = 60_000
const BACKOFF_MS = 1000
const BACKOFF_MAX_MS = 30_000
/** Clock ticks with no line before the ticker polls again. */
const SILENT_TICKS = 3

/** The `$` a serve child runs on: the session start's, also inside its timers. */
type ServeHost = SpawnHost & Pick<EngineInterface, 'clock' | 'ui'>

export type ServeSource = {
  /** The argv of the child running (or last started). */
  argv: readonly string[]
  /** The argv the ticker wants now; the next spawn uses it. */
  wanted: readonly string[]
  /** The live child; null while dead (a backoff respawn pending, or broken). */
  child: SpawnedLines | null
  /** Bumped per spawn: a replaced child's exit is ignored. */
  gen: number
  /** Clock times of recent unexpected exits. */
  exits: number[]
  broken: boolean
  /** Clock ticks since the last line. */
  quiet: number
  debounce: Timer | null
  /** A line for the ticker; true when it was a v1 answer (serve is alive). */
  onLine: (line: string) => Promise<boolean>
}

const same = (a: readonly string[], b: readonly string[]) => a.length === b.length && a.every((x, i) => x === b[i])

/**
 * Spawns `serve` once per runtime (a later session start leaves the running
 * child alone). Never throws.
 */
export function startServe(
  $: ServeHost,
  rt: Runtime,
  argv: readonly string[],
  onLine: (line: string) => Promise<boolean>,
): void {
  if (!SERVE_ENABLED || rt.serveDisabled || rt.serve !== undefined) return
  const s: ServeSource = { argv, wanted: argv, child: null, gen: 0, exits: [], broken: false, quiet: 0, debounce: null, onLine }
  rt.serve = s
  spawnServe($, rt, s)
}

function spawnServe($: ServeHost, rt: Runtime, s: ServeSource): void {
  try {
    s.argv = s.wanted
    s.quiet = 0
    const gen = ++s.gen
    const child = spawnLines($, rt.bin, s.argv, async line => {
      if (await s.onLine(line)) s.quiet = 0
    })
    s.child = child
    void awaitExit($, rt, s, gen, child)
  } catch (e) {
    debugLog($, `serve not spawned: ${String(e)}`)
  }
}

/** An unexpected exit: back off and respawn, or give up after 3 quick exits. */
async function awaitExit($: ServeHost, rt: Runtime, s: ServeSource, gen: number, child: SpawnedLines): Promise<void> {
  const r = await child.done
  if (gen !== s.gen) return // replaced on purpose
  s.child = null
  let now = 0
  try {
    now = await $.clock.now()
  } catch {
    // no clock: every exit counts as recent
  }
  s.exits = [...s.exits.filter(t => now - t < QUICK_WINDOW_MS), now]
  if (s.exits.length >= QUICK_EXITS) {
    s.broken = true
    s.debounce?.cancel()
    debugLog($, `serve exited ${QUICK_EXITS} times within ${QUICK_WINDOW_MS / 1000} s (last: code ${r.code}, signal ${r.signal}): polling instead`)
    return
  }
  const ms = Math.min(BACKOFF_MS * 2 ** (s.exits.length - 1), BACKOFF_MAX_MS)
  try {
    $.clock.after(ms, () => {
      if (gen === s.gen && s.child === null && !s.broken) spawnServe($, rt, s)
    })
  } catch (e) {
    debugLog($, `serve respawn not scheduled: ${String(e)}`)
  }
}

/**
 * The ticker's argv now. A change restarts the child after 500 ms without
 * another change (each call with a new argv resets the wait). Never throws.
 */
export function serveWants($: ServeHost, rt: Runtime, argv: readonly string[]): void {
  const s = rt.serve
  if (s === undefined || s.broken || same(argv, s.wanted)) return
  s.wanted = argv
  s.debounce?.cancel()
  s.debounce = null
  if (same(argv, s.argv)) return // back to what runs
  try {
    s.debounce = $.clock.after(DEBOUNCE_MS, () => {
      s.debounce = null
      restartServe($, rt, s)
    })
  } catch (e) {
    debugLog($, `serve restart not scheduled: ${String(e)}`)
  }
}

/** Replaces the live child when the wanted argv still differs; a dead one respawns with it anyway. */
function restartServe($: ServeHost, rt: Runtime, s: ServeSource): void {
  if (s.broken || s.child === null || same(s.wanted, s.argv)) return
  const old = s.child
  s.gen++ // its exit is not a failure
  s.child = null
  old.stop()
  spawnServe($, rt, s)
}

/**
 * One clock tick: should the ticker poll `frame`? Yes without a serve source,
 * when it is broken, while no child is live, or after 3 ticks with no line.
 */
export function servePolls(rt: Runtime): boolean {
  const s = rt.serve
  if (s === undefined || s.broken || s.child === null) return true
  return ++s.quiet >= SILENT_TICKS
}
