import { expect, test } from 'claude-code/testing'
import type { On } from 'claude-code'

import { FeedQueue, MAX_FEED_COUNT } from './feed'
import { defaultRun, engine, feeds, fx, gate, ok, START, startReady } from './testkit'
import type { OpAnswer } from './testkit'

// The kit loads the plugin as its own module instance, so a test cannot reach
// the live Runtime; everything is asserted through what the binary was asked.

const read = { tool: 'Read', file_path: '/x' } as const
const count = (n: number) => ['purrgotchi', 'feed', '--json', '--count', String(n)]
const beneath = (on: On) => on('tool.call', () => ({ result: 'file text' }))
const end = { reason: 'prompt_input_exit', sessionId: 's1', resume: { id: 's1' } } as const
const busy = () => ok(fx.error('state_unavailable', 'state lock busy'), 1)

/** The handshake as usual; the feeds answered in turn by `script`, then plain. */
const scripted =
  (...script: (() => OpAnswer)[]) =>
  (argv: string[]): OpAnswer =>
    argv[1] === 'feed' ? (script.shift() ?? (() => ok(fx.feed())))() : defaultRun(argv)

test('passes the tool result through unchanged', async ($, on) => {
  beneath(on)
  await startReady($, on)
  expect(await $.tool.call(read)).toMatchObject({ result: 'file text' })
})

test('hanging binary never delays the tool result', async ($, on) => {
  const g = gate()
  beneath(on)
  const spies = await startReady($, on, {
    run: scripted(async () => {
      await g.wait
      return ok(fx.feed())
    }),
  })
  expect(await $.tool.call(read)).toMatchObject({ result: 'file text' })
  expect(feeds(spies.runs)).toEqual([count(1)])
  g.release()
  await spies.clock.settle()
})

test('feeds once per call with count 1', async ($, on) => {
  beneath(on)
  const spies = await startReady($, on)
  await $.tool.call(read)
  await spies.clock.settle()
  expect(feeds(spies.runs)).toEqual([count(1)])
  await spies.clock.advance(2000) // nothing left for the clock
  expect(feeds(spies.runs)).toEqual([count(1)])
})

test('coalesces a burst into one feed --count N', async ($, on) => {
  const g = gate()
  beneath(on)
  const spies = await startReady($, on, {
    run: scripted(async () => {
      await g.wait
      return ok(fx.feed())
    }),
  })
  await Promise.all(Array.from({ length: 5 }, () => $.tool.call(read)))
  await spies.clock.settle()
  expect(feeds(spies.runs)).toEqual([count(1)])
  g.release()
  await spies.clock.settle()
  expect(feeds(spies.runs)).toEqual([count(1), count(4)])
})

test('requeues on state_unavailable', async ($, on) => {
  beneath(on)
  const spies = await startReady($, on, { run: scripted(busy) })
  await $.tool.call(read)
  await spies.clock.settle()
  expect(feeds(spies.runs)).toEqual([count(1)]) // no tight retry loop
  await spies.clock.advance(2000)
  expect(feeds(spies.runs)).toEqual([count(1), count(1)])
  await spies.clock.advance(2000) // credited once, not twice
  expect(feeds(spies.runs)).toEqual([count(1), count(1)])
})

test('drops a batch after a second rejection', async ($, on) => {
  beneath(on)
  const spies = await startReady($, on, {
    run: argv => (argv[1] === 'feed' ? { deny: 'timed out' } : defaultRun(argv)),
  })
  await $.tool.call(read)
  await spies.clock.settle()
  await spies.clock.advance(2000)
  await spies.clock.advance(2000)
  await spies.clock.advance(2000)
  expect(feeds(spies.runs)).toEqual([count(1), count(1)])
  expect(spies.toasts).toEqual([])
})

test('no spawn when status is missing', async ($, on) => {
  beneath(on)
  const spies = await startReady($, on, { run: () => ({ deny: 'ENOENT: purrgotchi' }) })
  for (let i = 0; i < 3; i++) expect(await $.tool.call(read)).toMatchObject({ result: 'file text' })
  await spies.clock.advance(2000)
  expect(spies.runs).toEqual([['purrgotchi', 'frame', '--json']])
})

test('layout off registers no tool.call hook', { options: { layout: 'off' } }, async ($, on) => {
  beneath(on)
  const spies = engine(on)
  await $.session.start(START)
  expect(await $.tool.call(read)).toMatchObject({ result: 'file text' })
  await spies.clock.advance(2000)
  expect(spies.runs).toEqual([])
})

test('session.end flushes pending', async ($, on) => {
  const g = gate()
  beneath(on)
  const spies = await startReady($, on, {
    run: scripted(async () => {
      await g.wait
      return ok(fx.feed())
    }),
  })
  await $.tool.call(read)
  await $.tool.call(read)
  await $.tool.call(read)
  const ended = $.session.end(end)
  g.release()
  expect(await ended).toEqual({ sessionId: 's1' })
  expect(feeds(spies.runs)).toEqual([count(1), count(2)])
})

test('session.end runs the engine end step without waiting for the flush', async ($, on) => {
  const g = gate()
  beneath(on)
  const spies = await startReady($, on, {
    run: scripted(async () => {
      await g.wait
      return ok(fx.feed())
    }),
  })
  await $.tool.call(read) // in flight, held by the gate
  const ended = $.session.end(end)
  await spies.clock.settle()
  expect(spies.ends).toEqual(['prompt_input_exit']) // not starved by the feed
  g.release()
  expect(await ended).toEqual({ sessionId: 's1' })
})

test('a denied tool call is not counted', async ($, on) => {
  on('tool.call', () => ({ deny: 'refused' }))
  const spies = await startReady($, on)
  expect(await $.tool.call(read)).toMatchObject({ deny: 'refused' })
  await spies.clock.advance(2000)
  expect(feeds(spies.runs)).toEqual([])
})

test('session.end flushes credits no kick has sent', async ($, on) => {
  beneath(on)
  const spies = await startReady($, on, { run: scripted(busy) })
  await $.tool.call(read) // kick meets a busy lock: 1 credit left pending
  await spies.clock.settle()
  await $.session.end(end)
  expect(feeds(spies.runs)).toEqual([count(1), count(1)])
})

test('safety clock flushes without a kick', async ($, on) => {
  beneath(on)
  const spies = await startReady($, on, { run: scripted(busy, busy) })
  await $.tool.call(read)
  await spies.clock.settle()
  await $.tool.call(read)
  await spies.clock.settle()
  expect(feeds(spies.runs)).toEqual([count(1), count(2)]) // both kicks met a busy lock
  await spies.clock.advance(2000)
  expect(feeds(spies.runs)).toEqual([count(1), count(2), count(2)])
})

test('FeedQueue saturates at MAX_FEED_COUNT', () => {
  const q = new FeedQueue()
  q.add(MAX_FEED_COUNT - 1)
  q.add(5)
  expect(q.pending).toBe(MAX_FEED_COUNT)
  expect(q.take()).toBe(MAX_FEED_COUNT)
  expect(q.pending).toBe(0)
})
