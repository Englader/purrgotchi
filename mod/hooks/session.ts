// Session start: capability handshake, binary status atom, and the one-time
// warnings (missing / outdated binary, classic hooks still in settings.json).
import type { EngineInterface } from 'claude-code'

import { writeBinary } from './atoms'
import { handshake } from './binary'
import { flushFeeds } from './feed'
import { ready, warnOnce } from './runtime'
import type { Runtime, SettingsScan } from './runtime'
import type { FrameJson } from '../types'

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null
const mentions = (v: unknown): boolean => typeof v === 'string' && v.toLowerCase().includes('purrgotchi')

// install.sh's BIN_RE (what --uninstall-hooks removes): `"<dir>/purrgotchi[.exe]"`,
// quoted or bare, or the `purr` wrapper, then the verb. A hook that merely
// mentions purrgotchi is not double feeding, and the uninstaller would not
// remove it, so the toast could never be cleared.
const BIN_RE = String.raw`^\s*("([^"]*[/\\])?purr(gotchi)?(\.(exe|cmd|sh|bat))?"|([^\s"]*[/\\])?purr(gotchi)?(\.(exe|cmd|sh|bat))?)\s+`
const runsVerb = (verb: string): RegExp => new RegExp(`${BIN_RE}${verb}(\\s|$)`, 'i')
const FEED_HOOK = runsVerb('feed')
const TURN_HOOK = runsVerb('turn')

/** Any `hooks.<event>[*].hooks[*].command` running the purrgotchi binary as `re` says. */
function eventRuns(hooks: Record<string, unknown>, event: string, re: RegExp): boolean {
  const groups = hooks[event]
  if (!Array.isArray(groups)) return false
  return groups.some(
    g =>
      isObj(g) && Array.isArray(g.hooks) && g.hooks.some(h => isObj(h) && typeof h.command === 'string' && re.test(h.command)),
  )
}

/** Pure scan of the user settings for classic Purrgotchi wiring. Never throws. */
export function scanSettings(s: unknown): SettingsScan {
  const hooks = isObj(s) && isObj(s.hooks) ? s.hooks : {}
  const statusLine = isObj(s) && isObj(s.statusLine) ? s.statusLine : {}
  return {
    classicHooks: eventRuns(hooks, 'PostToolUse', FEED_HOOK) || eventRuns(hooks, 'UserPromptSubmit', TURN_HOOK),
    classicStatusline: mentions(statusLine.command),
  }
}

export const WARN = {
  missing: 'Purrgotchi binary not found — set Binary path in /config or run `cargo install --path rust`',
  outdated: (version: string) =>
    `Purrgotchi binary ${version} is too old for this mod — rebuild it: \`cargo install --path rust\``,
  classicHooks:
    'Old Purrgotchi hooks are still in settings.json (double feeding) — run `bash install.sh --uninstall-hooks`',
} as const

/**
 * Idempotent: a hot reload fires `session.start` again and re-runs the
 * handshake; `warned` resets only with a fresh runtime (module reload).
 */
export async function onSessionStart($: EngineInterface, rt: Runtime): Promise<{ frame: FrameJson | null }> {
  const { status, frame } = await handshake($, rt.bin)
  rt.status = status
  try {
    await writeBinary($, status)
  } catch {
    // view state only; the runtime copy is what hooks check
  }

  if (status.kind === 'missing') warnOnce($, rt, 'missing', WARN.missing)
  if (status.kind === 'outdated') warnOnce($, rt, 'outdated', WARN.outdated(status.version))

  try {
    rt.scan = scanSettings(await $.settings.read({ source: 'user' }))
  } catch {
    // keep the defaults
  }
  if (ready(rt) && rt.scan.classicHooks) warnOnce($, rt, 'classic-hooks', WARN.classicHooks)

  // Safety net for feeds: a timer is a long-lived context, so credits go out
  // even if a kick from a returned tool.call hook did not. Once per runtime;
  // a hot reload kills it with the old runtime and the new one starts its own.
  if (ready(rt)) {
    rt.flushTimer ??= $.clock.every(2000, () => {
      void flushFeeds($, rt)
    })
  }

  return { frame }
}
