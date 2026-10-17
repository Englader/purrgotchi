// Action commands: `/purr-pet`, `/purr-play`, `/purr-treat`, `/purr-rename`,
// `/purr-hatch` (dev). Registered at session start, answered in `command.run`
// by the binary's `--json` verbs. The model reads `text`, so it stays short.
// The engine's module scan needs `$` passed only to plain functions.
import type { EngineInterface, On } from 'claude-code'

import { errText, isError, runJson, runRaw, TIMEOUT } from './binary'
import { kittenOf } from './celebrate'
import type { CelebrateHost } from './celebrate'
import { debugLog, ready } from './runtime'
import type { Runtime } from './runtime'
import type { ActionJson, BinaryStatus, FrameJson } from '../types'

export type CommandName =
  | 'purr-pet'
  | 'purr-play'
  | 'purr-treat'
  | 'purr-rename'
  | 'purr-card'
  | 'purr-stats'
  | 'purr-hatch'

export const COMMANDS: readonly {
  name: CommandName
  description: string
  argumentHint?: string
  dev?: true
}[] = [
  { name: 'purr-pet', description: 'Pet the kitten (+bond)' },
  { name: 'purr-play', description: 'Play with the kitten (starts a playground scene)' },
  { name: 'purr-treat', description: 'Give the kitten a treat (costs coins)' },
  { name: 'purr-rename', description: 'Rename the kitten', argumentHint: '<name>' },
  { name: 'purr-card', description: "Show the kitten's trading card" },
  { name: 'purr-stats', description: "Show the kitten's stats" },
  {
    name: 'purr-hatch',
    description: 'Hatch a new kitten (dev)',
    argumentHint: '[--rarity r] [--breed "B"] [--shiny]',
    dev: true,
  },
]

const UNREACHABLE = 'The kitten is unreachable right now (purrgotchi failed).'
const NAME_MAX = 32
const HATCH_USAGE = 'Usage: /purr-hatch [--rarity common|uncommon|rare|epic|legendary] [--breed "Name"] [--shiny]'
const RARITIES: readonly string[] = ['common', 'uncommon', 'rare', 'epic', 'legendary']

/** What a command says when the binary cannot be used. */
export function NOT_READY_TEXT(status: BinaryStatus): string {
  switch (status.kind) {
    case 'missing':
      return 'Purrgotchi binary not found — set Binary path in /config or run `cargo install --path rust`'
    case 'outdated':
      return `Purrgotchi binary ${status.version} is too old for this mod — rebuild: \`cargo install --path rust\``
    default:
      return 'Purrgotchi is not ready yet — try again in a moment.'
  }
}

/**
 * Registers every command (dev ones only with `devCommands`), each in its own
 * try/catch: a refused name skips that command, never the session start.
 */
export async function registerCommandNames($: Pick<EngineInterface, 'command' | 'ui'>, rt: Runtime): Promise<void> {
  for (const c of COMMANDS) {
    if (c.dev === true && !rt.options.devCommands) continue
    try {
      await $.command.register({
        name: c.name,
        description: c.description,
        ...(c.argumentHint !== undefined && { argumentHint: c.argumentHint }),
      })
    } catch (e) {
      debugLog($, `command /${c.name} refused: ${errText(e)}`)
    }
  }
}

/** Trimmed, 1 to 32 characters, no control characters. */
export function validateName(raw: string): { ok: true; name: string } | { ok: false; text: string } {
  const name = raw.trim()
  if (name === '') return { ok: false, text: 'Usage: /purr-rename <name>' }
  if ([...name].length > NAME_MAX) return { ok: false, text: `Names are limited to ${NAME_MAX} characters.` }
  if (/\p{Cc}/u.test(name)) return { ok: false, text: "Names can't contain control characters." }
  return { ok: true, name }
}

type ActionOutcome = { text: string; isError: boolean; result?: ActionJson }

/** One action verb's `--json` run, through `rt.serial`. Never throws. */
async function runVerb($: CelebrateHost, rt: Runtime, args: readonly string[]): Promise<ActionOutcome> {
  const out = await rt.serial.run(() => runJson<ActionJson>($, rt.bin, args, { timeoutMs: TIMEOUT.command }))
  if (out.kind === 'failed') {
    debugLog($, `${args[0]} failed (${out.reason}): ${out.detail}`)
    return { text: UNREACHABLE, isError: true }
  }
  const v = out.value
  if (isError(v)) return { text: v.message ?? v.detail ?? v.error, isError: true }
  return { text: v.message, isError: !v.ok, result: v }
}

