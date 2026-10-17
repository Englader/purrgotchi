// The only module that runs the purrgotchi binary. Argv only, never a shell;
// every `run` has an explicit timeout (a spawned child lives until stopped);
// nothing here throws.
import type { EngineInterface, HookStream, ProcessSpawnChunk, ProcessSpawnResult } from 'claude-code'

import type { PurrOptions } from './options'
import type { BinaryStatus, ErrorCode, ErrorJson, FrameJson } from '../types'

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

/** `resolveBinary`, with a leading `~` as the home directory. Never throws. */
export async function locateBinary($: Pick<EngineInterface, 'env'>, o: PurrOptions): Promise<string> {
  const bin = resolveBinary(o)
  if (!bin.startsWith('~')) return bin
  try {
    return expandHome(bin, (await $.env.get('HOME')) ?? (await $.env.get('USERPROFILE')))
  } catch {
    return bin // no home: the path stays as typed
  }
}

/** A leading `~` (alone, or before `/` or `\`) as `home`; anything else, or no home, as is. */
export function expandHome(path: string, home: string | undefined): string {
  if (home === undefined || home === '' || !/^~(?=$|[/\\])/.test(path)) return path
  return home.replace(/[/\\]+$/, '') + path.slice(1)
}

export function isError(v: unknown): v is ErrorJson {
  if (typeof v !== 'object' || v === null) return false
  const o = v as Record<string, unknown>
  return o.ok === false && typeof o.error === 'string'
}

export const errText = (e: unknown): string => (e instanceof Error ? e.message : String(e))

/** No undefined keys: the engine sees exactly what the caller set. */
function runInitOf(init: RunInit): RunInit {
  return {
    ...(init.stdin !== undefined && { stdin: init.stdin }),
    ...(init.env !== undefined && { env: init.env }),
    timeoutMs: init.timeoutMs ?? TIMEOUT.command,
  }
}

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
 * Runs `[bin, ...args]` and parses its one JSON line. Never throws. Stdout is
 * parsed whatever the exit code: every `ok: false` exits non-zero (1, or 2 for
 * `bad_args`) and is still `json`. Only no parseable v1 JSON is `failed`.
 */
