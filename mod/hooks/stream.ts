// The `purrgotchi band` child: one long-lived stream whose stdout lines go to
// the band, chunk by chunk.
//
// Lifetime (2.1.302): a spawn started from the `session.start` `$` (also from
// inside its clock callbacks) keeps its child after the hook returns. The
// engine kills it when the session ends or the module unloads (a hot reload
// too). stdin is closed from the start, so there is no control channel: a new
// size or mood is a new argv, and a new argv is a respawn (debounced 300 ms).
//
// After an unexpected exit it respawns after 1 s, then 2 s. The third exit
// within 60 s (an old binary without `band` exits 2 at once) slows it to one
// try a minute, never a permanent stop. The module scan needs
// every `$` passed by name to a plain function, so `BandStream` is state only.
import type { EngineInterface, HookStream, ProcessSpawnChunk, ProcessSpawnResult, Timer } from 'claude-code'

const DEBOUNCE_MS = 300
const QUICK_EXITS = 3
const QUICK_WINDOW_MS = 60_000
const GIVE_UP_MS = 60_000
const BACKOFF_MS = 1000
const BACKOFF_MAX_MS = 30_000

/** The `$` the child runs on: the session start's, also inside its timers. */
export type StreamHost = {
  readonly process: Pick<EngineInterface['process'], 'spawn'>
  readonly clock: Pick<EngineInterface['clock'], 'now' | 'after'>
  readonly ui: Pick<EngineInterface['ui'], 'log'>
}

/** A child streaming lines: `done` settles (never rejects) when it is over. */
export type SpawnedLines = { done: Promise<ProcessSpawnResult>; stop(): void }

export type BandStream = {
  /** The argv of the child running (or last started). */
  argv: readonly string[]
  /** The argv the band wants now; the next spawn uses it. */
  wanted: readonly string[]
  /** The live child; null while dead (a backoff respawn pending). */
  child: SpawnedLines | null
  /** Bumped per spawn: a replaced child's exit and lines are ignored. */
  gen: number
  /** Clock times of recent unexpected exits. */
  exits: number[]
  /** Inside a slow-retry episode (logged once; a stdout line ends it). */
  givingUp: boolean
  /** Children spawned so far. */
  spawns: number
  debounce: Timer | null
  /** Each stdout chunk's complete, non-empty lines, in order. */
  onLines: (lines: string[]) => void
}

const same = (a: readonly string[], b: readonly string[]) => a.length === b.length && a.every((x, i) => x === b[i])

export const errText = (e: unknown): string => (e instanceof Error ? e.message : String(e))

/** A debug-log line. Never throws. */
export function debugLog($: { readonly ui: Pick<EngineInterface['ui'], 'log'> }, text: string): void {
  try {
    $.ui.log(text, { to: 'debug' })
  } catch {
    // observer
  }
}

/** Complete lines of `rest + text`: the non-empty ones, and the partial tail to carry. */
export function splitLines(rest: string, text: string): { rest: string; lines: string[] } {
  const parts = (rest + text).split('\n')
  const tail = parts.pop() ?? ''
  return { rest: tail, lines: parts.filter(l => l.trim() !== '') }
}

export function newStream(argv: readonly string[], onLines: (lines: string[]) => void): BandStream {
  return { argv, wanted: argv, child: null, gen: 0, exits: [], givingUp: false, spawns: 0, debounce: null, onLines }
}

/** Spawns the stream's first child. Never throws. */
export function startStream($: StreamHost, s: BandStream): void {
  if (s.child === null) spawnChild($, s)
}

/** Stops the child for good (a new session start replaces the stream). */
export function stopStream(s: BandStream): void {
  s.gen++
  s.debounce?.cancel()
  s.debounce = null
  s.child?.stop()
  s.child = null
}

function spawnChild($: StreamHost, s: BandStream): void {
  s.argv = s.wanted
  const gen = ++s.gen
  s.spawns++
  const child = spawnLines($, s.argv, lines => {
    if (gen !== s.gen) return
    s.givingUp = false
    s.onLines(lines)
  })
  s.child = child
  void awaitExit($, s, gen, child)
}

