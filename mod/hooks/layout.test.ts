import { expect, test } from 'claude-code/testing'

import { cmd, engine, fx, ok, START, startReady } from './testkit'

const ticks = (runs: string[][]) => runs.filter(a => a[1] === 'frame' && a.includes('--width'))

test('/purr-layout bottom persists through config.set', async ($, on) => {
  const spies = await startReady($, on)
  const r = await $.command.run(cmd('purr-layout', 'bottom'))
  expect(spies.configs).toEqual([{ key: 'purrgotchi.layout', value: 'bottom' }])
  expect(r.text).toBe('Layout set to bottom.')
  expect(r.exitCode).toBeUndefined()
})

test('/purr-layout trims and accepts every option', async ($, on) => {
  const spies = await startReady($, on)
  for (const v of ['dock', 'bottom', 'classic', 'off']) await $.command.run(cmd('purr-layout', `  ${v} `))
  expect(spies.configs.map(c => c.value)).toEqual(['dock', 'bottom', 'classic', 'off'])
})

test('an empty argument lists the options', async ($, on) => {
  const spies = await startReady($, on)
  const r = await $.command.run(cmd('purr-layout'))
  expect(r.text).toBe('Layout is dock. Options: dock, bottom, classic, off.')
  expect(r.exitCode).toBeUndefined()
  expect(spies.configs).toEqual([])
})

test('an invalid layout gets the usage line and exitCode 1', async ($, on) => {
  const spies = await startReady($, on)
  const r = await $.command.run(cmd('purr-layout', 'sideways'))
  expect(r.text).toBe('Layout is dock. Options: dock, bottom, classic, off.')
  expect(r.exitCode).toBe(1)
  expect(spies.configs).toEqual([])
})

test('a refused config.set reports why with exitCode 1', async ($, on) => {
  await startReady($, on, { config: { deny: 'locked by policy' } })
  const r = await $.command.run(cmd('purr-layout', 'bottom'))
  expect(r.text).toBe("Couldn't change layout: locked by policy")
  expect(r.exitCode).toBe(1)
})

test('purr-layout works with a missing binary', async ($, on) => {
  const spies = engine(on, { run: () => ({ deny: 'ENOENT: purrgotchi' }) })
  await $.session.start(START)
  const r = await $.command.run(cmd('purr-layout', 'classic'))
  expect(r.text).toBe('Layout set to classic.')
  expect(spies.configs).toHaveLength(1)
})

test('bottom reports itself as the current layout', { options: { layout: 'bottom' } }, async ($, on) => {
  await startReady($, on)
  const r = await $.command.run(cmd('purr-layout'))
  expect(r.text).toBe('Layout is bottom. Options: dock, bottom, classic, off.')
})

test('off layout still registers purr-layout and nothing else', { options: { layout: 'off' } }, async ($, on) => {
  const spies = engine(on)
  await $.session.start(START)
  expect(spies.registered).toEqual(['purr-layout'])
  expect(spies.runs).toEqual([])
  expect(spies.opens).toEqual([])
  expect(spies.statuses).toEqual([])
  const r = await $.command.run(cmd('purr-layout'))
  expect(r.text).toBe('Layout is off. Options: dock, bottom, classic, off.')
  const set = await $.command.run(cmd('purr-layout', 'dock'))
  expect(set.text).toBe('Layout set to dock.')
  expect(spies.configs).toEqual([{ key: 'purrgotchi.layout', value: 'dock' }])
  expect(spies.runs).toEqual([])
})

test('every other layout registers purr-layout too', async ($, on) => {
  const spies = await startReady($, on)
  expect(spies.registered).toContain('purr-layout')
})

test('classic ticks at 5 s and never auto-opens', { options: { layout: 'classic' } }, async ($, on) => {
  const spies = await startReady($, on, {
    run: argv => (argv[1] === 'frame' ? ok(fx.frame()) : ok(fx.feed())),
  })
  await spies.clock.settle()
  expect(spies.opens).toEqual([])
  const before = ticks(spies.runs).length
  await spies.clock.advance(4000)
  expect(ticks(spies.runs)).toHaveLength(before)
  await spies.clock.advance(1000)
  expect(ticks(spies.runs)).toHaveLength(before + 1)
})
