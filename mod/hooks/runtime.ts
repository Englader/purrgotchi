// Per-load shared context. A module reload (hot reload, config change) makes
// a fresh Runtime, so everything here is transient by design.
import type { EngineInterface, Timer } from 'claude-code'

import { resolveBinary, Serial } from './binary'
import { FeedQueue } from './feed'
import type { PurrOptions } from './options'
import type { BinaryStatus } from '../types'

export type SettingsScan = { classicHooks: boolean; classicStatusline: boolean }

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
  /** The 2 s safety flush, started once per runtime when the binary is ready. */
  flushTimer?: Timer
}

export function createRuntime(options: PurrOptions): Runtime {
  return {
    options,
    bin: resolveBinary(options),
    status: { kind: 'unknown' },
    scan: { classicHooks: false, classicStatusline: false },
    serial: new Serial(),
    warned: new Set(),
    feeds: new FeedQueue(),
  }
}

/** Every game hook checks this first and just passes through otherwise. */
export function ready(rt: Runtime): boolean {
  return rt.status.kind === 'ready'
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
