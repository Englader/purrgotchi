// Pure HUD formatters and the chrome palette shared by the pane, the band and
// the status entry. No `$`, no game rules: those come from the binary.
import type { FrameJson } from '../types'

/** `colors.rs`'s palette, verbatim (chrome only; game colours come from the binary). */
export const PALETTE = {
  magenta: '#ea00d9', // frame, accents
  cyan: '#0abdc6', // primary data
  yellow: '#f3f31b', // money, limits
  purple: '#711c91', // section rules
  violet: '#b45cff', // section rules
  mint: '#57eca6', // good
  red: '#ff5468', // danger
  dim: '#6d6488', // labels
} as const

const finiteOr0 = (n: number): number => (Number.isFinite(n) ? n : 0)

/** 10 segments of `▰` / `▱`; NaN or negative is empty, over 100 is full. */
export function bar(pct: number): string {
  const filled = Math.min(10, Math.max(0, Math.round(finiteOr0(pct) / 10)))
  return '▰'.repeat(filled) + '▱'.repeat(10 - filled)
}

/** `12s ago`, `4m ago`, `3h ago`, `2d ago`; a time ahead of now is `just now`. */
export function ago(secs: number): string {
  if (!(secs >= 0)) return 'just now'
  const s = Math.floor(secs)
  if (s < 60) return `${s}s ago`
  if (s < 3600) return `${Math.floor(s / 60)}m ago`
  if (s < 86_400) return `${Math.floor(s / 3600)}h ago`
  return `${Math.floor(s / 86_400)}d ago`
}

const pad2 = (n: number): string => String(n).padStart(2, '0')

/** `45m 0s`, `2h 05m`, `3d 4h`. */
export function duration(ms: number): string {
  const s = Math.floor(Math.max(0, finiteOr0(ms)) / 1000)
  if (s < 3600) return `${Math.floor(s / 60)}m ${s % 60}s`
  if (s < 86_400) return `${Math.floor(s / 3600)}h ${pad2(Math.floor((s % 3600) / 60))}m`
  return `${Math.floor(s / 86_400)}d ${Math.floor((s % 86_400) / 3600)}h`
}

/** Time to a window's reset: `37m`, `2h 00m`, `4d 15h`, `<1m`; null when unknown. */
export function resetsIn(resetsAt: string | undefined, nowMs: number): string | null {
  if (resetsAt === undefined) return null
  const at = Date.parse(resetsAt)
  if (!Number.isFinite(at)) return null
  const mins = Math.floor((at - nowMs) / 60_000)
  if (!(mins >= 1)) return '<1m'
  if (mins < 60) return `${mins}m`
  if (mins < 1440) return `${Math.floor(mins / 60)}h ${pad2(mins % 60)}m`
  return `${Math.floor(mins / 1440)}d ${Math.floor((mins % 1440) / 60)}h`
}

/** `87k`, `1.2M`; null when unknown. */
export function kTokens(n: number | null): string | null {
  if (n === null || !Number.isFinite(n)) return null
  if (n < 1000) return String(Math.max(0, Math.round(n)))
  const k = Math.round(n / 1000)
  if (k < 1000) return `${k}k`
  return `${(n / 1_000_000).toFixed(1).replace(/\.0$/, '')}M`
}

/** The last two path segments (`/` or `\` separated), joined by `/`. */
export function shortCwd(p: string | null): string | null {
  if (p === null) return null
  const parts = p.split(/[\\/]+/).filter(x => x !== '')
  return parts.length === 0 ? p : parts.slice(-2).join('/')
}

/** The one-line status entry (spec §4.8); undefined clears it. */
export function statusText(f: FrameJson | null): string | undefined {
  if (f === null) return undefined
  return `=^.^= ${f.name.toLowerCase()} lv${f.level} · ${f.coins}¢ · ${f.mood}`
}
