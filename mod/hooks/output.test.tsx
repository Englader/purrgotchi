import { expect, test } from 'claude-code/testing'

import { cmd, defaultRun, fx, ok, startReady } from './testkit'
import type { EngineOptions } from './testkit'

// The card and stats rows, driven through the engine: `/purr-card` stores the
// lines, then a CommandOutput mount draws them (or falls back).

const SURFACES = ['terminal', 'desktop'] as const
const CARD = [
  [{ text: '╭─ THALIA ─╮', fg: '#ea00d9' }],
  [],
  [{ text: 'lv 12', fg: '#0abdc6', bold: true }],
]
const SUMMARY = 'Thalia — lv 12 young Maine Coon, 340¢, bond 50, mood calm'

/** card / stats answer `lines`; frame and the handshake answer `frame`. */
const answering =
  (lines: string, frame = fx.frame()): NonNullable<EngineOptions['run']> =>
  argv =>
    argv.includes('--version')
      ? defaultRun(argv)
      : argv[1] === 'frame'
        ? ok(frame)
        : ok(lines)

let rows = 0

function mountRow(
  $: Parameters<Extract<Parameters<typeof test>[1], (...a: never[]) => unknown>>[0],
  surface: 'terminal' | 'desktop',
  command: string,
  text: string,
  columns = 120,
) {
  return $.ui.mount({
    plugin: 'purrgotchi',
    surface,
    component: 'CommandOutput',
    requestId: `r${++rows}`,
    props: { command, args: '', text, isErrored: false },
    viewport: { columns, rows: 40 },
  })
}

test('card stores lines and answers a summary', async ($, on) => {
  const spies = await startReady($, on, { run: answering(fx.lines(CARD)) })
  const r = await $.command.run(cmd('purr-card'))
  expect(r.text).toBe(SUMMARY)
  expect(r.exitCode).toBeUndefined()
  expect(spies.runs.filter(a => a[1] === 'card')).toEqual([['purrgotchi', 'card', '--json']])
})

test('stats answers a summary from stats --json', async ($, on) => {
  const spies = await startReady($, on, { run: answering(fx.lines(CARD)) })
  const r = await $.command.run(cmd('purr-stats'))
  expect(r.text).toBe(SUMMARY)
  expect(spies.runs.filter(a => a[1] === 'stats')).toEqual([['purrgotchi', 'stats', '--json']])
})

test('the summary falls back when frame is unavailable', async ($, on) => {
  await startReady($, on, {
    run: argv => (argv[1] === 'frame' && argv.length === 3 ? ok(fx.noKitten, 1) : answering(fx.lines(CARD))(argv)),
  })
  expect((await $.command.run(cmd('purr-card'))).text).toBe('Purrgotchi card')
})

