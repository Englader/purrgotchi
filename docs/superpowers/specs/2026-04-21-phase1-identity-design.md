# Purrgotchi Phase 1 — Identity Foundation Design

_Date: 2026-04-21 | Target: v3.4.0_

---

## Overview

Phase 1 turns Purrgotchi from a mood-rotating sprite into a **character** with a deterministic identity: breed, rarity, five feline stats, and a personality blurb. Two new CLI commands ship (`hatch`, `card`). The statusline gains a rarity indicator. Existing state is wiped on upgrade (wipe-and-rehatch migration, no preservation).

---

## Decisions locked

- **Cat-only breeds** — all 16 breeds are felines. Other species deferred or never.
- **Stat floors apply to middle 3 stats only** — the dump stat can still roll `0..=20` regardless of rarity. Legendaries have one real vulnerability.
- **Full 320 personality blurbs** — one unique blurb per (breed × peak_stat × dump_stat) combination. Stored as a flat lookup table in `identity.rs`. Blurb assembled once at hatch, persisted in state.json.
- **Sprites: tier-shared for Phase 1** — one sprite set per rarity tier (5 sets). Breeds within a tier share sprites. Full breed-specific sprites drawn in Phase 2.
- **Architecture: layered (B)** — `identity.rs` (data), `hatch.rs` (roll logic), `card.rs` (renderer). State stays flat JSON.

---

## Breed table

| Rarity | Breeds |
|---|---|
| Common (60%) | Tabby, Tuxedo, Calico, Ginger, Grey Shorthair |
| Uncommon (25%) | Siamese, Russian Blue, Bengal |
| Rare (10%) | Maine Coon, Norwegian Forest, Turkish Van |
| Epic (4%) | Sphinx, Scottish Fold |
| Legendary (1%) | Void Cat, Snow Leopard Cat, Chimera |

Shiny: independent 1% chance on any tier. Modifier, not a separate breed.

---

## Stats

Five feline stats, indexed 0–4:

| Index | Stat |
|---|---|
| 0 | CURIOSITY |
| 1 | MISCHIEF |
| 2 | LAZINESS |
| 3 | AFFECTION |
| 4 | WISDOM |

**Stat floors by rarity** (applied to the 3 middle stats only — NOT the dump stat):

| Rarity | Floor |
|---|---|
| Common | 5 |
| Uncommon | 15 |
| Rare | 25 |
| Epic | 35 |
| Legendary | 50 |

**Roll algorithm:**
1. Pick peak stat index at random → roll `rand(80..=100)`
2. Pick dump stat index (different from peak) → roll `rand(0..=20)`
3. Roll remaining 3 stats → `rand(floor..=80)` using rarity floor

---

## Hatch algorithm

Inputs: `username` (env `USER`/`USERNAME`), `hostname` (env `COMPUTERNAME`/`HOSTNAME`), `birth_time` (current unix timestamp).

1. Seed = FNV-1a hash of `"{username}:{hostname}:{birth_time}"`
2. Seed a deterministic LCG RNG (no extra crate — implemented inline)
3. Roll rarity via cumulative thresholds: `[0,60)` = common, `[60,85)` = uncommon, `[85,95)` = rare, `[95,99)` = epic, `[99,100)` = legendary
4. Pick breed index within rarity tier's breed list
5. Roll shiny: independent `rand(0..=99) == 0`
6. Roll stats per algorithm above
7. Look up personality blurb: `BLURBS[breed_idx][peak_idx][dump_idx]`
8. Store everything in `state.json` version 2

**Dev backdoor flags** (skip the roll, force values):
- `purrgotchi hatch --rarity legendary --breed "void cat" --shiny`
- Env vars: `PURR_FORCE_RARITY`, `PURR_FORCE_BREED`, `PURR_FORCE_SHINY`

---

## State schema

Version bumps from 1 → 2. New fields added:

```
version: 2
breed: "Void Cat"
rarity: "legendary"
shiny: true
stats: [85, 42, 8, 61, 93]   // [CURIOSITY, MISCHIEF, LAZINESS, AFFECTION, WISDOM]
personality: "..."             // full blurb string, stored at hatch
```

**Migration:** `load()` checks `version`. If `version == 1` (or field missing), returns `None`. Caller re-hatches. No partial migration.

---

## Module structure

