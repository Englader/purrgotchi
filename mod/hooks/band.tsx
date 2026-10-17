// The bottom band (`layout: bottom`): a reader of the `frame`, `measure` and
// `binary` atoms drawn on `AbovePrompt`. Full (four columns: portrait ·
// kitten · session · limits) when there are rows to spare, else one compact
// row. Its renders have no side effect: the band draws no scene strip.
import type { EngineInterface, On, RenderElement, RenderInput } from 'claude-code'

import { readBinary, readFrame, readMeasure } from './atoms'
import { errText } from './binary'
import { NO_KITTEN } from './commands'
import { barPct, costText, ctxText, moodColor, PALETTE, statusText, timeText } from './hud'
import { field, limitRows } from './lines'
import type { LineEls } from './lines'
import { spriteElement } from './pixels'
import type { SpriteEls } from './pixels'
import { binaryProblem, debugLog } from './runtime'
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

type BandModel = {
  frame: FrameJson | null
  measure: MeasureView
  status: BinaryStatus
  /** The band's `bodyColumns` and the rows it may take. */
  cols: number
  rows: number
  nowMs: number
  surface: string
}

/** The five-hour window as `{bar} {pct}%`, or null. */
function fiveHourText(m: MeasureView): string | null {
  const r = m.rateLimits.find(x => x.kind === 'five_hour')
  return r !== undefined && Number.isFinite(r.percentUsed) ? barPct(r.percentUsed) : null
}

function compact({ Text }: LineEls, m: BandModel, f: FrameJson): RenderElement {
  const ctx = ctxText(m.measure, false)
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
      {hud !== undefined ? field(els, 'xp   ', barPct(hud.xp_pct), PALETTE.cyan) : null}
      {field(els, 'coin ', `${f.coins}¢`, PALETTE.yellow)}
      {field(els, 'mood ', `◆ ${f.mood}`, moodColor(f.mood))}
    </Box>
  )
}

function sessionColumn(els: LineEls, m: BandModel): RenderElement {
  const { Box } = els
  const ctx = ctxText(m.measure)
  const cost = costText(m.measure)
  const time = timeText(m.measure, m.nowMs)
  return (
    <Box flexDirection="column" width={SESSION_WIDTH} flexShrink={1}>
      {ctx !== null ? field(els, 'ctx  ', ctx, PALETTE.cyan) : null}
      {cost !== null ? field(els, 'cost ', cost, PALETTE.yellow) : null}
      {time !== null ? field(els, 'time ', time, PALETTE.cyan) : null}
    </Box>
  )
}

/** The band's whole tree: a pure function of its inputs. */
function bandTree(els: SpriteEls, m: BandModel): RenderElement {
  const { Box, Text } = els
  const dim = (text: string) => (
    <Text wrap="truncate" color={PALETTE.dim}>
      {text}
    </Text>
  )
  if (m.status.kind !== 'ready') return dim(binaryProblem(m.status))
  const f = m.frame
  if (f === null) return dim(NO_KITTEN)
  if (m.rows < COMPACT_ROWS || m.cols < COMPACT_COLUMNS) return compact(els, m, f)

  return (
    <Box flexDirection="row" columnGap={GAP}>
      <Box flexShrink={0}>{spriteElement(els, m.surface, f, m.cols, m.rows).el}</Box>
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
async function drawBand($: EngineInterface, e: BandEvent): Promise<RenderElement | null> {
  try {
    const { bodyColumns: cols, maxRows: rows, hasSurvey } = e.props
    if (hasSurvey || (e.surface !== 'terminal' && e.surface !== 'desktop')) return null
    if (!Number.isFinite(cols) || !Number.isFinite(rows)) return null
    const [frame, measure, status] = await Promise.all([readFrame($), readMeasure($), readBinary($)])
    return bandTree($.ui.resolve(e), { frame, measure, status, cols, rows, nowMs: Date.now(), surface: e.surface })
  } catch (err) {
    debugLog($, `band render failed: ${errText(err)}`)
    return null
  }
}

export function registerBand(on: On): void {
  on('ui.render', { component: 'AbovePrompt' }, async ($, e, next) => (await drawBand($, e)) ?? next(e))
}
