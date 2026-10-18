import { expect, test } from 'claude-code/testing'
import type { Engine } from 'claude-code/testing'

import { bandArgv, blankCells, pickRows } from './band'
import { START, bandFrame, bandHello, engine, gate, line } from './testkit'
import type { EngineOptions, SpawnScript } from './testkit'

// The band driven through the engine: a session start, then an
// `AbovePrompt` mount the way a surface draws it. The binary is PURR_BIN.

const BIN = '/opt/purr/purrgotchi'
const ENV = { env: { PURR_BIN: BIN }, exists: [BIN] } as const
const argvAt = (cols: number, rows: 5 | 6 = 5) => bandArgv(BIN, cols, rows, 30, 0)
/** Valid cells for 115 x 5 (or `cols` x `rows`), told apart by `seed`. */
function cells(seed: number, cols = 115, rows = 5): string {
  const w = new Uint32Array(cols * rows * 3).fill(0x0100_0000)
  for (let i = 0; i < cols * rows; i++) w[i * 3] = 0x20
  w[0] = 0x2580
  w[1] = seed
  return (new Uint8Array(w.buffer) as unknown as { toBase64: () => string }).toBase64()
}
const [CELLS1, ONE, TWO, SPLIT, CAT, OLD] = [1, 2, 3, 4, 5, 6].map(n => cells(n))
const flag = (argv: string[] | undefined, name: string) => argv?.[argv.indexOf(name) + 1]

type Surface = 'terminal' | 'desktop'

const props = (bodyColumns: number, maxRows = 40, hasSurvey = false) =>
  ({ hasSurvey, isWorking: false, maxRows, bodyColumns, scroll: { offset: 0, bodyRows: maxRows }, view: {} }) as const

function mountBand($: Engine, bodyColumns = 115, maxRows = 40, surface: Surface = 'terminal', hasSurvey = false) {
  return $.ui.mount({ plugin: 'purrgotchi', surface, component: 'AbovePrompt', requestId: 'above-prompt', props: props(bodyColumns, maxRows, hasSurvey) })
}

/** Every child: `steps`, then open and silent until stopped. */
const silent = (steps: SpawnScript['steps'] = []) => () => ({ steps })

async function started($: Engine, on: Parameters<typeof engine>[0], o: EngineOptions = {}) {
  const spies = engine(on, { ...ENV, ...o })
  await $.session.start(START)
  return spies
}

test('pickRows: the requested height if it fits, else the largest that does', () => {
  expect(pickRows(5, 40)).toBe(5)
  expect(pickRows(6, 5)).toBe(5)
  expect(pickRows(6, 15)).toBe(6)
  expect(pickRows(5, 4)).toBe(0)
  expect(pickRows(6, 2)).toBe(0)
})

test('blankCells is default-colour spaces', () => {
  const bytes = Uint8Array.from(atob(blankCells(2, 1)), c => c.charCodeAt(0))
  expect([...new Uint32Array(bytes.buffer)]).toEqual([0x20, 0x0100_0000, 0x0100_0000, 0x20, 0x0100_0000, 0x0100_0000])
})

test('first render spawns band argv', async ($, on) => {
  const spies = await started($, on, { spawn: silent() })
  await mountBand($)
  await spies.clock.advance(100)
  expect(spies.spawns).toEqual([[BIN, 'band', '--cols', '115', '--rows', '5', '--fps', '30', '--context-pct', '0']])
})

test('hello then frame blits once with its cells', async ($, on) => {
  const g = gate()
  const spies = await started($, on, { spawn: silent([bandHello(115, 5), { wait: g.wait }, bandFrame(1, CELLS1)]) })
  const band = await mountBand($)
  await spies.clock.advance(100)
  expect(spies.blits).toEqual([])
  g.release()
  await spies.clock.settle()
  expect(spies.blits).toEqual([{ requestId: 'above-prompt', key: 'purr-band', cells: CELLS1 }])
  await band.redraw()
  expect(await band.drawn()).toMatchObject({ type: 'Raster', props: { key: 'purr-band', columns: 115, rows: 5, cells: CELLS1 } })
})

test('hello and frame in one chunk blit the frame', async ($, on) => {
  const spies = await started($, on, { spawn: silent([bandHello(115, 5) + bandFrame(1, CELLS1)]) })
  await mountBand($)
  await spies.clock.advance(100)
  expect(spies.blits.map(b => b.cells)).toEqual([CELLS1])
})

test('two frames in one chunk blit only the latest', async ($, on) => {
  const g = gate()
  const spies = await started($, on, {
    spawn: silent([bandHello(115, 5), { wait: g.wait }, bandFrame(1, ONE) + bandFrame(2, TWO)]),
  })
  await mountBand($)
  await spies.clock.advance(100)
  g.release()
  await spies.clock.settle()
  expect(spies.blits.map(b => b.cells)).toEqual([TWO])
})

