import { expect, test } from 'claude-code/testing'

import { cmd, defaultRun, fx, ok, startReady } from './testkit'
import type { EngineOptions } from './testkit'

// Tests reach the commands only through the engine: `$.command.run` and what
// the binary was asked.

const dev = { options: { devCommands: true } } as const
const SIX = ['purr-pet', 'purr-play', 'purr-treat', 'purr-rename', 'purr-card', 'purr-stats']
const verbRuns = (runs: string[][], verb: string) => runs.filter(a => a[1] === verb)

/** The handshake as usual; every other run answered with `answer`. */
const answering = (answer: string, exitCode = 0): EngineOptions['run'] => argv =>
  argv.includes('frame') || argv.includes('--version') ? defaultRun(argv) : ok(answer, exitCode)

test('registers the six commands', async ($, on) => {
  const spies = await startReady($, on)
  expect(spies.registered).toEqual(SIX)
})

test('devCommands adds purr-hatch', dev, async ($, on) => {
  const spies = await startReady($, on)
  expect(spies.registered).toEqual([...SIX, 'purr-hatch'])
})

test('a refused name does not break start', async ($, on) => {
  const spies = await startReady($, on, { refuse: ['purr-card'], run: answering(fx.action({ message: 'purr' })) })
  expect(spies.registered).toEqual(SIX)
  expect(spies.logs.some(l => l.includes('purr-card'))).toBe(true)
  expect((await $.command.run(cmd('purr-pet'))).text).toBe('purr')
})

test('pet returns the message', async ($, on) => {
  const spies = await startReady($, on, {
    run: answering(fx.action({ result: 'applied', message: 'Thalia purrs. bond 50 → 55' })),
  })
  const r = await $.command.run(cmd('purr-pet'))
  expect(r.text).toBe('Thalia purrs. bond 50 → 55')
  expect(r.exitCode).toBeUndefined()
  expect(verbRuns(spies.runs, 'pet')).toEqual([['purrgotchi', 'pet', '--json']])
})

test('play and treat run their own verbs', async ($, on) => {
  const spies = await startReady($, on, { run: answering(fx.action({ message: 'ok' })) })
  await $.command.run(cmd('purr-play'))
  await $.command.run(cmd('purr-treat'))
  expect(verbRuns(spies.runs, 'play')).toEqual([['purrgotchi', 'play', '--json']])
  expect(verbRuns(spies.runs, 'treat')).toEqual([['purrgotchi', 'treat', '--json']])
})

for (const result of ['cooldown', 'insufficient_coins'] as const) {
  test(`${result} is not an error`, async ($, on) => {
    await startReady($, on, { run: answering(fx.action({ result, message: 'not now — wait 4m' })) })
    const r = await $.command.run(cmd('purr-treat'))
    expect(r.text).toBe('not now — wait 4m')
    expect(r.exitCode).toBeUndefined()
  })
}

test('action error sets exitCode 1', async ($, on) => {
  await startReady($, on, {
    run: answering(fx.action({ ok: false, result: 'error', message: 'pet: could not update state: x' }), 1),
  })
  const r = await $.command.run(cmd('purr-pet'))
  expect(r.text).toBe('pet: could not update state: x')
  expect(r.exitCode).toBe(1)
})

test('an error object (no kitten, bad args) is an error answer', async ($, on) => {
  await startReady($, on, { run: answering(fx.noKitten, 1) })
  const r = await $.command.run(cmd('purr-pet'))
  expect(r.text).toContain('No kitten')
  expect(r.exitCode).toBe(1)
})

test('a binary that fails to answer is unreachable', async ($, on) => {
  await startReady($, on, { run: answering('not json at all') })
  const r = await $.command.run(cmd('purr-pet'))
  expect(r.text).toContain('unreachable')
  expect(r.exitCode).toBe(1)
})

test('binary missing answers not-found', async ($, on) => {
  const spies = await startReady($, on, { run: () => ({ deny: 'ENOENT' }) })
  const r = await $.command.run(cmd('purr-pet'))
  expect(r.text).toContain('Binary path')
  expect(r.exitCode).toBe(1)
  expect(verbRuns(spies.runs, 'pet')).toEqual([])
})

test('rename passes one argv element', async ($, on) => {
  const spies = await startReady($, on, {
    run: answering(fx.action({ result: 'renamed', name: 'x', message: 'kitten renamed' })),
  })
  const r = await $.command.run(cmd('purr-rename', '  Sir "Purr" \\ 😺  '))
  expect(r.text).toBe('kitten renamed')
  expect(verbRuns(spies.runs, 'rename')).toEqual([['purrgotchi', 'rename', 'Sir "Purr" \\ 😺', '--json']])
})

