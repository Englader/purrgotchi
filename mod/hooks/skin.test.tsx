import { expect, test } from 'claude-code/testing'
import type { Engine } from 'claude-code/testing'
import type { On, RenderPropsOf } from 'claude-code'

import { engine, START } from './testkit'

// The opt-in Netrunner restyle, driven through the engine: a mount of each
// engine component the way a surface draws it. `engine(on)` draws the row
// beneath the plugin (what `next(e)` reaches) as its props' plain text.

const SURFACES = ['terminal', 'desktop', 'mobile'] as const
type Surface = (typeof SURFACES)[number]

const SKIN = { options: { restyle: true } } as const
const SKIN_ONLY = { options: { layout: 'off', restyle: true } } as const

const USER: RenderPropsOf['UserMessage'] = { text: 'hack the planet', origin: { kind: 'composer' }, isExpanded: false }
const SPINNER: RenderPropsOf['Spinner'] = { word: 'Sauteing', message: null, suffix: '…', mode: 'responding' }
const TOOL: RenderPropsOf['ToolUse'] = {
  tool_use_id: 't1',
  tool: 'Read',
  input: { file_path: '/x' },
  isRunning: false,
  isErrored: false,
  isInterrupted: false,
  output: 'line1',
}

let n = 0
const viewport = (columns: number) => ({ columns, rows: 40 })

function mountUser($: Engine, surface: Surface, props: Partial<RenderPropsOf['UserMessage']> = {}) {
  return $.ui.mount({
    plugin: 'purrgotchi',
    surface,
    component: 'UserMessage',
    requestId: `m${++n}`,
    props: { ...USER, ...props },
    viewport: viewport(120),
  })
}

function mountSpinner($: Engine, surface: 'terminal' | 'desktop', props: Partial<RenderPropsOf['Spinner']> = {}) {
  return $.ui.mount({
    plugin: 'purrgotchi',
    surface,
    component: 'Spinner',
    requestId: `s${++n}`,
    props: { ...SPINNER, ...props },
    viewport: viewport(120),
  })
}

function mountTool($: Engine, surface: Surface, props: Partial<RenderPropsOf['ToolUse']> = {}, columns = 120) {
  return $.ui.mount({
    plugin: 'purrgotchi',
    surface,
    component: 'ToolUse',
    requestId: `t${++n}`,
    props: { ...TOOL, ...props },
    viewport: viewport(columns),
  })
}

const read = { tool: 'Read', file_path: '/x' } as const
const prompt = { prompt: 'hi', session_id: 's1', transcript_path: '/tmp/s1.jsonl' } as const
const beneath = (on: On) => {
  on('tool.call', () => ({ result: 'file text' }))
  on('classic.UserPromptSubmit', () => ({}))
}

for (const surface of SURFACES) {
  test(`user message gets the magenta bar on ${surface}`, SKIN, async ($, on) => {
    engine(on)
    const row = await mountUser($, surface)
    expect(await row.find({ text: /▌/ })).toBeDefined()
    expect(await row.find({ text: /hack the planet/ })).toBeDefined()
  })

  test(`expanded user message untouched on ${surface}`, SKIN, async ($, on) => {
    engine(on)
    const row = await mountUser($, surface, { isExpanded: true })
    expect(await row.find({ text: /▌/ })).toBeUndefined()
    expect(await row.find({ text: /hack the planet/ })).toBeDefined()
  })

  test(`tool row keeps its output on ${surface}`, SKIN, async ($, on) => {
    engine(on)
    const row = await mountTool($, surface)
    expect(await row.find({ text: /line1/ })).toBeDefined()
    expect(await row.find({ text: /READ/ })).toBeDefined()
  })

  test(`unknown props fall back on ${surface}`, SKIN, async ($, on) => {
    engine(on)
    const user = await mountUser($, surface, { text: 7 as unknown as string })
    expect(await user.find({ text: /▌/ })).toBeUndefined()
    const tool = await mountTool($, surface, { tool: 7 as unknown as string })
    expect(await tool.find({ text: /READ|TOOL/ })).toBeUndefined()
  })
}