export async function runJson<T>(
  $: ProcessHost,
  bin: string,
  args: readonly string[],
  init: RunInit = {},
): Promise<RunOutcome<T>> {
  let r: Awaited<ReturnType<ProcessHost['process']['run']>>
  try {
    r = await $.process.run([bin, ...args], runInitOf(init))
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

/**
 * Runs `[bin, ...args]` for a verb with no `--json` (`hatch`) and returns what
 * the child printed. Never throws.
 */
export async function runRaw(
  $: ProcessHost,
  bin: string,
  args: readonly string[],
  init: RunInit = {},
): Promise<{ exitCode: number; stdout: string; stderr: string } | { failed: string }> {
  try {
    const r = await $.process.run([bin, ...args], runInitOf(init))
    return { exitCode: r.exitCode, stdout: r.stdout, stderr: r.stderr }
  } catch (e) {
    return { failed: errText(e) }
  }
}

/** The `ok: true` value of an outcome, else null. */
export function okValue<T extends { ok: true }>(out: RunOutcome<T>): T | null {
  return out.kind === 'json' && !isError(out.value) && out.value.ok === true ? out.value : null
}

/** The `error` code of an `ok: false` outcome, else null. */
export function errorCode(out: RunOutcome<unknown>): string | null {
  return out.kind === 'json' && isError(out.value) ? out.value.error : null
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

const STATE_ERRORS: readonly string[] = ['no_kitten', 'state_unavailable', 'state_corrupt'] satisfies ErrorCode[]

/**
 * Capability handshake: does `frame --json` answer the v1 contract?
 * `no_kitten` counts as ready (fresh machine; the first feed hatches), as do
 * `state_unavailable` / `state_corrupt`. `bad_args` or no JSON is outdated.
 */
export async function handshake(
  $: ProcessHost,
  bin: string,
): Promise<{ status: BinaryStatus; frame: FrameJson | null; noKitten: boolean }> {
  const out = await runJson<FrameJson>($, bin, ['frame', '--json'], { timeoutMs: TIMEOUT.handshake })
  if (out.kind === 'failed' && out.reason === 'rejected') {
    return { status: { kind: 'missing', detail: out.detail }, frame: null, noKitten: false }
  }
  const frame = okValue(out)
  // Errors only a v1 binary can give about its own state are compatible too, with
  // no frame (not hatched yet, lock busy, or a corrupt state.json it reports).
  const code = errorCode(out)
  const compatible = frame !== null || (code !== null && STATE_ERRORS.includes(code))
  // Only `no_kitten` means "none yet"; a busy lock or corrupt state says nothing.
  const noKitten = code === 'no_kitten'
  return { status: { kind: compatible ? 'ready' : 'outdated', version: await readVersion($, bin) }, frame, noKitten }
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

/** The slice of `$` `spawnLines` needs. */
export type SpawnHost = {
  readonly process: Pick<EngineInterface['process'], 'spawn'>
  readonly ui: Pick<EngineInterface['ui'], 'log'>
}

/** A child streaming lines: `done` settles (never rejects) when it is over. */
export type SpawnedLines = { done: Promise<ProcessSpawnResult>; stop(): void }

/** No child: the spawn was refused, or its first pull rejected. */
const SPAWN_FAILED: ProcessSpawnResult = { code: null, signal: 'spawn-failed' }
/** The stream broke after it had started. */
const STREAM_ERROR: ProcessSpawnResult = { code: null, signal: 'stream-error' }
/** Ended by `stop()`. */
const STOPPED: ProcessSpawnResult = { code: null, signal: 'stopped' }

/**
 * `buffer` (the last partial line) plus `chunk`, as the complete lines and the
 * new partial line. Pure; empty lines are kept (the caller skips them).
 */
export function splitLines(buffer: string, chunk: string): { lines: string[]; rest: string } {
  const parts = (buffer + chunk).split('\n')
  const rest = parts.pop() ?? ''
  return { lines: parts, rest }
}

type SpawnControl = { stopped: boolean; stream: HookStream<ProcessSpawnChunk, ProcessSpawnResult> | null }

/** The `.catch` of a stopped stream's `return()` (a top-level function: the module scan needs one). */
function ignoreStopFailure(): void {}

/**
 * Spawns `[bin, ...args]` (stdin closed at once) and calls `onLine` with each
 * complete, non-empty stdout line, in order, one at a time. stderr is ignored
 * but for one debug line; a partial line at exit is dropped. `stop()` ends the
 * stream (the engine kills the child) and no line is delivered after it.
 * Never throws; `done` never rejects.
 */
export function spawnLines(
  $: SpawnHost,
  bin: string,
  args: readonly string[],
  onLine: (line: string) => unknown,
): SpawnedLines {
  const ctl: SpawnControl = { stopped: false, stream: null }
  const done = pumpLines($, [bin, ...args], onLine, ctl)
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

/** A debug-log line. Never throws. */
function spawnLog($: SpawnHost, text: string): void {
  try {
    $.ui.log(text, { to: 'debug' })
  } catch {
    // observer
  }
}

async function pumpLines(
  $: SpawnHost,
  argv: string[],
  onLine: (line: string) => unknown,
  ctl: SpawnControl,
): Promise<ProcessSpawnResult> {
  let stream: HookStream<ProcessSpawnChunk, ProcessSpawnResult>
  try {
    stream = $.process.spawn({ argv })
    ctl.stream = stream
  } catch (e) {
    spawnLog($, `spawn ${argv[1] ?? ''} refused: ${errText(e)}`)
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
        if (!stderrSeen) spawnLog($, `${argv[1] ?? ''} stderr: ${chunk.text.slice(0, 200)}`)
        stderrSeen = true
        continue
      }
      const split = splitLines(rest, chunk.text)
      rest = split.rest
      for (const line of split.lines) {
        if (ctl.stopped) break
        if (line.trim() === '') continue
        try {
          await onLine(line)
        } catch (e) {
          spawnLog($, `${argv[1] ?? ''} line failed: ${errText(e)}`)
        }
      }
    }
    if (ctl.stopped) return STOPPED
    return await stream.result
  } catch (e) {
    if (ctl.stopped) return STOPPED
    spawnLog($, `${argv[1] ?? ''} ${pulled ? 'stream broke' : 'did not start'}: ${errText(e)}`)
    return pulled ? STREAM_ERROR : SPAWN_FAILED
  }
}
