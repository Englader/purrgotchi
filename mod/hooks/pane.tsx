// The kitten pane: a pure reader of the `frame`, `measure` and `binary` atoms,
// drawn in the cyberpunk palette (spec §4.8) and sized to `bodyColumns`. Also
// `/purr` (an asked open), the unasked open at session start, and the pet /
// treat / play buttons, which answer through the same code path as the
// commands. The only side effect of a render is the width hint in `rt.paneColumns`,
// which the ticker turns into the scene strip's `--width`.
import type {
  ButtonProps,
  ElementConstructor,
  EngineInterface,
  On,
  RenderElement,
  RenderInput,
} from 'claude-code'

import { readBinary, readFrame, readMeasure } from './atoms'
import { errText } from './binary'
import { show } from './celebrate'
import { act, NO_KITTEN, UNREACHABLE } from './commands'
import { ago, bar, barPct, costText, ctxText, moodColor, PALETTE, shortCwd, timeText } from './hud'
import { field, limitRows, lineWidth, renderLines, SURFACES } from './lines'
import { spriteElement } from './pixels'
import type { SpriteEls } from './pixels'
import { binaryProblem, debugLog } from './runtime'
import type { Runtime } from './runtime'
import type { BinaryStatus, FrameJson, MeasureView, Span } from '../types'

export const PANE_ID = 'kitten'
/** The 56-column mockup plus the border. */
const PANE_COLUMNS = 58
const TITLE = 'Purrgotchi'

/** At or above: sprite and info side by side. */
const SIDE_BY_SIDE = 56
/** Below: the kitten section, speech and buttons only. */
const KITTEN_ONLY = 40
/** The rounded border and one column of padding on each side. */
const CHROME = 4
/** The pixel sprite is 32 columns: beside it the info column needs 19 and the gap is 3. */
const SIDE_BY_SIDE_PIXELS = CHROME + 32 + 3 + 19

type Verb = 'pet' | 'treat' | 'play'
const BUTTONS: readonly { verb: Verb; hotkey: string }[] = [
  { verb: 'pet', hotkey: 'p' },
  { verb: 'treat', hotkey: 't' },
  { verb: 'play', hotkey: 'y' },
]

/** The elements the pane draws with (every surface carries them). */
type PaneEls = SpriteEls & { Button: ElementConstructor<ButtonProps> }

type PaneModel = {
  frame: FrameJson | null
  measure: MeasureView
  status: BinaryStatus
  /** The pane's `bodyColumns`. */
  cols: number
  nowMs: number
  /** `mobile` has no keyboard: no hotkeys, no `esc close`. */
  surface: string
  onAction: (verb: Verb) => void
}

/** `SESSION ─────` with an optional right-hand part, as wide as the body. */
function rule({ Text }: PaneEls, title: string, inner: number, right: Span[] = []): RenderElement {
  const rightWidth = lineWidth(right)
  const dashes = Math.max(3, inner - title.length - 1 - (rightWidth > 0 ? rightWidth + 1 : 0))
  return (
    <Text wrap="truncate" mono>
      <Text color={PALETTE.violet} bold>
        {`${title} `}
      </Text>
      <Text color={PALETTE.purple}>{'─'.repeat(dashes)}</Text>
      {right.length > 0 ? ' ' : ''}
      {right.map(s => (
        <Text color={s.fg}>{s.text}</Text>
      ))}
    </Text>
  )
}

function buttons(els: PaneEls, m: PaneModel): RenderElement {
  const { Box, Button, Text } = els
  const keys = m.surface !== 'mobile'
  return (
    <Box flexDirection="row" flexWrap="wrap" columnGap={2}>
      {BUTTONS.map(b => (
        <Button key={b.verb} {...(keys && { hotkey: b.hotkey })} label={b.verb} onPress={() => m.onAction(b.verb)} />
      ))}
      {keys ? <Text color={PALETTE.dim}>esc close</Text> : null}
    </Box>
  )
}

