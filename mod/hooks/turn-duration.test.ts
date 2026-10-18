import { expect, test } from 'claude-code/testing'

import { ENGINE, T, engineDraws, mountOn, shape } from './testkit'

const M = '#ea00d9'
const C = '#0abdc6'

test('turn duration draws the executed line', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'TurnDuration', { word: 'Baked', durationMs: 3200 })
  expect(shape(await ui.drawn())).toEqual(
    T({ wrap: 'truncate-end' }, T({ color: M }, '◢◤ '), T({ color: C, bold: true }, 'EXECUTED IN 3.2s'), T({ color: M }, ' ◢◤')),
  )
})

test('turn duration passes on desktop', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'TurnDuration', { word: 'Baked', durationMs: 3200 }, 'desktop')
  expect(shape(await ui.drawn())).toEqual(ENGINE)
})