for (const surface of SURFACES) {
  test(`renders spans on ${surface}`, async ($, on) => {
    await startReady($, on, { run: answering(fx.lines(CARD)) })
    const { text } = await $.command.run(cmd('purr-card'))
    const row = await mountRow($, surface, 'purr-card', text ?? '')
    expect(await row.find({ text: /THALIA/ })).toBeDefined()
    expect(await row.find({ text: /lv 12/ })).toBeDefined()
    expect(await row.find({ text: /Thalia — lv 12/ })).toBeDefined()
  })

  test(`unknown key falls back on ${surface}`, async ($, on) => {
    await startReady($, on, { run: answering(fx.lines(CARD)) })
    await $.command.run(cmd('purr-card'))
    const row = await mountRow($, surface, 'purr-card', 'something else')
    expect(await row.find({ text: /something else/ })).toBeDefined()
    expect(await row.find({ text: /THALIA/ })).toBeUndefined()
  })

  test(`older row keeps its own content on ${surface}`, async ($, on) => {
    const stats = (coins: number) => fx.lines([[{ text: `coins ${coins}` }]])
    let current = answering(stats(340), fx.frame({ coins: 340 }))
    await startReady($, on, { run: (argv, init) => current(argv, init) })
    const a = (await $.command.run(cmd('purr-stats'))).text ?? ''
    current = answering(stats(351), fx.frame({ coins: 351 }))
    const b = (await $.command.run(cmd('purr-stats'))).text ?? ''
    expect(a).not.toBe(b)
    const rowA = await mountRow($, surface, 'purr-stats', a)
    const rowB = await mountRow($, surface, 'purr-stats', b)
    expect(await rowA.find({ text: /coins 340/ })).toBeDefined()
    expect(await rowA.find({ text: /coins 351/ })).toBeUndefined()
    expect(await rowB.find({ text: /coins 351/ })).toBeDefined()
    expect(await rowB.find({ text: /coins 340/ })).toBeUndefined()
  })

  test(`the cap keeps the newest 20 on ${surface}`, async ($, on) => {
    let n = 0
    await startReady($, on, {
      run: argv =>
        argv.includes('--version')
          ? defaultRun(argv)
          : argv[1] === 'frame'
            ? ok(fx.frame({ coins: n }))
            : ok(fx.lines([[{ text: `row ${n}` }]])),
    })
    const texts: string[] = []
    for (n = 1; n <= 21; n++) texts.push((await $.command.run(cmd('purr-stats'))).text ?? '')
    const first = await mountRow($, surface, 'purr-stats', texts[0] ?? '')
    expect(await first.find({ text: /row 1$/ })).toBeUndefined()
    const last = await mountRow($, surface, 'purr-stats', texts[20] ?? '')
    expect(await last.find({ text: /row 21/ })).toBeDefined()
    const second = await mountRow($, surface, 'purr-stats', texts[1] ?? '')
    expect(await second.find({ text: /row 2$/ })).toBeDefined()
  })

  test(`a narrow viewport falls back on ${surface}`, async ($, on) => {
    const wide = fx.lines([[{ text: 'x'.repeat(46) }]])
    await startReady($, on, { run: answering(wide) })
    const { text } = await $.command.run(cmd('purr-card'))
    const row = await mountRow($, surface, 'purr-card', text ?? '', 30)
    expect(await row.find({ text: /xxxx/ })).toBeUndefined()
    expect(await row.find({ text: /Thalia — lv 12/ })).toBeDefined()
    const roomy = await mountRow($, surface, 'purr-card', text ?? '', 46)
    expect(await roomy.find({ text: /xxxx/ })).toBeDefined()
  })

  test(`an errored row falls back on ${surface}`, async ($, on) => {
    await startReady($, on, { run: answering(fx.lines(CARD)) })
    const { text } = await $.command.run(cmd('purr-card'))
    const row = await $.ui.mount({
      plugin: 'purrgotchi',
      surface,
      component: 'CommandOutput',
      requestId: 'errored',
      props: { command: 'purr-card', args: '', text: text ?? '', isErrored: true },
      viewport: { columns: 120, rows: 40 },
    })
    expect(await row.find({ text: /THALIA/ })).toBeUndefined()
  })
}

test('stats without a kitten says so, without exitCode', async ($, on) => {
  await startReady($, on, { run: argv => (argv[1] === 'stats' ? ok(fx.noKitten, 1) : defaultRun(argv)) })
  const r = await $.command.run(cmd('purr-stats'))
  expect(r.text).toContain('No kitten yet')
  expect(r.exitCode).toBeUndefined()
})

test('card with an unreachable binary is exitCode 1', async ($, on) => {
  await startReady($, on, { run: argv => (argv[1] === 'card' ? ok('not json') : defaultRun(argv)) })
  const r = await $.command.run(cmd('purr-card'))
  expect(r.text).toContain('unreachable')
  expect(r.exitCode).toBe(1)
})

test('card with a binary error shows its message', async ($, on) => {
  await startReady($, on, { run: argv => (argv[1] === 'card' ? ok(fx.error('state_corrupt', 'state.json is corrupt'), 1) : defaultRun(argv)) })
  const r = await $.command.run(cmd('purr-card'))
  expect(r).toMatchObject({ text: 'state.json is corrupt', exitCode: 1 })
})

test('hatch also stores the card under its row', { options: { devCommands: true } }, async ($, on) => {
  await startReady($, on, {
    run: argv =>
      argv[1] === 'hatch' ? ok('hatched\n') : argv[1] === 'card' ? ok(fx.lines(CARD)) : defaultRun(argv),
  })
  const { text } = await $.command.run(cmd('purr-hatch', ''))
  expect(text).toBe('Hatched Thalia — rare Maine Coon.')
  const row = await mountRow($, 'terminal', 'purr-hatch', text ?? '')
  expect(await row.find({ text: /THALIA/ })).toBeDefined()
})

test('an unrecognised surface is not drawn by us', async ($, on) => {
  await startReady($, on, { run: answering(fx.lines(CARD)) })
  const { text } = await $.command.run(cmd('purr-card'))
  const row = await mountRow($, 'mobile' as 'terminal', 'purr-card', text ?? '')
  expect(await row.find({ text: /Thalia — lv 12/ })).toBeDefined()
})
