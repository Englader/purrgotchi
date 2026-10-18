import { expect, test } from 'claude-code/testing'
import type { Engine } from 'claude-code/testing'

import { ok, START, engine } from './testkit'
import type { EngineOptions } from './testkit'

const ID = 'custom:purrgotchi:neon-grid'
const VIOLET_ID = 'custom:purrgotchi:neon-grid-violet'
const PAINT = '\\033]11;#000000\\007'
const VIOLET_PAINT = '\\033]11;#1a1633\\007'
const RESET = '\\033]111\\007'
const SENT = ok('sent to /dev/pts/3\n')

const row = (value: string) => ({ key: 'theme', label: 'Theme', kind: 'choice', value, provider: { kind: 'engine' }, isLocked: false })
const set = ($: Engine, key: string, value: string | boolean) =>
  $.config.set({ key, value, previous: '', provider: { plugin: 'engine', tier: 'core' }, origin: { kind: 'composer' } } as never)
const end = ($: Engine, reason: string) => $.session.end({ reason, sessionId: 'sess-1', resume: { id: 'sess-1' } } as never)

/** The engine beneath: the theme row holds `theme`; a `config.set` is written (or denied). */
function world(on: Parameters<typeof engine>[0], theme: string, o: EngineOptions & { deny?: string; listThrows?: boolean } = {}) {
  const run = o.run ?? (() => SENT)
  const spies = engine(on, { ...o, run: (argv, init) => (isSend(argv) ? run(argv, init) : ok('', 1)) })
  on('config.list', () => {
    if (o.listThrows === true) throw new Error('boom')
    return { value: [row(theme)] } as never
  })
  on('session.end', () => ({ sessionId: 'sess-1' }))
  on('config.set', ($, e) => (o.deny === undefined ? { value: e.value } : { deny: o.deny }))
  return spies
}
/** The `sh -c` sends; the band's own probes (`purrgotchi --version`) are not ours. */
const isSend = (argv: string[]) => argv[0] === 'sh' || argv[1] === 'term-write'
const sent = (spies: { runs: string[][] }) => spies.runs.filter(isSend).length

async function envs(on: Parameters<typeof engine>[0], theme: string, o: EngineOptions = {}) {
  const seen: (string | undefined)[] = []
  const spies = world(on, theme, {
    ...o,
    run: (argv, init) => {
      if (isSend(argv)) seen.push(init?.env?.SEQ)
      return (o.run ?? (() => SENT))(argv, init)
    },
  })
  return { spies, seen }
}

test('session.start on Neon Grid paints the terminal background', async ($, on) => {
  const { spies, seen } = await envs(on, ID)
  await $.session.start(START)
  expect(sent(spies)).toBe(1)
  expect(spies.runs.find(isSend)?.slice(0, 2)).toEqual(['sh', '-c'])
  expect(seen).toEqual([PAINT])
})

test('session.start on another theme does nothing', async ($, on) => {
  const { spies } = await envs(on, 'dark')
  await $.session.start(START)
  expect(sent(spies)).toBe(0)
})

test('switching to Neon Grid paints', async ($, on) => {
  const { seen } = await envs(on, 'dark')
  await $.session.start(START)
  expect(await set($, 'theme', ID)).toEqual({ value: ID })
  expect(seen).toEqual([PAINT])
})

test('switching away after a paint resets', async ($, on) => {
  const { seen } = await envs(on, ID)
  await $.session.start(START)
  expect(await set($, 'theme', 'dark')).toEqual({ value: 'dark' })
  expect(seen).toEqual([PAINT, RESET])
})

test('switching away without a paint does nothing', async ($, on) => {
  const { spies } = await envs(on, 'dark')
  await $.session.start(START)
  expect(await set($, 'theme', 'light')).toEqual({ value: 'light' })
  expect(sent(spies)).toBe(0)
})

test('another row is not the theme', async ($, on) => {
  const { spies } = await envs(on, 'dark')
  await set($, 'verbose', true)
  expect(sent(spies)).toBe(0)
})

