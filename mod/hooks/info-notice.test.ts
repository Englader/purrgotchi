import { expect, test } from 'claude-code/testing'

import { ENGINE, T, engineDraws, mountOn, shape } from './testkit'

const M = '#ea00d9'
const C = '#0abdc6'
const Y = '#f3f31b'
const frame = (text: string, ...rest: unknown[]) => T({ wrap: 'truncate-end' }, T({ color: M }, '▸ '), T({ color: C }, text), ...rest)

test('notice keeps the engine words, uppercased, and its command', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'InfoNotice', { text: 'Using model from settings', command: '/model' })
  expect(shape(await ui.drawn())).toEqual(frame('USING MODEL FROM SETTINGS', ' ', T({ color: Y }, '/model')))
})

test('notice without a command ends at the text', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'InfoNotice', { text: 'Using model from settings', command: null })
  expect(shape(await ui.drawn())).toEqual(frame('USING MODEL FROM SETTINGS'))
})

test('a corpus notice keeps its words', async ($, on) => {
  engineDraws(on)
  const text = 'Debug mode enabled · logging to /tmp/x/probe.debug.log'
  const ui = await mountOn($, 'InfoNotice', { text, command: null })
  expect(shape(await ui.drawn())).toEqual(frame('DEBUG MODE ENABLED · LOGGING TO /tmp/x/probe.debug.log'))
})

test('notice keeps paths in their case', async ($, on) => {
  engineDraws(on)
  const text = 'Wrote C:\\Users\\Me\\a.txt and v1.2.3 to ./Out/File'
  const ui = await mountOn($, 'InfoNotice', { text, command: null })
  expect(shape(await ui.drawn())).toEqual(frame('WROTE C:\\Users\\Me\\a.txt AND v1.2.3 TO ./Out/File'))
})

test('empty notice passes', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'InfoNotice', { text: '', command: null })
  expect(shape(await ui.drawn())).toEqual(ENGINE)
})

test('notice passes on desktop', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'InfoNotice', { text: 'hi', command: null }, 'desktop')
  expect(shape(await ui.drawn())).toEqual(ENGINE)
})

test('trees truncate, never wrap (notice)', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'InfoNotice', { text: 'x'.repeat(300), command: '/model' })
  expect(((await ui.drawn()) as { props: { wrap?: string } }).props.wrap).toBe('truncate-end')
})
