// The bottom band (`layout: bottom`): a reader of the `frame`, `measure` and
// `binary` atoms drawn on `AbovePrompt`. Full (four columns: portrait ·
// kitten · session · limits) when there are rows to spare, else one compact
// row. The only side effect of a render is the width hint in `rt.view`.
import type { EngineInterface, On, RenderElement, RenderInput } from 'claude-code'

import { readBinary, readFrame, readMeasure } from './atoms'
import { errText } from './binary'
import { bar, duration, kTokens, PALETTE, statusText } from './hud'
import type { LineEls } from './lines'
import { spriteElement } from './pixels'
import type { SpriteEls } from './pixels'
import { field, limitRows, moodColor, NO_KITTEN_TEXT } from './pane'
import { binaryProblem, debugLog } from './runtime'
import type { Runtime } from './runtime'
import type { BinaryStatus, FrameJson, MeasureView } from '../types'

/** Below: the compact row. */
const COMPACT_ROWS = 7
const COMPACT_COLUMNS = 60
/** Below: the limits column / the session column is dropped. */
const LIMITS_COLUMNS = 100
const SESSION_COLUMNS = 80
const KITTEN_WIDTH = 24
const SESSION_WIDTH = 26
const GAP = 2

export type BandModel = {
  frame: FrameJson | null
  measure: MeasureView
  status: BinaryStatus
  /** The band's `bodyColumns` and the rows it may take. */
  cols: number
  rows: number
  nowMs: number
  surface: string
}

const pct = (n: number): number => Math.max(0, Math.round(n))

/** `ctx {bar} {pct}%`, or null when the window's fill is not known. */
function ctxText(m: MeasureView): string | null {
  return m.contextPct !== null && Number.isFinite(m.contextPct) ? `${bar(m.contextPct)} ${pct(m.contextPct)}%` : null
}

/** The five-hour window as `{bar} {pct}%`, or null. */
function fiveHourText(m: MeasureView): string | null {
  const r = m.rateLimits.find(x => x.kind === 'five_hour')
  return r !== undefined && Number.isFinite(r.percentUsed) ? `${bar(r.percentUsed)} ${pct(r.percentUsed)}%` : null
}

function compact({ Text }: LineEls, m: BandModel, f: FrameJson): RenderElement {
  const ctx = ctxText(m.measure)
  const five = fiveHourText(m.measure)
  const parts = [statusText(f) ?? '', ctx !== null ? `ctx ${ctx}` : '', five !== null ? `5h ${five}` : '']
  return (
    <Text wrap="truncate" color={PALETTE.cyan} mono>
      {parts.filter(p => p !== '').join('  ')}
    </Text>
  )
}

function kittenColumn(els: LineEls, f: FrameJson): RenderElement {
  const { Box, Text } = els
  const hud = f.hud
  return (
    <Box flexDirection="column" width={KITTEN_WIDTH} flexShrink={1}>
      <Text wrap="truncate" mono>
        <Text color={hud?.name_fg ?? PALETTE.cyan} bold>
          {f.name.toUpperCase()}
        </Text>
        <Text color={PALETTE.dim}>{` lv ${f.level}`}</Text>
      </Text>
      {hud !== undefined ? field(els, 'xp   ', `${bar(hud.xp_pct)} ${hud.xp_pct}%`, PALETTE.cyan) : null}
      {field(els, 'coin ', `${f.coins}¢`, PALETTE.yellow)}
      {field(els, 'mood ', `◆ ${f.mood}`, moodColor(f.mood))}
    </Box>
  )
}

function sessionColumn(els: LineEls, m: BandModel): RenderElement {
  const { Box } = els
  const ms = m.measure
  const ctx = ctxText(ms)
  const tokens = kTokens(ms.contextTokens)
  return (
    <Box flexDirection="column" width={SESSION_WIDTH} flexShrink={1}>
      {ctx !== null ? field(els, 'ctx  ', `${ctx}${tokens !== null ? ` · ${tokens}` : ''}`, PALETTE.cyan) : null}
      {ms.costUsd !== null ? field(els, 'cost ', `$${ms.costUsd.toFixed(2)} session`, PALETTE.yellow) : null}
      {ms.startedAt !== null ? field(els, 'time ', duration(m.nowMs - ms.startedAt), PALETTE.cyan) : null}
    </Box>
  )
}

/** The head-and-shoulders pixel portrait, else the ASCII sprite cut to the rows the band has. */
function portrait(els: SpriteEls, m: BandModel, f: FrameJson): RenderElement {
  return spriteElement(els, m.surface, { ...f, sprite: f.sprite.slice(0, m.rows) }, 'portrait', m.cols, m.rows)
}

/** The band's whole tree: a pure function of its inputs. */
export function bandTree(els: SpriteEls, m: BandModel): RenderElement {
  const { Box, Text } = els
  const dim = (text: string) => (
    <Text wrap="truncate" color={PALETTE.dim}>
      {text}
    </Text>
  )
  if (m.status.kind !== 'ready') return dim(binaryProblem(m.status))
  const f = m.frame
  if (f === null) return dim(NO_KITTEN_TEXT)
  if (m.rows < COMPACT_ROWS || m.cols < COMPACT_COLUMNS) return compact(els, m, f)

  return (
    <Box flexDirection="row" columnGap={GAP}>
      <Box flexShrink={0}>{portrait(els, m, f)}</Box>
      {kittenColumn(els, f)}
      {m.cols >= SESSION_COLUMNS ? sessionColumn(els, m) : null}
      {m.cols >= LIMITS_COLUMNS ? (
        <Box flexDirection="column" flexGrow={1} flexShrink={1}>
          {limitRows(els, m.measure.rateLimits, m.nowMs)}
        </Box>
      ) : null}
    </Box>
  )
}

type BandEvent = RenderInput<'AbovePrompt'>

/** The band's tree, or null for "let the engine draw it". Never throws. */
async function drawBand($: EngineInterface, rt: Runtime, e: BandEvent): Promise<RenderElement | null> {
  try {
    const { bodyColumns: cols, maxRows: rows, hasSurvey } = e.props
    if (hasSurvey || (e.surface !== 'terminal' && e.surface !== 'desktop')) return null
    if (!Number.isFinite(cols) || !Number.isFinite(rows)) return null
    rt.view.bandColumns = cols
    const [frame, measure, status] = await Promise.all([readFrame($), readMeasure($), readBinary($)])
    return bandTree($.ui.resolve(e), { frame, measure, status, cols, rows, nowMs: Date.now(), surface: e.surface })
  } catch (err) {
    debugLog($, `band render failed: ${errText(err)}`)
    return null
  }
}

export function registerBand(on: On, rt: Runtime): void {
  on('ui.render', { component: 'AbovePrompt' }, async ($, e, next) => (await drawBand($, rt, e)) ?? next(e))
}
