import { expect, test } from 'claude-code/testing'

import { spinnerWord } from './neon'
import { ENGINE, engineDraws, mountOn, shape } from './testkit'

const SPIN = { word: 'Sautéing', message: null, suffix: '…', mode: 'responding' }
const last = (seen: { props: unknown[] }) => seen.props[seen.props.length - 1]

test('spinner rewrites only the word', async ($, on) => {
  const seen = engineDraws(on)
  const ui = await mountOn($, 'Spinner', SPIN)
  expect(last(seen)).toEqual({ ...SPIN, word: spinnerWord('Sautéing') })
  expect(shape(await ui.drawn())).toEqual(ENGINE)
})

test('spinner keeps an engine message', async ($, on) => {
  const seen = engineDraws(on)
  const props = { ...SPIN, message: 'Compacting conversation' }
  await mountOn($, 'Spinner', props)
  expect(last(seen)).toEqual(props)
})

test('spinner passes on desktop', async ($, on) => {
  const seen = engineDraws(on)
  await mountOn($, 'Spinner', SPIN, 'desktop')
  expect(last(seen)).toEqual(SPIN)
})

test("a new turn's word gives a new pick", async ($, on) => {
  const seen = engineDraws(on)
  const ui = await mountOn($, 'Spinner', SPIN)
  await ui.redraw({ ...SPIN, word: 'Reticulating' } as never)
  expect((last(seen) as { word: string }).word).toBe(spinnerWord('Reticulating'))
})

test('an empty engine word still gets a deterministic rewrite', async ($, on) => {
  const seen = engineDraws(on)
  await mountOn($, 'Spinner', { ...SPIN, word: '' })
  expect(last(seen)).toEqual({ ...SPIN, word: spinnerWord('') })
})
