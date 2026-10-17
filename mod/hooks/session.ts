// Session start: capability handshake, binary status atom, and the one-time
// warnings (missing / outdated binary, classic hooks still in settings.json).
import type { EngineInterface } from 'claude-code'

import { writeBinary } from './atoms'
import { handshake } from './binary'
import { ready, warnOnce } from './runtime'
import type { Runtime, SettingsScan } from './runtime'
import type { FrameJson } from '../types'

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null
const mentions = (v: unknown): boolean => typeof v === 'string' && v.toLowerCase().includes('purrgotchi')

/** Any `hooks.<event>[*].hooks[*].command` naming purrgotchi. */
function eventHasPurr(hooks: Record<string, unknown>, event: string): boolean {
  const groups = hooks[event]
  if (!Array.isArray(groups)) return false
  return groups.some(g => isObj(g) && Array.isArray(g.hooks) && g.hooks.some(h => isObj(h) && mentions(h.command)))
}

/** Pure scan of the user settings for classic Purrgotchi wiring. Never throws. */
export function scanSettings(s: unknown): SettingsScan {
  const hooks = isObj(s) && isObj(s.hooks) ? s.hooks : {}
  const statusLine = isObj(s) && isObj(s.statusLine) ? s.statusLine : {}
  return {
    classicHooks: eventHasPurr(hooks, 'PostToolUse') || eventHasPurr(hooks, 'UserPromptSubmit'),
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

  return { frame }
}