test('a denied theme change paints nothing and is passed through', async ($, on) => {
  const spies = world(on, 'dark', { deny: 'locked' })
  expect(await set($, 'theme', ID)).toEqual({ deny: 'locked' })
  expect(sent(spies)).toBe(0)
})

test('a denied switch away does not reset', async ($, on) => {
  const spies = world(on, ID, { deny: 'locked' })
  await $.session.start(START)
  expect(await set($, 'theme', 'dark')).toEqual({ deny: 'locked' })
  expect(sent(spies)).toBe(1)
})

test('no terminal: the change resolves and nothing counts as painted', async ($, on) => {
  const { spies } = await envs(on, 'dark', { run: () => ok('no tty\n', 1) })
  await $.session.start(START)
  expect(await set($, 'theme', ID)).toEqual({ value: ID })
  expect(await set($, 'theme', 'dark')).toEqual({ value: 'dark' })
  expect(sent(spies)).toBe(1) // only the failed paint: no reset
})

test('a process that throws never blocks the change', async ($, on) => {
  const { spies } = await envs(on, 'dark', {
    run: () => {
      throw new Error('boom')
    },
  })
  await $.session.start(START)
  expect(await set($, 'theme', ID)).toEqual({ value: ID })
  expect(await set($, 'theme', 'dark')).toEqual({ value: 'dark' })
  expect(sent(spies)).toBe(1)
})

test('a theme list that throws leaves start alone', async ($, on) => {
  const spies = world(on, 'dark', { listThrows: true })
  await $.session.start(START)
  expect(sent(spies)).toBe(0)
})

test('session.end after a paint resets, within the exit budget', async ($, on) => {
  const inits: (number | undefined)[] = []
  const { seen } = await envs(on, ID, {
    run: (argv, init) => {
      if (isSend(argv)) inits.push(init?.timeoutMs)
      return SENT
    },
  })
  await $.session.start(START)
  await end($, 'prompt_input_exit')
  expect(seen).toEqual([PAINT, RESET])
  expect(inits[1]).toBeLessThanOrEqual(1000)
})

test('/clear keeps the backdrop', async ($, on) => {
  const { seen } = await envs(on, ID)
  await $.session.start(START)
  await end($, 'clear')
  expect(seen).toEqual([PAINT])
})

test('session.end without a paint does nothing', async ($, on) => {
  const { spies } = await envs(on, 'dark')
  await $.session.start(START)
  await end($, 'prompt_input_exit')
  expect(sent(spies)).toBe(0)
})

test('the reset is sent once', async ($, on) => {
  const { seen } = await envs(on, ID)
  await $.session.start(START)
  await set($, 'theme', 'dark')
  await end($, 'prompt_input_exit')
  expect(seen).toEqual([PAINT, RESET])
})

// Windows: no ptys; the binary's `term-write` takes the raw bytes in PURR_SEQ.
const BIN = 'C:/purr/purrgotchi.exe'
const WIN_PAINT = ']11;#000000'
const WIN_VIOLET_PAINT = ']11;#1a1633'
const WIN_RESET = '\x1b]111\x07'
const isWrite = (argv: string[]) => argv[1] === 'term-write'
const WIN_ENV = { OS: 'Windows_NT', PURR_BIN: BIN }

function winWorld(on: Parameters<typeof engine>[0], theme: string, o: { env?: Record<string, string>; exists?: string[]; exit?: () => number } = {}) {
  const seen: { argv: string[]; seq: string | undefined; ms: number | undefined }[] = []
  const spies = world(on, theme, {
    env: o.env ?? WIN_ENV,
    exists: o.exists ?? [BIN],
    run: (argv, init) => {
      if (isWrite(argv)) seen.push({ argv, seq: init?.env?.PURR_SEQ, ms: init?.timeoutMs })
      return ok('', isWrite(argv) ? (o.exit?.() ?? 0) : 1)
    },
  })
  return { spies, seen }
}