/** An unexpected exit: back off and respawn, slowly after 3 quick exits. */
async function awaitExit($: StreamHost, s: BandStream, gen: number, child: SpawnedLines): Promise<void> {
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
  let ms = Math.min(BACKOFF_MS * 2 ** (s.exits.length - 1), BACKOFF_MAX_MS)
  if (s.exits.length >= QUICK_EXITS) {
    ms = GIVE_UP_MS
    if (!s.givingUp) {
      s.givingUp = true
      debugLog($, `band exited ${s.exits.length} times within ${QUICK_WINDOW_MS / 1000} s (last: code ${r.code}, signal ${r.signal}): retrying every ${GIVE_UP_MS / 1000} s`)
    }
  }
  try {
    $.clock.after(ms, () => {
      if (gen === s.gen && s.child === null) spawnChild($, s)
    })
  } catch (e) {
    debugLog($, `band respawn not scheduled: ${errText(e)}`)
  }
}

/**
 * The argv the band wants now. A change restarts the child after 300 ms
 * without another change (each new argv resets the wait). Never throws.
 */
export function streamWants($: StreamHost, s: BandStream, argv: readonly string[]): void {
  if (same(argv, s.wanted)) return
  s.wanted = argv
  s.debounce?.cancel()
  s.debounce = null
  if (same(argv, s.argv)) return // back to what runs
  try {
    s.debounce = $.clock.after(DEBOUNCE_MS, () => {
      s.debounce = null
      restartChild($, s)
    })
  } catch (e) {
    debugLog($, `band restart not scheduled: ${errText(e)}`)
  }
}

/** Replaces the live child when the wanted argv still differs; a dead one respawns with it anyway. */
function restartChild($: StreamHost, s: BandStream): void {
  if (s.child === null || same(s.wanted, s.argv)) return
  const old = s.child
  s.gen++ // its exit is not a failure
  s.child = null
  old.stop()
  spawnChild($, s)
}

/** No child: the spawn was refused, or its first pull rejected. */
const SPAWN_FAILED: ProcessSpawnResult = { code: null, signal: 'spawn-failed' }
/** The stream broke after it had started. */
const STREAM_ERROR: ProcessSpawnResult = { code: null, signal: 'stream-error' }
/** Ended by `stop()`. */
const STOPPED: ProcessSpawnResult = { code: null, signal: 'stopped' }

type SpawnControl = { stopped: boolean; stream: HookStream<ProcessSpawnChunk, ProcessSpawnResult> | null }

/** The `.catch` of a stopped stream's `return()` (a top-level function: the module scan needs one). */
function ignoreStopFailure(): void {}

/**
 * Spawns `argv` (stdin closed at once) and calls `onLines` with each stdout
 * chunk's complete, non-empty lines. stderr is ignored but for one debug
 * line; a partial line at exit is dropped. `stop()` ends the stream (the
 * engine kills the child) and no line is delivered after it. Never throws;
 * `done` never rejects.
 */
export function spawnLines($: StreamHost, argv: readonly string[], onLines: (lines: string[]) => void): SpawnedLines {
  const ctl: SpawnControl = { stopped: false, stream: null }
  const done = pumpLines($, argv, onLines, ctl)
  return {
    done,
    stop: () => {
      if (ctl.stopped) return
      ctl.stopped = true
      try {
        ctl.stream?.return(STOPPED).catch(ignoreStopFailure)
      } catch {
        // already over
      }
    },
  }
}

async function pumpLines(
  $: StreamHost,
  argv: readonly string[],
  onLines: (lines: string[]) => void,
  ctl: SpawnControl,
): Promise<ProcessSpawnResult> {
  let stream: HookStream<ProcessSpawnChunk, ProcessSpawnResult>
  try {
    stream = $.process.spawn({ argv: [...argv] })
    ctl.stream = stream
  } catch (e) {
    debugLog($, `spawn band refused: ${errText(e)}`)
    return SPAWN_FAILED
  }
  let rest = ''
  let pulled = false
  let stderrSeen = false
  try {
    for await (const chunk of stream) {
      pulled = true
      if (ctl.stopped) break
      if (chunk.stream === 'stderr') {
        if (!stderrSeen) debugLog($, `band stderr: ${chunk.text.slice(0, 200)}`)
        stderrSeen = true
        continue
      }
      const split = splitLines(rest, chunk.text)
      rest = split.rest
      if (split.lines.length === 0) continue
      try {
        onLines(split.lines)
      } catch (e) {
        debugLog($, `band line failed: ${errText(e)}`)
      }
    }
    if (ctl.stopped) return STOPPED
    return await stream.result
  } catch (e) {
    if (ctl.stopped) return STOPPED
    debugLog($, `band ${pulled ? 'stream broke' : 'did not start'}: ${errText(e)}`)
    return pulled ? STREAM_ERROR : SPAWN_FAILED
  }
}
