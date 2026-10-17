// Band options from env: the cat's height and the stream's frame rate.
import type { EngineInterface } from 'claude-code'

/** Band heights in terminal rows (6, 8 or 12 px). */
export type Rows = 3 | 4 | 6

export type BandOptions = { rows: Rows; fps: number }

/** `PURR_BAND_ROWS`: 3, 4 or 6; anything else (or unset) is 4. */
export function parseRows(v: string | undefined): Rows {
  const n = Number(v?.trim())
  return n === 3 || n === 6 ? n : 4
}

/** `PURR_BAND_FPS`: whole frames per second, 15..60; unset or NaN is 30. */
export function parseFps(v: string | undefined): number {
  const n = v === undefined || v.trim() === '' ? NaN : Number(v)
  return Number.isNaN(n) ? 30 : Math.min(60, Math.max(15, Math.round(n)))
}

export async function readOptions($: Pick<EngineInterface, 'env'>): Promise<BandOptions> {
  return {
    rows: parseRows(await $.env.get('PURR_BAND_ROWS')),
    fps: parseFps(await $.env.get('PURR_BAND_FPS')),
  }
}
