// Action commands: `/purr-pet`, `/purr-play`, `/purr-treat`, `/purr-rename`,
// `/purr-hatch` (dev). Registered at session start, answered in `command.run`
// by the binary's `--json` verbs. The model reads `text`, so it stays short.
// The engine's module scan needs `$` passed only to plain functions.
import type { EngineInterface, On, StateDollar } from 'claude-code'

import { outputKey, writeOutput } from './atoms'
import { errText, isError, okValue, runJson, runRaw, TIMEOUT } from './binary'
import { kittenOf } from './celebrate'
import type { CelebrateHost } from './celebrate'
import { debugLog, ready } from './runtime'
import type { Runtime } from './runtime'
import type { ActionJson, BinaryStatus, FrameJson, LinesJson, Span } from '../types'

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

/**
 * Trimmed, 1 to 32 characters, no control characters, and not `--json` (the
 * binary reads that argv element as its flag, never as the name).
 */
export function validateName(raw: string): { ok: true; name: string } | { ok: false; text: string } {
  const name = raw.trim()
  if (name === '') return { ok: false, text: 'Usage: /purr-rename <name>' }
  if (name === '--json') return { ok: false, text: 'That name is reserved; pick another.' }
  if ([...name].length > NAME_MAX) return { ok: false, text: `Names are limited to ${NAME_MAX} characters.` }
  if (/\p{Cc}/u.test(name)) return { ok: false, text: "Names can't contain control characters." }
  return { ok: true, name }
}

/** `$` for handlers that also keep a printed row. */
type Host = CelebrateHost & StateDollar

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

async function hatch($: Host, rt: Runtime, rawArgs: string) {
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
  // A new kitten: the cached one is wrong until a frame names it.
  rt.hadNoKitten = false
  rt.kitten = null
  const out = await rt.serial.run(() => runJson<FrameJson>($, rt.bin, ['frame', '--json'], { timeoutMs: TIMEOUT.frame }))
  if (out.kind === 'json' && out.value.ok === true) {
    const f = out.value as FrameJson
    rt.kitten = kittenOf(f)
    const text = `Hatched ${f.name} — ${f.rarity} ${f.breed}.`
    const card = okValue(await runJson<LinesJson>($, rt.bin, ['card', '--json'], { timeoutMs: TIMEOUT.command }))
    await storeLines($, 'purr-hatch', text, card?.lines ?? null)
    return { text }
  }
  return { text: 'Hatched a new kitten.' }
}

const NO_KITTEN = 'No kitten yet — it hatches on your first tool call.'

/** `{name} — lv {level} {stage} {breed}, {coins}¢, bond {bond}, mood {mood}`; the numbers keep rows apart. */
export function kittenSummary(f: FrameJson | null, fallback: string): string {
  if (f === null) return fallback
  return `${f.name} — lv ${f.level} ${f.stage} ${f.breed}, ${f.coins}¢, bond ${f.bond}, mood ${f.mood}`
}

type Fetched = { lines: Span[][] | null; frame: FrameJson | null; error?: { text: string; exitCode: 1 | undefined } }

/** `card` or `stats` with `frame`, side by side (both read-only: no `serial`). */
async function fetchLines($: Host, rt: Runtime, verb: 'card' | 'stats'): Promise<Fetched> {
  const [out, fr] = await Promise.all([
    runJson<LinesJson>($, rt.bin, [verb, '--json'], { timeoutMs: TIMEOUT.command }),
    runJson<FrameJson>($, rt.bin, ['frame', '--json'], { timeoutMs: TIMEOUT.frame }),
  ])
  const frame = okValue(fr)
  const lines = okValue(out)
  if (lines !== null) return { lines: lines.lines, frame }
  if (out.kind === 'failed') {
    debugLog($, `${verb} failed (${out.reason}): ${out.detail}`)
    return { lines: null, frame, error: { text: UNREACHABLE, exitCode: 1 } }
  }
  const v = out.value
  if (isError(v)) {
    return v.error === 'no_kitten'
      ? { lines: null, frame, error: { text: NO_KITTEN, exitCode: undefined } }
      : { lines: null, frame, error: { text: v.message ?? v.detail ?? v.error, exitCode: 1 } }
  }
  return { lines: null, frame, error: { text: UNREACHABLE, exitCode: 1 } }
}

/** Keeps the coloured lines for the render hook. Never throws. */
async function storeLines($: Host, command: string, text: string, lines: Span[][] | null): Promise<void> {
  if (lines === null) return
  try {
    await writeOutput($, { key: outputKey(command, text), lines })
  } catch (e) {
    debugLog($, `${command} row not stored: ${errText(e)}`)
  }
}

/** `/purr-card` and `/purr-stats`: a one-line summary for the model, the full thing drawn by `CommandOutput`. */
async function showLines($: Host, rt: Runtime, verb: 'card' | 'stats') {
  const r = await fetchLines($, rt, verb)
  if (r.error !== undefined) return { text: r.error.text, ...(r.error.exitCode !== undefined && { exitCode: r.error.exitCode }) }
  const text = kittenSummary(r.frame, verb === 'card' ? 'Purrgotchi card' : 'Purrgotchi stats')
  await storeLines($, `purr-${verb}`, text, r.lines)
  return { text }
}

/** The `.catch` of every command hook (a top-level function: the module scan needs one). */
function commandFailed() {
  return { text: UNREACHABLE, exitCode: 1 }
}

/** The `command.run` answers. */
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
  on('command.run', { command: 'purr-card' }, async $ =>
    ready(rt) ? showLines($, rt, 'card') : notReady(),
  ).catch(commandFailed)
  on('command.run', { command: 'purr-stats' }, async $ =>
    ready(rt) ? showLines($, rt, 'stats') : notReady(),
  ).catch(commandFailed)
  if (rt.options.devCommands) {
    on('command.run', { command: 'purr-hatch' }, async ($, e) =>
      ready(rt) ? hatch($, rt, e.args) : notReady(),
    ).catch(commandFailed)
  }
}