test('agent/task message untouched', SKIN, async ($, on) => {
  engine(on)
  const task = await mountUser($, 'terminal', { origin: { kind: 'task-notification' }, task: { id: 'b1', status: 'completed' } })
  expect(await task.find({ text: /▌/ })).toBeUndefined()
  const peer = await mountUser($, 'terminal', { origin: { kind: 'peer' } })
  expect(await peer.find({ text: /▌/ })).toBeUndefined()
  const from = await mountUser($, 'terminal', { from: { name: 'scout' } })
  expect(await from.find({ text: /▌/ })).toBeUndefined()
})

test('a very long user message keeps the engine row', SKIN, async ($, on) => {
  engine(on)
  const row = await mountUser($, 'desktop', { text: 'x'.repeat(4001) })
  expect(await row.find({ text: /▌/ })).toBeUndefined()
})

test('tool chips are coloured per tool', SKIN, async ($, on) => {
  engine(on)
  for (const [tool, label] of [
    ['Edit', 'EDIT'],
    ['Write', 'WRITE'],
    ['Bash', 'BASH'],
    ['Grep', 'GREP'],
  ] as const) {
    const row = await mountTool($, 'terminal', { tool, output: 'out' })
    expect(await row.find({ text: new RegExp(label) })).toBeDefined()
    expect(await row.find({ text: /out/ })).toBeDefined()
  }
})

test('narrow mobile tool chip is the short form', SKIN, async ($, on) => {
  engine(on)
  const row = await mountTool($, 'mobile', { tool: 'Bash', output: 'ok' }, 32)
  expect(await row.find({ text: /BASH/ })).toBeUndefined()
  expect(await row.find({ text: /^ B $/ })).toBeDefined()
  expect(await row.find({ text: /ok/ })).toBeDefined()
})

for (const surface of ['terminal', 'desktop'] as const) {
  test(`spinner word rewritten on ${surface}`, SKIN, async ($, on) => {
    engine(on)
    const row = await mountSpinner($, surface)
    expect(await row.find({ text: /▓▒░ jacking in/ })).toBeDefined()
    expect(await row.find({ text: /Sauteing/ })).toBeUndefined()
  })
}

test('spinner message override wins', SKIN, async ($, on) => {
  engine(on)
  const row = await mountSpinner($, 'terminal', { message: 'Compacting' })
  expect(await row.find({ text: /Compacting/ })).toBeDefined()
  expect(await row.find({ text: /jacking in/ })).toBeUndefined()
})

test('spinner untouched on mobile', SKIN, async ($, on) => {
  engine(on)
  // Spinner is not raised on mobile; were it mounted there, the engine row draws.
  const row = await $.ui.mount({
    plugin: 'purrgotchi',
    surface: 'mobile',
    component: 'Spinner',
    props: SPINNER,
    viewport: viewport(40),
  })
  expect(await row.find({ text: /jacking in/ })).toBeUndefined()
})

test('restyle off registers no render hooks', async ($, on) => {
  engine(on)
  expect(await (await mountUser($, 'terminal')).find({ text: /▌/ })).toBeUndefined()
  expect(await (await mountSpinner($, 'terminal')).find({ text: /jacking in/ })).toBeUndefined()
  expect(await (await mountTool($, 'terminal')).find({ text: /READ/ })).toBeUndefined()
})

test('skin-only never touches the binary', SKIN_ONLY, async ($, on) => {
  beneath(on)
  const spies = engine(on)
  expect(await $.session.start(START)).toEqual({ cwd: '/tmp' })
  expect(await $.tool.call(read)).toMatchObject({ result: 'file text' })
  expect(await $.classic.UserPromptSubmit(prompt)).toEqual({})
  await spies.clock.advance(5000)
  expect(spies.runs).toEqual([])
  expect(spies.toasts).toEqual([])
  expect(spies.opens).toEqual([])
  for (const surface of SURFACES) {
    expect(await (await mountUser($, surface)).find({ text: /▌/ })).toBeDefined()
  }
})
