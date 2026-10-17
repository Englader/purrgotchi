// The `CommandOutput` row of `/purr-card`, `/purr-stats` and `/purr-hatch`:
// the stored coloured lines under the one-line summary. Read-only (a render
// hook may not write). Anything it does not recognise is the engine's own row.
import type { EngineInterface, On, RenderElement, RenderInput } from 'claude-code'

import { readOutput } from './atoms'
import { lineWidth, renderLines, SURFACES } from './lines'

type OutputEvent = RenderInput<'CommandOutput'>

/** The row's tree, or null for "let the engine draw it". Never throws. */
async function drawOutput($: Pick<EngineInterface, 'ui' | 'state'>, e: OutputEvent): Promise<RenderElement | null> {
  try {
    if (e.props.isErrored || !SURFACES.includes(e.surface)) return null
    const entry = await readOutput($, e.props.command, e.props.text)
    if (entry === null) return null
    const cols = e.viewport?.columns
    if (cols !== undefined && cols < Math.max(0, ...entry.lines.map(lineWidth))) return null
    const els = $.ui.resolve(e)
    const { Box, Text } = els
    return (
      <Box flexDirection="column">
        <Text dimColor>{e.props.text}</Text>
        {renderLines(els, entry.lines)}
      </Box>
    )
  } catch {
    return null
  }
}

export function registerOutputs(on: On): void {
  on('ui.render', { component: 'CommandOutput', props: { command: 'purr-card' } }, async ($, e, next) =>
    (await drawOutput($, e)) ?? next(e),
  )
  on('ui.render', { component: 'CommandOutput', props: { command: 'purr-stats' } }, async ($, e, next) =>
    (await drawOutput($, e)) ?? next(e),
  )
  on('ui.render', { component: 'CommandOutput', props: { command: 'purr-hatch' } }, async ($, e, next) =>
    (await drawOutput($, e)) ?? next(e),
  )
}
