// The opt-in Netrunner restyle (`restyle: true`, every layout, skin-only
// included): the person's own turns get a magenta bar, tool rows a neon tag
// chip in front of the engine's full row (output included), and the spinner
// says `▓▒░ jacking in`. Pure `ui.render` rewriting: any surface or props shape
// a hook does not recognise, and every expanded row, is the engine's own.
import type { EngineInterface, On, RenderElement, RenderInput } from 'claude-code'

import { PALETTE } from './hud'

type UserEvent = RenderInput<'UserMessage'>
type ToolEvent = RenderInput<'ToolUse'>

/** colors.rs `PURPLE` (rare): the chip of every tool without its own colour. */
const RARE_PURPLE = '#aa64ff'
const ROW_SURFACES: readonly string[] = ['terminal', 'desktop', 'vscode', 'mobile']
/** Spinner is raised on these alone. */
const SPINNER_SURFACES: readonly string[] = ['terminal', 'desktop', 'vscode']
/** Longer prompts keep the engine's row (its own capping and folding). */
const MAX_USER_TEXT = 4000
/** Below: a mobile chip is its short form. */
const NARROW_COLUMNS = 40
const OTHER_LABEL_MAX = 10

export const JACK_IN = '▓▒░ jacking in'

const CHIPS: Readonly<Record<string, { label: string; short: string; color: string }>> = {
  Read: { label: 'READ', short: 'R', color: PALETTE.cyan },
  Edit: { label: 'EDIT', short: 'E', color: PALETTE.magenta },
  Write: { label: 'WRITE', short: 'W', color: PALETTE.magenta },
  Bash: { label: 'BASH', short: 'B', color: PALETTE.yellow },
}

/** The tool's chip: its own, or the name upper-cased in rare purple. */
function chipOf(tool: string): { label: string; short: string; color: string } {
  const known = CHIPS[tool]
  if (known !== undefined) return known
  const label = tool.toUpperCase().slice(0, OTHER_LABEL_MAX)
  return { label, short: label.slice(0, 2), color: RARE_PURPLE }
}

/** A row the person typed: at the composer or through the Remote Control bridge. */
function isPersonRow(p: UserEvent['props']): boolean {
  const kind = typeof p.origin === 'object' && p.origin !== null ? p.origin.kind : undefined
  return (kind === 'composer' || kind === 'bridge') && p.task === undefined && p.from === undefined
}

/** The person's turn with a magenta bar, or null for "let the engine draw it". Never throws. */
function drawUser($: Pick<EngineInterface, 'ui'>, e: UserEvent): RenderElement | null {
  try {
    const p = e.props
    if (!ROW_SURFACES.includes(e.surface) || typeof p.text !== 'string' || p.isExpanded !== false) return null
    if (p.text.length > MAX_USER_TEXT || !isPersonRow(p)) return null
    const { Box, Text } = $.ui.resolve(e)
    return (
      <Box flexDirection="row">
        <Box flexShrink={0}>
          <Text color={PALETTE.magenta}>▌ </Text>
        </Box>
        <Box flexShrink={1}>
          <Text wrap="wrap">{p.text}</Text>
        </Box>
      </Box>
    )
  } catch {
    return null
  }
}

/**
 * The chip in front of the engine's own row (`inner`, output included: a
 * `ToolUse` has no `isExpanded`, so nothing may be hidden). Never throws.
 */
function drawTool($: Pick<EngineInterface, 'ui'>, e: ToolEvent, inner: RenderElement): RenderElement {
  try {
    const chip = chipOf(e.props.tool)
    const narrow = e.surface === 'mobile' && (e.viewport?.columns ?? Infinity) < NARROW_COLUMNS
    const { Box, Text } = $.ui.resolve(e)
    return (
      <Box flexDirection="row">
        <Box flexShrink={0}>
          <Text color={chip.color} bold inverse>{` ${narrow ? chip.short : chip.label} `}</Text>
          <Text> </Text>
        </Box>
        <Box flexDirection="column" flexGrow={1} flexShrink={1}>
          {inner}
        </Box>
      </Box>
    )
  } catch {
    return inner
  }
}

const isToolRow = (e: ToolEvent): boolean =>
  ROW_SURFACES.includes(e.surface) && typeof e.props.tool === 'string' && e.props.tool !== ''

export function registerSkin(on: On): void {
  // A state override (`message`) wins over the restyled word.
  on('ui.render', { component: 'Spinner' }, ($, e, next) =>
    SPINNER_SURFACES.includes(e.surface) && typeof e.props.word === 'string' && e.props.message === null
      ? next({ ...e, props: { ...e.props, word: JACK_IN } })
      : next(e),
  )
  on('ui.render', { component: 'UserMessage' }, ($, e, next) => drawUser($, e) ?? next(e))
  on('ui.render', { component: 'ToolUse' }, async ($, e, next) =>
    isToolRow(e) ? drawTool($, e, await next(e)) : next(e),
  )
}
