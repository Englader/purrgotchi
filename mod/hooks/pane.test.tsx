import { expect, test } from 'claude-code/testing'
import type { Engine } from 'claude-code/testing'

import { pixelFixtures } from './fixtures/pixels'
import { cmd, defaultRun, engine, fx, ok, paneProps, START, startReady } from './testkit'
import type { EngineOptions } from './testkit'
import type { HudJson, SceneJson } from '../types'

// The kitten pane, driven through the engine: a session start (handshake and
// ticker), then a `Pane` mount the way a surface draws it. Everything is
// asserted through the drawn tree and what the engine saw.

const SURFACES = ['terminal', 'desktop', 'mobile'] as const
type Surface = (typeof SURFACES)[number]

const ticks = (runs: string[][]) => runs.filter(a => a[1] === 'frame' && a.includes('--width'))

function mountPane($: Engine, surface: Surface, cols = 58) {
  return $.ui.mount({ plugin: 'purrgotchi', surface, component: 'Pane', requestId: 'kitten', props: paneProps(cols) })
}

/** The handshake and every frame answer `frame`; other verbs answer `action`. */
const answering =
  (frame: string, action = fx.action()): NonNullable<EngineOptions['run']> =>
  argv =>
    argv.includes('--version') || argv[1] === 'frame' ? (argv[1] === 'frame' ? ok(frame) : defaultRun(argv)) : ok(action)

/** The fixture Maine Coon (adult, alert), the breed `fx.frame()` names. */
const MAINE_PIXELS = pixelFixtures[2]!.pixels
/** A row of the default ASCII sprite. */
const ASCII_SPRITE = /\( o\.o \)/

for (const surface of SURFACES) {
  test(`draws name, stats and buttons at 58 cols on ${surface}`, async ($, on) => {
    const spies = await startReady($, on)
    await spies.clock.settle()
    const pane = await mountPane($, surface)
    expect(await pane.find({ text: /THALIA/ })).toBeDefined()
    expect(await pane.find({ text: /lv 12/ })).toBeDefined()
    expect(await pane.find({ text: /340¢/ })).toBeDefined()
    expect(await pane.find({ text: /mood ◆ calm/ })).toBeDefined()
    expect(await pane.find({ type: 'Button', key: 'pet' })).toBeDefined()
  })

  test(`missing binary placeholder on ${surface}`, async ($, on) => {
    engine(on, { run: () => ({ deny: 'ENOENT: purrgotchi' }) })
    await $.session.start(START)
    const pane = await mountPane($, surface)
    expect(await pane.find({ text: /Binary path/ })).toBeDefined()
    expect(await pane.find({ text: /THALIA/ })).toBeUndefined()
  })

  test(`no kitten placeholder on ${surface}`, async ($, on) => {
    const spies = await startReady($, on, { run: answering(fx.noKitten) })
    await spies.clock.settle()
    const pane = await mountPane($, surface)
    expect(await pane.find({ text: /No kitten yet/ })).toBeDefined()
    expect(await pane.find({ type: 'Button', key: 'pet' })).toBeDefined()
  })
}

test('mobile buttons carry no hotkey and no esc hint', async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  const pane = await mountPane($, 'mobile')
  expect((await pane.find({ type: 'Button', key: 'pet' }))?.props.hotkey).toBeUndefined()
  expect(await pane.find({ text: /esc close/ })).toBeUndefined()
  const term = await mountPane($, 'terminal')
  expect((await term.find({ type: 'Button', key: 'pet' }))?.props.hotkey).toBe('p')
  expect(await term.find({ text: /esc close/ })).toBeDefined()
})

test('session + limits rows from measure', async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  const resetsAt = new Date(Date.now() + 2 * 3_600_000 + 30_000).toISOString()
  await $.session.measure({
    context: { percent: 42, tokens: 87_000, window: 1_000_000 },
    rateLimits: [
      { kind: 'five_hour', percentUsed: 37, resetsAt },
      { kind: 'spend_limit', percentUsed: 5 },
    ],
    changed: ['context', 'rateLimits'],
  })
  await spies.clock.settle()
  const pane = await mountPane($, 'terminal')
  expect(await pane.find({ text: /42% · 87k/ })).toBeDefined()
  expect(await pane.find({ text: /5h .* 37% · resets 2h 00m/ })).toBeDefined()
  expect(await pane.find({ text: /spend/ })).toBeUndefined()
  expect(await pane.find({ text: /model claude-opus-5-5/ })).toBeDefined()
})

