import { expect, test } from 'claude-code/testing'

import { PALETTE, SPINNER_WORDS, THEME, durationText, parseBackgroundHint, parseHint, spinnerWord, turnText, type PaletteName } from './neon'

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

test('parseHint rejects prose as a key', () => {
  expect(parseHint('Unable to find')).toBeNull()
  expect(parseHint('Unable to find · esc to interrupt')).toBeNull()
  expect(parseBackgroundHint('(Unable to run in background)')).toBeNull()
})

test('parseHint accepts key-shaped tokens', () => {
  expect(parseHint('Ctrl+X to stop')).toEqual([{ key: 'CTRL+X', word: 'STOP' }])
  expect(parseHint('ctrl+shift+k to clear')).toEqual([{ key: 'CTRL+SHIFT+K', word: 'CLEAR' }])
  expect(parseHint('pagedown to find')).toEqual([{ key: 'PAGEDOWN', word: 'FIND' }])
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

const ENGINE_THEME_KEYS = ['autoAccept', 'autoAcceptShimmer', 'skill', 'bashBorder', 'claude', 'claudeShimmer', 'claudeBlue_FOR_SYSTEM_SPINNER', 'claudeBlueShimmer_FOR_SYSTEM_SPINNER', 'permission', 'permissionShimmer', 'planMode', 'ide', 'promptBorder', 'promptBorderShimmer', 'text', 'inverseText', 'inactive', 'inactiveShimmer', 'subtle', 'suggestion', 'remember', 'background', 'success', 'error', 'warning', 'merged', 'warningShimmer', 'diffAdded', 'diffRemoved', 'diffAddedDimmed', 'diffRemovedDimmed', 'diffAddedWord', 'diffRemovedWord', 'red_FOR_SUBAGENTS_ONLY', 'blue_FOR_SUBAGENTS_ONLY', 'green_FOR_SUBAGENTS_ONLY', 'yellow_FOR_SUBAGENTS_ONLY', 'purple_FOR_SUBAGENTS_ONLY', 'orange_FOR_SUBAGENTS_ONLY', 'pink_FOR_SUBAGENTS_ONLY', 'cyan_FOR_SUBAGENTS_ONLY', 'professionalBlue', 'chromeYellow', 'clawd_body', 'clawd_background', 'userMessageBackground', 'userMessageBackgroundHover', 'composerSidebarBackground', 'selectionBg', 'bashMessageBackgroundColor', 'memoryBackgroundColor', 'rate_limit_fill', 'rate_limit_empty', 'fastMode', 'fastModeShimmer', 'effortUltra', 'briefLabelYou', 'briefLabelClaude', 'rainbow_red', 'rainbow_orange', 'rainbow_yellow', 'rainbow_green', 'rainbow_blue', 'rainbow_indigo', 'rainbow_violet', 'rainbow_red_shimmer', 'rainbow_orange_shimmer', 'rainbow_yellow_shimmer', 'rainbow_green_shimmer', 'rainbow_blue_shimmer', 'rainbow_indigo_shimmer', 'rainbow_violet_shimmer']

const channel = (c: number): number => {
  const s = c / 255
  return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4
}
const luminance = (hex: string): number => {
  const n = parseInt(hex.slice(1), 16)
  return 0.2126 * channel((n >> 16) & 255) + 0.7152 * channel((n >> 8) & 255) + 0.0722 * channel(n & 255)
}
const contrast = (a: string, b: string): number => {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x)
  return ((hi ?? 0) + 0.05) / ((lo ?? 0) + 0.05)
}

const BG_KEYS = [
  'userMessageBackground', 'userMessageBackgroundHover', 'composerSidebarBackground', 'selectionBg',
  'bashMessageBackgroundColor', 'memoryBackgroundColor', 'diffAdded', 'diffRemoved', 'diffAddedDimmed',
  'diffRemovedDimmed', 'diffAddedWord', 'diffRemovedWord',
]
const EXEMPT = ['inverseText', 'clawd_background', 'rate_limit_empty']
const hex = (k: string): string => PALETTE[THEME[k] as PaletteName]

test('THEME has exactly the 72 engine keys in engine order', () => {
  expect(Object.keys(THEME)).toEqual(ENGINE_THEME_KEYS)
})

test('every THEME value names a palette hex', () => {
  for (const k of Object.keys(THEME)) expect(hex(k)).toMatch(/^#[0-9a-f]{6}$/)
})

test('spec colours are pinned', () => {
  const pins: [string, string][] = [
    ['claude', '#ea00d9'], ['promptBorder', '#ea00d9'], ['text', '#e4dcff'], ['userMessageBackground', '#2a1640'],
    ['diffAdded', '#0f3a2a'], ['diffRemoved', '#4a1020'], ['bashBorder', '#f3f31b'], ['success', '#57eca6'],
    ['error', '#ff5468'],
    // The dark base's own value: the HUD statusline's dim labels look as before.
    ['inactive', '#999999'],
  ]
  for (const [k, v] of pins) expect(hex(k)).toBe(v)
})

test('foreground keys read on the ground', () => {
  const floor: Record<string, number> = { subtle: 2.0 }
  for (const k of Object.keys(THEME)) {
    if (BG_KEYS.includes(k) || EXEMPT.includes(k)) continue
    expect(contrast(hex(k), PALETTE.ground)).toBeGreaterThanOrEqual(floor[k] ?? 4.5)
  }
})

test('background keys keep text readable', () => {
  for (const k of BG_KEYS) expect(contrast(PALETTE.ice, hex(k))).toBeGreaterThanOrEqual(4.5)
})
