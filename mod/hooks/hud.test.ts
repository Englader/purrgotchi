import { expect, test } from 'claude-code/testing'

import { ago, bar, duration, kTokens, resetsIn, shortCwd, statusText } from './hud'
import { fx } from './testkit'
import type { FrameJson } from '../types'

test('bar: 10 segments, clamped, NaN as 0', () => {
  expect(bar(0)).toBe('▱▱▱▱▱▱▱▱▱▱')
  expect(bar(4)).toBe('▱▱▱▱▱▱▱▱▱▱')
  expect(bar(5)).toBe('▰▱▱▱▱▱▱▱▱▱')
  expect(bar(42)).toBe('▰▰▰▰▱▱▱▱▱▱')
  expect(bar(100)).toBe('▰▰▰▰▰▰▰▰▰▰')
  expect(bar(250)).toBe('▰▰▰▰▰▰▰▰▰▰')
  expect(bar(-3)).toBe('▱▱▱▱▱▱▱▱▱▱')
  expect(bar(Number.NaN)).toBe('▱▱▱▱▱▱▱▱▱▱')
})

test('ago', () => {
  expect(ago(-5)).toBe('just now')
  expect(ago(0)).toBe('0s ago')
  expect(ago(12)).toBe('12s ago')
  expect(ago(59)).toBe('59s ago')
  expect(ago(60)).toBe('1m ago')
  expect(ago(3599)).toBe('59m ago')
  expect(ago(3600)).toBe('1h ago')
  expect(ago(90000)).toBe('1d ago')
})

test('duration', () => {
  expect(duration(0)).toBe('0m 0s')
  expect(duration(2_700_000)).toBe('45m 0s')
  expect(duration(7_500_000)).toBe('2h 05m')
  expect(duration(3 * 86_400_000 + 4 * 3_600_000)).toBe('3d 4h')
})

test('resetsIn', () => {
  const now = Date.parse('2026-10-17T12:00:00Z')
  const at = (ms: number) => new Date(now + ms).toISOString()
  expect(resetsIn(undefined, now)).toBeNull()
  expect(resetsIn('garbage', now)).toBeNull()
  expect(resetsIn(at(-60_000), now)).toBe('<1m')
  expect(resetsIn(at(30_000), now)).toBe('<1m')
  expect(resetsIn(at(37 * 60_000), now)).toBe('37m')
  expect(resetsIn(at(2 * 3_600_000), now)).toBe('2h 00m')
  expect(resetsIn(at(4 * 86_400_000 + 15 * 3_600_000), now)).toBe('4d 15h')
})

test('kTokens', () => {
  expect(kTokens(null)).toBeNull()
  expect(kTokens(87_000)).toBe('87k')
  expect(kTokens(1_200_000)).toBe('1.2M')
  expect(kTokens(Number.NaN)).toBeNull()
})

test('shortCwd: last two segments', () => {
  expect(shortCwd(null)).toBeNull()
  expect(shortCwd('/home/u/purrgotchi')).toBe('u/purrgotchi')
  expect(shortCwd('C:\\Users\\u\\purrgotchi')).toBe('u/purrgotchi')
})

test('statusText', () => {
  expect(statusText(JSON.parse(fx.frame()) as FrameJson)).toBe('=^.^= thalia lv12 · 340¢ · calm')
  expect(statusText(null)).toBeUndefined()
})
