// The only module that runs the purrgotchi binary. Argv only, never a shell;
// every call has an explicit timeout; nothing here throws.
import type { EngineInterface } from 'claude-code'

import type { PurrOptions } from './options'
import type { BinaryStatus, ErrorJson, FrameJson } from '../types'

/** The slice of `$` this module needs (a hook's `$`, or a test adapter). */
export type ProcessHost = { readonly process: Pick<EngineInterface['process'], 'run'> }

export const TIMEOUT = { feed: 5000, turn: 5000, frame: 3000, command: 5000, quip: 5000, handshake: 3000 } as const

export type RunInit = { stdin?: string; env?: Record<string, string>; timeoutMs?: number }

export type RunOutcome<T> =
  | { kind: 'json'; exitCode: number; value: T | ErrorJson }
  | { kind: 'failed'; reason: 'rejected' | 'not-json' | 'wrong-version'; exitCode?: number; detail: string }

/** `binaryPath` (trimmed) when set, else `purrgotchi` from PATH. */
export function resolveBinary(o: PurrOptions): string {
  const p = o.binaryPath.trim()
  return p === '' ? 'purrgotchi' : p
}

export function isError(v: unknown): v is ErrorJson {
  if (typeof v !== 'object' || v === null) return false
  const o = v as Record<string, unknown>
  return o.ok === false && typeof o.error === 'string'
}

const errText = (e: unknown): string => (e instanceof Error ? e.message : String(e))

/** First non-empty stdout line as a plain object, else null. */
function parseLine(stdout: string): Record<string, unknown> | null {
  const line = stdout.split('\n').find(l => l.trim() !== '')
  if (line === undefined) return null
  try {
    const v: unknown = JSON.parse(line)
    return typeof v === 'object' && v !== null && !Array.isArray(v) ? (v as Record<string, unknown>) : null
  } catch {
    return null
  }
}

/**
 * Runs `[bin, ...args]` and parses its one JSON line. Never throws. A non-zero
 * exit that still carries `v: 1` JSON (e.g. `no_kitten`, exit 1) is `json`.
 */
export async function runJson<T>(
  $: ProcessHost,
  bin: string,
  args: readonly string[],
  init: RunInit = {},
): Promise<RunOutcome<T>> {
  // No undefined keys: the engine sees exactly what the caller set.
  const runInit: RunInit = {
    ...(init.stdin !== undefined && { stdin: init.stdin }),
    ...(init.env !== undefined && { env: init.env }),
    timeoutMs: init.timeoutMs ?? TIMEOUT.command,
  }
  let r: Awaited<ReturnType<ProcessHost['process']['run']>>
  try {
    r = await $.process.run([bin, ...args], runInit)
  } catch (e) {
    return { kind: 'failed', reason: 'rejected', detail: errText(e) }
  }
  const obj = parseLine(r.stdout)
  if (obj === null) {
    const detail = (r.stderr.trim() || r.stdout.trim()).slice(0, 200)
    return { kind: 'failed', reason: 'not-json', exitCode: r.exitCode, detail }
  }
  if (obj.v !== 1) {
    return { kind: 'failed', reason: 'wrong-version', exitCode: r.exitCode, detail: `v=${String(obj.v)}` }
  }
  return { kind: 'json', exitCode: r.exitCode, value: obj as T | ErrorJson }
}

/** `purrgotchi 3.8.0` → `3.8.0`; anything else → `unknown`. Never throws. */
async function readVersion($: ProcessHost, bin: string): Promise<string> {
  try {
    const r = await $.process.run([bin, '--version'], { timeoutMs: TIMEOUT.handshake })
    return /^purrgotchi (\S+)/.exec(r.stdout.trim())?.[1] ?? 'unknown'
  } catch {
    return 'unknown'
  }
}

/**
 * Capability handshake: does `frame --json` answer the v1 contract?
 * `no_kitten` counts as ready (fresh machine; the first feed hatches).
 */
export async function handshake(
  $: ProcessHost,
  bin: string,
): Promise<{ status: BinaryStatus; frame: FrameJson | null }> {
  const out = await runJson<FrameJson>($, bin, ['frame', '--json'], { timeoutMs: TIMEOUT.handshake })
  if (out.kind === 'failed' && out.reason === 'rejected') {
    return { status: { kind: 'missing', detail: out.detail }, frame: null }
  }
  if (out.kind === 'json' && !isError(out.value) && out.value.ok === true) {
    return { status: { kind: 'ready', version: await readVersion($, bin) }, frame: out.value }
  }
  if (out.kind === 'json' && isError(out.value) && out.value.error === 'no_kitten') {
    return { status: { kind: 'ready', version: await readVersion($, bin) }, frame: null }
  }
  return { status: { kind: 'outdated', version: await readVersion($, bin) }, frame: null }
}

/** One job at a time, in call order; a rejected job does not break the chain. */
export class Serial {
  private tail: Promise<unknown> = Promise.resolve()

  run<T>(job: () => Promise<T>): Promise<T> {
    const p = this.tail.then(job)
    this.tail = p.then(
      () => undefined,
      () => undefined,
    )
    return p
  }
}
