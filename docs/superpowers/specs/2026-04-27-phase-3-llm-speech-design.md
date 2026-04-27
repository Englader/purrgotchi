# Phase 3 — LLM-powered in-character speech (v3.5.0)

_Designed: 2026-04-27. Bundles items L1–L5 from the kitten mechanics master plan._

## Goal

Give the kitten an occasional in-character voice that **reacts to what the user
just did** in Claude Code, filtered through her identity (name, breed, rarity,
peak/dump stats, personality blurb).

Static speech remains the always-on default. The LLM is occasional flavor — not
the engine. The user should never feel a slowdown, never see an error, never
need to know the LLM exists if they don't have an API key.

## Locked decisions

| # | Decision | Rationale |
|---|---|---|
| 1 | LLM reacts to **transcript context filtered through personality** | User-stated UX goal: "react to what I'm doing, but tailored to her personality" |
| 2 | Reactions are **probabilistic, ~1-in-5 prompts**, not every prompt | Cats don't comment continuously. Cheaper. Less intrusive. |
| 3 | Transcript scope: last user message + last assistant message + last 5 tool calls, **capped at ~2K input tokens** | Enough context for substantive reactions. Fits Approach C from brainstorming. |
| 4 | Output cap: **~50 tokens, one short line ≤ 60 chars** | Speech bubble is a single line. Quips should land fast. |
| 5 | Quip is a **transient overlay**, displayed for **6 renders** (~6s), then static resumes | Matches existing `LevelUp` celebration vocabulary. |
| 6 | LLM call is **fully detached from the hot path** (Approach A: `purrgotchi react` subprocess) | UserPromptSubmit must stay <50ms. |
| 7 | Hatch (L5) calls Haiku **synchronously**, with "✨ summoning personality…" stderr line | Hatch is a one-time event; ~1–2s blocking is part of the "summoning" feel. |
| 8 | `ANTHROPIC_API_KEY` env var **gates all LLM paths**. Absent → silent static fallback. No prompts, no errors. | L3 from master plan. |
| 9 | Personality blurb storage uses literal `{name}` placeholder, **substituted at render time** | Fixes the L5 known-quirk: renaming a kitten today leaves the old name baked in. |
| 10 | No `state.json` schema bump | Quip cache is a separate file. Personality field stays a String. All-additive. |
| 11 | HTTP client: **`ureq`** (blocking, no async runtime, ~12 deps, rustls for HTTPS) | Matches "small fast Rust binary" character. |
| 12 | Priority of speech bubble: **Celebration > Birthday > LLM-quip > Mood** | Hand-written milestone copy beats whatever Haiku improvises. |
| 13 | Quip cache lives at `~/.claude/purrgotchi/llm-quip.json`, **separate from `state.json`** | Async worker mustn't race `feed`/`pet`/`play`/`treat` writers. |
| 14 | Quips older than 5 minutes (`generated_at`) are dropped by `consume` | Stale quip from a previous Claude Code session shouldn't leak in. |
| 15 | Existing kittens with pre-substituted personality are **not retroactively migrated** | Render-time `{name}` substitution is a no-op for them; the rename bug stays for old kittens but the fix lands for new hatches. |

## Architecture

Three independent code paths share `llm.rs`:

```
                    ┌─────────────────────────────────┐
                    │  llm.rs   (new shared module)   │
                    │  - http_post_haiku()            │
                    │  - api_key()                    │
                    │  - HaikuRequest / HaikuResponse │
                    │  - LlmError                     │
                    └────────┬────────────────────────┘
                             │ used by
            ┌────────────────┼────────────────┐
            │                │                │
       L1 path           L5 path           L4: debug::log
       react cmd         hatch cmd          inside both
```

### L1 — per-turn reaction (the hot path)