| File | Responsibility |
|---|---|
| `identity.rs` | Breed table, stat constants, rarity floors, star strings, 320 blurb lookup table |
| `hatch.rs` | Roll logic — FNV hash, LCG RNG, rarity draw, stat generation, blurb lookup, dev backdoor parsing |
| `card.rs` | `purrgotchi card` renderer |
| `state.rs` | Extended State struct + version-check migration |
| `main.rs` | Two new match arms: `"card"` and `"hatch"` |
| `statusline.rs` | Reads `rarity` + `shiny` from state; renders RARITY column in H6 |

---

## `purrgotchi card` output format

```
╔══════════════════════════════════════════╗
║  VOID CAT  //  Shadowmere  //  ★★★★★    ║
║  [SHINY]   Born: 2026-04-21              ║
╠══════════════════════════════════════════╣
║                                          ║
║    /\_____/\                             ║
║   ( ◉     ◉ )                            ║
║    (  ___  )                             ║
║     \_____/                              ║
║                                          ║
╠══════════════════════════════════════════╣
║  CURIOSITY  ████████░░  85               ║
║  MISCHIEF   ████░░░░░░  42               ║
║  LAZINESS   ░░░░░░░░░░   8               ║
║  AFFECTION  ██████░░░░  61               ║
║  WISDOM     █████████░  93               ║
╠══════════════════════════════════════════╣
║  "Born from the space between stars,     ║
║   Shadowmere sees what others miss.      ║
║   Moving, however, is another matter."   ║
╚══════════════════════════════════════════╝
```

- `[SHINY]` line omitted if shiny=false
- Stars use Unicode `★`/`☆` (no nerd font required)
- Personality blurb wraps to fit the box width
- Breed sprite in card can be larger than the 16-char statusline sprite

---

## HUD integration — H6 RARITY column

H6 row's currently-empty W3 column (28 chars) gains a rarity indicator:

```
║ LV: 12 │ COINS: 847 ║ MOOD: calm │ FED: 3m ago ║ RARITY: ★★★★★  ║
```

With shiny:
```
║ LV: 12 │ COINS: 847 ║ MOOD: calm │ FED: 3m ago ║ RARITY: ✦★★★★★ ║
```

Star strings per rarity:
- Common: `★☆☆☆☆`
- Uncommon: `★★☆☆☆`
- Rare: `★★★☆☆`
- Epic: `★★★★☆`
- Legendary: `★★★★★`

Shiny adds `✦` prefix. Falls back to empty string if state has no rarity field (fresh install before first hatch).

---

## Sprite strategy for Phase 1

One sprite set per rarity tier. Breeds within a tier share sprites. Existing sprites serve as common tier. Four new tier sets drawn for Phase 1:

| Tier | Representative look |
|---|---|
| Common | Existing sprites (tabby/generic cat) |
| Uncommon | Sleeker, more defined silhouette |
| Rare | Larger, fluffier (Maine Coon style) |
| Epic | Alien/unusual (Sphinx — no fur lines, prominent ears) |
| Legendary | Striking solid form with glowing eyes (Void Cat style) |

Each tier set covers all 6 moods × up to 2 frames = up to 12 sprite files per tier. Common = existing 11 files. Four new tiers × 12 files = 48 new sprite files maximum.

Full breed-specific sprites deferred to Phase 2.

---

## Personality blurb data

320 blurbs stored as a 3D array in `identity.rs`:

```
BLURBS[breed_idx (0..=15)][peak_idx (0..=4)][dump_slot (0..=3)]
```

`dump_slot` indexes the 4 non-peak stats in ascending stat-index order. Example: if peak=WISDOM (4), dump_slot 0=CURIOSITY, 1=MISCHIEF, 2=LAZINESS, 3=AFFECTION. This gives a dense `[16][5][4]` array with no invalid entries.

Each blurb is a standalone string ~2-3 sentences. No template assembly — each is unique. Written to match the breed's voice and the peak/dump contrast.

Example index `[14][4][2]` = Void Cat, peak WISDOM, dump LAZINESS:
> "Born from the space between stars, Shadowmere sees what others miss. Their wisdom borders on the eerie—they often know before you ask. Moving, however, is a different matter entirely."

---

## What Phase 1 does NOT include

- Name coloring by rarity (Phase 2 — B3)
- Bond mechanics, interactions (`pet`, `play`, `treat`) (Phase 2 — C series)
- Sprite evolution by level (Phase 2 — B1)
- LLM-generated blurbs or speech (Phase 3)
- Full breed-specific sprite sets (Phase 2)
