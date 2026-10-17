// Every `$.state` atom, in one place. Atoms hold view state only; the binary
// owns all durable state. Keys must be declared in types/index.d.ts.
//
// The engine's module scan (2.1.302) needs every atom's `plugin` and `key` as
// string literals (not a shared const), and accepts `update` / `read` only
// on an atom defined in the same file. So the writers (and later the readers)
// live here too, and other modules call them instead of importing atoms.
import { atom, read, update } from 'claude-code'
import type { StateDollar } from 'claude-code'

import type { BinaryStatus, FrameJson, MeasureView, OutputEntry } from '../types'

export const binaryAtom = atom({ plugin: 'purrgotchi', key: 'binary' } as const, { kind: 'unknown' } as BinaryStatus)

export async function writeBinary($: StateDollar, status: BinaryStatus): Promise<void> {
  await update($, binaryAtom, () => status)
}

// Full-card rows of `/purr-card`, `/purr-stats` and `/purr-hatch`: the command's
// one-line `text` stays what the model reads; the coloured lines are drawn by
// the `CommandOutput` render hook, found again by `outputKey`.
export const outputsAtom = atom({ plugin: 'purrgotchi', key: 'outputs' } as const, [] as OutputEntry[])

/** The newest rows kept; older ones fall back to the engine's plain text. */
export const OUTPUT_CAP = 20

/** A row's identity: the summary carries the numbers, so a later run is a new row. */
export function outputKey(command: string, text: string): string {
  return `${command}\u0000${text}`
}

/** Stores a row (the newest of its key), keeping at most `OUTPUT_CAP`. */
export async function writeOutput($: StateDollar, entry: OutputEntry): Promise<void> {
  await update($, outputsAtom, list => [...list.filter(x => x.key !== entry.key), entry].slice(-OUTPUT_CAP))
}

/** The stored row for a command's printed `text`, or null. In a render, this subscribes it. */
export async function readOutput($: StateDollar, command: string, text: string): Promise<OutputEntry | null> {
  const key = outputKey(command, text)
  return (await read($, outputsAtom)).find(x => x.key === key) ?? null
}

// The ticker's latest `frame --json` (null: no kitten yet). The pane, band and
// status entry only read it.
export const frameAtom = atom({ plugin: 'purrgotchi', key: 'frame' } as const, null as FrameJson | null)

export async function writeFrame($: StateDollar, frame: FrameJson | null): Promise<void> {
  await update($, frameAtom, () => frame)
}

export const EMPTY_MEASURE: MeasureView = {
  contextPct: null,
  contextTokens: null,
  contextWindow: null,
  rateLimits: [],
  costUsd: null,
  model: null,
  cwd: null,
  startedAt: null,
}

// Session figures: seeded at session start, then pushed by `session.measure`.
export const measureAtom = atom({ plugin: 'purrgotchi', key: 'measure' } as const, EMPTY_MEASURE)

/** Merges `patch` into the stored figures. */
export async function writeMeasure($: StateDollar, patch: Partial<MeasureView>): Promise<void> {
  await update($, measureAtom, m => ({ ...m, ...patch }))
}
