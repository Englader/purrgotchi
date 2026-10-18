import { expect, test } from 'claude-code/testing'

import { SPINNER_WORDS, durationText, parseBackgroundHint, parseHint, spinnerWord, turnText } from './neon'

test('spinnerWord is framed, ALL-CAPS and stable per engine word', () => {
  const w = spinnerWord('Sautéing')
  expect(w).toMatch(/^▓▒░ [A-Z][A-Z +]* ░▒▓$/)
  expect(spinnerWord('Sautéing')).toBe(w)
  expect(spinnerWord('')).toMatch(/^▓▒░ [A-Z][A-Z +]* ░▒▓$/)
})

test('spinnerWord rotates across engine words', () => {
  const words = ['Baking', 'Brewing', 'Sautéing', 'Calculating', 'Booping', 'Ruminating', 'Scampering', 'Reticulating']
  expect(new Set(words.map(spinnerWord)).size).toBeGreaterThanOrEqual(4)
})

test('every spinner word is ALL-CAPS', () => {
  for (const w of SPINNER_WORDS) expect(w).toMatch(/^[A-Z][A-Z ]*$/)
})

test('durationText edges', () => {
  const cases: [number, string][] = [
    [0, '0.0s'], [3_200, '3.2s'], [59_999, '59.9s'], [60_000, '1m 0s'],
    [64_000, '1m 4s'], [3_723_000, '1h 2m'], [-5, '0.0s'], [NaN, '0.0s'],
  ]
  for (const [ms, text] of cases) expect(durationText(ms)).toBe(text)
  expect(turnText(3_200)).toBe('EXECUTED IN 3.2s')
})

test('parseHint maps known parts', () => {
  const cases: [string, { key: string; word: string }[]][] = [
    ['? for shortcuts', [{ key: '?', word: 'SHORTCUTS' }]],
    ['esc to clear', [{ key: 'ESC', word: 'CLEAR' }]],
    ['! for shell mode', [{ key: '!', word: 'SHELL MODE' }]],
    ['esc to interrupt and send', [{ key: 'ESC', word: 'ABORT + SEND' }]],
    // 2.1.302 probe corpus
    ['(shift+tab to cycle)', [{ key: 'SHIFT+TAB', word: 'CYCLE' }]],
    ['(shift+tab to cycle) · ← for agents', [{ key: 'SHIFT+TAB', word: 'CYCLE' }, { key: '←', word: 'AGENTS' }]],
    ['(shift+tab to cycle) · esc to interrupt', [{ key: 'SHIFT+TAB', word: 'CYCLE' }, { key: 'ESC', word: 'ABORT' }]],
  ]
  for (const [hint, want] of cases) expect(parseHint(hint)).toEqual(want)
})

test("parseHint keeps the engine's key", () => {
  expect(parseHint('ctrl+g to interrupt')).toEqual([{ key: 'CTRL+G', word: 'ABORT' }])
})

test('parseHint passes the unknown', () => {
  for (const h of ['', '   ', '2 background tasks', '? for shortcuts 2 background tasks', 'esc to interrupt now'])
    expect(parseHint(h)).toBeNull()
})

test('parseBackgroundHint', () => {
  expect(parseBackgroundHint('(ctrl+b to run in background)')).toEqual({ key: 'CTRL+B', word: 'SEND TO BACKGROUND' })
  expect(parseBackgroundHint('(alt+b to run in background)')?.key).toBe('ALT+B')
  expect(parseBackgroundHint('')).toBeNull()
  expect(parseBackgroundHint('ctrl+b to run in background')).toBeNull()
})
