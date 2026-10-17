import { expect, test } from 'claude-code/testing'

import { celebrationToast } from './celebrate'
import { defaultRun, fx, ok, startReady, withVersion } from './testkit'
import type { OpAnswer } from './testkit'

const ctx = { name: 'Thalia', level: 13, total: 1234, toastStreaks: true }
const toast = (kind: string | null, c = ctx) => celebrationToast(kind, c)

test('celebrationToast: the full table', () => {
  expect(toast('LevelUp')).toEqual({ text: 'Thalia reached level 13 ✦', notify: false })
  expect(toast('EvolvingYoung')).toEqual({ text: 'Thalia evolved into a young cat!', notify: true })
  expect(toast('EvolvingAdult')).toEqual({ text: 'Thalia evolved into an adult cat!', notify: true })
  expect(toast('MilestoneCenturion')).toEqual({ text: '100 tool calls together — Centurion', notify: false })
  expect(toast('MilestoneVeteran')).toEqual({ text: '500 tool calls together — Veteran', notify: false })
  expect(toast('MilestoneMaster')).toEqual({ text: '1000 tool calls together — Master', notify: false })
  expect(toast('MilestoneLegendary', { ...ctx, total: 3004 })).toEqual({
    text: '3000 tool calls together — Legendary',
    notify: true,
  })
  expect(toast('Streak')).toEqual({ text: '5-call streak — Thalia is on fire', notify: false })
  expect(toast('RandomDrop')).toEqual({ text: 'Thalia found +10¢', notify: false })
  expect(toast('Greeting')).toEqual({ text: 'Welcome back — Thalia missed you', notify: false })
})

test('celebrationToast: streaks only when enabled', () => {
  expect(toast('Streak', { ...ctx, toastStreaks: false })).toBeNull()
})

test('celebrationToast: null, None and unknown kinds are silent', () => {
  expect(toast(null)).toBeNull()
  expect(toast('None')).toBeNull()
  expect(toast('Fireworks')).toBeNull()
})

test('celebrationToast: an unknown name reads "Your kitten"', () => {
  expect(toast('LevelUp', { ...ctx, name: '' })?.text).toBe('Your kitten reached level 13 ✦')
})

const ev = { tool: 'Read', file_path: '/x' } as const

/** The handshake as usual; every feed answers `feed`. */
const feedWith =
  (feed: string) =>
  (argv: string[]): OpAnswer =>
    argv[1] === 'feed' ? ok(feed) : defaultRun(argv)

test('feed LevelUp toasts with the cached name', async ($, on) => {
  on('tool.call', () => ({ result: 'file text' }))
  const spies = await startReady($, on, { run: feedWith(fx.feed({ celebration: 'LevelUp', level: 13 })) })
  await $.tool.call(ev)
  await spies.clock.settle()
  expect(spies.toasts).toEqual(['Thalia reached level 13 ✦'])
  expect(spies.notifies).toEqual([])
})

test('evolution notifies, and a refused notify does not throw', async ($, on) => {
  on('tool.call', () => ({ result: 'file text' }))
  const spies = await startReady($, on, {
    run: feedWith(fx.feed({ celebration: 'EvolvingYoung' })),
    notify: 'deny',
  })
  await $.tool.call(ev)
  await spies.clock.settle()
  expect(spies.toasts).toEqual(['Thalia evolved into a young cat!'])
  expect(spies.notifies).toEqual(['Thalia evolved into a young cat!'])
})

test('turn Greeting toasts', async ($, on) => {
  on('classic.UserPromptSubmit', () => ({}))
  const spies = await startReady($, on, {
    run: argv => (argv[1] === 'turn' ? ok(fx.turn({ celebration: 'Greeting' })) : defaultRun(argv)),
  })
  await $.classic.UserPromptSubmit({ prompt: 'hi', session_id: 's1', transcript_path: '/tmp/s1.jsonl' })
  await spies.clock.settle()
  expect(spies.toasts).toEqual(['Welcome back — Thalia missed you'])
})

test('null celebration: no toast', async ($, on) => {
  on('tool.call', () => ({ result: 'file text' }))
  const spies = await startReady($, on)
  await $.tool.call(ev)
  await spies.clock.settle()
  expect(spies.toasts).toEqual([])
})

/** `no_kitten` for the handshake's frame, then the new kitten's frame. */
const hatching = (feedAnswers: string[]) => {
  let frames = 0
  return (argv: string[]): OpAnswer => {
    if (argv.includes('--version')) return withVersion(ok(''))(argv)
    if (argv[1] === 'frame') return frames++ === 0 ? ok(fx.noKitten, 1) : ok(fx.frame({ name: 'Mochi', rarity: 'common', breed: 'Tabby' }))
    return ok(feedAnswers.shift() ?? fx.feed())
  }
}

test('first hatch toast replaces the celebration, once', async ($, on) => {
  on('tool.call', () => ({ result: 'file text' }))
  const spies = await startReady($, on, { run: hatching([fx.feed({ celebration: 'RandomDrop' })]) })
  await $.tool.call(ev)
  await spies.clock.settle()
  expect(spies.toasts).toEqual(['Mochi hatched — a common Tabby!'])
  await $.tool.call(ev)
  await spies.clock.settle()
  expect(spies.runs.filter(a => a[1] === 'frame')).toHaveLength(2) // handshake + one hatch frame
})

test('a busy lock at the handshake is not "no kitten": no hatch toast', async ($, on) => {
  on('tool.call', () => ({ result: 'file text' }))
  const spies = await startReady($, on, {
    run: argv =>
      argv[1] === 'frame' ? ok(fx.error('state_unavailable', 'state lock busy'), 1) : argv[1] === 'feed' ? ok(fx.feed()) : defaultRun(argv),
  })
  await $.tool.call(ev)
  await spies.clock.settle()
  expect(spies.toasts).toEqual([])
  expect(spies.runs.filter(a => a[1] === 'frame')).toHaveLength(1)
})

test('an unknown kitten still toasts the level the feed reports', async ($, on) => {
  on('tool.call', () => ({ result: 'file text' }))
  const spies = await startReady($, on, {
    run: argv =>
      argv[1] === 'frame'
        ? ok(fx.error('state_unavailable', 'state lock busy'), 1)
        : argv[1] === 'feed'
          ? ok(fx.feed({ celebration: 'LevelUp', level: 13 }))
          : defaultRun(argv),
  })
  await $.tool.call(ev)
  await spies.clock.settle()
  expect(spies.toasts).toEqual(['Your kitten reached level 13 ✦'])
})
