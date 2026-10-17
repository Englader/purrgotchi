import { expect, test } from 'claude-code/testing'

import { cmd, defaultRun, engine, fx, gate, ok, START, startReady } from './testkit'
import type { EngineOptions, OpAnswer } from './testkit'

// Everything is asserted through what the binary was asked and what the
// engine saw (`ui.status`, `ui.log`, `ui.toast`): the live Runtime is out of reach.

const frames = (runs: string[][]) => runs.filter(a => a[1] === 'frame')
/** Ticker runs (the handshake's `frame --json` has no `--width`). */
const ticks = (runs: string[][]) => frames(runs).filter(a => a.includes('--width'))
const STATUS = '=^.^= thalia lv12 · 340¢ · calm'

/** The handshake as usual; every tick answered by `tick`, any other verb by `action` when given. */
const ticking =
  (tick: () => OpAnswer, action?: string): EngineOptions['run'] =>
  argv => {
    if (argv.includes('--width')) return tick()
    if (action !== undefined && argv[1] !== 'frame' && !argv.includes('--version')) return ok(action)
    return defaultRun(argv)
  }

test('starts on session start and ticks every 1000 ms (dock)', async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.advance(3000)
  await spies.clock.settle()
  // 1 handshake + 1 immediate + 3 ticks
  expect(frames(spies.runs)).toHaveLength(5)
})

test('classic layout ticks every 5000 ms', { options: { layout: 'classic' } }, async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  expect(ticks(spies.runs)).toHaveLength(1)
  await spies.clock.advance(4999)
  expect(ticks(spies.runs)).toHaveLength(1)
  await spies.clock.advance(1)
  expect(ticks(spies.runs)).toHaveLength(2)
  await spies.clock.advance(5000)
  expect(ticks(spies.runs)).toHaveLength(3)
})

test('off layout never ticks', { options: { layout: 'off' } }, async ($, on) => {
  const spies = engine(on)
  await $.session.start(START)
  await spies.clock.advance(5000)
  expect(frames(spies.runs)).toEqual([])
})

test('argv: width 0 before any pane render, ctx from measure', async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  await $.session.measure({ context: { percent: 42.4, window: 1e6 }, rateLimits: [], changed: ['context'] })
  await spies.clock.settle()
  expect(ticks(spies.runs).at(-1)).toEqual([
    'purrgotchi',
    'frame',
    '--json',
    '--width',
    '0',
    '--context-pct',
    '42.4',
    '--advance',
    '--pixels',
  ])
})

test('no --advance when classic statusline installed', async ($, on) => {
  const spies = await startReady($, on, { settings: { statusLine: { type: 'command', command: 'purrgotchi statusline' } } })
  await spies.clock.advance(1000)
  expect(ticks(spies.runs).length).toBeGreaterThan(0)
  for (const a of ticks(spies.runs)) expect(a).not.toContain('--advance')
})

test('single-flight under a hung frame', async ($, on) => {
  const g = gate()
  const spies = await startReady($, on, {
    run: ticking(async () => {
      await g.wait
      return ok(fx.frame())
    }),
  })
  await spies.clock.advance(5000)
  expect(ticks(spies.runs)).toHaveLength(1)
  g.release()
  await spies.clock.settle()
  await spies.clock.advance(1000)
  expect(ticks(spies.runs)).toHaveLength(2)
  expect(spies.toasts).toEqual([])
})

test('no_kitten clears the frame and status', async ($, on) => {
  const spies = await startReady($, on, { run: ticking(() => ok(fx.noKitten, 1)) })
  await spies.clock.settle()
  expect(spies.statuses[0]).toBe(STATUS)
  expect(spies.statuses).toHaveLength(2)
  expect(spies.statuses.at(-1)).toBeUndefined()
})

test('a busy lock keeps the last status', async ($, on) => {
  const spies = await startReady($, on, { run: ticking(() => ok(fx.error('state_unavailable', 'lock busy'), 1)) })
  await spies.clock.advance(2000)
  expect(spies.statuses).toEqual([STATUS])
})

