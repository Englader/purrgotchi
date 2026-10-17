// userConfig → typed options. The engine fills defaults, but a stored value can
// still be missing or the wrong type, so every field is narrowed here.

export type Layout = 'dock' | 'bottom' | 'classic' | 'off'
export type Quips = 'off' | 'session-model' | 'own-key'

export type PurrOptions = {
  binaryPath: string
  layout: Layout
  restyle: boolean
  openPaneOnStart: boolean
  toastStreaks: boolean
  quips: Quips
  devCommands: boolean
}

export const LAYOUTS: readonly Layout[] = ['dock', 'bottom', 'classic', 'off']
const QUIPS: readonly Quips[] = ['off', 'session-model', 'own-key']

const str = (v: unknown, fallback: string): string => (typeof v === 'string' ? v : fallback)
const bool = (v: unknown, fallback: boolean): boolean => (typeof v === 'boolean' ? v : fallback)
const pick = <T extends string>(v: unknown, allowed: readonly T[], fallback: T): T =>
  typeof v === 'string' && (allowed as readonly string[]).includes(v) ? (v as T) : fallback

export function readOptions(raw: Readonly<Record<string, unknown>>): PurrOptions {
  return {
    binaryPath: str(raw.binaryPath, ''),
    layout: pick(raw.layout, LAYOUTS, 'dock'),
    restyle: bool(raw.restyle, false),
    openPaneOnStart: bool(raw.openPaneOnStart, true),
    toastStreaks: bool(raw.toastStreaks, false),
    quips: pick(raw.quips, QUIPS, 'own-key'),
    devCommands: bool(raw.devCommands, false),
  }
}

/** `layout: 'off'` means no game: no hooks touch the binary at all. */
export function gameEnabled(o: PurrOptions): boolean {
  return o.layout !== 'off'
}
