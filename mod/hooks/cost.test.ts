import { expect, test } from 'claude-code/testing'

import { defaultRun, fx, mountPane, ok, startReady } from './testkit'
import type { EngineOptions } from './testkit'

const costs = (runs: string[][]) => runs.filter(a => a[1] === 'cost')

/** Every `cost` run answers a lifetime total of 3.42; the rest is the default binary. */
const withCost: NonNullable<EngineOptions['run']> = argv => (argv[1] === 'cost' ? ok(fx.cost()) : defaultRun(argv))

const measure = { context: { window: 1_000_000 }, rateLimits: [] }

test('cost change calls cost with session id', async ($, on) => {
  const spies = await startReady($, on, { run: withCost, sessionId: 's-1' })
  await $.session.measure({ ...measure, cost: { usd: 0.12 }, changed: ['cost'] })
  await spies.clock.settle()
  expect(costs(spies.runs)).toEqual([['purrgotchi', 'cost', '--session', 's-1', '--total', '0.120000', '--json']])
})

test('context-only change does not', async ($, on) => {
  const spies = await startReady($, on, { run: withCost })
  await $.session.measure({ ...measure, cost: { usd: 0.12 }, changed: ['context'] })
  await spies.clock.settle()
  expect(costs(spies.runs)).toEqual([])
})

test('missing cost does not', async ($, on) => {
  const spies = await startReady($, on, { run: withCost })
  await $.session.measure({ ...measure, changed: ['cost'] })
  await spies.clock.settle()
  expect(costs(spies.runs)).toEqual([])
})

test('negative cost does not', async ($, on) => {
  const spies = await startReady($, on, { run: withCost })
  await $.session.measure({ ...measure, cost: { usd: -1 }, changed: ['cost'] })
  await spies.clock.settle()
  expect(costs(spies.runs)).toEqual([])
})

test('session.id failure skips', async ($, on) => {
  const spies = await startReady($, on, { run: withCost, sessionId: 'deny' })
  await $.session.measure({ ...measure, cost: { usd: 0.12 }, changed: ['cost'] })
  await spies.clock.settle()
  expect(costs(spies.runs)).toEqual([])
  expect(spies.toasts).toEqual([])
})

test('empty session id skips', async ($, on) => {
  const spies = await startReady($, on, { run: withCost, sessionId: '' })
  await $.session.measure({ ...measure, cost: { usd: 0.12 }, changed: ['cost'] })
  await spies.clock.settle()
  expect(costs(spies.runs)).toEqual([])
})

test('lifetime total shows in pane', async ($, on) => {
  const spies = await startReady($, on, { run: withCost })
  await spies.clock.settle()
  await $.session.measure({ ...measure, cost: { usd: 0.123 }, changed: ['cost'] })
  await spies.clock.settle()
  const pane = await mountPane($)
  expect(await pane.find({ text: /cost \$0\.12 · all \$3\.42/ })).toBeDefined()
})

test('a large lifetime total still fits its half-width pane cell', async ($, on) => {
  const big: NonNullable<EngineOptions['run']> = argv =>
    argv[1] === 'cost' ? ok(fx.cost({ lifetime_cost_usd: 1234.56 })) : defaultRun(argv)
  const spies = await startReady($, on, { run: big })
  await spies.clock.settle()
  await $.session.measure({ ...measure, cost: { usd: 12.34 }, changed: ['cost'] })
  await spies.clock.settle()
  const pane = await mountPane($)
  // inner 54, so each paired cell is 27 wide and truncates beyond it.
  expect(await pane.find({ text: /^cost (?=.*1234\.56).{1,22}$/ })).toBeDefined()
})

test('cost error keeps the pane without a total', async ($, on) => {
  const spies = await startReady($, on, { run: argv => (argv[1] === 'cost' ? ok(fx.error('write_failed')) : defaultRun(argv)) })
  await spies.clock.settle()
  await $.session.measure({ ...measure, cost: { usd: 0.123 }, changed: ['cost'] })
  await spies.clock.settle()
  const pane = await mountPane($)
  expect(await pane.find({ text: /cost \$0\.12 session/ })).toBeDefined()
  expect(await pane.find({ text: /total/ })).toBeUndefined()
  expect(spies.toasts).toEqual([])
})

test('binary missing: no cost run', async ($, on) => {
  const spies = await startReady($, on, { run: () => ({ deny: 'ENOENT: purrgotchi' }) })
  await $.session.measure({ ...measure, cost: { usd: 0.12 }, changed: ['cost'] })
  await spies.clock.settle()
  expect(costs(spies.runs)).toEqual([])
})

test('layout off: no cost run', { options: { layout: 'off' } }, async ($, on) => {
  const spies = await startReady($, on)
  await $.session.measure({ ...measure, cost: { usd: 0.12 }, changed: ['cost'] })
  await spies.clock.settle()
  expect(spies.runs).toEqual([])
})