test('status entry updates only on change', async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.advance(2000)
  expect(ticks(spies.runs)).toHaveLength(3)
  expect(spies.statuses).toEqual([STATUS])
})

test('status entry follows the level', async ($, on) => {
  let level = 12
  const spies = await startReady($, on, { run: ticking(() => ok(fx.frame({ level }))) })
  await spies.clock.settle()
  level = 13
  await spies.clock.advance(1000)
  expect(spies.statuses).toEqual([STATUS, '=^.^= thalia lv13 · 340¢ · calm'])
})

test('pet pokes the ticker', async ($, on) => {
  const spies = await startReady($, on, { run: ticking(() => ok(fx.frame()), fx.action()) })
  await spies.clock.settle()
  const before = ticks(spies.runs).length
  await $.command.run(cmd('purr-pet'))
  await spies.clock.settle()
  expect(ticks(spies.runs)).toHaveLength(before + 1)
})

test('measure poke on context change only', async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  const before = ticks(spies.runs).length
  await $.session.measure({ context: { window: 1e6 }, rateLimits: [], cost: { usd: 0.12 }, changed: ['cost'] })
  await spies.clock.settle()
  expect(ticks(spies.runs)).toHaveLength(before)
})

test('5 consecutive failures log once, never toast', async ($, on) => {
  const spies = await startReady($, on, { run: ticking(() => ({ deny: 'timed out' })) })
  await spies.clock.advance(5000)
  expect(ticks(spies.runs)).toHaveLength(6)
  expect(spies.logs.filter(l => l.includes('frame'))).toHaveLength(1)
  expect(spies.toasts).toEqual([])
})

test('dock ticks with --pixels', async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  expect(ticks(spies.runs).length).toBeGreaterThan(0)
  for (const a of ticks(spies.runs)) expect(a.at(-1)).toBe('--pixels')
  // the handshake is not a drawing tick
  expect(frames(spies.runs).filter(a => !a.includes('--width'))[0]).not.toContain('--pixels')
})

test('bottom ticks with --pixels', { options: { layout: 'bottom' } }, async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  for (const a of ticks(spies.runs)) expect(a).toContain('--pixels')
})

test('classic ticks without --pixels', { options: { layout: 'classic' } }, async ($, on) => {
  const spies = await startReady($, on)
  await spies.clock.settle()
  expect(ticks(spies.runs).length).toBeGreaterThan(0)
  for (const a of ticks(spies.runs)) expect(a).not.toContain('--pixels')
})

test('retries without --pixels after exit 2', async ($, on) => {
  let first = true
  const spies = await startReady($, on, {
    run: ticking(() => {
      if (!first) return ok(fx.frame())
      first = false
      return ok('', 2)
    }),
  })
  await spies.clock.settle()
  await spies.clock.advance(3000)
  const t = ticks(spies.runs)
  expect(t[0]).toContain('--pixels')
  expect(t.length).toBeGreaterThan(1)
  for (const a of t.slice(1)) expect(a).not.toContain('--pixels')
  expect(spies.toasts).toEqual([])
})

test('a bad_args answer to --pixels also drops the flag', async ($, on) => {
  const spies = await startReady($, on, {
    run: ticking(() => ok(fx.badArgs('unknown flag --pixels'), 2)),
  })
  await spies.clock.settle()
  await spies.clock.advance(2000)
  const t = ticks(spies.runs)
  expect(t[0]).toContain('--pixels')
  for (const a of t.slice(1)) expect(a).not.toContain('--pixels')
})

test('other failures keep asking for --pixels', async ($, on) => {
  const spies = await startReady($, on, { run: ticking(() => ({ deny: 'timed out' })) })
  await spies.clock.advance(2000)
  for (const a of ticks(spies.runs)) expect(a).toContain('--pixels')
})