test('missing rate limits and cost omit rows', async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  await $.session.measure({ context: { window: 1_000_000 }, rateLimits: [], changed: ['rateLimits'] })
  const pane = await mountPane($, 'terminal')
  expect(await pane.find({ text: /5h/ })).toBeUndefined()
  expect(await pane.find({ text: /cost/ })).toBeUndefined()
  expect(await pane.findAll({ text: /NaN|undefined/ })).toEqual([])
})

test('cost shows the session cost', async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  await $.session.measure({ context: { window: 1_000_000 }, rateLimits: [], cost: { usd: 0.123 }, changed: ['cost'] })
  const pane = await mountPane($, 'terminal')
  expect(await pane.find({ text: /cost \$0\.12 session/ })).toBeDefined()
})

test('stacks under 56 and drops sections under 40', async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  const mid = await mountPane($, 'terminal', 44)
  expect(await mid.find({ text: /SESSION/ })).toBeDefined()
  expect(await mid.find({ text: /THALIA/ })).toBeDefined()
  await mid.redraw(paneProps(30))
  const narrow = mid
  expect(await narrow.find({ text: /SESSION/ })).toBeUndefined()
  expect(await narrow.find({ text: /LIMITS/ })).toBeUndefined()
  expect(await narrow.find({ text: /THALIA/ })).toBeDefined()
  expect(await narrow.find({ type: 'Button', key: 'play' })).toBeDefined()
})

test('hud rows: stars, xp, fed and the peak label', async ($, on) => {
  const hud: HudJson = {
    total_tool_calls: 610,
    xp_pct: 20,
    last_fed_at: 1_792_000_000 - 12,
    stars: '★★★☆☆',
    name_fg: '#b45cff',
    peak: { label: 'OFF PEAK', range: '8PM-2PM', is_peak: false },
  }
  const spies = await startReady($, on, { run: answering(fx.frame({ hud })) })
  await spies.clock.settle()
  const pane = await mountPane($, 'terminal')
  expect((await pane.find({ type: 'Text', text: 'THALIA' }))?.props.color).toBe('#b45cff')
  expect(await pane.find({ text: /★★★☆☆ rare/ })).toBeDefined()
  expect(await pane.find({ text: /xp {3}▰▰▱▱▱▱▱▱▱▱ 20%/ })).toBeDefined()
  expect(await pane.find({ text: /fed {2}12s ago/ })).toBeDefined()
  expect(await pane.find({ text: /LIMITS ─+ ◉ OFF PEAK 8PM-2PM/ })).toBeDefined()
})

for (const cols of [30, 44, 56, 80]) {
  test(`no unwrapped row is wider than the body at ${cols} cols`, async ($, on) => {
    const scene: SceneJson = {
      kind: 'Run',
      elapsed: 1,
      duration: 8,
      finale: false,
      width: Math.max(0, cols - 4),
      rows: [[{ text: '.'.repeat(Math.max(0, cols - 4)) }]],
    }
    const spies = await startReady($, on, { run: answering(fx.frame({ scene })) })
    await spies.clock.settle()
    const resetsAt = new Date(Date.now() + 2 * 3_600_000 + 30_000).toISOString()
    await $.session.measure({
      context: { percent: 42, tokens: 87_000, window: 1_000_000 },
      rateLimits: [{ kind: 'five_hour', percentUsed: 37, resetsAt }],
      cost: { usd: 0.12 },
      changed: ['context', 'rateLimits', 'cost'],
    })
    const pane = await mountPane($, 'terminal', cols)
    const texts = await pane.findAll({ type: 'Text' })
    const loose = texts.filter(t => t.props.wrap === undefined && [...t.text].length > cols - 4)
    // Rows the surface truncates are `wrap="truncate"`; every other one fits.
    expect(loose.map(t => t.text)).toEqual([])
  })
}

