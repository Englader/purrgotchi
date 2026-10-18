import { expect, test } from 'claude-code/testing'

import { parseHint } from './neon'
import { ENGINE, T, engineDraws, mountOn, shape } from './testkit'

const Y = '#f3f31b'
const hintTree = (...pairs: [string, string][]) =>
  T(
    { wrap: 'truncate-end' },
    ...pairs.flatMap(([key, word], i) => [...(i === 0 ? [] : ['   ']), T({ color: Y }, `[${key}]`), T({ dimColor: true }, ` ${word}`)]),
  )

// The 2.1.302 probe corpus: [hint, isDraft, isWorking].
const CORPUS: [string, boolean, boolean][] = [
  ['(shift+tab to cycle) · ← for agents', false, false],
  ['(shift+tab to cycle) · ← for agents', true, false],
  ['(shift+tab to cycle)', true, false],
  ['(shift+tab to cycle)', false, false],
  ['! for shell mode', false, false],
  ['(shift+tab to cycle)', false, true],
  ['(shift+tab to cycle) · esc to interrupt', false, true],
  ['(shift+tab to cycle) · esc to interrupt', true, true],
  ['(shift+tab to cycle)', true, true],
  ['(shift+tab to cycle) · esc to interrupt', false, false],
]

test('idle hint draws shortcuts', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'PromptHint', { isDraft: false, isWorking: false, hint: '? for shortcuts' })
  expect(shape(await ui.drawn())).toEqual(hintTree(['?', 'SHORTCUTS']))
})

test('two parts keep their order with a gap', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'PromptHint', { isDraft: false, isWorking: false, hint: 'esc to clear ? for shortcuts' })
  expect(shape(await ui.drawn())).toEqual(hintTree(['ESC', 'CLEAR'], ['?', 'SHORTCUTS']))
})

test('every mapped corpus hint draws, every other corpus hint passes', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'PromptHint', { isDraft: false, isWorking: false, hint: '' })
  for (const [hint, isDraft, isWorking] of CORPUS) {
    ui.redraw({ isDraft, isWorking, hint })
    const parts = parseHint(hint)
    const want = isDraft && isWorking ? ENGINE : parts === null ? ENGINE : hintTree(...parts.map((p): [string, string] => [p.key, p.word]))
    expect(shape(await ui.drawn())).toEqual(want)
  }
})

test('the agents corpus hint draws its two pairs', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'PromptHint', { isDraft: false, isWorking: false, hint: '(shift+tab to cycle) · ← for agents' })
  expect(shape(await ui.drawn())).toEqual(hintTree(['SHIFT+TAB', 'CYCLE'], ['←', 'AGENTS']))
})

test('mixed known and unknown parts pass', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'PromptHint', { isDraft: false, isWorking: false, hint: '? for shortcuts 2 background tasks' })
  expect(shape(await ui.drawn())).toEqual(ENGINE)
})

test('tail or draft while working passes', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'PromptHint', { isDraft: false, isWorking: false, hint: '? for shortcuts', tail: 'x' })
  expect(shape(await ui.drawn())).toEqual(ENGINE)
  ui.redraw({ isDraft: true, isWorking: true, hint: 'esc to interrupt' })
  expect(shape(await ui.drawn())).toEqual(ENGINE)
})

test('prompt hint passes on desktop', async ($, on) => {
  engineDraws(on)
  const ui = await mountOn($, 'PromptHint', { isDraft: false, isWorking: false, hint: '? for shortcuts' }, 'desktop')
  expect(shape(await ui.drawn())).toEqual(ENGINE)
})
