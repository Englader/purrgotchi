import { expect, test } from 'claude-code/testing'

import { splitLines } from './stream'

test('splitLines: two lines in one chunk give the second', () => {
  expect(splitLines('', 'a\nb\n')).toEqual({ rest: '', lines: ['a', 'b'], last: 'b' })
})

test('splitLines: a split line is carried, then completed', () => {
  const first = splitLines('', 'a\nbb')
  expect(first).toEqual({ rest: 'bb', lines: ['a'], last: 'a' })
  const half = splitLines('', '{"v":1')
  expect(half).toEqual({ rest: '{"v":1', lines: [], last: null })
  expect(splitLines(half.rest, '}\n')).toEqual({ rest: '', lines: ['{"v":1}'], last: '{"v":1}' })
})

test('splitLines: empty lines are skipped', () => {
  expect(splitLines('', '\n\n \nx\n')).toEqual({ rest: '', lines: ['x'], last: 'x' })
})