```
Claude Code: user submits prompt
   ↓ UserPromptSubmit hook
purrgotchi turn  (target <50ms total)
   - existing: streak reset, greeting check, last_turn_at update, save_atomic
   - NEW: if api_key().is_some()
          && Lcg(now ^ hash(session_id)).roll_1_in_5()
          && transcript_path is non-empty:
        spawn detached `purrgotchi react <transcript_path>`,
        drop the Child handle, parent exits

   ──── meanwhile, in another OS process ────

purrgotchi react <transcript_path>   (0.5–2s, no one's waiting)
   - api_key() check (defense in depth)
   - state::load() → identity inputs
   - mood = best-effort guess from state alone (last_fed/pet/play/treat deltas).
            Cannot derive `panic`/`alert` here — those need context %, which the
            worker doesn't have. Defaults to calm/happy/hungry/sleep. The system
            prompt is written so the quip stays sensible across mood.
   - read_transcript_tail(path) → last user msg + last assistant msg
                                + last 5 tool calls, ≤8KB final string
   - build_prompt(state, mood, tail)
   - llm::call_haiku(req, max_tokens=50, timeout=10s)
   - on Ok: sanitize → quip_cache::write
   - on Err: debug::log, exit 0
   - exit 0
```

```
purrgotchi statusline   (every render)
   - existing render path
   - speech selection chain:
       celebration_kind != None → compute_celebration_speech
       else if is_birthday      → compute_birthday_speech
       else if let Some(q) = quip_cache::consume()  → q (in C_PINK)
       else                     → compute_speech(mood, now)
```

`quip_cache::consume()` reads the quip, returns the text if `renders_remaining > 0`,
decrements, atomic-writes back. Deletes the file when count hits 0 or `generated_at`
is older than 5 min.

### L5 — personality blurb at hatch (sync, one-shot)

```
purrgotchi hatch  (existing breed/rarity/stats roll)
   - if !api_key().is_some() OR PURR_FORCE_STATIC_BLURB env set:
        personality = static table lookup (existing 320-entry path)
   - else:
        eprintln!("✨ summoning personality…")
        prompt = "Write a 1-2 sentence personality blurb for {name},
                  a [rarity] [breed] cat with peak [PEAK] and dump [DUMP].
                  Use the literal token '{name}' — don't substitute."
        match llm::call_haiku(req, max_tokens=80, timeout=10s):
          Ok(resp) → if validate_placeholder_or_no_proper_noun(resp.text):
                       personality = resp.text.trim()
                     else: fall through to static
          Err(_)   → static fallback
   - state stores personality string with `{name}` literal
   - card / stats display does s.personality.replace("{name}", &s.name) at render
```

### L4 — debug logging

Both paths emit one line to existing `debug::log` per call:

```
[llm] react  ok    latency_ms=842   in_tok=1530  out_tok=18  model=claude-haiku-4-5
[llm] react  fail  latency_ms=10003 err=Timeout
[llm] hatch  ok    latency_ms=624   in_tok=42    out_tok=64  model=claude-haiku-4-5
```

## Components

### New files

- **`src/llm.rs`** — shared HTTP + API surface
  - `pub fn api_key() -> Option<String>` — reads `ANTHROPIC_API_KEY`, returns `None` on missing/empty
  - `pub fn call_haiku(req: HaikuRequest) -> Result<HaikuResponse, LlmError>` — blocking POST to `https://api.anthropic.com/v1/messages` via `ureq` (10s connect + 10s read timeout)
  - `HaikuRequest { system: String, user: String, max_tokens: u32 }`
  - `HaikuResponse { text: String, input_tokens: u32, output_tokens: u32, latency_ms: u64 }`
  - `LlmError { MissingKey, Network(String), Http(u16), MalformedResponse, Timeout }`
  - All errors are non-fatal at the call site.

- **`src/react.rs`** — L1 worker subcommand
  - `pub fn run(transcript_path: &str)` — top-level entry for `purrgotchi react`
  - `fn read_transcript_tail(path: &str) -> Option<TranscriptTail>` — parses JSONL from end-of-file backward; collects last user msg + last assistant msg (text only, strip tool-use blocks) + last 5 tool calls; truncates each chunk; final string capped at 8KB
  - `fn build_prompt(state: &State, mood: &str, tail: &TranscriptTail) -> HaikuRequest`
  - `fn sanitize_quip(raw: &str) -> String` — first line, strip surrounding quotes, replace newlines with space, truncate at word boundary ≤ 80 chars. (60 chars is the target told to Haiku in the system prompt; 80 is a defensive enforced cap in case Haiku overshoots.)

- **`src/quip_cache.rs`** — file at `~/.claude/purrgotchi/llm-quip.json`
  - `Quip { quip: String, generated_at: u64, renders_remaining: u8 }`
  - `pub fn write(quip: &str)` — atomic temp-file + rename; sets `renders_remaining = 6`, `generated_at = now`
  - `pub fn read() -> Option<Quip>` — None on missing/malformed
  - `pub fn consume() -> Option<String>` — returns quip text + decrements + atomic-write; deletes file on `renders_remaining == 0` or stale (>5 min)