test('scene rows render and width is fed back', async ($, on) => {
  const scene: SceneJson = {
    kind: 'Run',
    elapsed: 1,
    duration: 8,
    finale: false,
    width: 54,
    rows: [[{ text: '~~~=^.^=~~~' }], [{ text: '___________' }]],
  }
  const spies = await startReady($, on, { run: answering(fx.frame({ scene })) })
  await spies.clock.settle()
  const pane = await mountPane($, 'terminal', 58)
  await spies.clock.advance(1000)
  expect(ticks(spies.runs).at(-1)?.slice(3, 5)).toEqual(['--width', '54'])
  expect(await pane.find({ text: /=\^\.\^=/ })).toBeDefined()
})

test('a narrow pane feeds width 0 (no scene)', async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  await mountPane($, 'terminal', 26)
  await spies.clock.advance(1000)
  expect(ticks(spies.runs).at(-1)?.slice(3, 5)).toEqual(['--width', '0'])
})

test('press p pets and toasts', async ($, on) => {
  const spies = await startReady($, on, { run: answering(fx.frame()) })
  await spies.clock.settle()
  const pane = await mountPane($, 'terminal')
  const before = ticks(spies.runs).length
  await pane.press({ key: 'pet' })
  await spies.clock.settle()
  expect(spies.runs.filter(a => a[1] === 'pet')).toEqual([['purrgotchi', 'pet', '--json']])
  expect(spies.toasts).toEqual(['♥ Thalia purrs contentedly. Bond: 50 → 65 (+15)'])
  expect(ticks(spies.runs)).toHaveLength(before + 1)
})

test('treat and play buttons run their verbs', async ($, on) => {
  const spies = await startReady($, on, { run: answering(fx.frame()) })
  await spies.clock.settle()
  const pane = await mountPane($, 'desktop')
  await pane.press({ key: 'treat' })
  await pane.press({ key: 'play' })
  expect(spies.runs.filter(a => a[1] === 'treat' || a[1] === 'play')).toEqual([
    ['purrgotchi', 'treat', '--json'],
    ['purrgotchi', 'play', '--json'],
  ])
})

test('a cooldown press toasts the message without poking', async ($, on) => {
  const cooldown = fx.action({ ok: false, result: 'cooldown', message: 'Thalia needs a moment.' })
  const spies = await startReady($, on, { run: answering(fx.frame(), cooldown) })
  await spies.clock.settle()
  const pane = await mountPane($, 'terminal')
  const before = ticks(spies.runs).length
  await pane.press({ key: 'pet' })
  expect(spies.toasts).toEqual(['Thalia needs a moment.'])
  expect(ticks(spies.runs)).toHaveLength(before)
})

test('/purr opens the pane asked', async ($, on) => {
  const spies = await startReady($, on)
  spies.opens.length = 0
  const r = await $.command.run(cmd('purr'))
  // ASCII sprite (6) beside the 7-row info column: 3 + 7 + 1 + 4 + 3 + 1.
  expect(spies.opens).toEqual([{ id: 'kitten', title: 'Purrgotchi', columns: 58, rows: 19, focus: true, closeOnEscape: true }])
  expect(r.text?.startsWith('Opened')).toBe(true)
  expect(r.exitCode).toBeUndefined()
})

test('/purr not placed reports reason', async ($, on) => {
  await startReady($, on, { open: { notPlaced: 'no surface attached' } })
  const r = await $.command.run(cmd('purr'))
  expect(r.text).toBe("Couldn't open the pane: no surface attached")
  expect(r.exitCode).toBeUndefined()
})

test('/purr works with a missing binary', async ($, on) => {
  const spies = engine(on, { run: () => ({ deny: 'ENOENT: purrgotchi' }) })
  await $.session.start(START)
  expect(spies.registered).toContain('purr')
  const r = await $.command.run(cmd('purr'))
  expect(r.text?.startsWith('Opened')).toBe(true)
})

test('auto-open on start for dock + openPaneOnStart', async ($, on) => {
  const spies = await startReady($, on)
  expect(spies.opens).toEqual([{ id: 'kitten', title: 'Purrgotchi', columns: 58, rows: 19 }])
})

