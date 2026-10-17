// Finds the purrgotchi binary. Argv only, never a shell; nothing here throws.
import type { EngineInterface } from 'claude-code'

/** A leading `~` (alone, or before `/` or `\`) as `home`; anything else, or no home, as is. */
export function expandHome(path: string, home: string | undefined): string {
  if (home === undefined || home === '' || !/^~(?=$|[/\\])/.test(path)) return path
  return home.replace(/[/\\]+$/, '') + path.slice(1)
}

async function exists($: Pick<EngineInterface, 'fs'>, path: string): Promise<boolean> {
  try {
    return await $.fs.exists(path)
  } catch {
    return false
  }
}

async function onPath($: Pick<EngineInterface, 'process'>): Promise<boolean> {
  try {
    const r = await $.process.run(['purrgotchi', '--version'], { timeoutMs: 3000 })
    return r.exitCode === 0 && /^purrgotchi \S+/.test(r.stdout.trim())
  } catch {
    return false
  }
}

/** `PURR_BIN`, then `purrgotchi` on PATH, then `~/.claude/purrgotchi/bin/purrgotchi`; else null. */
export async function locateBinary($: Pick<EngineInterface, 'env' | 'fs' | 'process'>): Promise<string | null> {
  try {
    const home = (await $.env.get('HOME')) ?? (await $.env.get('USERPROFILE'))
    const explicit = (await $.env.get('PURR_BIN'))?.trim()
    if (explicit !== undefined && explicit !== '') {
      const path = expandHome(explicit, home)
      if (await exists($, path)) return path
    }
    if (await onPath($)) return 'purrgotchi'
    if (home !== undefined && home !== '') {
      const fallback = `${home.replace(/[/\\]+$/, '')}/.claude/purrgotchi/bin/purrgotchi`
      if (await exists($, fallback)) return fallback
    }
  } catch {
    // no binary
  }
  return null
}