/** Name, rarity, breed, stage and the stat rows; rows needing `hud` drop without it. */
function info(els: PaneEls, f: FrameJson): RenderElement {
  const { Box, Text } = els
  const hud = f.hud
  const nameColor = hud?.name_fg ?? PALETTE.cyan
  return (
    <Box flexDirection="column" flexShrink={1}>
      <Text wrap="truncate" color={nameColor} bold>
        {f.name.toUpperCase()}
      </Text>
      <Text wrap="truncate" color={nameColor}>
        {hud !== undefined ? `${hud.stars} ${f.rarity}` : f.rarity}
      </Text>
      <Text wrap="truncate">{f.breed}</Text>
      <Text wrap="truncate">{`${f.stage} · lv ${f.level}`}</Text>
      {hud !== undefined ? field(els, 'xp   ', barPct(hud.xp_pct), PALETTE.cyan) : null}
      {field(els, 'bond ', `${bar(f.bond)} ${f.bond}`, PALETTE.mint)}
      {field(els, 'coin ', `${f.coins}¢`, PALETTE.yellow)}
      {field(els, 'mood ', `◆ ${f.mood}`, moodColor(f.mood))}
      {hud !== undefined ? field(els, 'fed  ', ago(f.now - hud.last_fed_at), PALETTE.cyan) : null}
    </Box>
  )
}

/** Two cells in one row from `cols` up, else one row each. */
function pairs(els: PaneEls, cells: RenderElement[], inner: number, wide: boolean): RenderElement[] {
  if (!wide) return cells
  const { Box } = els
  const half = Math.floor(inner / 2)
  const rows: RenderElement[] = []
  for (let i = 0; i < cells.length; i += 2) {
    rows.push(
      <Box flexDirection="row">
        <Box width={half}>{cells[i]}</Box>
        {i + 1 < cells.length ? <Box width={inner - half}>{cells[i + 1]}</Box> : null}
      </Box>,
    )
  }
  return rows
}

function session(els: PaneEls, m: PaneModel, inner: number, wide: boolean): RenderElement {
  const { Box } = els
  const ms = m.measure
  const cells: RenderElement[] = []
  if (ms.model !== null) cells.push(field(els, 'model ', ms.model, PALETTE.cyan))
  const dir = shortCwd(ms.cwd)
  if (dir !== null) cells.push(field(els, 'dir ', dir, PALETTE.cyan))
  const cost = costText(ms)
  if (cost !== null) cells.push(field(els, 'cost ', cost, PALETTE.yellow))
  const time = timeText(ms, m.nowMs)
  if (time !== null) cells.push(field(els, 'time ', time, PALETTE.cyan))
  const rows = pairs(els, cells, inner, wide)
  const ctx = ctxText(ms)
  if (ctx !== null) rows.push(field(els, 'ctx  ', ctx, PALETTE.cyan))
  return (
    <Box flexDirection="column">
      {rule(els, 'SESSION', inner)}
      {rows}
    </Box>
  )
}

function limits(els: PaneEls, m: PaneModel, inner: number): RenderElement {
  const { Box } = els
  const peak = m.frame?.hud?.peak
  const right: Span[] =
    peak !== undefined
      ? [
          { text: '◉ ', fg: PALETTE.magenta },
          { text: `${peak.label} `, fg: PALETTE.cyan },
          { text: peak.range, fg: PALETTE.yellow },
        ]
      : []
  return (
    <Box flexDirection="column">
      {rule(els, 'LIMITS', inner, right)}
      {limitRows(els, m.measure.rateLimits, m.nowMs)}
    </Box>
  )
}