/** `pet`, `play` or `treat`. A cooldown or an empty wallet is a normal answer. */
export function runAction($: CelebrateHost, rt: Runtime, verb: 'pet' | 'play' | 'treat'): Promise<ActionOutcome> {
  return runVerb($, rt, [verb, '--json'])
}

/** Splits on whitespace; a double-quoted run is one token (quotes dropped). */
function tokenize(args: string): string[] | null {
  const tokens: string[] = []
  let cur = ''
  let has = false
  let quoted = false
  for (const ch of args) {
    if (ch === '"') {
      quoted = !quoted
      has = true
    } else if (!quoted && /\s/u.test(ch)) {
      if (has) tokens.push(cur)
      cur = ''
      has = false
    } else {
      cur += ch
      has = true
    }
  }
  if (quoted) return null
  if (has) tokens.push(cur)
  return tokens
}

/** `--rarity r`, `--breed "B"`, `--shiny`; anything else is the usage text. */
export function parseHatchArgs(args: string): { ok: true; argv: string[] } | { ok: false; text: string } {
  const bad = { ok: false, text: HATCH_USAGE } as const
  const tokens = tokenize(args)
  if (tokens === null) return bad
  const argv: string[] = []
  for (let i = 0; i < tokens.length; i++) {
    const t = tokens[i]
    if (t === '--shiny') {
      argv.push(t)
    } else if (t === '--rarity') {
      const v = tokens[++i]?.toLowerCase()
      if (v === undefined || !RARITIES.includes(v)) return bad
      argv.push(t, v)
    } else if (t === '--breed') {
      const v = tokens[++i]
      if (v === undefined || v.trim() === '' || v.startsWith('--')) return bad
      argv.push(t, v)
    } else {
      return bad
    }
  }
  return { ok: true, argv }
}

const reply = (o: ActionOutcome) => ({ text: o.text, ...(o.isError && { exitCode: 1 }) })

async function rename($: CelebrateHost, rt: Runtime, rawName: string) {
  const v = validateName(rawName)
  if (!v.ok) return { text: v.text }
  const o = await runVerb($, rt, ['rename', v.name, '--json'])
  if (!o.isError && o.result?.name !== undefined && rt.kitten !== null) rt.kitten.name = o.result.name
  return reply(o)
}

async function hatch($: CelebrateHost, rt: Runtime, rawArgs: string) {
  const p = parseHatchArgs(rawArgs)
  if (!p.ok) return { text: p.text }
  const r = await rt.serial.run(() => runRaw($, rt.bin, ['hatch', ...p.argv], { timeoutMs: TIMEOUT.command }))
  if ('failed' in r) {
    debugLog($, `hatch failed: ${r.failed}`)
    return { text: UNREACHABLE, exitCode: 1 }
  }
  if (r.exitCode !== 0) {
    const first = (r.stderr.trim() || r.stdout.trim()).split('\n')[0] ?? ''
    return { text: first === '' ? 'Hatch failed.' : first, exitCode: 1 }
  }
  rt.hadNoKitten = false
  const out = await rt.serial.run(() => runJson<FrameJson>($, rt.bin, ['frame', '--json'], { timeoutMs: TIMEOUT.frame }))
  if (out.kind === 'json' && out.value.ok === true) {
    const f = out.value as FrameJson
    rt.kitten = kittenOf(f)
    return { text: `Hatched ${f.name} — ${f.rarity} ${f.breed}.` }
  }
  return { text: 'Hatched a new kitten.' }
}

/** The `.catch` of every command hook (a top-level function: the module scan needs one). */
function commandFailed() {
  return { text: UNREACHABLE, exitCode: 1 }
}

/** The `command.run` answers. `card` and `stats` arrive in Task 2.3. */
export function registerCommands(on: On, rt: Runtime): void {
  const notReady = () => ({ text: NOT_READY_TEXT(rt.status), exitCode: 1 })

  on('command.run', { command: 'purr-pet' }, async $ =>
    ready(rt) ? reply(await runAction($, rt, 'pet')) : notReady(),
  ).catch(commandFailed)
  on('command.run', { command: 'purr-play' }, async $ =>
    ready(rt) ? reply(await runAction($, rt, 'play')) : notReady(),
  ).catch(commandFailed)
  on('command.run', { command: 'purr-treat' }, async $ =>
    ready(rt) ? reply(await runAction($, rt, 'treat')) : notReady(),
  ).catch(commandFailed)
  on('command.run', { command: 'purr-rename' }, async ($, e) =>
    ready(rt) ? rename($, rt, e.args) : notReady(),
  ).catch(commandFailed)
  if (rt.options.devCommands) {
    on('command.run', { command: 'purr-hatch' }, async ($, e) =>
      ready(rt) ? hatch($, rt, e.args) : notReady(),
    ).catch(commandFailed)
  }
}
