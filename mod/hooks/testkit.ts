// Test-only helpers (imported only by *.test.ts). Nothing sits beneath the
// plugin in `claude plugin test`, so `engine(on, o)` answers every op the mod
// uses. Tests never run the real binary.
import { mock } from 'claude-code/testing'
import type { MockClock } from 'claude-code/testing'
import type { EngineInterface, On, ProcessRunInit, ProcessSpawnResult, ResultOf } from 'claude-code'

export type RunAnswer = ResultOf['process.run']
export type OpAnswer = RunAnswer | Promise<RunAnswer>

/** A `process.run` op answer: the child exited with `exitCode`. */
export function ok(stdout: string, exitCode = 0, stderr = ''): RunAnswer {
  return { value: { exitCode, stdout, stderr, isStdoutTruncated: false, isStderrTruncated: false } }
}

/** One NDJSON line. */
export const line = (v: object): string => `${JSON.stringify(v)}\n`

/** A `band` frame line carrying `c`. */
export const bandFrame = (f: number, c = 'AAAA'): string => line({ v: 1, f, t: 1_800_000_000_000 + f, c })

/** The hello line a `band` stream opens with. */
export const bandHello = (cols: number, rows: number, stateDir = '/tmp/state'): string =>
  line({ v: 1, hello: true, cols, rows, fps: 30, state_dir: stateDir })

/**
 * A scripted `process.spawn` child: each step is a stdout chunk, a stderr
 * chunk, or a wait on a promise the test releases (`$.clock.sleep` is refused
 * test-side, so timing is gates only). Then it exits with `end`, or, without
 * one, stays open and silent until the plugin stops it. Every wait gives way
 * to the engine's abort (`return()` on the stream), ending as `SIGTERM`.
 */
export type SpawnScript = {
  steps?: readonly (string | { stderr: string } | { wait: Promise<void> })[]
  end?: ProcessSpawnResult
}

/** A child killed from outside, as the engine reports it. */
const KILLED: ProcessSpawnResult = { code: null, signal: 'SIGTERM' }

export type Spies = {
  /** `process.run` argv, in order. */
  runs: string[][]
  /** `process.spawn` argv, in order. */
  spawns: string[][]
  /** Spawned children the plugin stopped (their stream's abort). */
  stops: number
  /** `ui.blit` calls, in order. */
  blits: { requestId: string; key?: string; cells?: string }[]
  /** `fs.write` calls, in order. */
  writes: { path: string; text: string }[]
  /** `ui.log` texts. */
  logs: string[]
  /** The mocked `$.clock` beneath the plugin (`advance`, `settle`). */
  clock: MockClock
}

export type EngineOptions = {
  run?: (argv: string[], init?: ProcessRunInit) => OpAnswer
  /**
   * The `n`th (1-based) `process.spawn`'s child. Default: no child can start
   * (the spawn is refused), as with a binary that has no `band`.
   */
  spawn?: (argv: string[], n: number) => SpawnScript
  /** What `$.env.get` answers (default: every variable unset). */
  env?: Readonly<Record<string, string>>
  /** Paths `$.fs.exists` answers true for (default: none). */
  exists?: readonly string[]
  /** What `ui.blit` answers (default: taken). */
  blit?: (n: number) => { deny?: string }
}

/** Answers every op the mod uses and records what it saw. */
export function engine(on: On, o: EngineOptions = {}): Spies {
  const spies: Spies = { runs: [], spawns: [], stops: 0, blits: [], writes: [], logs: [], clock: mock.clock(on) }
  mock.env(on, o.env ?? {})
  on('session.start', ($, e) => ({ cwd: e.cwd }))
  on('fs.exists', ($, e) => ({ value: (o.exists ?? []).includes(e.path) }))
  on('fs.write', ($, e) => {
    spies.writes.push({ path: e.path, text: e.text })
    return { value: undefined }
  })
  on('ui.log', ($, e) => {
    spies.logs.push(e.text)
    return { value: undefined }
  })
  on('ui.blit', ($, e) => {
    const b = e as { requestId: string; key?: string; cells?: string }
    spies.blits.push({ requestId: b.requestId, ...(b.key !== undefined && { key: b.key }), ...(b.cells !== undefined && { cells: b.cells }) })
    return { value: o.blit?.(spies.blits.length) ?? {} }
  })
  on('process.run', ($, e) => {
    const argv = [...e.argv]
    spies.runs.push(argv)
    return (o.run ?? (() => ok('', 1)))(argv, e.init)
  })
  on('process.spawn', async function* ($, e, next) {
    const argv = [...e.argv]
    spies.spawns.push(argv)
    const script = o.spawn?.(argv, spies.spawns.length)
    if (script === undefined) return { deny: 'spawn refused: no child in this test' }
    const aborted = new Promise<'abort'>(resolve => {
      const stop = () => {
        spies.stops++
        resolve('abort')
      }
      if (next.signal.aborted) stop()
      else next.signal.addEventListener('abort', stop, { once: true })
    })
    for (const step of script.steps ?? []) {
      if (typeof step === 'string') yield { stream: 'stdout' as const, text: step }
      else if ('stderr' in step) yield { stream: 'stderr' as const, text: step.stderr }
      else if ((await Promise.race([step.wait.then(() => 'go' as const), aborted])) === 'abort') return { value: KILLED }
    }
    if (script.end !== undefined) return { value: script.end }
    await aborted
    return { value: KILLED }
  })
  return spies
}

export const START = { cwd: '/tmp', surface: 'terminal', isInteractive: true } as const

/** A promise the test opens with `release()`. */
export function gate(): { wait: Promise<void>; release: () => void } {
  let release = () => {}
  const wait = new Promise<void>(r => {
    release = r
  })
  return { wait, release }
}

/** The slice of `$` that `locateBinary` reads, backed by plain data (no engine). */
export type FakeHost = Pick<EngineInterface, 'env' | 'fs' | 'process'>

export function fakeHost(o: {
  env?: Readonly<Record<string, string>>
  exists?: readonly string[]
  run?: (argv: string[], init?: ProcessRunInit) => { exitCode: number; stdout: string } | 'throw'
}): FakeHost {
  return {
    env: { get: async (name: string) => o.env?.[name] },
    fs: { exists: async (path: string) => (o.exists ?? []).includes(path) },
    process: {
      run: async (argv: readonly string[], init?: ProcessRunInit) => {
        const r = o.run?.([...argv], init) ?? { exitCode: 1, stdout: '' }
        if (r === 'throw') throw new Error('ENOENT')
        return { exitCode: r.exitCode, stdout: r.stdout, stderr: '', isStdoutTruncated: false, isStderrTruncated: false }
      },
    },
  } as unknown as FakeHost
}
