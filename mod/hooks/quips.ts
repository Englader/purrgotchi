// Session-model quips (`quips: session-model`): after a finished turn, a dice
// roll, then the binary builds the prompt (`quip-prompt`), the session's own
// model answers it (`$.model.complete`, so no API key) and the binary
// sanitizes and stores the reply (`quip-store`). Every failure is silent: the
// kitten just keeps its static pool.
import type { EngineInterface, On } from 'claude-code'

import { errText, okValue, runJson, TIMEOUT } from './binary'
import { debugLog, ready } from './runtime'
import type { Runtime } from './runtime'
import { pokeTicker } from './ticker'
import type { QuipPromptJson, QuipStoreJson } from '../types'

/** Mirrors `REACT_DICE_DENOM = 5` in the binary's hooks.rs (`quip-prompt` has no dice of its own). */
export const QUIP_PROBABILITY = 1 / 5
export const QUIP_MAX_CHARS = 500
export const QUIP_TIMEOUT_MS = 15000
/** The smallest model: a quip is a few words. */
const QUIP_MODEL = 'haiku'
const USER_MAX_CHARS = 2000
const ASSISTANT_MAX_CHARS = 4000

/**
 * The turn's dice roll, 0 <= draw < 1: an FNV-1a hash of the turn's id, which
 * is unique per turn, so the gate is a fair `QUIP_PROBABILITY` draw. A pure
 * function of the id (not `Math.random`) so a test picks the outcome by id.
 */
export function quipDraw(turnId: string): number {
  let h = 0x811c9dc5
  for (let i = 0; i < turnId.length; i++) {
    h ^= turnId.charCodeAt(i)
    h = Math.imul(h, 0x01000193)
  }
  return (h >>> 0) / 0x1_0000_0000
}

/** The slice of a `turn.complete` input the gate reads. */
export type QuipTurn = { answer: string; turnId: string; agentId?: string; isAborted: boolean; reason: string }

/** Only a main-loop turn the model answered, with something to remark on, once at a time, and the dice. */
export function shouldQuip(rt: Runtime, e: QuipTurn): boolean {
  if (!ready(rt) || rt.quipInFlight) return false
  if (e.agentId !== undefined || e.isAborted || e.reason !== 'answer' || e.answer.trim() === '') return false
  return quipDraw(e.turnId) < QUIP_PROBABILITY
}

type QuipHost = Parameters<typeof pokeTicker>[0] & Pick<EngineInterface, 'model'>

/**
 * Prompt → model → store. A model reply is model text, which is allowed only
 * here: it goes to `quip-store` as one argv element and the binary sanitizes
 * it. Never throws; at most one at a time.
 */
export async function runQuip($: QuipHost, rt: Runtime, answer: string): Promise<void> {
  rt.quipInFlight = true
  try {
    const stdin = JSON.stringify({ last_user: rt.lastUser, last_assistant: answer.slice(0, ASSISTANT_MAX_CHARS) })
    const p = okValue(await runJson<QuipPromptJson>($, rt.bin, ['quip-prompt', '--json'], { stdin, timeoutMs: TIMEOUT.quip }))
    if (p === null) return
    const m = await $.model.complete({
      model: QUIP_MODEL,
      system: p.system,
      prompt: p.user,
      maxTokens: p.max_tokens,
      timeoutMs: QUIP_TIMEOUT_MS,
    })
    if (!m.isAnswered) return
    const text = m.text.trim().slice(0, QUIP_MAX_CHARS)
    if (text === '') return
    const stored = okValue(await runJson<QuipStoreJson>($, rt.bin, ['quip-store', text, '--json'], { timeoutMs: TIMEOUT.quip }))
    if (stored !== null) pokeTicker($, rt)
  } catch (e) {
    // The engine refused to send, or the host went away: never a toast.
    debugLog($, `quip failed: ${errText(e)}`)
  } finally {
    rt.quipInFlight = false
  }
}

/** Registered only for `quips: session-model`. Observers: always `next(e)`. */
export function registerQuips(on: On, rt: Runtime): void {
  on('prompt.submit', async ($, e, next) => {
    const r = await next(e)
    rt.lastUser = String(e.text ?? '').slice(0, USER_MAX_CHARS)
    return r
  }).catch(($, e, next) => next(e))

  on('turn.complete', async ($, e, next) => {
    const r = await next(e)
    if (shouldQuip(rt, e)) void runQuip($, rt, e.answer)
    return r
  }).catch(($, e, next) => next(e))
}
