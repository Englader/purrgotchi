// Draws the binary's `Span[][]` lines (card, stats) as one coloured block.
import type { BoxProps, ElementConstructor, RenderElement, TextProps } from 'claude-code'

import type { Span } from '../types'

/** The two elements every surface carries. */
export type LineEls = { Box: ElementConstructor<BoxProps>; Text: ElementConstructor<TextProps> }

/** Cells in a line: characters, not bytes. */
export function lineWidth(line: Span[]): number {
  return line.reduce((n, s) => n + [...s.text].length, 0)
}

/**
 * One `Text` per line, one nested `Text` per span. The terminal ignores `mono`
 * (it is always monospace); desktop draws its monospaced face. An empty line
 * keeps its row as a single space.
 */
export function renderLines({ Box, Text }: LineEls, lines: Span[][]): RenderElement {
  return (
    <Box flexDirection="column">
      {lines.map(line => (
        <Text mono>
          {line.length === 0
            ? ' '
            : line.map(s => (
                <Text color={s.fg} bold={s.bold} dimColor={s.dim} mono>
                  {s.text}
                </Text>
              ))}
        </Text>
      ))}
    </Box>
  )
}
