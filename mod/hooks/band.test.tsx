import { expect, test } from 'claude-code/testing'
import type { Engine } from 'claude-code/testing'

import { pixelFixtures } from './fixtures/pixels'
import { defaultRun, fx, ok, paneProps, startReady } from './testkit'
import type { EngineOptions } from './testkit'
import type { HudJson } from '../types'

// The bottom band, driven through the engine: a session start (handshake and
// ticker), then an `AbovePrompt` mount the way a surface draws it.

const SURFACES = ['terminal', 'desktop'] as const
type Surface = (typeof SURFACES)[number]

const BOTTOM = { options: { layout: 'bottom' } } as const

function mountBand($: Engine, surface: Surface, maxRows: number, bodyColumns: number, hasSurvey = false) {
  return $.ui.mount({
    plugin: 'purrgotchi',
    surface,
    component: 'AbovePrompt',
    requestId: 'band',
    props: { hasSurvey, isWorking: false, maxRows, bodyColumns, scroll: { offset: 0, bodyRows: maxRows }, view: {} },
  })
}

const answering =
  (frame: string): NonNullable<EngineOptions['run']> =>
  argv =>
    argv[1] === 'frame' ? ok(frame) : defaultRun(argv)

async function measured($: Engine) {
  const resetsAt = new Date(Date.now() + 2 * 3_600_000 + 30_000).toISOString()
  await $.session.measure({
    context: { percent: 42, tokens: 87_000, window: 1_000_000 },
    rateLimits: [
      { kind: 'five_hour', percentUsed: 37, resetsAt },
      { kind: 'seven_day', percentUsed: 12 },
    ],
    cost: { usd: 0.12 },
    changed: ['context', 'rateLimits', 'cost'],
  })
}

const hud: HudJson = {
  total_tool_calls: 610,
  xp_pct: 20,
  last_fed_at: 1_792_000_000 - 12,
  stars: '★★★☆☆',
  name_fg: '#b45cff',
  peak: { label: 'OFF PEAK', range: '8PM-2PM', is_peak: false },
}

for (const surface of SURFACES) {
  test(`full band at 12 rows x 120 cols on ${surface}`, BOTTOM, async ($, on) => {
    const spies = await startReady($, on, { run: answering(fx.frame({ hud })) })
    await spies.clock.settle()
    await measured($)
    const band = await mountBand($, surface, 12, 120)
    expect(await band.find({ text: /THALIA/ })).toBeDefined()
    expect(await band.find({ text: /lv 12/ })).toBeDefined()
    expect(await band.find({ text: /xp {3}▰▰▱▱▱▱▱▱▱▱ 20%/ })).toBeDefined()
    expect(await band.find({ text: /340¢/ })).toBeDefined()
    expect(await band.find({ text: /mood ◆ calm/ })).toBeDefined()
    expect(await band.find({ text: /ctx {2}.* 42% · 87k/ })).toBeDefined()
    expect(await band.find({ text: /cost \$0\.12/ })).toBeDefined()
    expect(await band.find({ text: /5h .* 37% · resets 2h 00m/ })).toBeDefined()
    expect(await band.find({ text: /7d .* 12%/ })).toBeDefined()
    expect(await band.findAll({ text: /NaN|undefined/ })).toEqual([])
  })

  test(`drops limits under 100 cols and session under 80 on ${surface}`, BOTTOM, async ($, on) => {
    const spies = await startReady($, on)
    await spies.clock.settle()
    await measured($)
    const band = await mountBand($, surface, 12, 90)
    expect(await band.find({ text: /ctx/ })).toBeDefined()
    expect(await band.find({ text: /5h/ })).toBeUndefined()
    await band.redraw({ hasSurvey: false, isWorking: false, maxRows: 12, bodyColumns: 70, scroll: { offset: 0, bodyRows: 12 }, view: {} })
    expect(await band.find({ text: /ctx/ })).toBeUndefined()
    expect(await band.find({ text: /THALIA/ })).toBeDefined()
  })

  test(`compact under 7 rows on ${surface}`, BOTTOM, async ($, on) => {
    const spies = await startReady($, on)
    await spies.clock.settle()
    await measured($)
    const band = await mountBand($, surface, 3, 120)
    const row = await band.find({ text: /=\^\.\^= thalia lv12/ })
    expect(row).toBeDefined()
    expect(row?.text).toMatch(/ctx .* 42%/)
    expect(row?.text).toMatch(/5h .* 37%/)
    expect(row?.props.wrap).toBe('truncate')
    expect(await band.find({ text: /THALIA/ })).toBeUndefined()
  })

  test(`compact under 60 cols on ${surface}`, BOTTOM, async ($, on) => {
    const spies = await startReady($, on)
    await spies.clock.settle()
    const band = await mountBand($, surface, 12, 50)
    expect(await band.find({ text: /=\^\.\^= thalia lv12/ })).toBeDefined()
    expect(await band.find({ text: /THALIA/ })).toBeUndefined()
  })

  test(`placeholders on ${surface}`, BOTTOM, async ($, on) => {
    const none = await startReady($, on, { run: argv => (argv[1] === 'frame' ? ok(fx.noKitten) : defaultRun(argv)) })
    await none.clock.settle()
    expect(await (await mountBand($, surface, 12, 120)).find({ text: /No kitten yet/ })).toBeDefined()
  })

  test(`missing binary placeholder on ${surface}`, BOTTOM, async ($, on) => {
    await startReady($, on, { run: () => ({ deny: 'ENOENT: purrgotchi' }) })
    const band = await mountBand($, surface, 12, 120)
    expect(await band.find({ text: /Binary path/ })).toBeDefined()
    expect(await band.find({ text: /THALIA/ })).toBeUndefined()
  })
}

