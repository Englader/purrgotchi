import { expect, test } from 'claude-code/testing'

import { expandHome, locateBinary } from './locate'
import { fakeHost } from './testkit'

const FALLBACK = '/home/u/.claude/purrgotchi/bin/purrgotchi'
const onPath = () => ({ exitCode: 0, stdout: 'purrgotchi 4.1.0\n' })

test('PURR_BIN wins when it exists', async () => {
  const $ = fakeHost({ env: { PURR_BIN: '/opt/p', HOME: '/home/u' }, exists: ['/opt/p', FALLBACK], run: onPath })
  expect(await locateBinary($)).toBe('/opt/p')
})

test('a PURR_BIN that does not exist falls through to PATH', async () => {
  const $ = fakeHost({ env: { PURR_BIN: '/nope', HOME: '/home/u' }, run: onPath })
  expect(await locateBinary($)).toBe('purrgotchi')
})

test('PATH needs a purrgotchi --version answer', async () => {
  const bad = fakeHost({ env: { HOME: '/home/u' }, exists: [FALLBACK], run: () => ({ exitCode: 0, stdout: 'other 1.0' }) })
  expect(await locateBinary(bad)).toBe(FALLBACK)
  const failing = fakeHost({ env: { HOME: '/home/u' }, exists: [FALLBACK], run: () => ({ exitCode: 2, stdout: 'purrgotchi 4.1.0' }) })
  expect(await locateBinary(failing)).toBe(FALLBACK)
  const throws = fakeHost({ env: { HOME: '/home/u' }, exists: [FALLBACK], run: () => 'throw' })
  expect(await locateBinary(throws)).toBe(FALLBACK)
})

test('the home fallback is used when PATH has none', async () => {
  expect(await locateBinary(fakeHost({ env: { HOME: '/home/u/' }, exists: [FALLBACK] }))).toBe(FALLBACK)
})

test('null when nothing is found', async () => {
  expect(await locateBinary(fakeHost({ env: { HOME: '/home/u' } }))).toBeNull()
  expect(await locateBinary(fakeHost({}))).toBeNull()
})

test('a leading ~ in PURR_BIN expands with HOME', async () => {
  const $ = fakeHost({ env: { PURR_BIN: '~/x', HOME: '/home/u' }, exists: ['/home/u/x'] })
  expect(await locateBinary($)).toBe('/home/u/x')
})

test('expandHome table', () => {
  expect(expandHome('~', '/home/u')).toBe('/home/u')
  expect(expandHome('~/bin/purr', '/home/u/')).toBe('/home/u/bin/purr')
  expect(expandHome('~\\bin\\purr.exe', 'C:\\Users\\u')).toBe('C:\\Users\\u\\bin\\purr.exe')
  expect(expandHome('~other/bin/purr', '/home/u')).toBe('~other/bin/purr')
  expect(expandHome('/opt/~/purr', '/home/u')).toBe('/opt/~/purr')
  expect(expandHome('~/bin/purr', undefined)).toBe('~/bin/purr')
  expect(expandHome('~/bin/purr', '')).toBe('~/bin/purr')
})
