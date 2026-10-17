import { expect, test } from 'claude-code/testing'
import type { Engine } from 'claude-code/testing'
import type { On, ProcessRunInit } from 'claude-code'

import { QUIP_PROBABILITY, quipDraw } from './quips'
import { defaultRun, fx, gate, modelText, modelUnanswered, ok, startReady } from './testkit'
import type { CompleteAnswer, EngineOptions } from './testkit'

const SESSION_MODEL = { options: { quips: 'session-model' } } as const

const beneath = (on: On) => {
  on('prompt.submit', ($, e) => ({ text: e.text }))
  on('turn.complete', ($, e) => ({ text: e.answer }))
}

type Done = { runs: string[][]; inits: Map<string, ProcessRunInit | undefined> }

/** Records each run's init by verb; `quip-prompt` and `quip-store` answer the fixtures (or `store`). */
function binary(store: string = fx.quipStore()): { run: NonNullable<EngineOptions['run']> } & Pick<Done, 'inits'> {
  const inits: Done['inits'] = new Map()
  const run = (argv: string[], init?: ProcessRunInit) => {
    const verb = argv[1] ?? ''
    inits.set(verb, init)
    if (verb === 'quip-prompt') return ok(fx.quipPrompt())
    if (verb === 'quip-store') return ok(store, store.includes('"ok":false') ? 1 : 0)
    return defaultRun(argv)
  }
  return { run, inits }
}


/** Turn ids the gate lets through / holds back (the draw is a pure function of the id). */
const idWhere = (pick: (draw: number) => boolean): string => {
  for (let i = 0; i < 1000; i++) if (pick(quipDraw(`turn-${i}`))) return `turn-${i}`
  throw new Error('no turn id found')
}
const FIRE = idWhere(d => d < QUIP_PROBABILITY)
const SKIP = idWhere(d => d >= QUIP_PROBABILITY)

const answer = { answer: 'Fixed it.', durationMs: 1, isAborted: false, turnId: FIRE, reason: 'answer' } as const

const verbs = (runs: string[][], verb: string) => runs.filter(a => a[1] === verb)

async function turnWith($: Engine, extra: Record<string, unknown> = {}) {
  await $.prompt.submit({ text: 'fix the bug', wait: false, origin: { kind: 'composer' } })
  await $.turn.complete({ ...answer, ...extra })
}

test('full quip path', SESSION_MODEL, async ($, on) => {
  beneath(on)
  const b = binary()
  const spies = await startReady($, on, { run: b.run, complete: () => modelText('  mrrp! nice fix  ') })
  await spies.clock.settle()
  await turnWith($)
  await spies.clock.settle()
  expect(JSON.parse(b.inits.get('quip-prompt')?.stdin ?? 'null')).toEqual({ last_user: 'fix the bug', last_assistant: 'Fixed it.' })
  expect(spies.completes).toHaveLength(1)
  expect(spies.completes[0]).toMatchObject({ model: 'haiku', system: 'You are a kitten.', prompt: 'Say a quip.', maxTokens: 60, timeoutMs: 15000 })
  expect(verbs(spies.runs, 'quip-store')).toEqual([['purrgotchi', 'quip-store', 'mrrp! nice fix', '--json']])
  expect(b.inits.get('quip-store')?.stdin).toBeUndefined()
  expect(spies.toasts).toEqual([])
})

test('a stored quip pokes the frame', SESSION_MODEL, async ($, on) => {
  beneath(on)
  const b = binary()
  const spies = await startReady($, on, { run: b.run, complete: () => modelText('mrrp') })
  await spies.clock.settle()
  const before = spies.runs.filter(a => a[1] === 'frame').length
  await turnWith($)
  await spies.clock.settle()
  expect(spies.runs.filter(a => a[1] === 'frame').length).toBeGreaterThan(before)
})

test('dice skip', SESSION_MODEL, async ($, on) => {
  beneath(on)
  const b = binary()
  const spies = await startReady($, on, { run: b.run, complete: () => modelText('mrrp') })
  await turnWith($, { turnId: SKIP })
  await spies.clock.settle()
  expect(verbs(spies.runs, 'quip-prompt')).toEqual([])
  expect(spies.completes).toEqual([])
})

