import { expect, test } from 'claude-code/testing'

import { fx, gate, ok, paneProps, startReady } from './testkit'
import type { SpawnScript, Spies } from './testkit'

// `purrgotchi serve` as the dock / bottom frame source: asserted through what
// was spawned and stopped, what `frame` runs the clock made, and the status
// entry each applied line shows. The live Runtime is out of reach.

const SERVE = ['purrgotchi', 'serve', '--interval-ms', '1000']
const STATUS = '=^.^= thalia lv12 · 340¢ · calm'
const coins = (n: number) => `=^.^= thalia lv12 · ${n}¢ · calm`
/** Ticker `frame` runs (the handshake's has no `--width`). */
const ticks = (runs: string[][]) => runs.filter(a => a[1] === 'frame' && a.includes('--width'))
const flag = (argv: string[] | undefined, name: string) => argv?.[argv.indexOf(name) + 1]

/** Every child: `steps`, then open and silent until stopped. */
const silent = (steps: SpawnScript['steps'] = []) => () => ({ steps })

test('dock layout spawns serve with flags', async ($, on) => {
  const spies = await startReady($, on, { spawn: silent() })
  await spies.clock.settle()
  expect(spies.spawns).toEqual([[...SERVE, '--width', '0', '--context-pct', '0.0', '--advance', '--pixels']])
})

test('bottom layout spawns serve too', { options: { layout: 'bottom' } }, async ($, on) => {
  const spies = await startReady($, on, { spawn: silent() })
  await spies.clock.settle()
  expect(spies.spawns).toHaveLength(1)
})

test('classic layout never spawns serve', { options: { layout: 'classic' } }, async ($, on) => {
  const spies = await startReady($, on, { spawn: silent() })
  await spies.clock.advance(10_000)
  expect(spies.spawns).toEqual([])
  expect(ticks(spies.runs)).toHaveLength(3) // immediate, 5 s, 10 s
})

test('two frames in one chunk and one split frame apply once each', async ($, on) => {
  const second = fx.frame({ coins: 2 })
  const g = gate()
  const spies = await startReady($, on, {
    spawn: silent([fx.frame({ coins: 1 }) + second.slice(0, 10), { wait: g.wait }, second.slice(10)]),
  })
  await spies.clock.settle()
  expect(spies.statuses).toEqual([STATUS, coins(1)])
  g.release()
  await spies.clock.settle()
  expect(spies.statuses).toEqual([STATUS, coins(1), coins(2)])
})

test('lines split anywhere: empty lines and a partial line at exit are not applied', async ($, on) => {
  const f = fx.frame({ coins: 3 })
  const spies = await startReady($, on, {
    spawn: (_, n) => (n === 1 ? { steps: ['\n\n', f.slice(0, 5), f.slice(5), fx.frame({ coins: 4 }).slice(0, 30)], end: { code: 0, signal: null } } : {}),
  })
  await spies.clock.settle()
  expect(spies.statuses).toEqual([STATUS, coins(3)])
})

test('garbage line skipped', async ($, on) => {
  const spies = await startReady($, on, { spawn: silent(['warning: x\n' + fx.frame({ coins: 7 }), { stderr: 'noise\n' }]) })
  await spies.clock.settle()
  expect(spies.statuses).toEqual([STATUS, coins(7)])
  expect(spies.toasts).toEqual([])
})

test('a no_kitten line clears the status; a busy lock keeps it', async ($, on) => {
  const g = gate()
  const spies = await startReady($, on, {
    spawn: silent([fx.error('state_unavailable', 'lock busy'), { wait: g.wait }, fx.noKitten]),
  })
  await spies.clock.settle()
  expect(spies.statuses).toEqual([STATUS])
  g.release()
  await spies.clock.settle()
  expect(spies.statuses).toEqual([STATUS, undefined])
})

test('a streaming serve replaces polling', async ($, on) => {
  const gates = [gate(), gate(), gate(), gate(), gate()]
  const steps = gates.flatMap(g => [fx.frame(), { wait: g.wait }])
  const spies = await startReady($, on, { spawn: silent(steps) })
  await spies.clock.settle()
  const before = ticks(spies.runs).length
  for (const g of gates) {
    await spies.clock.advance(1000)
    g.release()
    await spies.clock.settle()
  }
  expect(ticks(spies.runs)).toHaveLength(before)
  expect(spies.spawns).toHaveLength(1)
})

test('polls while serve is silent for 3 s', async ($, on) => {
  const spies = await startReady($, on, { spawn: silent() })
  await spies.clock.settle()
  const before = ticks(spies.runs).length // the start's immediate tick
  await spies.clock.advance(2999)
  expect(ticks(spies.runs)).toHaveLength(before)
  await spies.clock.advance(1)
  expect(ticks(spies.runs)).toHaveLength(before + 1)
  expect(spies.spawns).toHaveLength(1)
})

