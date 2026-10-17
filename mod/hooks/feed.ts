// Feed: a non-blocking, coalescing `tool.call` observer.
//
// Awaiting `feed` inside `tool.call` would delay the tool result by one spawn,
// and one spawn per call races for the binary's 500 ms state lock under
// parallel tool calls (a busy lock drops the credit). So the hook only counts
// the call after `next(e)` and kicks a flush it does not await. A flush sends
// everything pending as one `feed --count N` through `rt.serial`; calls that
// land meanwhile go out in the same flush's next round. A 2 s clock started
// at session start flushes too, so neither trigger relies on the other.
import type { EngineInterface, On } from 'claude-code'

import { onFeedResult } from './celebrate'
import { errorCode, errText, okValue, runJson, TIMEOUT } from './binary'
import { debugLog, ready } from './runtime'
import type { Runtime } from './runtime'
import type { FeedJson } from '../types'

/** Mirrors `cli::MAX_FEED_COUNT`; a larger `--count` is a usage error. */
export const MAX_FEED_COUNT = 10_000

/** The slice of `$` a flush needs (any hook's or timer's `$`). */
export type FeedHost = Pick<EngineInterface, 'process' | 'ui'>

/**
 * The pending count and the flush in progress. State only: the engine's
 * module scan needs every `$` passed by name to a plain function, never to a
 * method, so the flush lives in `flushFeeds($, rt)` below.
 */
export class FeedQueue {
  private count = 0
  /** The flush in progress, shared by every caller until it settles. */
  inflight: Promise<void> | null = null
  /** The pending credits already failed once by rejection (timeout). */
  retried = false

  get pending(): number {
    return this.count
  }

  add(n: number): void {
    this.count = Math.min(MAX_FEED_COUNT, this.count + n)
  }

  /** Takes everything pending (the batch to send). */
  take(): number {
    const n = this.count
    this.count = 0
    return n
  }
}

/**
 * Sends everything pending in `rt.feeds`, single-flight (a call during a flush
 * gets that flush's promise). A requeue ends the round, so a busy lock or a
 * hanging binary waits for the next kick or clock tick. Never throws.
 */
export function flushFeeds($: FeedHost, rt: Runtime): Promise<void> {
  const q = rt.feeds
  if (q.inflight !== null) return q.inflight
  if (q.pending === 0 || !ready(rt)) return Promise.resolve()
  q.inflight = drainFeeds($, rt).finally(() => {
    q.inflight = null
  })
  return q.inflight
}

async function drainFeeds($: FeedHost, rt: Runtime): Promise<void> {
  while (rt.feeds.pending > 0) {
    const n = rt.feeds.take()
    try {
      if (!(await sendFeed($, rt, n))) return
    } catch (e) {
      debugLog($, `feed failed (dropped ${n}): ${errText(e)}`)
      return
    }
  }
}

/** One `feed --count n`. False when the round should stop. */
async function sendFeed($: FeedHost, rt: Runtime, n: number): Promise<boolean> {
  const q = rt.feeds
  const args = ['feed', '--json', '--count', String(n)]
  const out = await rt.serial.run(() => runJson<FeedJson>($, rt.bin, args, { timeoutMs: TIMEOUT.feed }))

  const fed = okValue(out)
  if (fed !== null) {
    q.retried = false
    await onFeedResult($, rt, fed)
    return true
  }
  const code = errorCode(out)
  if (code === 'state_unavailable') {
    q.add(n) // lock busy: the credits go out with the next flush
    return false
  }
  if (out.kind === 'failed' && out.reason === 'rejected' && !q.retried) {
    q.retried = true // timeout: one more try, so a hanging binary cannot loop
    q.add(n)
    return false
  }
  q.retried = false
  const why = code ?? (out.kind === 'failed' ? `${out.reason}: ${out.detail}` : 'not ok')
  debugLog($, `feed failed (dropped ${n}): ${why}`)
  return false
}

/**
 * Counts every tool call that ran (subagents' too, as classic PostToolUse
 * does). A `deny` (permission refused, or a result withheld) is not counted.
 */
export function registerFeed(on: On, rt: Runtime): void {
  on('tool.call', async ($, e, next) => {
    const r = await next(e)
    if (ready(rt) && typeof r.deny !== 'string') {
      rt.feeds.add(1)
      void flushFeeds($, rt)
    }
    return r
  }).catch(($, e, next) => next(e))

  // session.end's one short bound (1.5 s) covers every hook and the engine's
  // own end step, so that step starts first and the flush runs beside it,
  // never ahead of it. At the bound the feed is aborted; losing a batch here
  // is acceptable. Timers are not cancelled: `/clear` ends the session with
  // no new start.
  on('session.end', async ($, e, next) => {
    const [r] = await Promise.all([next(e), flushFeeds($, rt).catch(() => {})])
    return r
  })
}