test('/purr asks for the full height of the pixel pane, scene included', async ($, on) => {
  const hud: HudJson = {
    total_tool_calls: 610,
    xp_pct: 20,
    last_fed_at: 1_792_000_000 - 12,
    stars: '★★★☆☆',
    name_fg: '#b45cff',
    peak: { label: 'OFF PEAK', range: '8PM-2PM', is_peak: false },
  }
  const scene: SceneJson = { kind: 'Run', elapsed: 1, duration: 8, finale: false, width: 54, rows: [[{ text: '.' }], [{ text: '.' }]] }
  const spies = await startReady($, on, { run: answering(fx.frame({ hud, pixels: MAINE_PIXELS, scene })) })
  spies.opens.length = 0
  await $.command.run(cmd('purr'))
  // border + header 3, sprite 16, speech 1, scene 2, session 4, limits 3, buttons 1
  expect(spies.opens.map(o => o.rows)).toEqual([30])
})

test('/purr before any kitten asks for the pixel pane height', async ($, on) => {
  const spies = await startReady($, on, { run: answering(fx.noKitten) })
  spies.opens.length = 0
  await $.command.run(cmd('purr'))
  expect(spies.opens.map(o => o.rows)).toEqual([28])
})

test('no auto-open without openPaneOnStart', { options: { openPaneOnStart: false } }, async ($, on) => {
  const spies = await startReady($, on)
  expect(spies.opens).toEqual([])
})

test('no auto-open in the bottom layout', { options: { layout: 'bottom' } }, async ($, on) => {
  const spies = await startReady($, on)
  expect(spies.opens).toEqual([])
})

for (const surface of SURFACES) {
  test(`pixel sprite replaces ASCII when pixels present on ${surface}`, async ($, on) => {
    const spies = await startReady($, on, { run: answering(fx.frame({ pixels: MAINE_PIXELS })) })
    await spies.clock.settle()
    const pane = await mountPane($, surface)
    expect(await pane.find({ text: ASCII_SPRITE })).toBeUndefined()
    if (surface === 'terminal') {
      const raster = await pane.find({ type: 'Raster' })
      expect(raster?.props).toMatchObject({ key: 'kitten-full', columns: 32, rows: 16 })
      expect(await pane.find({ type: 'Svg' })).toBeUndefined()
    } else {
      const svg = await pane.find({ type: 'Svg' })
      expect(svg?.props.alt).toMatch(/Thalia the Maine Coon/)
      expect(svg?.props.source).toContain('viewBox="0 0 32 32"')
      expect(await pane.find({ type: 'Raster' })).toBeUndefined()
    }
    // the info column is still drawn beside it
    expect(await pane.find({ text: /THALIA/ })).toBeDefined()
    expect(await pane.find({ type: 'Button', key: 'pet' })).toBeDefined()
  })

  test(`falls back to ASCII when pixels absent on ${surface}`, async ($, on) => {
    const spies = await startReady($, on, { run: answering(fx.frame()) })
    await spies.clock.settle()
    const pane = await mountPane($, surface)
    expect(await pane.find({ text: ASCII_SPRITE })).toBeDefined()
    expect(await pane.find({ type: 'Raster' })).toBeUndefined()
    expect(await pane.find({ type: 'Svg' })).toBeUndefined()
  })
}

test('ASCII sprite when the body is too narrow for 32 columns', async ($, on) => {
  const spies = await startReady($, on, { run: answering(fx.frame({ pixels: MAINE_PIXELS })) })
  await spies.clock.settle()
  // 4 columns of border and padding: a 36 column pane leaves 32, under the 34 floor
  const pane = await mountPane($, 'terminal', 36)
  expect(await pane.find({ text: ASCII_SPRITE })).toBeDefined()
  expect(await pane.find({ type: 'Raster' })).toBeUndefined()
  await pane.redraw(paneProps(44))
  expect(await pane.find({ text: ASCII_SPRITE })).toBeUndefined()
  expect(await pane.find({ type: 'Raster' })).toBeDefined()
})

test('a malformed pixel grid falls back to ASCII', async ($, on) => {
  const spies = await startReady($, on, { run: answering(fx.frame({ pixels: [['#ffffff']] })) })
  await spies.clock.settle()
  const pane = await mountPane($, 'terminal')
  expect(await pane.find({ text: ASCII_SPRITE })).toBeDefined()
  expect(await pane.find({ type: 'Raster' })).toBeUndefined()
})
