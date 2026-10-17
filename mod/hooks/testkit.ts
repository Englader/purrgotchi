// Test-only helpers (imported only by *.test.ts). Nothing sits beneath the
// plugin in `claude plugin test`, so `engine(on)` answers every op the mod
// uses, and `fx` holds Phase 0-shaped binary output. Tests never run the
// real binary.
import { mock } from 'claude-code/testing'
import type { MockClock } from 'claude-code/testing'
import type { ConfigValue, ModelCompleteInput, On, PaneOpenArgs, ProcessRunInit, ProcessSpawnResult, ResultOf } from 'claude-code'

import type { ProcessHost } from './binary'
import type { ActionJson, CostJson, FeedJson, FrameJson, QuipPromptJson, QuipStoreJson, Span, TurnJson } from '../types'

export type RunAnswer = ResultOf['process.run']
export type OpAnswer = RunAnswer | Promise<RunAnswer>
export type CompleteAnswer = ResultOf['model.complete']

const USAGE = { input_tokens: 1, output_tokens: 1, cache_read_input_tokens: 0, cache_creation_input_tokens: 0 }

/** A `model.complete` op answer: the model replied `text`. */
export const modelText = (text: string): CompleteAnswer => ({ value: { isAnswered: true, text, usage: USAGE } })

/** A `model.complete` op answer: the call ended with no reply. */
export const modelUnanswered: CompleteAnswer = { value: { isAnswered: false, reason: 'empty-reply', usage: USAGE } }

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
  cost: (patch: Partial<CostJson> = {}): string =>
    line({ v: 1, ok: true, credit: 0.12, lifetime_cost_usd: 3.42, ...patch } satisfies CostJson),
  quipPrompt: (patch: Partial<QuipPromptJson> = {}): string =>
    line({ v: 1, ok: true, mood: 'calm', system: 'You are a kitten.', user: 'Say a quip.', max_tokens: 60, ...patch } satisfies QuipPromptJson),
  quipStore: (stored = 'mrrp! nice fix'): string => line({ v: 1, ok: true, stored } satisfies QuipStoreJson),
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
  toasts: string[]
  notifies: string[]
  statuses: (string | undefined)[]
  runs: string[][]
  /** `process.spawn` argv, in order. */
  spawns: string[][]
  /** Spawned children the plugin stopped (their stream's abort). */
  stops: number
  /** The engine's own `session.end` step, each time it ran (its `reason`). */
  ends: string[]
  /** The command names `command.register` was asked for, in order. */
  registered: string[]
  /** `ui.log` texts. */
  logs: string[]
  /** `ui.open` arguments, in order. */
  opens: PaneOpenArgs[]
  /** `config.set` rows a plugin asked for, in order. */
  configs: { key: string; value: ConfigValue }[]
  /** `model.complete` requests, in order. */
  completes: ModelCompleteInput[]
  /** The mocked `$.clock` beneath the plugin (`advance`, `settle`). */
  clock: MockClock
}

export type EngineOptions = {
  run?: (argv: string[], init?: ProcessRunInit) => OpAnswer
  /**
   * The `n`th (1-based) `process.spawn`'s child. Default: no child can start
   * (the first pull rejects), as with a binary that has no `serve`.
   */
  spawn?: (argv: string[], n: number) => SpawnScript
  /** What `$.settings.read` answers (default `{}`); `'deny'` makes it reject. */
  settings?: object | 'deny'
  /** What `ui.notify` answers (default: sent nowhere, `no-surface`); `'deny'` makes it reject. */
  notify?: 'deny'
  /** Command names `command.register` refuses (rejects). */
  refuse?: readonly string[]
  /** What `$.env.get` answers (default: every variable unset). */
  env?: Readonly<Record<string, string>>
  /** What `ui.open` answers (default: placed); `notPlaced` answers not placed, with that reason. */
  open?: { notPlaced: string }
  /** What `$.config.set` answers (default: written); `deny` refuses with that reason. */
  config?: { deny: string }
  /** What `$.model.complete` answers (default: no reply). */
  complete?: (req: ModelCompleteInput) => CompleteAnswer | Promise<CompleteAnswer>
  /** What `$.session.id` answers (default `s-1`); `'deny'` makes it reject. */
  sessionId?: string
}

/** User settings shapes for the migration scan. */
export const settingsFx = {
  /** `hooks.<event>` with one command hook, as install.sh writes it. */
  hook: (event: string, command: string, matcher = '*'): object => ({
    hooks: { [event]: [{ matcher, hooks: [{ type: 'command', command }] }] },
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
  const spies: Spies = { toasts: [], notifies: [], statuses: [], runs: [], spawns: [], stops: 0, ends: [], registered: [], logs: [], completes: [], opens: [], configs: [], clock: mock.clock(on) }
  mock.env(on, o.env ?? {})
  on('session.start', ($, e) => ({ cwd: e.cwd }))
  // The engine echoes a measurement's units; the session figures it answers.
  on('session.measure', ($, e) => ({ changed: e.changed }))
  on('session.usage', () => ({ value: { startedAt: 0, context: { window: 1_000_000 }, rateLimits: [] } }))
  on('session.model', () => ({ value: 'claude-opus-5-5' }))
  on('session.cwd', () => ({ value: '/tmp' }))
  on('session.id', () => (o.sessionId === 'deny' ? { deny: 'session id unavailable' } : { value: o.sessionId ?? 's-1' }))
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
  on('ui.open', ($, e) => {
    spies.opens.push({ ...e })
    return o.open === undefined ? { value: { isPlaced: true } } : { value: { isPlaced: false, reason: o.open.notPlaced } }
  })
  on('config.set', ($, e) => {
    spies.configs.push({ key: e.key, value: e.value })
    return o.config === undefined ? { value: e.value } : { deny: o.config.deny }
  })
  on('ui.log', ($, e) => {
    spies.logs.push(e.text)
    return { value: undefined }
  })
  // The engine's own row (what `next(e)` reaches): the props it would draw, as plain text.
  on('ui.render', ($, e) => {
    const { Text } = $.ui.resolve(e)
    const text =
      e.component === 'CommandOutput' || e.component === 'UserMessage'
        ? e.props.text
        : e.component === 'Spinner'
          ? `${e.props.message ?? e.props.word}${e.props.suffix}`
          : e.component === 'ToolUse'
            ? `${e.props.tool} ${typeof e.props.output === 'string' ? e.props.output : ''}`
            : ''
    return Text({ children: [text] })
  })
  on('settings.read', () =>
    o.settings === 'deny' ? { deny: 'settings unavailable' } : { value: { ...(o.settings ?? {}) } },
  )
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
  on('model.complete', ($, e) => {
    spies.completes.push({ ...e })
    return (o.complete ?? (() => modelUnanswered))(e)
  })
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

/** The `Pane` props a surface hands the kitten pane's render hook at `cols` body columns. */
export const paneProps = (cols: number) =>
  ({
    title: 'Purrgotchi',
    isFocused: true,
    bodyColumns: cols,
    placement: 'dock',
    scroll: { offset: 0, bodyRows: 30 },
    view: {},
  }) as const

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