### Modified files

- **`src/main.rs`** — add `react <transcript>` subcommand → `react::run`
- **`src/hooks.rs`** — `turn_inner` is changed to read stdin (it currently doesn't) and parse the `Payload` for `transcript_path` and `session_id`; rolls dice; on hit spawns detached `purrgotchi react` (drops the `Child`). The existing turn behaviour (streak reset, greeting check, `last_turn_at` update) is preserved and runs *before* the dice roll, so a missing/unparseable payload still does the existing work and just skips the spawn.
- **`src/statusline.rs`** — speech selection chain at lines 108–114 inserts `quip_cache::consume()` between birthday and mood
- **`src/hatch.rs`** — after stat/breed roll, calls `llm::call_haiku` for blurb if API key present; falls back to static table on any failure
- **`src/card.rs`** + **`src/hooks.rs::stats`** — `s.personality.replace("{name}", &s.name)` at render
- **`src/paths.rs`** — add `pub fn quip_cache_file() -> PathBuf`
- **`src/payload.rs`** — add `transcript_path: String` to `Payload` (already in Claude Code's hook input schema)
- **`Cargo.toml`** — add `ureq = { version = "2", default-features = false, features = ["tls", "json"] }`

### No changes

- **`src/state.rs`** — no schema bump
- **`src/speech.rs`** — keep static pools as-is; the LLM path is parallel, not a replacement

## Spawn semantics (cross-platform)

```rust
use std::process::{Command, Stdio};

fn spawn_react_worker(transcript_path: &str) {
    let exe = std::env::current_exe().ok();
    let Some(exe) = exe else { return; };
    let mut cmd = Command::new(exe);
    cmd.args(["react", transcript_path])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NO_WINDOW:  u32 = 0x0800_0000;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW);
    }

    // On Unix, default spawn() with nulled stdio detaches cleanly.
    let _ = cmd.spawn();   // intentionally drop the Child handle
}
```

`current_exe()` rather than a hardcoded path so the same binary calls itself.
Failures (couldn't resolve exe path, spawn failed) are silent — the user gets a
static-speech render that turn, no harm done.

## Prompt design

### L1 react

**System:**
```
You are the inner voice of {NAME}, a {RARITY} {BREED} cat with peak {PEAK}
and dump {DUMP}. Personality: {PERSONALITY_BLURB_RENDERED}.

You watch the human work in their terminal. Sometimes you make a short, dry,
in-character remark about what they just did. Speak as the cat — first person
or third person, your choice — but keep it to ONE short line, ≤60 characters.

Voice rules:
- React to the user's last action or message; don't summarize.
- Cat-flavored vocabulary is welcome (mrow, purr, paw) but don't lean on it.
- No emoji. No quotation marks around the line. No preamble. No trailing period
  unless the line genuinely ends in one.
- If nothing in the transcript is interesting, output exactly: mrow.
```

**User:**
```
Here's what just happened:

USER: {last_user_message_truncated_500c}

A: {last_assistant_message_truncated_800c}

RECENT TOOL CALLS:
- {tool_name}({input_preview_200c}) → {output_preview_200c}
- ... (up to 5)

Mood signal: {mood}
```

`max_tokens=50`, `model=claude-haiku-4-5`. Response sanitized via `sanitize_quip`
before caching.

### L5 hatch personality

**System:**
```
You write 1-2 sentence personality blurbs for cats in a desktop pet game.
The voice is dry, observational, character-rich — like a fortune cookie
written by someone who knows this cat well.

Output rules:
- 1 to 2 sentences, max 200 characters total.
- Use the literal token {name} wherever the cat's name appears. Do not
  substitute a name. If the blurb doesn't naturally need the name, you may
  omit it entirely — but then no other name either.
- No emoji. No quotation marks. No preamble.
```

**User:**
```
Cat: {RARITY} {BREED}
Peak stat: {PEAK_STAT}
Dump stat: {DUMP_STAT}

Write the blurb.
```

`max_tokens=80`. Validate response contains `{name}` literal OR no capitalized
non-sentence-start words (cheap proper-noun heuristic). Fail → static.

## Error handling

Every LLM path has the same fallback ladder. The user never sees an error.

| Failure | L1 (react) | L5 (hatch) |
|---|---|---|
| `ANTHROPIC_API_KEY` missing | `turn` skips spawn entirely | static table |
| Network unreachable / DNS fail | log + exit 0; statusline serves mood speech | static table |
| HTTP 4xx | log + exit; static next render | static table |
| HTTP 5xx / 529 | log + exit; static next render | static table |
| Timeout (10s connect, 10s read) | log + exit; static | static |
| Malformed JSON response | log + exit; static | static |
| Response body empty | log + exit; static | static |
| Response > expected chars | sanitize/truncate at word boundary | use as-is, capped |
| Newlines in quip | replaced with single space | n/a |
| Transcript file missing/unreadable | log + exit; no quip generated | n/a |
| Empty transcript after parsing | skip call, log "empty transcript", exit 0 | n/a |
| Quip cache file corrupt | `consume` returns `None`, deletes the file | n/a |
| Stale quip (>5 min by `generated_at`) | `consume` deletes without serving | n/a |
| Two `react` workers race | last-writer-wins (atomic temp+rename) — fine | n/a |
| Manual `purrgotchi react` with junk path | worker exits 0 silently | n/a |
| Validation fail (no `{name}`, has proper noun) | n/a | static fallback |

**No retries.** A quip is fungible — try again next dice hit.

## Testing strategy

**Unit tests (pure, no network/spawn/fs):**

- `llm::parse_haiku_response` — valid JSON, missing fields, error envelope, empty content
- `react::read_transcript_tail` — golden JSONL fixtures, 8KB cap, missing file, malformed lines
- `react::build_prompt` — identity substitution, truncation boundaries, mood inclusion
- `react::sanitize_quip` — strips quotes, strips trailing newlines, truncates at word boundary, replaces internal newlines
- `quip_cache::Quip` — serialize/deserialize round-trip
- `quip_cache::consume` — fresh quip serves + decrements; expired (>5min) returns None + deletes; renders_remaining=0 returns None + deletes; corrupt file returns None + deletes
- `hooks::should_fire_react` — pure dice + api-key + transcript-path checker
- `hatch::personality_with_placeholder` — render-side substitution; idempotent for existing kittens (no placeholder present)
- Statusline priority chain — mock cache × celebration × birthday → asserts speech selection

**Integration tests** (gated behind `LLM_INTEGRATION=1` env, skipped by default):

- `react` worker against real Anthropic API — verifies wire format
- `hatch --rarity legendary` with API key — full flow + `{name}` token validation

**Manual smoke test (release checklist):**

1. Build release. Unset `ANTHROPIC_API_KEY`. Submit ~20 prompts. Confirm:
   zero `react` processes spawn, no log lines, statusline never shows a quip.
2. Set key. Submit ~25 prompts. Confirm: ~5 quips appear, each lasting ~6
   renders, in pink. Debug log shows `[llm] react ok` lines with sane
   latency/tokens.
3. Hatch a fresh kitten with key set. Watch for "✨ summoning personality…"
   stderr line. Confirm card shows an LLM blurb that uses `{name}`. Run
   `purrgotchi rename Whiskers`. Run `purrgotchi card`. Confirm new name
   flows through every appearance.
4. Hatch with key UNSET. Confirm static blurb path, no delay.

## Cost ballpark

Per react call: ~1.5K in × $0.80/MTok + ~50 out × $4/MTok ≈ **$0.0014**.

At 50–200 prompts/day × 20% hit rate = 10–40 calls/day = **$0.014–$0.056/day**.

Hatch is one-shot, ~$0.0005, negligible.

These costs are user-side Anthropic API spend, separate from Claude Code's
`lifetime_cost_usd`. Tracking purrgotchi's own LLM spend is out of scope for
this bundle (could be a future point release).

## Out of scope

- Retries on LLM call failure
- Caching quips for reuse across sessions
- Mood-change invalidation of in-flight quips (we accept slight mood mismatch)
- Per-user prompt customization
- Streaming Haiku responses
- Tracking purrgotchi's own LLM spend in `lifetime_cost_usd`
- Migrating existing kittens' pre-substituted personality strings to use `{name}`
- Models other than `claude-haiku-4-5`
- Web search / extended thinking / tool use in the LLM call
