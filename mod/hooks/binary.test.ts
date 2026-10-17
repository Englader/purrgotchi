import { expect, test } from 'claude-code/testing'
import type { ProcessRunInit } from 'claude-code'

import { expandHome, handshake, isError, resolveBinary, runJson, Serial } from './binary'
import { readOptions } from './options'
import { fx, gate, host, ok, withVersion } from './testkit'
import type { FeedJson } from '../types'

// binary.ts is unit-tested through `host(...)`, a plugin-side `$.process.run`
// answered with the same `{ value } | { deny }` op answers `engine(on)` takes.
// session.test.ts covers the same code through the engine's own `$`.

test('resolveBinary prefers binaryPath', () => {
  expect(resolveBinary(readOptions({ binaryPath: ' /opt/p ' }))).toBe('/opt/p')
  expect(resolveBinary(readOptions({}))).toBe('purrgotchi')
  expect(resolveBinary(readOptions({ binaryPath: '   ' }))).toBe('purrgotchi')
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

test('runJson parses one line', async () => {
  const seen: string[][] = []
  const h = host(argv => {
    seen.push(argv)
    return ok(fx.feed())
  })
  const out = await runJson<FeedJson>(h, 'purrgotchi', ['feed', '--json'])
  expect(out.kind).toBe('json')
  if (out.kind !== 'json' || isError(out.value)) throw new Error('expected feed json')
  expect(typeof out.value.total_tool_calls).toBe('number')
  expect(seen).toEqual([['purrgotchi', 'feed', '--json']])
})

test('runJson keeps JSON on exit 1', async () => {
  const out = await runJson(host(() => ok(fx.noKitten, 1)), 'purrgotchi', ['frame', '--json'])
  expect(out).toMatchObject({ kind: 'json', exitCode: 1 })
  if (out.kind !== 'json') throw new Error('expected json')
  expect(isError(out.value)).toBe(true)
})

test('runJson maps rejection', async () => {
  const out = await runJson(host(() => ({ deny: 'ENOENT' })), 'purrgotchi', ['feed', '--json'])
  expect(out).toMatchObject({ kind: 'failed', reason: 'rejected', detail: 'ENOENT' })
})

test('runJson maps garbage', async () => {
  const out = await runJson(host(() => ok('kitten renamed to: x\n')), 'purrgotchi', ['rename', 'x'])
  expect(out).toMatchObject({ kind: 'failed', reason: 'not-json', exitCode: 0 })
})

test('runJson maps v:2', async () => {
  const out = await runJson(host(() => ok('{"v":2,"ok":true}')), 'purrgotchi', ['feed', '--json'])
  expect(out).toMatchObject({ kind: 'failed', reason: 'wrong-version' })
})

test('runJson passes stdin, env, timeout', async () => {
  const inits: (ProcessRunInit | undefined)[] = []
  const h = host((argv, init) => {
    inits.push(init)
    return ok(fx.turn())
  })
  await runJson(h, 'purrgotchi', ['turn', '--json'], { stdin: 'S', env: { A: '' }, timeoutMs: 5000 })
  expect(inits).toEqual([{ stdin: 'S', env: { A: '' }, timeoutMs: 5000 }])
})

test('handshake ready with frame', async () => {
  const h = host(withVersion(ok(fx.frame())))
  const { status, frame } = await handshake(h, 'purrgotchi')
  expect(status).toEqual({ kind: 'ready', version: '3.8.0' })
  expect(frame?.name).toBe('Thalia')
})

test('no_kitten is ready', async () => {
  const h = host(withVersion(ok(fx.noKitten, 1)))
  const { status, frame } = await handshake(h, 'purrgotchi')
  expect(status).toEqual({ kind: 'ready', version: '3.8.0' })
  expect(frame).toBeNull()
})

test('runJson keeps bad_args JSON on exit 2', async () => {
  const out = await runJson(host(() => ok(fx.badArgs('feed: --count must be an integer'), 2)), 'purrgotchi', [
    'feed',
    '--json',
    '--count',
    '0',
  ])
  expect(out).toMatchObject({ kind: 'json', exitCode: 2 })
  if (out.kind !== 'json' || !isError(out.value)) throw new Error('expected error json')
  expect(out.value.error).toBe('bad_args')
  expect(out.value.detail).toBe('feed: --count must be an integer')
})

for (const code of ['state_unavailable', 'state_corrupt'] as const) {
  test(`handshake ${code} is ready without a frame`, async () => {
    const h = host(withVersion(ok(fx.error(code), 1)))
    const { status, frame } = await handshake(h, 'purrgotchi')
    expect(status).toEqual({ kind: 'ready', version: '3.8.0' })
    expect(frame).toBeNull()
  })
}

test('handshake bad_args is outdated', async () => {
  const h = host(withVersion(ok(fx.badArgs('frame: ?'), 2)))
  const { status } = await handshake(h, 'purrgotchi')
  expect(status).toEqual({ kind: 'outdated', version: '3.8.0' })
})

test('handshake missing', async () => {
  const seen: string[][] = []
  const h = host(argv => {
    seen.push(argv)
    return { deny: 'ENOENT: purrgotchi' }
  })
  const { status, frame } = await handshake(h, 'purrgotchi')
  expect(status).toMatchObject({ kind: 'missing' })
  expect(frame).toBeNull()
  expect(seen).toEqual([['purrgotchi', 'frame', '--json']])
})

test('handshake outdated (3.8 usage)', async () => {
  const h = host(withVersion(ok('', 2, 'Usage: purrgotchi <command>\n'), '3.7.0'))
  const { status, frame } = await handshake(h, 'purrgotchi')
  expect(status).toEqual({ kind: 'outdated', version: '3.7.0' })
  expect(frame).toBeNull()
})

test('Serial runs in order and survives rejection', async () => {
  const serial = new Serial()
  const done: number[] = []
  const g = gate()
  const slow = serial.run(async () => {
    await g.wait
    done.push(1)
    return 1
  })
  const failing = serial
    .run(async () => {
      done.push(2)
      throw new Error('boom')
    })
    .then(
      () => 'resolved',
      (e: unknown) => (e instanceof Error ? e.message : 'non-error'),
    )
  const fast = serial.run(async () => {
    done.push(3)
    return 3
  })
  expect(done).toEqual([])
  g.release()
  expect(await slow).toBe(1)
  expect(await failing).toBe('boom')
  expect(await fast).toBe(3)
  expect(done).toEqual([1, 2, 3])
})
