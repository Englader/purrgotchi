import { expect, test } from 'claude-code/testing'

import { parseFps, parseRows, readOptions } from './options'
import { fakeHost } from './testkit'

test('parseRows accepts 3, 4 and 6 and defaults everything else to 4', () => {
  expect(parseRows('3')).toBe(3)
  expect(parseRows('6')).toBe(6)
  expect(parseRows(undefined)).toBe(4)
  expect(parseRows('5')).toBe(4)
  expect(parseRows(' 4 ')).toBe(4)
  expect(parseRows('')).toBe(4)
  expect(parseRows(' 3 ')).toBe(3)
})

test('parseFps defaults to 30 and clamps to 15..60', () => {
  expect(parseFps(undefined)).toBe(30)
  expect(parseFps('60')).toBe(60)
  expect(parseFps('90')).toBe(60)
  expect(parseFps('5')).toBe(15)
  expect(parseFps('abc')).toBe(30)
  expect(parseFps('')).toBe(30)
  expect(parseFps('24.6')).toBe(25)
})

test('readOptions reads both env names', async () => {
  const $ = fakeHost({ env: { PURR_BAND_ROWS: '6', PURR_BAND_FPS: '20' } })
  expect(await readOptions($)).toEqual({ rows: 6, fps: 20 })
  expect(await readOptions(fakeHost({}))).toEqual({ rows: 4, fps: 30 })
})