test('yields to a survey', BOTTOM, async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  const band = await mountBand($, 'terminal', 12, 120, true)
  expect(await band.find({ text: /THALIA|thalia/ })).toBeUndefined()
})

test('other surfaces get the engine rendering', BOTTOM, async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  const band = await $.ui.mount({
    plugin: 'purrgotchi',
    surface: 'mobile',
    component: 'AbovePrompt',
    requestId: 'band',
    props: { hasSurvey: false, isWorking: false, maxRows: 12, bodyColumns: 120, scroll: { offset: 0, bodyRows: 12 }, view: {} },
  })
  expect(await band.find({ text: /THALIA|thalia/ })).toBeUndefined()
})

test('dock layout registers no band hook', async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  const band = await mountBand($, 'terminal', 12, 120)
  expect(await band.find({ text: /THALIA|thalia/ })).toBeUndefined()
})

const lastTickWidth = (runs: string[][]) =>
  runs.filter(a => a[1] === 'frame' && a.includes('--width')).at(-1)?.slice(3, 5)

test('the band never asks for a scene strip (it draws none)', BOTTOM, async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  await mountBand($, 'terminal', 12, 120)
  await spies.clock.advance(1000)
  expect(lastTickWidth(spies.runs)).toEqual(['--width', '0'])
})

test('a pane opened under bottom still sizes the scene strip', BOTTOM, async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  await mountBand($, 'terminal', 12, 120)
  await $.ui.mount({ plugin: 'purrgotchi', surface: 'terminal', component: 'Pane', requestId: 'kitten', props: paneProps(58) })
  await spies.clock.advance(1000)
  expect(lastTickWidth(spies.runs)).toEqual(['--width', '54'])
})

for (const surface of SURFACES) {
  test(`portrait uses the cropped grid on ${surface}`, BOTTOM, async ($, on) => {
    const spies = await startReady($, on, { run: answering(fx.frame({ pixels: pixelFixtures[2]!.pixels })) })
    await spies.clock.settle()
    const band = await mountBand($, surface, 12, 120)
    expect(await band.find({ text: /\( o\.o \)/ })).toBeUndefined()
    if (surface === 'terminal') {
      expect((await band.find({ type: 'Raster' }))?.props).toMatchObject({ key: 'kitten-portrait', columns: 32, rows: 10 })
    } else {
      const svg = await band.find({ type: 'Svg' })
      expect(svg?.props.alt).toMatch(/Thalia the Maine Coon/)
      expect(svg?.props.source).toMatch(/viewBox="0 \d+ 32 20"/)
    }
    expect(await band.find({ text: /THALIA/ })).toBeDefined()
  })

  test(`portrait is shorter when the band has few rows on ${surface}`, BOTTOM, async ($, on) => {
    const spies = await startReady($, on, { run: answering(fx.frame({ pixels: pixelFixtures[2]!.pixels })) })
    await spies.clock.settle()
    const band = await mountBand($, surface, 8, 120)
    if (surface === 'terminal') expect((await band.find({ type: 'Raster' }))?.props.rows).toBe(8)
    else expect((await band.find({ type: 'Svg' }))?.props.source).toMatch(/viewBox="0 \d+ 32 16"/)
  })

  test(`ASCII sprite without pixels on ${surface}`, BOTTOM, async ($, on) => {
    const spies = await startReady($, on, { run: answering(fx.frame()) })
    await spies.clock.settle()
    const band = await mountBand($, surface, 12, 120)
    expect(await band.find({ text: /\( o\.o \)/ })).toBeDefined()
    expect(await band.find({ type: 'Raster' })).toBeUndefined()
    expect(await band.find({ type: 'Svg' })).toBeUndefined()
  })
}
