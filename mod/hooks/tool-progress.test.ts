import { expect, test } from 'claude-code/testing'

import { ENGINE, T, engineDraws, mountOn, shape } from './testkit'

const Y = '#f3f31b'
const pill = (key: string) =>
  T({ wrap: 'truncate-end' }, '     ', T({ color: Y }, `[${key}]`), T({ dimColor: true }, ' SEND TO BACKGROUND'))
const props = (hint: string) => ({ tool_use_id: 't1', kind: 'background_hint' as const, hint })

test('tool progress draws the background key', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'ToolProgress', props('(ctrl+b to run in background)'))
  expect(shape(await ui.drawn())).toEqual(pill('CTRL+B'))
})

test('tool progress keeps a rebound key', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'ToolProgress', props('(alt+b to run in background)'))
  expect(shape(await ui.drawn())).toEqual(pill('ALT+B'))
})

test('unknown or empty progress passes', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'ToolProgress', props(''))
  expect(shape(await ui.drawn())).toEqual(ENGINE)
  ui.redraw(props('working…'))
  expect(shape(await ui.drawn())).toEqual(ENGINE)
})

test('tool progress passes on desktop', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'ToolProgress', props('(ctrl+b to run in background)'), 'desktop')
  expect(shape(await ui.drawn())).toEqual(ENGINE)
})