test('a split frame line is carried to the next chunk', async ($, on) => {
  const f = bandFrame(1, SPLIT)
  const g = gate()
  const spies = await started($, on, { spawn: silent([bandHello(115, 5) + f.slice(0, 9), { wait: g.wait }, f.slice(9)]) })
  await mountBand($)
  await spies.clock.advance(100)
  expect(spies.blits).toEqual([])
  g.release()
  await spies.clock.settle()
  expect(spies.blits.map(b => b.cells)).toEqual([SPLIT])
})

test('an error line blanks the band once', async ($, on) => {
  const err = line({ v: 1, ok: false, error: 'no_kitten', message: 'no kitten' })
  const g = gate()
  const spies = await started($, on, { spawn: silent([bandHello(115, 5) + bandFrame(1, CAT), { wait: g.wait }, err + err]) })
  const band = await mountBand($)
  await spies.clock.advance(100)
  g.release()
  await spies.clock.settle()
  expect(spies.blits.map(b => b.cells)).toEqual([CAT, blankCells(115, 5)])
  await band.redraw()
  expect(await band.drawn()).toMatchObject({ type: 'Raster', props: { cells: blankCells(115, 5) } })
})

test('render before start passes then invalidates', async ($, on) => {
  const spies = engine(on, { ...ENV, spawn: silent() })
  const band = await mountBand($)
  expect(await band.find({ type: 'Raster' })).toBeUndefined()
  await $.session.start(START)
  await spies.clock.settle()
  expect(await band.drawn()).toMatchObject({ type: 'Raster', props: { key: 'purr-band', columns: 115, rows: 5, cells: blankCells(115, 5) } })
})

test('desktop surface and surveys pass through and never spawn', async ($, on) => {
  const spies = await started($, on, { spawn: silent() })
  const desk = await mountBand($, 115, 40, 'desktop')
  expect(await desk.find({ type: 'Raster' })).toBeUndefined()
  const survey = await mountBand($, 115, 40, 'terminal', true)
  expect(await survey.find({ type: 'Raster' })).toBeUndefined()
  await spies.clock.advance(1000)
  expect(spies.spawns).toEqual([])
})

test('maxRows 2 draws nothing', async ($, on) => {
  const spies = await started($, on, { spawn: silent() })
  const band = await mountBand($, 115, 2)
  expect(await band.find({ type: 'Raster' })).toBeUndefined()
  await spies.clock.advance(1000)
  expect(spies.spawns).toEqual([])
})

test('a band narrower than 24 draws nothing', async ($, on) => {
  const spies = await started($, on, { spawn: silent() })
  const band = await mountBand($, 23)
  expect(await band.find({ type: 'Raster' })).toBeUndefined()
  await spies.clock.advance(1000)
  expect(spies.spawns).toEqual([])
})

test('a short maxRows falls back to the largest height that fits', async ($, on) => {
  const spies = await started($, on, { env: { PURR_BIN: BIN, PURR_BAND_ROWS: '6' }, spawn: silent() })
  const band = await mountBand($, 115, 5)
  expect(await band.drawn()).toMatchObject({ type: 'Raster', props: { rows: 5 } })
  await spies.clock.advance(100)
  expect(flag(spies.spawns[0], '--rows')).toBe('5')
})

test('child exit respawns after 1 s then 2 s', async ($, on) => {
  const spies = await started($, on, { spawn: () => ({ steps: [bandHello(115, 5)], end: { code: 0, signal: null } }) })
  await mountBand($)
  await spies.clock.advance(100)
  expect(spies.spawns).toHaveLength(1)
  await spies.clock.advance(999)
  expect(spies.spawns).toHaveLength(1)
  await spies.clock.advance(1)
  expect(spies.spawns).toHaveLength(2)
  await spies.clock.advance(1999)
  expect(spies.spawns).toHaveLength(2)
  await spies.clock.advance(1)
  expect(spies.spawns).toHaveLength(3)
})

test('resize storm respawns once', async ($, on) => {
  const g = gate()
  const spies = await started($, on, {
    spawn: (_, n) => ({ steps: n === 1 ? [bandHello(115, 5), { wait: g.wait }, bandFrame(9, OLD)] : [] }),
  })
  const band = await mountBand($)
  await spies.clock.advance(100)
  expect(spies.spawns).toHaveLength(1)
  for (const cols of [120, 110, 100, 90, 80]) {
    await band.redraw(props(cols))
    await spies.clock.advance(50)
  }
  // A frame the old child sized for 115 columns: not blitted.
  g.release()
  await spies.clock.settle()
  expect(spies.blits).toEqual([])
  expect(await band.drawn()).toMatchObject({ type: 'Raster', props: { columns: 80, cells: blankCells(80, 5) } })
  await spies.clock.advance(200) // 250 ms after the last redraw
  expect(spies.spawns).toHaveLength(1)
  await spies.clock.advance(200) // 450 ms after it
  expect(spies.spawns).toHaveLength(2)
  expect(spies.spawns[1]).toEqual(argvAt(80))
  expect(spies.stops).toBe(1)
  await spies.clock.advance(5000)
  expect(spies.spawns).toHaveLength(2)
})