test('windows: start on Neon Grid runs term-write with real ESC/BEL bytes', async ($, on) => {
  const { spies, seen } = winWorld(on, ID)
  await $.session.start(START)
  expect(seen).toEqual([{ argv: [BIN, 'term-write'], seq: WIN_PAINT, ms: 1000 }])
  expect(spies.runs.filter(a => a[0] === 'sh')).toEqual([]) // never the sh path
})

test('windows: switching away after a paint resets with OSC 111', async ($, on) => {
  const { seen } = winWorld(on, ID)
  await $.session.start(START)
  expect(await set($, 'theme', 'dark')).toEqual({ value: 'dark' })
  expect(seen.map(s => s.seq)).toEqual([WIN_PAINT, WIN_RESET])
})

test('windows: no binary found skips silently', async ($, on) => {
  const { seen } = winWorld(on, 'dark', { env: { OS: 'Windows_NT' }, exists: [] })
  await $.session.start(START)
  expect(await set($, 'theme', ID)).toEqual({ value: ID })
  expect(await set($, 'theme', 'dark')).toEqual({ value: 'dark' })
  expect(seen).toEqual([])
})

test('windows: a failed paint (no console) is never reset', async ($, on) => {
  const { seen } = winWorld(on, 'dark', { exit: () => 1 })
  await $.session.start(START)
  await set($, 'theme', ID)
  await set($, 'theme', 'dark')
  await end($, 'prompt_input_exit')
  expect(seen.map(s => s.seq)).toEqual([WIN_PAINT])
})

test('windows: session.end resets once; /clear keeps the backdrop', async ($, on) => {
  const { seen } = winWorld(on, ID)
  await $.session.start(START)
  await end($, 'clear')
  expect(seen.map(s => s.seq)).toEqual([WIN_PAINT])
  await end($, 'prompt_input_exit')
  expect(seen.map(s => s.seq)).toEqual([WIN_PAINT, WIN_RESET])
})

test('session.start on Neon Grid Violet paints the violet ground', async ($, on) => {
  const { seen } = await envs(on, VIOLET_ID)
  await $.session.start(START)
  expect(seen).toEqual([VIOLET_PAINT])
})

test('switching to Neon Grid Violet paints violet', async ($, on) => {
  const { seen } = await envs(on, 'dark')
  await $.session.start(START)
  await set($, 'theme', VIOLET_ID)
  expect(seen).toEqual([VIOLET_PAINT])
})

test('switching between the Neon themes repaints, without a reset', async ($, on) => {
  const { seen } = await envs(on, ID)
  await $.session.start(START)
  await set($, 'theme', VIOLET_ID)
  await set($, 'theme', ID)
  expect(seen).toEqual([PAINT, VIOLET_PAINT, PAINT])
})

test('switching away from Neon Grid Violet resets once', async ($, on) => {
  const { seen } = await envs(on, VIOLET_ID)
  await $.session.start(START)
  await set($, 'theme', 'dark')
  await end($, 'prompt_input_exit')
  expect(seen).toEqual([VIOLET_PAINT, RESET])
})

test('a lookalike theme id is not a Neon theme', async ($, on) => {
  const { spies } = await envs(on, 'dark')
  await set($, 'theme', 'custom:purrgotchi:neon-grid-x')
  await set($, 'theme', 'toString')
  expect(sent(spies)).toBe(0)
})

test('windows: black and violet paint with their own bytes; switching repaints, then resets', async ($, on) => {
  const { seen } = winWorld(on, ID)
  await $.session.start(START)
  await set($, 'theme', VIOLET_ID)
  await set($, 'theme', ID)
  await set($, 'theme', 'dark')
  expect(seen.map(s => s.seq)).toEqual([WIN_PAINT, WIN_VIOLET_PAINT, WIN_PAINT, WIN_RESET])
})

test('windows: start on Neon Grid Violet paints violet', async ($, on) => {
  const { seen } = winWorld(on, VIOLET_ID)
  await $.session.start(START)
  expect(seen.map(s => s.seq)).toEqual([WIN_VIOLET_PAINT])
})
