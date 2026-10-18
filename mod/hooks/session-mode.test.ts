import { expect, test } from 'claude-code/testing'

import { ENGINE, T, engineDraws, mountOn, shape } from './testkit'

const M = '#ea00d9'
const C = '#0abdc6'
const mode = (label: string) => [T({ color: M }, '⟦ '), T({ color: C }, label), T({ color: M }, ' ⟧')]

test('modes are framed and all kept', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'SessionMode', { modes: ['focus', 'memory paused'] })
  expect(shape(await ui.drawn())).toEqual(T({ wrap: 'truncate-end' }, ...mode('FOCUS'), ' ', ...mode('MEMORY PAUSED')))
})

test('no modes passes', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'SessionMode', { modes: [] })
  expect(shape(await ui.drawn())).toEqual(ENGINE)
})

test('session mode passes on desktop', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'SessionMode', { modes: ['focus'] }, 'desktop')
  expect(shape(await ui.drawn())).toEqual(ENGINE)
})

test('trees truncate, never wrap (modes)', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'SessionMode', { modes: Array.from({ length: 12 }, (_, i) => `mode number ${i}`) })
  expect(((await ui.drawn()) as { props: { wrap?: string } }).props.wrap).toBe('truncate-end')
})
