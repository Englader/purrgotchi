// Turn: `classic.UserPromptSubmit` → `turn --json`, with the classic stdin
// passed through byte-compatible (`session_id`, `transcript_path`, `prompt`,
// …); the binary reads only what it needs. The prompt never waits on it.
import type { EngineInterface, On } from 'claude-code'

import { flushFeeds } from './feed'
import { runJson, TIMEOUT } from './binary'
import type { PurrOptions } from './options'
import { ready } from './runtime'
import type { Runtime } from './runtime'
import type { TurnJson } from '../types'

/**
 * `own-key` keeps the classic behaviour (the binary may spawn its `react`
 * worker with the user's provider key). `off` and `session-model` blank the
 * provider: an empty `PURR_LLM_PROVIDER` is "unset" to the binary and process
 * env beats its `.env`, so no `react` worker is spawned.
 */
export function turnEnv(o: PurrOptions): Record<string, string> | undefined {
  return o.quips === 'own-key' ? undefined : { PURR_LLM_PROVIDER: '' }
}

/**
 * Credited tool calls land first (`turn` zeroes the streak), then `turn`
 * through `rt.serial`. Never throws.
 */
export async function runTurn($: Pick<EngineInterface, 'process' | 'ui'>, rt: Runtime, stdin: string): Promise<void> {
  try {
    await flushFeeds($, rt)
    // Phase 2: the TurnJson celebration is handled here.
    await rt.serial.run(() =>
      runJson<TurnJson>($, rt.bin, ['turn', '--json'], { stdin, env: turnEnv(rt.options), timeoutMs: TIMEOUT.turn }),
    )
  } catch {
    // observer
  }
}

export function registerTurn(on: On, rt: Runtime): void {
  // Fires after `prompt.submit`, with or without a settings hook. Kicked, not
  // awaited: nothing is awaited before `next(e)`.
  on('classic.UserPromptSubmit', async ($, e, next) => {
    if (ready(rt)) void runTurn($, rt, JSON.stringify(e))
    return next(e)
  }).catch(($, e, next) => next(e))
}
