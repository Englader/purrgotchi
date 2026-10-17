// Session start: capability handshake, binary status atom, the one-time
// warnings (missing / outdated binary, classic hooks still in settings.json),
// and the frame clock.
import type { EngineInterface } from 'claude-code'

import { writeBinary } from './atoms'
import { handshake, locateBinary } from './binary'
import { kittenOf } from './celebrate'
import { registerCommandNames } from './commands'
import { flushFeeds } from './feed'
import { seedMeasure } from './measure'
import { binaryProblem, ready, warnOnce } from './runtime'
import type { Runtime, SettingsScan } from './runtime'
import { showFrame, startTicker, tickIntervalMs } from './ticker'
import type { FrameJson } from '../types'

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null
const mentions = (v: unknown): boolean => typeof v === 'string' && v.toLowerCase().includes('purrgotchi')

// install.sh's BIN_RE: `"<dir>/purrgotchi[.exe]"`, quoted or bare, or the `purr`
// wrapper, then the verb. A hook that merely mentions purrgotchi is not double
// feeding. The warning is wider than --uninstall-hooks on purpose (any case,
// any matcher): a hand-edited hook that still double feeds is warned about too,
// and the toast says to remove it by hand.
const BIN_RE = String.raw`^\s*("([^"]*[/\\])?purr(gotchi)?(\.(exe|cmd|sh|bat))?"|([^\s"]*[/\\])?purr(gotchi)?(\.(exe|cmd|sh|bat))?)\s+`
const runsVerb = (verb: string, flags: string): RegExp => new RegExp(`${BIN_RE}${verb}(\\s|$)`, flags)

/** Per event: what double feeds (`wide`) and what --uninstall-hooks removes (`strict`, under `star` only). */
const CLASSIC = [
  { event: 'PostToolUse', wide: runsVerb('feed', 'i'), strict: runsVerb('feed', ''), star: true },
  { event: 'UserPromptSubmit', wide: runsVerb('turn', 'i'), strict: runsVerb('turn', ''), star: false },
] as const

/** Every `hooks.<event>[*].hooks[*].command` running the purrgotchi binary's feed / turn. */
function scanHooks(hooks: Record<string, unknown>): Pick<SettingsScan, 'classicHooks' | 'handEdited'> {
  let classicHooks = false
  let handEdited = false
  for (const { event, wide, strict, star } of CLASSIC) {
    const groups = hooks[event]
    if (!Array.isArray(groups)) continue
    for (const g of groups) {
      if (!isObj(g) || !Array.isArray(g.hooks)) continue
      for (const h of g.hooks) {
        if (!isObj(h) || typeof h.command !== 'string' || !wide.test(h.command)) continue
        classicHooks = true
        if (!strict.test(h.command) || (star && g.matcher !== '*')) handEdited = true
      }
    }
  }
  return { classicHooks, handEdited }
}

/** Pure scan of the user settings for classic Purrgotchi wiring. Never throws. */
export function scanSettings(s: unknown): SettingsScan {
  const hooks = isObj(s) && isObj(s.hooks) ? s.hooks : {}
  const statusLine = isObj(s) && isObj(s.statusLine) ? s.statusLine : {}
  return { ...scanHooks(hooks), classicStatusline: mentions(statusLine.command) }
}

const WARN = {
  classicHooks:
    'Old Purrgotchi hooks are still in settings.json (double feeding) — run `bash install.sh --uninstall-hooks`',
  handEdited:
    'Old Purrgotchi feed/turn hooks are still in settings.json (double feeding) — remove them from ~/.claude/settings.json by hand',
} as const

/**
 * Idempotent: a hot reload fires `session.start` again and re-runs the
 * handshake; `warned` resets only with a fresh runtime (module reload).
 */
export async function onSessionStart($: EngineInterface, rt: Runtime): Promise<{ frame: FrameJson | null }> {
  rt.bin = await locateBinary($, rt.options)
  const { status, frame, noKitten } = await handshake($, rt.bin)
  rt.status = status
  // state_unavailable / state_corrupt keep the last state: neither a kitten nor "none".
  if (frame !== null) {
    rt.kitten = kittenOf(frame)
    rt.hadNoKitten = false
  } else if (noKitten) {
    rt.kitten = null
    rt.hadNoKitten = true
  }
  try {
    await writeBinary($, status)
  } catch {
    // view state only; the runtime copy is what hooks check
  }

  // Commands exist whatever the binary's status: a handler explains what is wrong.
  try {
    await registerCommandNames($, rt)
  } catch {
    // each registration has its own try/catch; this is the last net
  }

  if (status.kind === 'missing' || status.kind === 'outdated') warnOnce($, rt, status.kind, binaryProblem(status))

  try {
    rt.scan = scanSettings(await $.settings.read({ source: 'user' }))
  } catch {
    // keep the defaults
  }
  if (ready(rt)) {
    if (rt.scan.classicHooks) {
      warnOnce($, rt, 'classic-hooks', rt.scan.handEdited ? WARN.handEdited : WARN.classicHooks)
    }
    // Feed safety net: a timer is a long-lived context, so credits go out even
    // if a kick from a returned tool.call hook did not. Once per runtime.
    rt.flushTimer ??= $.clock.every(2000, () => {
      void flushFeeds($, rt)
    })
  }
  // After the scan: the ticker passes `--advance` only without a classic statusline.
  if (ready(rt) && tickIntervalMs(rt.options.layout) !== null) {
    rt.ctxPct = (await seedMeasure($)).contextPct
    // A busy lock or corrupt state says nothing: the last frame and status stay.
    if (frame !== null || noKitten) await showFrame($, rt, frame)
    startTicker($, rt)
  }

  return { frame }
}
