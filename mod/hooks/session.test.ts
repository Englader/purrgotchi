import { expect, test } from 'claude-code/testing'

import { scanSettings } from './session'
import { engine, fx, ok, settingsFx } from './testkit'

const start = { cwd: '/tmp', surface: 'terminal', isInteractive: true } as const

const frames = (runs: string[][]) => runs.filter(a => a.includes('frame')).length
const versions = (runs: string[][]) => runs.filter(a => a.includes('--version')).length

test('scanSettings table', () => {
  const none = { classicHooks: false, classicStatusline: false }
  expect(scanSettings({})).toEqual(none)
  expect(scanSettings({ statusLine: { command: '/x/purrgotchi statusline' } })).toEqual({
    classicHooks: false,
    classicStatusline: true,
  })
  expect(scanSettings(settingsFx.hook('PostToolUse', 'C:/p/purrgotchi.exe feed')).classicHooks).toBe(true)
  expect(scanSettings(settingsFx.hook('PostToolUse', 'C:/p/PurrGotchi.exe feed')).classicHooks).toBe(true)
  expect(scanSettings(settingsFx.hook('UserPromptSubmit', 'purrgotchi turn')).classicHooks).toBe(true)
  expect(scanSettings(settingsFx.hook('PostToolUse', 'other feed')).classicHooks).toBe(false)
  expect(scanSettings(settingsFx.hook('Stop', 'purrgotchi feed')).classicHooks).toBe(false)
  // What install.sh writes (and --uninstall-hooks removes): `"<bin>" feed|turn`.
  expect(scanSettings(settingsFx.hook('PostToolUse', '"/r/rust/target/release/purrgotchi" feed')).classicHooks).toBe(true)
  expect(scanSettings(settingsFx.hook('UserPromptSubmit', '"C:\\p\\purrgotchi.exe" turn')).classicHooks).toBe(true)
  expect(scanSettings(settingsFx.hook('PostToolUse', 'purr feed')).classicHooks).toBe(true)
  // A hook that merely mentions purrgotchi is not double feeding.
  expect(scanSettings(settingsFx.hook('PostToolUse', '/home/x/purrgotchi/scripts/notify.sh')).classicHooks).toBe(false)
  expect(scanSettings(settingsFx.hook('PostToolUse', 'bash /home/x/purrgotchi/feed.sh')).classicHooks).toBe(false)
  expect(scanSettings(settingsFx.hook('PostToolUse', '"/x/purrgotchi" statusline')).classicHooks).toBe(false)
  expect(scanSettings(settingsFx.hook('PostToolUse', '"/x/purrgotchi" turn')).classicHooks).toBe(false)
  expect(scanSettings(settingsFx.hook('UserPromptSubmit', '"/x/purrgotchi" feed')).classicHooks).toBe(false)
  expect(scanSettings(settingsFx.hook('PostToolUse', '"/x/purrgotchi" feeder')).classicHooks).toBe(false)
  expect(scanSettings({ hooks: 'garbage' })).toEqual(none)
  expect(scanSettings({ hooks: { PostToolUse: [null, { hooks: [{}] }] } })).toEqual(none)
  expect(scanSettings({ hooks: { PostToolUse: 'x', UserPromptSubmit: [{ hooks: 'y' }] } })).toEqual(none)
  expect(scanSettings({ statusLine: { command: 7 } })).toEqual(none)
  expect(scanSettings(null)).toEqual(none)
  expect(scanSettings('settings')).toEqual(none)
})

test('missing binary toasts once and marks status', async ($, on) => {
  const spies = engine(on, { run: () => ({ deny: 'ENOENT: purrgotchi' }) })
  expect(await $.session.start(start)).toEqual({ cwd: '/tmp' })
  expect(await $.session.start(start)).toEqual({ cwd: '/tmp' })
  expect(spies.toasts).toHaveLength(1)
  expect(spies.toasts[0]).toContain('Binary path')
  expect(frames(spies.runs)).toBe(2)
  expect(versions(spies.runs)).toBe(0)
})

test('outdated binary toasts with version', async ($, on) => {
  const spies = engine(on, {
    run: argv => (argv.includes('--version') ? ok('purrgotchi 3.7.0\n') : ok('', 2, 'Usage: purrgotchi <command>\n')),
  })
  await $.session.start(start)
  expect(spies.toasts).toHaveLength(1)
  expect(spies.toasts[0]).toContain('3.7.0')
  expect(spies.toasts[0]).toContain('too old')
})

test('ready binary with a hatched kitten does not toast', async ($, on) => {
  const spies = engine(on)
  await $.session.start(start)
  expect(spies.toasts).toEqual([])
  expect(spies.runs).toEqual([
    ['purrgotchi', 'frame', '--json'],
    ['purrgotchi', '--version'],
  ])
})

test('no_kitten counts as ready', async ($, on) => {
  const spies = engine(on, {
    run: argv => (argv.includes('--version') ? ok('purrgotchi 3.8.0\n') : ok(fx.noKitten, 1)),
    settings: settingsFx.hook('PostToolUse', 'purrgotchi feed'),
  })
  await $.session.start(start)
  // ready, so the double-feed warning is the only toast
  expect(spies.toasts).toHaveLength(1)
  expect(spies.toasts[0]).toContain('--uninstall-hooks')
})

test('classic hooks toast only when ready', async ($, on) => {
  const spies = engine(on, { settings: settingsFx.hook('PostToolUse', 'purrgotchi feed') })
  await $.session.start(start)
  await $.session.start(start)
  expect(spies.toasts).toHaveLength(1)
  expect(spies.toasts[0]).toContain('--uninstall-hooks')
})

test('classic hooks with a missing binary give only the missing toast', async ($, on) => {
  const spies = engine(on, {
    run: () => ({ deny: 'ENOENT' }),
    settings: settingsFx.hook('PostToolUse', 'purrgotchi feed'),
  })
  await $.session.start(start)
  expect(spies.toasts).toHaveLength(1)
  expect(spies.toasts[0]).toContain('Binary path')
})

test('statusLine alone does not toast', async ($, on) => {
  const spies = engine(on, { settings: { statusLine: { type: 'command', command: '/x/purrgotchi statusline' } } })
  await $.session.start(start)
  expect(spies.toasts).toEqual([])
})

test('binaryPath option is the argv[0]', { options: { binaryPath: '/opt/purr' } }, async ($, on) => {
  const spies = engine(on)
  await $.session.start(start)
  expect(spies.runs[0]).toEqual(['/opt/purr', 'frame', '--json'])
})

test('layout off never runs the binary', { options: { layout: 'off' } }, async ($, on) => {
  const spies = engine(on)
  expect(await $.session.start(start)).toEqual({ cwd: '/tmp' })
  expect(spies.runs).toEqual([])
  expect(spies.toasts).toEqual([])
})

test('settings.read failure is harmless', async ($, on) => {
  const spies = engine(on, { settings: 'deny' })
  expect(await $.session.start(start)).toEqual({ cwd: '/tmp' })
  expect(spies.toasts).toEqual([])
})
