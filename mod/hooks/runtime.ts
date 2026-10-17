// Per-load shared context. A module reload (hot reload, config change) makes
// a fresh Runtime, so everything here is transient by design.
import type { EngineInterface, Timer } from 'claude-code'

import { resolveBinary, Serial } from './binary'
import type { KittenId } from './celebrate'
import { FeedQueue } from './feed'
import type { PurrOptions } from './options'
import type { ServeSource } from './serve'
import type { FrameTicker } from './ticker'
import type { BinaryStatus } from '../types'

/**
 * `classicHooks`: a settings hook still feeds or turns. `handEdited`: one of
 * them is not what `install.sh --uninstall-hooks` removes.
 */
export type SettingsScan = { classicHooks: boolean; handEdited: boolean; classicStatusline: boolean }

export type Runtime = {
  options: PurrOptions
  bin: string
  status: BinaryStatus
  scan: SettingsScan
  serial: Serial
  /** Toast keys already shown this load (at most one toast per failure class). */
  warned: Set<string>
  /** Tool calls counted but not yet sent to the binary. */
  feeds: FeedQueue
  /** The cached kitten (null = unknown); levels and stages follow the feed results. */
  kitten: KittenId | null
  /** The handshake said `no_kitten`: the next successful feed hatched one. */
  hadNoKitten: boolean
  /** The 2 s safety flush, started once per runtime when the binary is ready. */
  flushTimer?: Timer
  /** The frame clock, started at the first ready session start. */
  ticker?: FrameTicker
  /** The pane's last `bodyColumns` (null until drawn); the ticker's `--width`. */
  paneColumns: number | null
  /** The context window's fill, from `session.measure` (null: not known yet). */
  ctxPct: number | null
  /** The status entry last shown (undefined: cleared). */
  lastStatus?: string
  /** Consecutive ticks with no usable answer. */
  frameFailures: number
  /** The binary rejected `--pixels` (an older build): ticks stop asking for it. */
  noPixels: boolean
  /** The `purrgotchi serve` frame stream (dock / bottom), once spawned. */
  serve?: ServeSource
  /** Never spawn `serve`: the clock polls `frame` instead. */
  serveDisabled: boolean
}

export function createRuntime(options: PurrOptions): Runtime {
  return {
    options,
    bin: resolveBinary(options),
    status: { kind: 'unknown' },
    scan: { classicHooks: false, handEdited: false, classicStatusline: false },
    serial: new Serial(),
    warned: new Set(),
    feeds: new FeedQueue(),
    kitten: null,
    hadNoKitten: false,
    paneColumns: null,
    ctxPct: null,
    frameFailures: 0,
    noPixels: false,
    serveDisabled: false,
  }
}

/** Every game hook checks this first and just passes through otherwise. */
export function ready(rt: Runtime): boolean {
  return rt.status.kind === 'ready'
}

/** What is wrong with the binary: a start-up toast, or a command's answer. */
export function binaryProblem(status: BinaryStatus): string {
  switch (status.kind) {
    case 'missing':
      return 'Purrgotchi binary not found — set Binary path in /config or run `cargo install --path rust`'
    case 'outdated':
      return `Purrgotchi binary ${status.version} is too old for this mod — rebuild it: \`cargo install --path rust\``
    default:
      return 'Purrgotchi is not ready yet — try again in a moment.'
  }
}

/** Toasts `text` once per runtime per `key`. Never throws. */
export function warnOnce($: Pick<EngineInterface, 'ui'>, rt: Runtime, key: string, text: string): void {
  if (rt.warned.has(key)) return
  rt.warned.add(key)
  try {
    $.ui.toast(text)
  } catch {
    // observer: a failed toast is not worth surfacing
  }
}

/** A debug-log line. Never throws. */
export function debugLog($: Pick<EngineInterface, 'ui'>, text: string): void {
  try {
    $.ui.log(text, { to: 'debug' })
  } catch {
    // observer
  }
}
