// The cat band: one `Raster` in `AbovePrompt`, repainted by `$.ui.blit` from
// the `purrgotchi band` stream (Rust paces the frames).
//
// The render hook only records the site and size and draws the latest cells.
// `reconcile` (session.start's 100 ms clock) owns the child: it spawns it,
// and turns a new size into a debounced respawn. A frame is blitted only when
// its stream's hello size is the mounted one; blits of another size would be
// denied anyway, and a stale frame must not count as shown.
import type { EngineInterface } from 'claude-code'

import { locateBinary } from './locate'
import { readOptions } from './options'
import type { BandOptions, Rows } from './options'
import { errText, newStream, startStream, stopStream, streamWants } from './stream'
import type { BandStream, StreamHost } from './stream'
import type { BandLine } from '../types'

export const KEY = 'purr-band'
/** The narrowest band the cat is drawn in. */
export const MIN_COLS = 24
/** The widest band (`purrgotchi band --cols` caps there too). */
export const MAX_COLS = 512
const DEFAULT_COLOR = 0x0100_0000
const SPACE = 0x20

/** The `$` a blit runs on (the session start's, as the stream's lines arrive there). */
export type BlitHost = {
  readonly ui: Pick<EngineInterface['ui'], 'blit'>
  readonly clock: Pick<EngineInterface['clock'], 'now'>
}

/** The `$` reconcile runs on: the stream's and the blits'. */
export type BandHost = StreamHost & BlitHost

type Stats = { lines: number; frames: number; sent: number; ok: number; denies: Record<string, number> }

type BandState = {
  opts: BandOptions | null
  bin: string | null
  requestId: string
  cols: number
  rows: Rows | 0
  helloCols: number
  helloRows: number
  cells: string | null
  lastOkAt: number
  stateDir: string | null
  ctxPct: number
  stream: BandStream | null
  stats: Stats
}

const fresh = (): BandState => ({
  opts: null,
  bin: null,
  requestId: '',
  cols: 0,
  rows: 0,
  helloCols: 0,
  helloRows: 0,
  cells: null,
  lastOkAt: 0,
  stateDir: null,
  ctxPct: 0,
  stream: null,
  stats: { lines: 0, frames: 0, sent: 0, ok: 0, denies: {} },
})

/** Module state: written by session.start, the render hook and the stream. */
export const B: BandState = fresh()

/** A new session start: the old child (if any) stops, everything else forgets. */
export function resetBand(): void {
  if (B.stream !== null) stopStream(B.stream)
  Object.assign(B, fresh())
}

/** `requested` if it fits in `maxRows`, else the largest of 6, 5 that does, else 0. */
export function pickRows(requested: Rows, maxRows: number): Rows | 0 {
  if (requested <= maxRows) return requested
  for (const r of [6, 5] as const) if (r <= maxRows) return r
  return 0
}

export function bandArgv(bin: string, cols: number, rows: Rows, fps: number, ctxPct: number): string[] {
  return [bin, 'band', '--cols', `${cols}`, '--rows', `${rows}`, '--fps', `${fps}`, '--context-pct', `${Math.round(ctxPct)}`]
}

let blank = { cols: 0, rows: 0, cells: '' }

/** Base64 cells of `cols x rows` terminal-default spaces (the last size cached). */
export function blankCells(cols: number, rows: number): string {
  if (blank.cols === cols && blank.rows === rows) return blank.cells
  const w = new Uint32Array(cols * rows * 3)
  for (let i = 0; i < w.length; i += 3) {
    w[i] = SPACE
    w[i + 1] = DEFAULT_COLOR
    w[i + 2] = DEFAULT_COLOR
  }
  const bytes = new Uint8Array(w.buffer) as unknown as { toBase64: () => string }
  blank = { cols, rows, cells: bytes.toBase64() }
  return blank.cells
}

/** The stream's lines are sized for what is mounted now. */
const sizeMatches = () => B.rows !== 0 && B.helloCols === B.cols && B.helloRows === B.rows

function parse(line: string): BandLine | null {
  try {
    const j = JSON.parse(line) as { v?: unknown }
    return typeof j === 'object' && j !== null && j.v === 1 ? (j as BandLine) : null
  } catch {
    return null
  }
}

function countDeny(why: string): void {
  const k = why.slice(0, 80)
  B.stats.denies[k] = (B.stats.denies[k] ?? 0) + 1
}

async function blitCells($: BlitHost, cells: string): Promise<void> {
  if (B.requestId === '') return
  B.stats.sent++
  try {
    const r = await $.ui.blit({ requestId: B.requestId, key: KEY, cells })
    if (r.deny !== undefined) {
      countDeny(r.deny)
      return
    }
    B.stats.ok++
    B.lastOkAt = await $.clock.now()
  } catch (e) {
    countDeny(`throw: ${errText(e)}`)
  }
}

/**
 * One stdout chunk's lines, in order: a hello sets the stream's size and the
 * state dir, an error blanks the band once, and only the latest frame after
 * them is stored and blitted (when its size is the mounted one).
 */
export function onBandLines($: BlitHost, lines: readonly string[]): void {
  let frame: string | null = null
  for (const text of lines) {
    const l = parse(text)
    if (l === null) continue
    B.stats.lines++
    if ('hello' in l) {
      B.helloCols = l.cols
      B.helloRows = l.rows
      B.stateDir = l.state_dir
      frame = null
    } else if ('c' in l) {
      frame = l.c
    } else if (l.ok === false) {
      frame = null
      if (B.cells !== null) {
        B.cells = null
        if (sizeMatches()) void blitCells($, blankCells(B.cols, B.rows))
      }
    }
  }
  if (frame === null || !sizeMatches()) return
  B.stats.frames++
  B.cells = frame
  void blitCells($, frame)
}

/**
 * session.start's 100 ms clock: spawn the stream once the band is mounted,
 * respawn (debounced) when its size changes. A band hidden for a moment (a
 * survey, a short terminal) keeps its child.
 */
export function reconcile($: BandHost): void {
  if (B.opts === null || B.bin === null || B.rows === 0) return
  const argv = bandArgv(B.bin, B.cols, B.rows, B.opts.fps, B.ctxPct)
  if (B.stream === null) {
    B.stream = newStream(argv, lines => onBandLines($, lines))
    startStream($, B.stream)
    return
  }
  streamWants($, B.stream, argv)
}

/** Reads the options and finds the binary; the caller starts the clock. */
export async function prepareBand($: Pick<EngineInterface, 'env' | 'fs' | 'process'>): Promise<void> {
  resetBand()
  B.opts = await readOptions($)
  B.bin = await locateBinary($)
}

/** Dev-only counters (`PURR_BAND_STATS=<path>`), for the live-check harness. */
export function statsJson(now: number): string {
  const s = B.stream
  return JSON.stringify({
    t: now,
    cols: B.cols,
    rows: B.rows,
    helloCols: B.helloCols,
    helloRows: B.helloRows,
    stateDir: B.stateDir,
    lastOkAt: B.lastOkAt,
    spawns: s?.spawns ?? 0,
    argv: s?.argv ?? [],
    ...B.stats,
  })
}
