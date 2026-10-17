import { expect, test } from 'claude-code/testing'
import type { On, ProcessRunInit } from 'claude-code'

import { turnEnv } from './turn'
import { readOptions } from './options'
import { defaultRun, engine, fx, gate, ok, START, startReady } from './testkit'
import type { OpAnswer } from './testkit'

const prompt = { prompt: 'hi', session_id: 's1', transcript_path: '/tmp/s1.jsonl' } as const
const read = { tool: 'Read', file_path: '/x' } as const
const beneath = (on: On) => {
  on('classic.UserPromptSubmit', () => ({}))
  on('tool.call', () => ({ result: 'file text' }))
}

/** Records each `turn` run's init; answers it with `answer` (default a plain turn). */
function turns(answer: () => OpAnswer = () => ok(fx.turn())) {
  const inits: (ProcessRunInit | undefined)[] = []
  const run = (argv: string[], init?: ProcessRunInit): OpAnswer => {
    if (argv[1] !== 'turn') return defaultRun(argv)
    inits.push(init)
    return answer()
  }
  return { inits, run }
}

test('passes classic stdin through', async ($, on) => {
  beneath(on)
  const t = turns()
  const spies = await startReady($, on, { run: t.run })
  await $.classic.UserPromptSubmit(prompt)
  await spies.clock.settle()
  expect(spies.runs.filter(a => a[1] === 'turn')).toEqual([['purrgotchi', 'turn', '--json']])
  expect(t.inits).toHaveLength(1)
  expect(JSON.parse(t.inits[0]?.stdin ?? 'null')).toMatchObject(prompt)
  expect(t.inits[0]?.timeoutMs).toBe(5000)
})

test('own-key passes no env', async ($, on) => {
  beneath(on)
  const t = turns()
  const spies = await startReady($, on, { run: t.run })
  await $.classic.UserPromptSubmit(prompt)
  await spies.clock.settle()
  expect(t.inits).toHaveLength(1)
  expect(t.inits[0]?.env).toBeUndefined()
})

for (const quips of ['session-model', 'off'] as const) {
  test(`${quips} quips suppress react`, { options: { quips } }, async ($, on) => {
    beneath(on)
    const t = turns()
    const spies = await startReady($, on, { run: t.run })
    await $.classic.UserPromptSubmit(prompt)
    await spies.clock.settle()
    expect(t.inits).toHaveLength(1)
    expect(t.inits[0]?.env).toEqual({ PURR_LLM_PROVIDER: '' })
  })
}

test('turnEnv table', () => {
  expect(turnEnv(readOptions({}))).toBeUndefined()
  expect(turnEnv(readOptions({ quips: 'own-key' }))).toBeUndefined()
  expect(turnEnv(readOptions({ quips: 'session-model' }))).toEqual({ PURR_LLM_PROVIDER: '' })
  expect(turnEnv(readOptions({ quips: 'off' }))).toEqual({ PURR_LLM_PROVIDER: '' })
})

test('flushes pending feeds before turn', async ($, on) => {
  beneath(on)
  const g = gate()
  let first = true
  const spies = await startReady($, on, {
    run: async argv => {
      if (argv[1] === 'feed' && first) {
        first = false
        await g.wait
      }
      return argv[1] === 'turn' ? ok(fx.turn()) : defaultRun(argv)
    },
  })
  await $.tool.call(read) // in flight, held by the gate
  await $.tool.call(read)
  await $.tool.call(read) // 2 pending behind it
  await $.classic.UserPromptSubmit(prompt)
  await spies.clock.settle()
  expect(spies.runs.some(a => a[1] === 'turn')).toBe(false) // turn waits for the feeds
  g.release()
  await spies.clock.settle()
  expect(spies.runs.filter(a => a[1] === 'feed' || a[1] === 'turn')).toEqual([
    ['purrgotchi', 'feed', '--json', '--count', '1'],
    ['purrgotchi', 'feed', '--json', '--count', '2'],
    ['purrgotchi', 'turn', '--json'],
  ])
})

test('hanging turn never blocks the prompt', async ($, on) => {
  beneath(on)
  const t = turns(() => new Promise<never>(() => {}))
  await startReady($, on, { run: t.run })
  expect(await $.classic.UserPromptSubmit(prompt)).toEqual({})
  expect(t.inits).toHaveLength(1)
})

test('missing binary: no turn spawn', async ($, on) => {
  beneath(on)
  const spies = await startReady($, on, { run: () => ({ deny: 'ENOENT: purrgotchi' }) })
  expect(await $.classic.UserPromptSubmit(prompt)).toEqual({})
  await spies.clock.settle()
  expect(spies.runs).toEqual([['purrgotchi', 'frame', '--json']])
})

test('layout off: no turn spawn', { options: { layout: 'off' } }, async ($, on) => {
  beneath(on)
  const spies = engine(on)
  await $.session.start(START)
  expect(await $.classic.UserPromptSubmit(prompt)).toEqual({})
  await spies.clock.settle()
  expect(spies.runs).toEqual([])
})
