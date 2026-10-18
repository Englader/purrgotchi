// Purrgotchi mod types: the NDJSON lines `purrgotchi band` streams.
// (no `export {}`, no `declare module`; the engine lays this file as the
// plugin's type root).
//
// Contract rules: one JSON object per line; accept only `v === 1`; ignore
// unknown keys.

/** First line: the stream's effective geometry and where the state lives. */
export type BandHello = { v: 1; hello: true; cols: number; rows: number; fps: number; state_dir: string }

/** One frame: `c` is base64 Raster cells (rows x cols x three LE u32). */
export type BandFrame = { v: 1; f: number; t: number; c: string }

/** A state read failure (at most once a second); the stream keeps running. */
export type BandError = { v: 1; ok: false; error: 'no_kitten' | 'state_unavailable' | 'state_corrupt'; message: string }

export type BandLine = BandHello | BandFrame | BandError
