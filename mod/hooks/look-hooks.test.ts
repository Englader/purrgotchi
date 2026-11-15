import { expect, test } from 'claude-code/testing'
import type { Engine } from 'claude-code/testing'
import type { On, ThemeSetArgs } from 'claude-code'

import { SURFACES } from './look'
import { SCENE_ALT, sceneSvg } from './scenes'
import { START, engine, shape } from './testkit'

type Seen = { sets: ThemeSetArgs[]; tried: ThemeSetArgs[]; clears: number; commands: string[] }

/** The engine beneath: records the theme ops; `refuse` names the picture files the app won't take. */
function world(on: On, refuse: readonly string[] = []): Seen {
  engine(on)
  const seen: Seen = { sets: [], tried: [], clears: 0, commands: [] }
  on('theme.set', ($, e) => {
    const args = structuredClone(e) as ThemeSetArgs
    seen.tried.push(args)
    const image = (args.backdrop as { image?: { source: string } } | undefined)?.image?.source
    if (image !== undefined && refuse.includes(image)) throw new Error(`no picture at ${image}`)
    seen.sets.push(args)
    return { value: { isDrawn: false, reason: 'test' } } as never
  })
  on('theme.clear', () => {
    seen.clears++
    return { value: { isDrawn: false, reason: 'test' } } as never
  })
  on('command.register', ($, e) => {
    seen.commands.push((e as { name: string }).name)
    return { value: {} } as never
  })
  return seen
}

const dark = (a: ThemeSetArgs | undefined) => (a?.colors as { inputs: { dark: Record<string, string> } } | undefined)?.inputs.dark
const last = (seen: Seen) => seen.sets[seen.sets.length - 1]
const run = ($: Engine, args: string) => $.command.run({ command: 'purr-look', args } as never) as Promise<{ text?: string }>
const backdrop = ($: Engine, surface: string, props: object) =>
  $.ui.mount({ plugin: 'purrgotchi', surface, component: 'Backdrop', requestId: 'backdrop', props } as never)
const SCENE = { scene: 'purr-grid', plugin: 'purrgotchi', glass: false }

test('start applies the default look', async ($, on) => {
  const seen = world(on)
  await $.session.start(START)
  expect(seen.sets.length).toBe(1)
  expect(last(seen)?.backdrop).toEqual({ scene: 'purr-grid', glass: false })
  expect(dark(last(seen))?.['gray-900']).toBe(SURFACES.grid[0])
  expect(seen.commands).toContain('purr-look')
})

test('start applies the configured look', { options: { desktopLook: 'violet', desktopBackdrop: 'city' } }, async ($, on) => {
  const seen = world(on)
  await $.session.start(START)
  expect(seen.sets.length).toBe(1)
  expect(last(seen)?.backdrop).toEqual({ scene: 'purr-city', glass: false })
  expect(dark(last(seen))?.['gray-900']).toBe(SURFACES.violet[0])
})

test('no backdrop sets colours alone', { options: { desktopBackdrop: 'none' } }, async ($, on) => {
  const seen = world(on)
  await $.session.start(START)
  expect(seen.sets.length).toBe(1)
  expect(last(seen)?.backdrop).toBeUndefined()
})

test('off at start touches nothing', { options: { desktopLook: 'off' } }, async ($, on) => {
  const seen = world(on)
  await $.session.start(START)
  expect(seen.tried.length).toBe(0)
  expect(seen.clears).toBe(0)
})

test('image uses the jpg', { options: { desktopBackdrop: 'image' } }, async ($, on) => {
  const seen = world(on)
  await $.session.start(START)
  expect(seen.sets.length).toBe(1)
  expect(last(seen)?.backdrop).toEqual({ image: { source: 'backdrops/custom.jpg' } })
})

test('image uses the png when the jpg is refused', { options: { desktopBackdrop: 'image' } }, async ($, on) => {
  const seen = world(on, ['backdrops/custom.jpg'])
  await $.session.start(START)
  expect(seen.sets.length).toBe(1)
  expect(last(seen)?.backdrop).toEqual({ image: { source: 'backdrops/custom.png' } })
})

test('image falls back to the grid scene', { options: { desktopLook: 'violet', desktopBackdrop: 'image' } }, async ($, on) => {
  const seen = world(on, ['backdrops/custom.jpg', 'backdrops/custom.png'])
  await $.session.start(START)
  expect(seen.tried.length).toBe(3)
  expect(seen.sets.length).toBe(1)
  expect(last(seen)?.backdrop).toEqual({ scene: 'purr-grid', glass: false })
  expect(dark(last(seen))?.['gray-900']).toBe(SURFACES.violet[0])
})

test('purr-look violet applies violet with the configured backdrop', { options: { desktopBackdrop: 'city' } }, async ($, on) => {
  const seen = world(on)
  await $.session.start(START)
  const r = await run($, 'violet')
  expect(r.text).toBe('Desktop look: Neon Grid Violet.')
  expect(seen.sets.length).toBe(2)
  expect(dark(last(seen))?.['gray-900']).toBe(SURFACES.violet[0])
  expect(last(seen)?.backdrop).toEqual({ scene: 'purr-city', glass: false })
})

test('purr-look off clears', async ($, on) => {
  const seen = world(on)
  await $.session.start(START)
  const r = await run($, ' OFF ')
  expect(r.text).toBe('Desktop look off.')
  expect(seen.clears).toBe(1)
  expect(seen.sets.length).toBe(1)
})

test('purr-look with a bad argument changes nothing', async ($, on) => {
  const seen = world(on)
  await $.session.start(START)
  for (const arg of ['', 'pink', 'grid violet']) {
    const r = await run($, arg)
    expect(r.text).toBe('Usage: /purr-look grid|violet|off (now: grid)')
  }
  expect(seen.sets.length).toBe(1)
  expect(seen.clears).toBe(0)
})

for (const surface of ['desktop', 'mobile'] as const) {
  test(`backdrop draws on ${surface}`, { options: { desktopLook: 'violet' } }, async ($, on) => {
    world(on)
    await $.session.start(START)
    const ui = await backdrop($, surface, SCENE)
    expect(shape(await ui.drawn())).toEqual({
      type: 'Svg',
      props: { source: sceneSvg('purr-grid', 'violet'), alt: SCENE_ALT['purr-grid'], isInteractive: true },
      children: [],
    })
  })
}

/** What the kit's engine draws beneath the plugin. */
const ENGINE_TEXT = { type: 'Text', props: {}, children: [''] }
const passes = (name: string, surface: string, props: object, options: Record<string, string> = {}) =>
  test(name, { options }, async ($, on) => {
    world(on)
    await $.session.start(START)
    const ui = await backdrop($, surface, props)
    expect(shape(await ui.drawn())).toEqual(ENGINE_TEXT)
  })

passes('backdrop passes on the terminal', 'terminal', SCENE)
passes("another plugin's scene passes", 'desktop', { ...SCENE, plugin: 'tide' })
passes('an unknown scene passes', 'desktop', { ...SCENE, scene: 'lagoon' })
