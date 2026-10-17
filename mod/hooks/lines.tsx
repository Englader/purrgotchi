// Draws the binary's `Span[][]` lines (card, stats) as one coloured block, and
// the labelled rows the pane and the band share.
import type { BoxProps, ElementConstructor, RenderElement, TextProps } from 'claude-code'

import { barPct, PALETTE, resetsIn } from './hud'
import type { MeasureView, Span } from '../types'

/** The surfaces the pane and the transcript rows draw on. */
export const SURFACES: readonly string[] = ['terminal', 'desktop', 'vscode', 'mobile']

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

/** A dim label then a bright value, one truncated row. */
export function field({ Text }: LineEls, label: string, value: string, color: string): RenderElement {
  return (
    <Text wrap="truncate" mono>
      <Text color={PALETTE.dim}>{label}</Text>
      <Text color={color}>{value}</Text>
    </Text>
  )
}

const LIMIT_LABELS: Readonly<Record<string, string>> = { five_hour: '5h', seven_day: '7d' }

/** One row per `five_hour` / `seven_day` rate limit with a usable figure; other kinds are skipped. */
export function limitRows(els: LineEls, rateLimits: MeasureView['rateLimits'], nowMs: number): RenderElement[] {
  const rows: RenderElement[] = []
  for (const r of rateLimits) {
    const label = LIMIT_LABELS[r.kind]
    if (label === undefined || !Number.isFinite(r.percentUsed)) continue
    const resets = resetsIn(r.resetsAt, nowMs)
    rows.push(field(els, label.padEnd(5), `${barPct(r.percentUsed)}${resets !== null ? ` · resets ${resets}` : ''}`, PALETTE.yellow))
  }
  return rows
}
