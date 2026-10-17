// Test-only helpers (imported only by *.test.ts). Nothing sits beneath the
// plugin in `claude plugin test`, so `engine(on)` answers every op the mod
// uses, and `fx` holds Phase 0-shaped binary output. Tests never run the
// real binary.
import { mock } from 'claude-code/testing'
import type { MockClock } from 'claude-code/testing'
import type { On, ProcessRunInit, ResultOf } from 'claude-code'

import type { ProcessHost } from './binary'
import type { ActionJson, FeedJson, FrameJson, Span, TurnJson } from '../types'

export type RunAnswer = ResultOf['process.run']
export type OpAnswer = RunAnswer | Promise<RunAnswer>

/** A `process.run` op answer: the child exited with `exitCode`. */
export function ok(stdout: string, exitCode = 0, stderr = ''): RunAnswer {
  return { value: { exitCode, stdout, stderr, isStdoutTruncated: false, isStderrTruncated: false } }
}

const line = (v: object): string => `${JSON.stringify(v)}\n`

const sprite = (): Span[][] =>
  ['  /\\_/\\', ' ( o.o )', '  > ^ <', ' /     \\', '(  u u  )', ''].map(text => [{ text, fg: '#0abdc6' }])

export const fx = {
  feed: (patch: Partial<FeedJson> = {}): string =>
    line({
      v: 1,
      ok: true,
      celebration: null,
      level: 12,
      stage: 'young',
      coins: 340,
      total_tool_calls: 1234,
      drop: false,
      count: 1,
      ...patch,
    } satisfies FeedJson),
  turn: (patch: Partial<TurnJson> = {}): string =>
    line({ v: 1, ok: true, celebration: null, react_spawned: false, ...patch } satisfies TurnJson),
  action: (patch: Partial<ActionJson> = {}): string =>
    line({
      v: 1,
      ok: true,
      result: 'applied',
      message: '♥ Thalia purrs contentedly. Bond: 50 → 65 (+15)',
      bond_old: 50,
      bond_new: 65,
      capped: false,
      ...patch,
    } satisfies ActionJson),
  lines: (lines: Span[][]): string => line({ v: 1, ok: true, lines }),
  frame: (patch: Partial<FrameJson> = {}): string =>
    line({
      v: 1,
      ok: true,
      now: 1_792_000_000,
      tick: 1_792_000_000,
      name: 'Thalia',
      breed: 'Maine Coon',
      rarity: 'rare',
      shiny: false,
      level: 12,
      stage: 'young',
      coins: 340,
      bond: 50,
      mood: 'calm',
      sprite: sprite(),
      speech: [{ text: 'mrow. all calm', fg: '#57eca6' }],
      speech_source: 'mood',
      scene: null,
      celebration: null,
      ...patch,
    } satisfies FrameJson),
  error: (code: string, message?: string): string => line({ v: 1, ok: false, error: code, message: message ?? code }),
  badArgs: (detail: string): string => line({ v: 1, ok: false, error: 'bad_args', detail }),
  noKitten: line({ v: 1, ok: false, error: 'no_kitten', message: 'No kitten hatched yet. Run: purrgotchi hatch' }),
}

/**
 * A plugin-side `$` slice for unit-testing `binary.ts` directly. A test's own
 * `$` has no `process` noun (ops are answered with `on`, not raised), so this
 * answers `$.process.run(argv, init)` with the same `{ value } | { deny }`
 * answers `engine(on, { run })` takes, a `deny` rejecting as the engine's call
 * does. Hooks under test reach `process.run` through `engine(on)` instead.
 */
export function host(run: (argv: string[], init?: ProcessRunInit) => OpAnswer): ProcessHost {
  return {
    process: {
      run: async (argv, init) => {
        const r = await run([...argv], init)
        if ('deny' in r) throw new Error(r.deny)
        return r.value
      },
    },
  }
}

export type Spies = {
  toasts: string[]
  notifies: string[]
  statuses: (string | undefined)[]
  runs: string[][]
  /** The mocked `$.clock` beneath the plugin (`advance`, `settle`). */
  clock: MockClock
}

export type EngineOptions = {
  run?: (argv: string[], init?: ProcessRunInit) => OpAnswer
  /** What `$.settings.read` answers (default `{}`); `'deny'` makes it reject. */
  settings?: object | 'deny'
}

/** User settings shapes for the migration scan. */
export const settingsFx = {
  /** `hooks.<event>` with one command hook, as install.sh writes it. */
  hook: (event: string, command: string): object => ({
    hooks: { [event]: [{ matcher: '', hooks: [{ type: 'command', command }] }] },
  }),
}

/** Default binary: a ready handshake, then a plain feed for anything else. */
export function defaultRun(argv: string[]): RunAnswer {
  if (argv.includes('--version')) return ok('purrgotchi 3.8.0\n')
  if (argv.includes('frame')) return ok(fx.frame())
  return ok(fx.feed())
}

/** Answers every op the mod uses and records what it saw. */
export function engine(on: On, o: EngineOptions = {}): Spies {
  const spies: Spies = { toasts: [], notifies: [], statuses: [], runs: [], clock: mock.clock(on) }
  on('session.start', ($, e) => ({ cwd: e.cwd }))
  on('session.end', ($, e) => ({ sessionId: e.sessionId }))
  on('command.register', ($, e) => ({ value: { command: e.name } }))
  on('ui.toast', ($, e) => {
    spies.toasts.push(e.text)
    return { value: undefined }
  })
  on('ui.notify', ($, e) => {
    spies.notifies.push(e.text)
    return { value: { isSent: false, reason: 'no-surface' } }
  })
  on('ui.status', ($, e) => {
    spies.statuses.push(e.text)
    return { value: undefined }
  })
  on('ui.log', () => ({ value: undefined }))
  on('settings.read', () =>
    o.settings === 'deny' ? { deny: 'settings unavailable' } : { value: { ...(o.settings ?? {}) } },
  )
  on('process.run', ($, e) => {
    const argv = [...e.argv]
    spies.runs.push(argv)
    return (o.run ?? defaultRun)(argv, e.init)
  })
  return spies
}

export const START = { cwd: '/tmp', surface: 'terminal', isInteractive: true } as const

/** `engine(on, o)` plus a session start, so the handshake has run. */
export async function startReady(
  $: { session: { start: (e: typeof START) => Promise<unknown> } },
  on: On,
  o: EngineOptions = {},
): Promise<Spies> {
  const spies = engine(on, o)
  await $.session.start(START)
  return spies
}

/** A promise the test opens with `release()`. */
export function gate(): { wait: Promise<void>; release: () => void } {
  let release = () => {}
  const wait = new Promise<void>(r => {
    release = r
  })
  return { wait, release }
}

/** The `feed` runs among `runs`, in order. */
export const feeds = (runs: string[][]): string[][] => runs.filter(a => a[1] === 'feed')