test('respawns once after width change (debounced)', async ($, on) => {
  const spies = await startReady($, on, { spawn: silent([fx.frame()]) })
  await spies.clock.settle()
  const pane = await $.ui.mount({ plugin: 'purrgotchi', surface: 'terminal', component: 'Pane', requestId: 'kitten', props: paneProps(58) })
  await pane.redraw(paneProps(60))
  await pane.redraw(paneProps(62))
  await spies.clock.advance(1000) // the watchdog sees the new width
  expect(spies.spawns).toHaveLength(1)
  await spies.clock.advance(499)
  expect(spies.spawns).toHaveLength(1)
  await spies.clock.advance(1)
  await spies.clock.settle()
  expect(spies.spawns).toHaveLength(2)
  expect(flag(spies.spawns[1], '--width')).toBe('58')
  expect(spies.stops).toBe(1)
  await spies.clock.advance(5000)
  expect(spies.spawns).toHaveLength(2)
})

test('ctx bucket change respawns, small change does not', async ($, on) => {
  const spies = await startReady($, on, { spawn: silent([fx.frame()]) })
  const measure = (percent: number) =>
    $.session.measure({ context: { percent, window: 1e6 }, rateLimits: [], changed: ['context'] })
  await measure(41.0)
  await spies.clock.advance(2000)
  expect(spies.spawns).toHaveLength(2)
  expect(flag(spies.spawns[1], '--context-pct')).toBe('40.0')
  await measure(43.9)
  await spies.clock.advance(2000)
  await measure(44.0)
  await spies.clock.advance(2000)
  expect(spies.spawns).toHaveLength(2)
  await measure(46.0)
  await spies.clock.advance(2000)
  expect(spies.spawns).toHaveLength(3)
  expect(flag(spies.spawns[2], '--context-pct')).toBe('45.0')
  expect(spies.stops).toBe(2)
})

test('a dead serve is respawned with backoff', async ($, on) => {
  const spies = await startReady($, on, { spawn: (_, n) => (n === 1 ? { steps: [fx.frame()], end: { code: 0, signal: null } } : {}) })
  await spies.clock.settle()
  expect(spies.spawns).toHaveLength(1)
  await spies.clock.advance(999)
  expect(spies.spawns).toHaveLength(1)
  await spies.clock.advance(1)
  expect(spies.spawns).toHaveLength(2)
  expect(spies.stops).toBe(0)
})

/** Seconds of clock, then the spawn and tick counts. */
async function after(spies: Spies, ms: number): Promise<{ spawns: number; ticks: number }> {
  await spies.clock.advance(ms)
  await spies.clock.settle()
  return { spawns: spies.spawns.length, ticks: ticks(spies.runs).length }
}

test('falls back to polling after 3 quick exits', async ($, on) => {
  const spies = await startReady($, on, { spawn: () => ({ end: { code: 2, signal: null } }) })
  await spies.clock.settle()
  expect((await after(spies, 3000)).spawns).toBe(3) // 0 s, +1 s, +2 s
  const broken = await after(spies, 1000)
  const later = await after(spies, 3000)
  expect(later.spawns).toBe(3)
  expect(later.ticks - broken.ticks).toBe(3)
  expect(spies.toasts).toEqual([])
  expect(spies.logs.filter(l => l.includes('serve'))).toHaveLength(1)
})

test('a binary that cannot spawn serve keeps polling every second', async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  const start = ticks(spies.runs).length
  const t = await after(spies, 10_000)
  expect(t.spawns).toBe(3)
  expect(t.ticks - start).toBe(10)
  expect(spies.toasts).toEqual([])
})

test('session.end clear does not stop serve', async ($, on) => {
  const spies = await startReady($, on, { spawn: silent([fx.frame()]) })
  await spies.clock.settle()
  await $.session.end({ reason: 'clear', sessionId: 's1', resume: { id: 's1' } })
  await spies.clock.advance(3000)
  expect(spies.stops).toBe(0)
  expect(spies.spawns).toHaveLength(1)
})

test('a later session start does not spawn a second serve', async ($, on) => {
  const spies = await startReady($, on, { spawn: silent([fx.frame()]) })
  await $.session.start({ cwd: '/tmp', surface: 'terminal', isInteractive: true })
  await spies.clock.advance(2000)
  expect(spies.spawns).toHaveLength(1)
  expect(spies.stops).toBe(0)
})

test('poke still runs one immediate frame', async ($, on) => {
  const spies = await startReady($, on, {
    spawn: silent([fx.frame()]),
    run: argv => (argv.includes('--version') ? ok('purrgotchi 3.8.0\n') : argv[1] === 'frame' ? ok(fx.frame()) : ok(fx.action())),
  })
  await spies.clock.settle()
  const before = ticks(spies.runs).length
  await $.command.run({ command: 'purr-pet', args: '', origin: { kind: 'composer' }, presentation: { isFullscreen: false, columns: 120 } })
  await spies.clock.settle()
  expect(ticks(spies.runs)).toHaveLength(before + 1)
})