test('a resize back to the running size does not respawn', async ($, on) => {
  const spies = await started($, on, { spawn: silent([bandHello(115, 5)]) })
  const band = await mountBand($)
  await spies.clock.advance(100)
  await band.redraw(props(100))
  await spies.clock.advance(150)
  await band.redraw(props(115))
  await spies.clock.advance(2000)
  expect(spies.spawns).toHaveLength(1)
  expect(spies.stops).toBe(0)
})

test('old binary backs off and never blits', async ($, on) => {
  const spies = await started($, on, { spawn: () => ({ steps: [{ stderr: 'usage: purrgotchi …\n' }], end: { code: 2, signal: null } }) })
  await mountBand($)
  await spies.clock.advance(100) // spawn 1, exits
  await spies.clock.advance(1000) // spawn 2 after 1 s, exits
  await spies.clock.advance(2000) // spawn 3 after 2 s, exits: the third within 60 s
  expect(spies.spawns).toHaveLength(3)
  await spies.clock.advance(59_999)
  expect(spies.spawns).toHaveLength(3)
  await spies.clock.advance(1)
  expect(spies.spawns).toHaveLength(4)
  expect(spies.blits).toEqual([])
  expect(spies.logs.filter(l => /retrying every 60 s/.test(l))).toHaveLength(1)
})

test('missing binary passes', async ($, on) => {
  const spies = engine(on, { spawn: silent() })
  await $.session.start(START)
  const band = await mountBand($)
  expect(await band.find({ type: 'Raster' })).toBeUndefined()
  await spies.clock.advance(1000)
  expect(spies.spawns).toEqual([])
})

const HIDDEN = props(115, 40, true)

test('a band hidden for 10 s stops its child and respawns once when shown', async ($, on) => {
  const spies = await started($, on, { spawn: silent([bandHello(115, 5)]) })
  const band = await mountBand($)
  await spies.clock.advance(100)
  expect(spies.spawns).toHaveLength(1)
  await band.redraw(HIDDEN)
  await spies.clock.advance(9000)
  expect(spies.stops).toBe(0)
  await spies.clock.advance(1500)
  expect(spies.stops).toBe(1)
  await spies.clock.advance(5000)
  expect(spies.spawns).toHaveLength(1)
  await band.redraw(props(115))
  await spies.clock.advance(100)
  expect(spies.spawns).toHaveLength(2)
  await spies.clock.advance(5000)
  expect(spies.spawns).toHaveLength(2)
})

test('a band hidden under 10 s keeps its child', async ($, on) => {
  const spies = await started($, on, { spawn: silent([bandHello(115, 5)]) })
  const band = await mountBand($)
  await spies.clock.advance(100)
  await band.redraw(HIDDEN)
  await spies.clock.advance(8000)
  await band.redraw(props(115))
  await spies.clock.advance(8000) // the hide clock restarted at the show
  await band.redraw(HIDDEN)
  await spies.clock.advance(8000)
  expect(spies.stops).toBe(0)
  expect(spies.spawns).toHaveLength(1)
})

test('5 straight denies pause blitting until the next render', async ($, on) => {
  const g = gate()
  const frames = [1, 2, 3, 4, 5, 6, 7, 8].map(n => bandFrame(n, cells(10 + n)))
  const spies = await started($, on, {
    blit: n => (n <= 5 ? { deny: 'no' } : {}),
    spawn: silent([bandHello(115, 5), ...frames, { wait: g.wait }, bandFrame(9, cells(30))]),
  })
  const band = await mountBand($)
  await spies.clock.advance(100)
  expect(spies.blits).toHaveLength(5) // frames 6-8 were skipped
  await band.redraw(props(115)) // a render: blitting resumes
  g.release()
  await spies.clock.settle()
  expect(spies.blits).toHaveLength(6)
  expect(spies.blits[5]?.cells).toBe(cells(30))
})

test('the first no-Raster deny does not count toward the backoff', async ($, on) => {
  const spies = await started($, on, {
    blit: n => (n === 1 ? { deny: 'no Raster "purr-band" mounted' } : n <= 6 ? { deny: 'no' } : {}),
    spawn: silent([bandHello(115, 5), ...[1, 2, 3, 4, 5, 6, 7].map(n => bandFrame(n, cells(10 + n)))]),
  })
  await mountBand($)
  await spies.clock.advance(100)
  expect(spies.blits).toHaveLength(6) // 1 uncounted + 5 counted
})
