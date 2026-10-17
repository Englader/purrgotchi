import { expect, test } from 'claude-code/testing'

import { gameEnabled, readOptions } from './options'

test('readOptions({}) returns all defaults', () => {
  expect(readOptions({})).toEqual({
    binaryPath: '',
    layout: 'dock',
    restyle: false,
    openPaneOnStart: true,
    toastStreaks: false,
    quips: 'own-key',
    devCommands: false,
  })
})

test('readOptions falls back on wrong types and unknown options', () => {
  const o = readOptions({ layout: 'sideways', quips: 7, restyle: 'yes' })
  expect(o.layout).toBe('dock')
  expect(o.quips).toBe('own-key')
  expect(o.restyle).toBe(false)
})

test('readOptions keeps valid values', () => {
  const o = readOptions({ binaryPath: '/opt/p', layout: 'off' })
  expect(o.binaryPath).toBe('/opt/p')
  expect(o.layout).toBe('off')
})

test('gameEnabled is false only for layout off', () => {
  expect(gameEnabled(readOptions({ layout: 'off' }))).toBe(false)
  for (const layout of ['dock', 'bottom', 'classic'] as const) {
    expect(gameEnabled(readOptions({ layout }))).toBe(true)
  }
})
