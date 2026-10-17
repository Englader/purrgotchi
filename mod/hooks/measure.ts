// Session figures for the pane: seeded at session start from `$.session.*`,
// then pushed by the engine's `session.measure` (context, rate limits, cost).
// `session.measure` carries no model, duration or lines added/removed.
import type { EngineInterface, On, SessionContextUsage, SessionCost, SessionRateLimit } from 'claude-code'

import { EMPTY_MEASURE, writeMeasure } from './atoms'
import { errText } from './binary'
import { debugLog } from './runtime'
import type { Runtime } from './runtime'
import { pokeTicker } from './ticker'
import type { MeasureView } from '../types'

const num = (n: number | undefined): number | null => (n !== undefined && Number.isFinite(n) ? n : null)

type Figures = { context: SessionContextUsage; rateLimits: readonly SessionRateLimit[]; cost?: SessionCost }

/** The engine's figures as the pane's view (no NaN, no undefined numbers). */
function figuresView(f: Figures): Pick<MeasureView, 'contextPct' | 'contextTokens' | 'contextWindow' | 'rateLimits' | 'costUsd'> {
  return {
    contextPct: num(f.context.percent),
    contextTokens: num(f.context.tokens),
    contextWindow: num(f.context.window),
    rateLimits: f.rateLimits
      .filter(r => Number.isFinite(r.percentUsed))
      .map(r => ({ kind: r.kind, percentUsed: r.percentUsed, ...(r.resetsAt !== undefined && { resetsAt: r.resetsAt }) })),
    costUsd: num(f.cost?.usd),
  }
}

/**
 * Seeds the view from `$.session.usage()`, `model()` and `cwd()`, each on its
 * own (one failing leaves its fields null), and returns it. Never throws.
 */
export async function seedMeasure($: Pick<EngineInterface, 'session' | 'ui' | 'state'>): Promise<MeasureView> {
  const view: Partial<MeasureView> = {}
  try {
    const u = await $.session.usage()
    Object.assign(view, figuresView(u), { startedAt: num(u.startedAt) })
  } catch (e) {
    debugLog($, `session.usage failed: ${errText(e)}`)
  }
  try {
    view.model = await $.session.model()
  } catch {
    // stays null
  }
  try {
    view.cwd = await $.session.cwd()
  } catch {
    // stays null
  }
  try {
    await writeMeasure($, view)
  } catch (e) {
    debugLog($, `measure not stored: ${errText(e)}`)
  }
  return { ...EMPTY_MEASURE, ...view }
}

/** Keeps the view current; a context move re-ticks the frame (mood follows the fill). */
export function registerMeasure(on: On, rt: Runtime): void {
  on('session.measure', async ($, e, next) => {
    const r = await next(e)
    try {
      const view = figuresView(e)
      rt.ctxPct = view.contextPct
      await writeMeasure($, view)
      if (e.changed.includes('context')) pokeTicker($, rt)
    } catch (err) {
      debugLog($, `measure failed: ${errText(err)}`)
    }
    return r
  })
}