for (const [name, extra] of [
  ['subagent turn skipped', { agentId: 'a1' }],
  ['aborted skipped', { isAborted: true, reason: 'aborted' }],
  ['refusal skipped', { reason: 'refusal', refusal: { category: null, explanation: null } }],
  ['error skipped', { reason: 'error' }],
  ['empty answer skipped', { answer: '' }],
] as const) {
  test(name, SESSION_MODEL, async ($, on) => {
    beneath(on)
    const b = binary()
    const spies = await startReady($, on, { run: b.run, complete: () => modelText('mrrp') })
    await turnWith($, extra)
    await spies.clock.settle()
    expect(verbs(spies.runs, 'quip-prompt')).toEqual([])
    expect(spies.completes).toEqual([])
  })
}

const stored = (answerOf: CompleteAnswer, name: string) =>
  test(name, SESSION_MODEL, async ($, on) => {
    beneath(on)
    const b = binary()
    const spies = await startReady($, on, { run: b.run, complete: () => answerOf })
    await turnWith($)
    await spies.clock.settle()
    expect(spies.completes).toHaveLength(1)
    expect(verbs(spies.runs, 'quip-store')).toEqual([])
    expect(spies.toasts).toEqual([])
  })

stored(modelUnanswered, 'unanswered model: no store')
stored(modelText('   \n '), 'empty reply: no store')
stored({ value: { isAnswered: false, reason: 'aborted', usage: { input_tokens: 0, output_tokens: 0, cache_read_input_tokens: 0, cache_creation_input_tokens: 0 } } }, 'aborted model call: no store')

test('rejected model call is silent', SESSION_MODEL, async ($, on) => {
  beneath(on)
  const b = binary()
  const spies = await startReady($, on, {
    run: b.run,
    complete: () => {
      throw new Error('engine refuses to send')
    },
  })
  await turnWith($)
  await spies.clock.settle()
  expect(verbs(spies.runs, 'quip-store')).toEqual([])
  expect(spies.toasts).toEqual([])
})

test('long reply truncated to 500', SESSION_MODEL, async ($, on) => {
  beneath(on)
  const b = binary()
  const spies = await startReady($, on, { run: b.run, complete: () => modelText('x'.repeat(900)) })
  await turnWith($)
  await spies.clock.settle()
  const store = verbs(spies.runs, 'quip-store')
  expect(store).toHaveLength(1)
  expect(store[0]?.[2]).toHaveLength(500)
})

test('single flight', SESSION_MODEL, async ($, on) => {
  beneath(on)
  const b = binary()
  const g = gate()
  const spies = await startReady($, on, {
    run: b.run,
    complete: async () => {
      await g.wait
      return modelText('mrrp')
    },
  })
  await turnWith($)
  await spies.clock.settle()
  await turnWith($)
  await spies.clock.settle()
  expect(verbs(spies.runs, 'quip-prompt')).toHaveLength(1)
  g.release()
  await spies.clock.settle()
  expect(verbs(spies.runs, 'quip-store')).toHaveLength(1)
  // free again once the first is over
  await turnWith($)
  await spies.clock.settle()
  expect(verbs(spies.runs, 'quip-prompt')).toHaveLength(2)
})

for (const quips of ['own-key', 'off'] as const) {
  test(`${quips} registers no quip hooks`, { options: { quips } }, async ($, on) => {
    beneath(on)
    const b = binary()
    const spies = await startReady($, on, { run: b.run, complete: () => modelText('mrrp') })
    await turnWith($)
    await spies.clock.settle()
    expect(verbs(spies.runs, 'quip-prompt')).toEqual([])
    expect(spies.completes).toEqual([])
  })
}

test('quip-store empty_quip error is silent', SESSION_MODEL, async ($, on) => {
  beneath(on)
  const b = binary(fx.error('empty_quip'))
  const spies = await startReady($, on, { run: b.run, complete: () => modelText('\u0007') })
  await turnWith($)
  await spies.clock.settle()
  expect(verbs(spies.runs, 'quip-store')).toHaveLength(1)
  expect(spies.toasts).toEqual([])
})

test('missing binary: no model call', SESSION_MODEL, async ($, on) => {
  beneath(on)
  const spies = await startReady($, on, { run: () => ({ deny: 'ENOENT: purrgotchi' }), complete: () => modelText('mrrp') })
  await turnWith($)
  await spies.clock.settle()
  expect(spies.completes).toEqual([])
})

test('prompt hook never changes the prompt', SESSION_MODEL, async ($, on) => {
  beneath(on)
  const spies = await startReady($, on)
  expect(await $.prompt.submit({ text: 'hello', wait: false, origin: { kind: 'composer' } })).toEqual({ text: 'hello' })
  await spies.clock.settle()
})
