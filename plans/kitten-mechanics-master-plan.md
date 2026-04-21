# Purrgotchi Kitten Mechanics Master Plan

_Started: 2026-04-21 — starting point: v3.3.0_

This is a resumable, multi-session roadmap for turning the Purrgotchi kitten from a mood-rotating sprite into a **character**: deterministic identity, breed, rarity, five stats, personality blurb, stat card, interaction commands, events, and eventually LLM-generated in-character speech.

Inspired by Anthropic's leaked Claude Code `/buddy` feature (April 2026) — we borrow the deterministic-roll + stats + rarity architecture, but stay strictly cat-themed ("purr" is the brand, no 18-species zoo).

---

## Guiding decisions (locked)

- **Wipe & re-hatch migration** — on v3.4.0 rollout, existing state.json schemas are detected and triggers a re-hatch. User starts from level 0 with a new deterministic roll. No attempt to preserve Thalia's history.
- **Cat-only** — all breeds are felines. No species diversity.
- **LLM is optional** — all features ship with static-table fallbacks. LLM-powered variants are gated behind `ANTHROPIC_API_KEY` and degrade gracefully when absent.
- **Dev backdoor** — `purrgotchi hatch --rarity legendary --shiny --breed "void cat"` lets the user override the roll at hatch. Also supports env-var form (`PURR_FORCE_RARITY=legendary`).
- **Deterministic first roll** — `hash(username + hostname + birth_time)` seeds the roll. Same machine = same kitten forever, unless explicitly re-hatched.
- **No stat progression** — like Buddy, the 5 stats are fixed at hatch. `level` and `coins` keep progressing via tool calls, but the stats are identity, not growth.

---

## Phase roadmap

### Phase 1 — Identity foundation (v3.4.0)

**Ships as one atomic state-schema bump.** All R items land together because they share the state.json migration.

| ID | Item |
|---|---|
| R1 | Deterministic hatch roll (hash of user+host+birth_time seeds RNG) |
| R2 | Breed tiers + rarity distribution (common 60% / uncommon 25% / rare 10% / epic 4% / legendary 1%, plus 1% shiny on any tier) |
| R3 | Five feline stats (CURIOSITY, MISCHIEF, LAZINESS, AFFECTION, WISDOM) with peak-and-dump distribution; stat floors rise with rarity |
| R4 | Static personality blurb table — picked by (breed, peak stat, dump stat). LLM override deferred to Phase 3. |
| R5 | `purrgotchi card` subcommand — prints the stat card (name, breed, rarity stars, stat bars, personality, birth date) |
| R6 | `purrgotchi hatch` subcommand with dev backdoor (`--rarity`, `--shiny`, `--breed`, env-var equivalents) |

**Also in Phase 1:**
- State schema migration — old-schema detection triggers re-hatch
- Sprite coverage — new ASCII sprites for each rarity tier (minimum 2 frames per tier, can share within tier)
- HUD integration — rarity stars render in the title row next to the kitten's name
- CHANGELOG + README updates

### Phase 2 — Character depth (v3.4.x point releases)

**Iterative.** Each item is its own point release. Pick order during the phase.

| ID | Item |
|---|---|
| B1 | Sprite evolution by level tiers (kitten → young cat → adult — 3 tiers) |
| B2 | Level-up celebration — one-shot `levelup` mood overriding sprite + speech for ~3 renders |
| B3 | Name color by rarity (common=cyan, uncommon=green, rare=purple, epic=gold, legendary=rainbow-cycle, shiny=animated) |
| C1 | `purrgotchi pet` — +1 bond, brief happy mood |
| C2 | `purrgotchi play` — costs coins, +bond, triggers excited mood |
| C3 | `purrgotchi treat` — resets hunger on demand, costs coins |
| C4 | `purrgotchi stats` — full state.json pretty-printed |
| C5 | `bond` stat — grows via interactions, decays when neglected |
| D1 | Tool-call streak detection — "5 in a row!" one-shot speech |
| D2 | Milestone events (100 / 500 / 1000 total tool calls) — unique speech per milestone |
| D3 | Birthday event — on anniversary of `birth_time`, special sprite/speech for the day |
| D4 | Daily greeting — first-prompt-of-day detection → "welcome back" |
| D5 | Rare random drops — low-probability "found a coin! +10" bonus |