/** The pane's whole tree: a pure function of its inputs. */
function paneTree(els: PaneEls, m: PaneModel): RenderElement {
  const { Box, Text } = els
  const cols = Math.max(0, Math.floor(m.cols))
  const inner = Math.max(1, cols - CHROME)
  const header = (
    <Box flexDirection="row" justifyContent="space-between">
      <Text color={PALETTE.magenta} bold wrap="truncate">
        PURRGOTCHI
      </Text>
      {cols >= KITTEN_ONLY ? (
        <Text color={PALETTE.dim} wrap="truncate">
          NETRUNNER
        </Text>
      ) : null}
    </Box>
  )
  const frameBox = (children: (RenderElement | null)[]) => (
    <Box flexDirection="column" borderStyle="round" borderColor={PALETTE.magenta} paddingX={1}>
      {header}
      {children}
    </Box>
  )

  if (m.status.kind === 'missing' || m.status.kind === 'outdated') {
    return frameBox([<Text wrap="wrap">{binaryProblem(m.status)}</Text>])
  }
  const f = m.frame
  if (f === null) return frameBox([<Text wrap="wrap">{NO_KITTEN}</Text>, buttons(els, m)])

  const sprite = spriteElement(els, m.surface, f, inner)
  const sideBySide = cols >= (sprite.isPixels ? SIDE_BY_SIDE_PIXELS : SIDE_BY_SIDE)
  const kitten = (
    <Box flexDirection={sideBySide ? 'row' : 'column'} {...(sideBySide && { columnGap: 3 })}>
      {sprite.el}
      {info(els, f)}
    </Box>
  )
  const speech =
    f.speech.length > 0 ? (
      <Text wrap="truncate">
        <Text color={PALETTE.magenta}>{'› "'}</Text>
        {f.speech.map(s => (
          <Text color={s.fg} bold={s.bold} dimColor={s.dim}>
            {s.text}
          </Text>
        ))}
        <Text color={PALETTE.magenta}>{'"'}</Text>
      </Text>
    ) : null
  if (cols < KITTEN_ONLY) return frameBox([kitten, speech, buttons(els, m)])

  return frameBox([
    kitten,
    speech,
    f.scene !== null ? renderLines(els, f.scene.rows) : null,
    session(els, m, inner, cols >= SIDE_BY_SIDE),
    limits(els, m, inner),
    buttons(els, m),
  ])
}

/** A button press: the command's own path, its answer as a toast (the pane has no transcript row). */
async function pressAction($: EngineInterface, rt: Runtime, verb: Verb): Promise<void> {
  let text: string
  try {
    text = (await act($, rt, verb)).text
  } catch (e) {
    debugLog($, `pane ${verb} failed: ${errText(e)}`)
    text = UNREACHABLE
  }
  show($, text, false)
}

type PaneEvent = RenderInput<'Pane'>

/** The pane's tree, or null for "let the engine draw it". Never throws. */
async function drawPane($: EngineInterface, rt: Runtime, e: PaneEvent): Promise<RenderElement | null> {
  try {
    const cols = e.props.bodyColumns
    if (!SURFACES.includes(e.surface) || typeof cols !== 'number' || !Number.isFinite(cols)) return null
    rt.paneColumns = cols
    const [frame, measure, status] = await Promise.all([readFrame($), readMeasure($), readBinary($)])
    const els = $.ui.resolve(e)
    return paneTree(els, {
      frame,
      measure,
      status,
      cols,
      nowMs: Date.now(),
      surface: e.surface,
      onAction: verb => {
        void pressAction($, rt, verb)
      },
    })
  } catch (err) {
    debugLog($, `pane render failed: ${errText(err)}`)
    return null
  }
}

/** `/purr`: an asked open, placed at any width (docked in fullscreen from 110 columns). */
async function openPane($: EngineInterface) {
  const r = await $.ui.open({ id: PANE_ID, title: TITLE, columns: PANE_COLUMNS, focus: true, closeOnEscape: true })
  if (r.isPlaced) return { text: 'Opened the Purrgotchi pane (p pet · t treat · y play).' }
  return { text: `Couldn't open the pane: ${r.reason}` }
}

/** The `.catch` of `/purr` (a top-level function: the module scan needs one). */
function openFailed() {
  return { text: "Couldn't open the pane." }
}

/**
 * The unasked open at session start (`dock` + `openPaneOnStart`): the engine
 * places it from 144 columns (110 once the person opened it), else holds it
 * undrawn until the terminal permits. Never throws.
 */
export async function autoOpenPane($: EngineInterface, rt: Runtime): Promise<void> {
  if (rt.options.layout !== 'dock' || !rt.options.openPaneOnStart) return
  try {
    await $.ui.open({ id: PANE_ID, title: TITLE, columns: PANE_COLUMNS })
  } catch (e) {
    debugLog($, `pane not opened: ${errText(e)}`)
  }
}

export function registerPane(on: On, rt: Runtime): void {
  on('ui.render', { component: 'Pane', requestId: PANE_ID }, async ($, e, next) => (await drawPane($, rt, e)) ?? next(e))
  on('command.run', { command: 'purr' }, $ => openPane($)).catch(openFailed)
}