test('rename rejects empty, long and control input without running', async ($, on) => {
  const spies = await startReady($, on)
  const say = async (args: string) => (await $.command.run(cmd('purr-rename', args))).text
  expect(await say('')).toBe('Usage: /purr-rename <name>')
  expect(await say('   ')).toBe('Usage: /purr-rename <name>')
  expect(await say('a'.repeat(33))).toBe('Names are limited to 32 characters.')
  expect(await say('a\u001bb')).toBe("Names can't contain control characters.")
  expect(await say('a\nb')).toBe("Names can't contain control characters.")
  expect(await say('a\u0007b')).toBe("Names can't contain control characters.")
  expect(await say('--json')).toBe("That name is reserved; pick another.")
  expect(verbRuns(spies.runs, 'rename')).toEqual([])
})

test('rename counts characters, not bytes', async ($, on) => {
  const spies = await startReady($, on, { run: answering(fx.action({ result: 'renamed', name: 'x', message: 'ok' })) })
  await $.command.run(cmd('purr-rename', '😺'.repeat(32)))
  expect(verbRuns(spies.runs, 'rename')).toHaveLength(1)
})

test('rename error comes back with exitCode 1', async ($, on) => {
  await startReady($, on, { run: answering(fx.error('bad_input', 'name is not allowed'), 1) })
  const r = await $.command.run(cmd('purr-rename', 'Mochi'))
  expect(r.text).toBe('name is not allowed')
  expect(r.exitCode).toBe(1)
})

test('rename updates the cached name', async ($, on) => {
  const run = (argv: string[]) =>
    argv[1] === 'rename'
      ? ok(fx.action({ result: 'renamed', name: 'Mochi', message: 'kitten renamed to: Mochi' }))
      : argv[1] === 'feed'
        ? ok(fx.feed({ celebration: 'LevelUp', level: 13 }))
        : defaultRun(argv)
  on('tool.call', () => ({ result: 'file text' }))
  const spies = await startReady($, on, { run })
  await $.command.run(cmd('purr-rename', 'Mochi'))
  await $.tool.call({ tool: 'Read', file_path: '/x' })
  await spies.clock.settle()
  expect(spies.toasts).toEqual(['Mochi reached level 13 ✦'])
})

test('hatch parses a quoted breed', dev, async ($, on) => {
  const spies = await startReady($, on, {
    run: argv => (argv[1] === 'hatch' ? ok('hatched\n') : defaultRun(argv)),
  })
  const r = await $.command.run(cmd('purr-hatch', '--rarity rare --breed "Maine Coon" --shiny'))
  expect(verbRuns(spies.runs, 'hatch')).toEqual([
    ['purrgotchi', 'hatch', '--rarity', 'rare', '--breed', 'Maine Coon', '--shiny'],
  ])
  expect(r.text).toBe('Hatched Thalia — rare Maine Coon.')
  expect(r.exitCode).toBeUndefined()
})

test('hatch failure returns the first stderr line', dev, async ($, on) => {
  await startReady($, on, {
    run: argv => (argv[1] === 'hatch' ? ok('', 1, 'unknown breed: Nope\nmore\n') : defaultRun(argv)),
  })
  const r = await $.command.run(cmd('purr-hatch', '--breed Nope'))
  expect(r).toMatchObject({ text: 'unknown breed: Nope', exitCode: 1 })
})

test('hatch rejects unknown flags and bad values without running', dev, async ($, on) => {
  const spies = await startReady($, on)
  for (const args of ['--force', '--rarity', '--rarity mythic', '--breed', '--breed "Maine']) {
    const r = await $.command.run(cmd('purr-hatch', args))
    expect(r.text).toContain('Usage: /purr-hatch')
  }
  expect(verbRuns(spies.runs, 'hatch')).toEqual([])
})

test('hatch with no frame afterwards forgets the old kitten', dev, async ($, on) => {
  let hatched = false
  on('tool.call', () => ({ result: 'file text' }))
  const spies = await startReady($, on, {
    run: argv => {
      if (argv[1] === 'hatch') {
        hatched = true
        return ok('hatched\n')
      }
      if (argv[1] === 'frame' && hatched) return ok(fx.error('state_unavailable', 'state lock busy'), 1)
      if (argv[1] === 'feed') return ok(fx.feed({ celebration: 'LevelUp', level: 2 }))
      return defaultRun(argv)
    },
  })
  expect((await $.command.run(cmd('purr-hatch', ''))).text).toBe('Hatched a new kitten.')
  await $.tool.call({ tool: 'Read', file_path: '/x' })
  await spies.clock.settle()
  expect(spies.toasts).toEqual(['Your kitten reached level 2 ✦'])
})
