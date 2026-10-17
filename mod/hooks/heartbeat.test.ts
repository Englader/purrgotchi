import { expect, test } from 'claude-code/testing'
import type { Engine } from 'claude-code/testing'

import { B } from './band'
import { moodBucket } from './heartbeat'
import { START, bandFrame, bandHello, engine } from './testkit'
import type { EngineOptions } from './testkit'

const BIN = '/opt/purr/purrgotchi'
const STATE = '/tmp/state'
const ENV = { env: { PURR_BIN: BIN }, exists: [BIN] } as const
const props = (bodyColumns: number) =>
  ({ hasSurvey: false, isWorking: false, maxRows: 40, bodyColumns, scroll: { offset: 0, bodyRows: 40 }, view: {} }) as const
const mount = ($: Engine) =>
  $.ui.mount({ plugin: 'purrgotchi', surface: 'terminal', component: 'AbovePrompt', requestId: 'above-prompt', props: props(115) })
const beats = (writes: { path: string; text: string }[]) => writes.filter(w => /\.alive$/.test(w.path))

/** A band whose child says hello and sends one frame, then stays silent. */
async function live($: Engine, on: Parameters<typeof engine>[0], o: EngineOptions = {}) {
  const spies = engine(on, { ...ENV, spawn: () => ({ steps: [bandHello(115, 5, STATE) + bandFrame(1)] }), ...o })
  await $.session.start(START)
  await mount($)
  await spies.clock.advance(100)
  return spies
}

test('moodBucket edges', () => {
  expect([40, 40.4, 40.6, 80, 81].map(moodBucket)).toEqual([0, 0, 1, 1, 2])
})

test('heartbeat writes epoch ms per session while blits succeed', async ($, on) => {
  const spies = await live($, on)
  await spies.clock.advance(1000)
  const w = beats(spies.writes)
  expect(w.length).toBeGreaterThan(0)
  expect(w[0]?.path).toBe(`${STATE}/band-sess-1.alive`)
  expect(w[0]?.text).toMatch(/^[1-9][0-9]*$/)
})

test('heartbeat stops when blits are denied', async ($, on) => {
  const spies = await live($, on, { blit: n => (n === 1 ? {} : { deny: 'no' }) })
  await spies.clock.advance(1000)
  expect(beats(spies.writes).length).toBeGreaterThan(0)
  await spies.clock.advance(3000)
  const n = beats(spies.writes).length
  await spies.clock.advance(3000)
  expect(beats(spies.writes)).toHaveLength(n)
})

test('heartbeat follows a new session id after clear', async ($, on) => {
  let id = 'sess-1'
  const spies = await live($, on, { sessionId: () => id })
  await spies.clock.advance(1000)
  id = 'sess-2'
  await spies.clock.advance(1000)
  const paths = beats(spies.writes).map(w => w.path)
  expect(paths).toContain(`${STATE}/band-sess-1.alive`)
  expect(paths.at(-1)).toBe(`${STATE}/band-sess-2.alive`)
})

test('an unreadable or invalid session id falls back to band.alive', async ($, on) => {
  let id = '../evil'
  const spies = await live($, on, { sessionId: () => id })
  await spies.clock.advance(1000)
  id = ''
  await spies.clock.advance(1000)
  expect(new Set(beats(spies.writes).map(w => w.path))).toEqual(new Set([`${STATE}/band.alive`]))
})

test('no state dir no heartbeat', async ($, on) => {
  const spies = engine(on, { ...ENV, spawn: () => ({}) })
  await $.session.start(START)
  await mount($)
  await spies.clock.advance(3000)
  expect(B.stateDir).toBeNull()
  expect(beats(spies.writes)).toEqual([])
})

test('crossing 40 percent respawns once with context-pct 41', async ($, on) => {
  let pct = 10
  const spies = await live($, on, { contextPercent: () => pct })
  await spies.clock.advance(1500)
  expect(spies.spawns).toHaveLength(1)
  pct = 41
  await spies.clock.advance(1000) // a tick sees it
  await spies.clock.advance(400) // debounce
  expect(spies.spawns).toHaveLength(2)
  expect(spies.spawns[1]?.at(-1)).toBe('41')
  pct = 60 // same bucket: no respawn
  await spies.clock.advance(3000)
  expect(spies.spawns).toHaveLength(2)
})
