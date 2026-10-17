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
  /** The engine's own `session.end` step, each time it ran (its `reason`). */
  ends: string[]
  /** The command names `command.register` was asked for, in order. */
  registered: string[]
  /** `ui.log` texts. */
  logs: string[]
  /** The mocked `$.clock` beneath the plugin (`advance`, `settle`). */
  clock: MockClock
}

export type EngineOptions = {
  run?: (argv: string[], init?: ProcessRunInit) => OpAnswer
  /** What `$.settings.read` answers (default `{}`); `'deny'` makes it reject. */
  settings?: object | 'deny'
  /** What `ui.notify` answers (default: sent nowhere, `no-surface`); `'deny'` makes it reject. */
  notify?: 'deny'
  /** Command names `command.register` refuses (rejects). */
  refuse?: readonly string[]
}

/** User settings shapes for the migration scan. */
export const settingsFx = {
  /** `hooks.<event>` with one command hook, as install.sh writes it. */
  hook: (event: string, command: string): object => ({
    hooks: { [event]: [{ matcher: '*', hooks: [{ type: 'command', command }] }] },
  }),
}

/** `--version` answers `purrgotchi <version>`; every other run gets `answer`. */
export const withVersion =
  (answer: RunAnswer, version = '3.8.0') =>
  (argv: string[]): RunAnswer =>
    argv.includes('--version') ? ok(`purrgotchi ${version}\n`) : answer

/** Default binary: a ready handshake, then a plain feed for anything else. */
export function defaultRun(argv: string[]): RunAnswer {
  return withVersion(argv.includes('frame') ? ok(fx.frame()) : ok(fx.feed()))(argv)
}

/** Answers every op the mod uses and records what it saw. */
export function engine(on: On, o: EngineOptions = {}): Spies {
  const spies: Spies = { toasts: [], notifies: [], statuses: [], runs: [], ends: [], registered: [], logs: [], clock: mock.clock(on) }
  on('session.start', ($, e) => ({ cwd: e.cwd }))
  on('session.end', ($, e) => {
    spies.ends.push(e.reason)
    return { sessionId: e.sessionId }
  })
  on('command.register', ($, e) => {
    spies.registered.push(e.name)
    return o.refuse?.includes(e.name) === true
      ? { deny: `"/${e.name}" refused: it is a built-in` }
      : { value: { command: e.name } }
  })
  on('ui.toast', ($, e) => {
    spies.toasts.push(e.text)
    return { value: undefined }
  })
  on('ui.notify', ($, e) => {
    spies.notifies.push(e.text)
    return o.notify === 'deny' ? { deny: 'notify unavailable' } : { value: { isSent: false, reason: 'no-surface' } }
  })
  on('ui.status', ($, e) => {
    spies.statuses.push(e.text)
    return { value: undefined }
  })
  on('ui.log', ($, e) => {
    spies.logs.push(e.text)
    return { value: undefined }
  })
  // The engine's own row (what `next(e)` reaches): the command's plain text.
  on('ui.render', ($, e) => {
    const { Text } = $.ui.resolve(e)
    return Text({ children: [e.component === 'CommandOutput' ? e.props.text : ''] })
  })
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

/** The full `CommandRunInput` the engine would build for `/name args`. */
export const cmd = (command: string, args = '') =>
  ({ command, args, origin: { kind: 'composer' }, presentation: { isFullscreen: false, columns: 120 } }) as const

/** A promise the test opens with `release()`. */
export function gate(): { wait: Promise<void>; release: () => void } {
  let release = () => {}
  const wait = new Promise<void>(r => {
    release = r
  })
  return { wait, release }
}

/** A plain feed answer held until `g` is released. */
export const held =
  (g: { wait: Promise<void> }) =>
  async (): Promise<RunAnswer> => {
    await g.wait
    return ok(fx.feed())
  }

/** The `feed` runs among `runs`, in order. */
export const feeds = (runs: string[][]): string[][] => runs.filter(a => a[1] === 'feed')