### Phase 3 — LLM-powered in-character speech (v3.5.0)

| ID | Item |
|---|---|
| L1 | Async speech generation on `UserPromptSubmit` hook: reads transcript tail + `{name, breed, rarity, stats, mood, personality}` → Haiku call → 5 mood-appropriate in-character phrases → cache file |
| L2 | Statusline reads cache; falls back to static phrases if cache missing / stale (>10 min) / no API key |
| L3 | `ANTHROPIC_API_KEY` env var required; without it, static speech only (no crash, no prompt) |
| L4 | Debug-log LLM call latency + token usage for visibility |
| L5 | LLM personality blurb at hatch — replaces R4's static-table entry for users with an API key |

---

## Workflow per item

For each item or bundle:

1. `/superpowers:brainstorming` — refine that item's design (can skip for tiny items)
2. Write spec to `plans/<phase>-<item>.md` (not committed — working doc)
3. `/superpowers:writing-plans` — turn spec into implementation plan
4. `/superpowers:executing-plans` OR `/superpowers:subagent-driven-development` — build it
5. `/superpowers:verification-before-completion` — verify before merge
6. `/superpowers:requesting-code-review` — review
7. Commit → tag → push → `/superpowers:finishing-a-development-branch`
8. Update this master plan's progress log
9. Next item

---

## Progress log

_Update as items ship._

- [ ] Phase 1 — Identity foundation (v3.4.0)
  - [ ] R1 deterministic roll
  - [ ] R2 breed + rarity
  - [ ] R3 five stats
  - [ ] R4 static personality blurb
  - [ ] R5 `purrgotchi card`
  - [ ] R6 `purrgotchi hatch` + backdoor
  - [ ] State schema migration
  - [ ] Rarity sprite set
  - [ ] HUD rarity stars
  - [ ] CHANGELOG + README
  - [ ] Tag v3.4.0 + push

- [ ] Phase 2 — Character depth (v3.4.x)
  - B/C/D items TBD per iteration

- [ ] Phase 3 — LLM-powered speech (v3.5.0)
  - [ ] L1-L5 as one bundle

---

## Resume protocol

When starting a new session on this project:

1. Read this file
2. Check the **Progress log** for the next unchecked item
3. `git log --oneline --tags` to confirm what's already shipped (tags v3.4.0, v3.4.1 … v3.5.0 mark milestones)
4. If a per-item spec exists in `plans/`, continue from it
5. Otherwise `/superpowers:brainstorming` the next item

---

## Open design questions

_Resolve as items come up for brainstorming._

- **Phase 1 — breed table size**: How many breeds per rarity tier? (Current rough plan: common 5, uncommon 3, rare 3, epic 2, legendary 3 — total ~16 breeds. Each needs a name and at minimum shares sprites with its tier.)
- **Phase 1 — stat floors**: Buddy uses floor values of 5/15/25/35/50 for Common/Uncommon/Rare/Epic/Legendary. Adopt as-is, or tune for purrgotchi?
- **Phase 1 — peak/dump math**: Exact algorithm? (Proposed: pick 1 stat → `rand_in(80..=100)`, pick different stat → `rand_in(0..=20)`, rest → `rand_in(floor..=80)`.)
- **Phase 1 — sprite budget**: New ASCII sprites per rarity tier — how many frames, how different? Cheapest approach = keep existing sprites as "common tabby", add 4 new tier sprites.
- **Phase 2 — bond decay rate**: If we add bond, how fast does it decay with inactivity?
- **Phase 3 — LLM cache staleness**: Is 10 min the right TTL? Should it invalidate on mood change?
- **Phase 3 — transcript context length**: How many lines of transcript tail to include in the LLM prompt? (Token cost vs. relevance tradeoff.)
