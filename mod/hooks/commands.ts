// Action commands: `/purr-pet`, `/purr-play`, `/purr-treat`, `/purr-rename`,
// `/purr-hatch` (dev). Registered at session start, answered in `command.run`
// by the binary's `--json` verbs. The model reads `text`, so it stays short.
// The engine's module scan needs `$` passed only to plain functions.
import type { EngineInterface, On, StateDollar } from 'claude-code'

import { outputKey, writeOutput } from './atoms'
import { errorCode, errText, isError, okValue, runJson, runRaw, TIMEOUT } from './binary'
import type { RunOutcome } from './binary'
import { kittenOf } from './celebrate'
import type { FeedHost } from './feed'
import { binaryProblem, debugLog, ready } from './runtime'
import type { Runtime } from './runtime'
import { pokeTicker } from './ticker'
import type { ActionJson, FrameJson, LinesJson, Span } from '../types'

export type CommandName =
  | 'purr'
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
  // Answered by pane.tsx (the pane owns its id and size).
  { name: 'purr', description: 'Open the Purrgotchi kitten pane' },
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
const notReady = (rt: Runtime) => ({ text: binaryProblem(rt.status), exitCode: 1 })

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
type Host = FeedHost & StateDollar

type ActionOutcome = { text: string; isError: boolean; result?: ActionJson }

/** What a run that gave no `ok: true` value tells the user (a failed run is logged). */
function failText($: FeedHost, label: string, out: RunOutcome<unknown>): string {
  if (out.kind === 'failed') {
    debugLog($, `${label} failed (${out.reason}): ${out.detail}`)
    return UNREACHABLE
  }
  const v = out.value
  return isError(v) ? (v.message ?? v.detail ?? v.error) : UNREACHABLE
}

/** One action verb's `--json` run, through `rt.serial`. Never throws. */
async function runVerb($: FeedHost, rt: Runtime, args: readonly [string, ...string[]]): Promise<ActionOutcome> {
  const out = await rt.serial.run(() => runJson<ActionJson>($, rt.bin, args, { timeoutMs: TIMEOUT.command }))
  if (out.kind === 'json' && !isError(out.value)) {
    const v = out.value
    return { text: v.message, isError: !v.ok, result: v }
  }
  return { text: failText($, args[0], out), isError: true }
}

/** `pet`, `play` or `treat`. A cooldown or an empty wallet is a normal answer. */
export function runAction($: FeedHost, rt: Runtime, verb: 'pet' | 'play' | 'treat'): Promise<ActionOutcome> {
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

/**
 * An action answered: the frame shows it within this tick (`play` starts a
 * scene). The pane's buttons answer through this too.
 */
export async function act($: Host, rt: Runtime, verb: 'pet' | 'play' | 'treat') {
  if (!ready(rt)) return notReady(rt)
  const o = await runAction($, rt, verb)
  if (!o.isError) pokeTicker($, rt)
  return reply(o)
}

async function rename($: Host, rt: Runtime, rawName: string) {
  if (!ready(rt)) return notReady(rt)
  const v = validateName(rawName)
  if (!v.ok) return { text: v.text }
  const o = await runVerb($, rt, ['rename', v.name, '--json'])
  if (!o.isError) {
    if (o.result?.name !== undefined && rt.kitten !== null) rt.kitten.name = o.result.name
    pokeTicker($, rt)
  }
  return reply(o)
}

async function hatch($: Host, rt: Runtime, rawArgs: string) {
  if (!ready(rt)) return notReady(rt)
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
  pokeTicker($, rt)
  const [fr, card] = await Promise.all([
    rt.serial.run(() => runJson<FrameJson>($, rt.bin, ['frame', '--json'], { timeoutMs: TIMEOUT.frame })),
    runJson<LinesJson>($, rt.bin, ['card', '--json'], { timeoutMs: TIMEOUT.command }),
  ])
  const f = okValue(fr)
  if (f === null) return { text: 'Hatched a new kitten.' }
  rt.kitten = kittenOf(f)
  const text = `Hatched ${f.name} — ${f.rarity} ${f.breed}.`
  await storeLines($, 'purr-hatch', text, okValue(card)?.lines ?? null)
  return { text }
}

const NO_KITTEN = 'No kitten yet — it hatches on your first tool call.'

/** `{name} — lv {level} {stage} {breed}, {coins}¢, bond {bond}, mood {mood}`; the numbers keep rows apart. */
export function kittenSummary(f: FrameJson | null, fallback: string): string {
  if (f === null) return fallback
  return `${f.name} — lv ${f.level} ${f.stage} ${f.breed}, ${f.coins}¢, bond ${f.bond}, mood ${f.mood}`
}

type Fetched = { lines: Span[][]; frame: FrameJson | null } | { error: ActionOutcome }

/** `card` or `stats` with `frame`, side by side (both read-only: no `serial`). */
async function fetchLines($: Host, rt: Runtime, verb: 'card' | 'stats'): Promise<Fetched> {
  const [out, fr] = await Promise.all([
    runJson<LinesJson>($, rt.bin, [verb, '--json'], { timeoutMs: TIMEOUT.command }),
    runJson<FrameJson>($, rt.bin, ['frame', '--json'], { timeoutMs: TIMEOUT.frame }),
  ])
  const lines = okValue(out)
  if (lines !== null) return { lines: lines.lines, frame: okValue(fr) }
  if (errorCode(out) === 'no_kitten') return { error: { text: NO_KITTEN, isError: false } }
  return { error: { text: failText($, verb, out), isError: true } }
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
  if (!ready(rt)) return notReady(rt)
  const r = await fetchLines($, rt, verb)
  if ('error' in r) return reply(r.error)
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
  on('command.run', { command: 'purr-pet' }, $ => act($, rt, 'pet')).catch(commandFailed)
  on('command.run', { command: 'purr-play' }, $ => act($, rt, 'play')).catch(commandFailed)
  on('command.run', { command: 'purr-treat' }, $ => act($, rt, 'treat')).catch(commandFailed)
  on('command.run', { command: 'purr-rename' }, ($, e) => rename($, rt, e.args)).catch(commandFailed)
  on('command.run', { command: 'purr-card' }, $ => showLines($, rt, 'card')).catch(commandFailed)
  on('command.run', { command: 'purr-stats' }, $ => showLines($, rt, 'stats')).catch(commandFailed)
  if (rt.options.devCommands) {
    on('command.run', { command: 'purr-hatch' }, ($, e) => hatch($, rt, e.args)).catch(commandFailed)
  }
}
