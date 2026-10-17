//! L1 worker subcommand: `purrgotchi react <transcript_path>`.
//!
//! Spawned detached from `hooks::turn` after a 1-in-5 dice hit. Reads the tail
//! of the Claude Code transcript JSONL file, builds a Haiku prompt that
//! references the kitten's identity, calls the API, sanitizes the response,
//! and writes the result to `quip_cache`. Errors are non-fatal — every failure
//! path logs and exits 0, leaving the statusline to serve a static line.

use crate::debug;
use crate::llm::{self, LlmRequest};
use crate::quip_cache;
use crate::state::{self, State};
use std::time::Duration;

const WATCHDOG_DEADLINE_SECS: u64 = 30;
const TRANSCRIPT_BUDGET_BYTES: usize = 8 * 1024;
const QUIP_HARD_CAP_CHARS: usize = 80;
const REACT_MAX_TOKENS: u32 = 50;
const TOOL_INPUT_PREVIEW_CHARS: usize = 200;
const TOOL_OUTPUT_PREVIEW_CHARS: usize = 200;
const USER_MSG_MAX_CHARS: usize = 500;
const ASSISTANT_MSG_MAX_CHARS: usize = 800;
const RECENT_TOOL_CALLS: usize = 5;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct TranscriptTail {
    pub last_user: String,
    pub last_assistant: String,
    pub recent_tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ToolCall {
    pub name: String,
    pub input_preview: String,
    pub output_preview: String,
}

impl TranscriptTail {
    pub fn is_empty(&self) -> bool {
        self.last_user.is_empty()
            && self.last_assistant.is_empty()
            && self.recent_tool_calls.is_empty()
    }
}

/// Truncate a string to `max_chars` characters (not bytes), respecting char
/// boundaries.
fn truncate_chars(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

/// Read the Claude Code transcript JSONL file and extract a structured tail.
/// Returns `None` if the file is missing or unreadable. Returns `Some(empty)`
/// if the file exists but yields nothing usable — caller should treat empty
/// as "no quip to generate" and skip the API call.
pub fn read_transcript_tail(path: &str) -> Option<TranscriptTail> {
    use std::io::{Read, Seek, SeekFrom};
    const READ_CAP_BYTES: u64 = 256 * 1024;
    // Long sessions grow the transcript to hundreds of MB; only the tail
    // matters, so cap the read instead of loading the whole file into RAM.
    let mut f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    if len > READ_CAP_BYTES {
        f.seek(SeekFrom::End(-(READ_CAP_BYTES as i64))).ok()?;
    }
    let mut bytes = Vec::with_capacity(len.min(READ_CAP_BYTES) as usize);
    f.read_to_end(&mut bytes).ok()?;
    // A mid-line seek start just means the first (partial) line fails JSON
    // parse and is skipped by the parser.
    let text = String::from_utf8_lossy(&bytes);
    Some(parse_transcript_jsonl(&text))
}

/// Pure parser. Walks JSONL backward, extracting the last user message, last
/// assistant message (text only — no tool_use blocks), and last N tool-call
/// pairs (tool_use + tool_result on the closest match by tool_use_id).
pub fn parse_transcript_jsonl(text: &str) -> TranscriptTail {
    use std::collections::HashMap;
    let mut last_user = String::new();
    let mut last_assistant = String::new();
    let mut tool_uses: Vec<(String, String, String)> = Vec::new(); // (id, name, input_preview)
    let mut tool_results: HashMap<String, String> = HashMap::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        // Claude Code transcript entries look like:
        //   {"type":"user","message":{"role":"user","content":"..."}}
        // or content is an array of blocks (with tool_use / tool_result blocks).
        let entry_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let msg = v.get("message").unwrap_or(&serde_json::Value::Null);
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or(entry_type);

        match role {
            "user" => {
                if let Some(text) = extract_user_text(msg)
                    && !text.is_empty()
                {
                    last_user = truncate_chars(&text, USER_MSG_MAX_CHARS);
                }
                // tool_result blocks arrive on user messages.
                if let Some(content) = msg.get("content").and_then(|c| c.as_array()) {
                    for block in content {
                        let bt = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        if bt == "tool_result" {
                            let id = block.get("tool_use_id").and_then(|t| t.as_str()).unwrap_or("").to_string();
                            let txt = extract_block_text(block);
                            if !id.is_empty() {
                                tool_results.insert(
                                    id,
                                    truncate_chars(&txt, TOOL_OUTPUT_PREVIEW_CHARS),
                                );
                            }
                        }
                    }
                }
            }
            "assistant" => {
                let mut text_parts: Vec<String> = Vec::new();
                if let Some(content) = msg.get("content").and_then(|c| c.as_array()) {
                    for block in content {
                        let bt = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        if bt == "text" {
                            if let Some(t) = block.get("text").and_then(|t| t.as_str()) {
                                text_parts.push(t.to_string());
                            }
                        } else if bt == "tool_use" {
                            let id = block.get("id").and_then(|t| t.as_str()).unwrap_or("").to_string();
                            let name = block.get("name").and_then(|t| t.as_str()).unwrap_or("").to_string();
                            let input_str = block.get("input")
                                .map(|i| i.to_string())
                                .unwrap_or_default();
                            tool_uses.push((id, name, truncate_chars(&input_str, TOOL_INPUT_PREVIEW_CHARS)));
                        }
                    }
                } else if let Some(s) = msg.get("content").and_then(|c| c.as_str()) {
                    text_parts.push(s.to_string());
                }
                let joined = text_parts.join(" ");
                if !joined.is_empty() {
                    last_assistant = truncate_chars(&joined, ASSISTANT_MSG_MAX_CHARS);
                }
            }
            _ => {}
        }
    }

    let mut recent: Vec<ToolCall> = tool_uses.iter().rev().take(RECENT_TOOL_CALLS).map(|(id, name, input)| {
        let output = tool_results.get(id).cloned().unwrap_or_default();
        ToolCall { name: name.clone(), input_preview: input.clone(), output_preview: output }
    }).collect();
    recent.reverse();

    TranscriptTail { last_user, last_assistant, recent_tool_calls: recent }
}

fn extract_user_text(msg: &serde_json::Value) -> Option<String> {
    if let Some(s) = msg.get("content").and_then(|c| c.as_str()) {
        return Some(s.to_string());
    }
    if let Some(arr) = msg.get("content").and_then(|c| c.as_array()) {
        let mut out = String::new();
        for block in arr {
            let bt = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if bt == "text"
                && let Some(t) = block.get("text").and_then(|t| t.as_str())
            {
                if !out.is_empty() { out.push(' '); }
                out.push_str(t);
            }
        }
        if !out.is_empty() {
            return Some(out);
        }
    }
    None
}

fn extract_block_text(block: &serde_json::Value) -> String {
    if let Some(s) = block.get("content").and_then(|c| c.as_str()) {
        return s.to_string();
    }
    if let Some(arr) = block.get("content").and_then(|c| c.as_array()) {
        let mut out = String::new();
        for inner in arr {
            if let Some(t) = inner.get("text").and_then(|t| t.as_str()) {
                if !out.is_empty() { out.push(' '); }
                out.push_str(t);
            }
        }
        return out;
    }
    String::new()
}

/// Build the LLM request from kitten state + transcript tail.
/// Mood is a best-effort guess — the worker doesn't have the live payload, so
/// only fed/pet/play/treat-derived moods are available; panic/alert never fire here.
pub fn build_prompt(s: &State, mood: &str, tail: &TranscriptTail) -> LlmRequest {
    let peak_idx = s.stats.iter().enumerate()
        .max_by_key(|&(_, &v)| v).map(|(i, _)| i).unwrap_or(0);
    let dump_idx = s.stats.iter().enumerate()
        .min_by_key(|&(_, &v)| v).map(|(i, _)| i).unwrap_or(4);
    let names = crate::identity::STAT_NAMES;
    let peak = names[peak_idx];
    let dump = names[dump_idx];
    let rendered_personality = s.personality.replace("{name}", &s.name);

    let system = format!(
        "You are the inner voice of {name}, a {rarity} {breed} cat with peak {peak} and \
dump {dump}. Personality: {personality}.\n\n\
You watch the human work in their terminal. Sometimes you make a short, dry, \
in-character remark about what they just did. Speak as the cat — first person \
or third person, your choice — but keep it to ONE short line, ≤60 characters.\n\n\
Voice rules:\n\
- React to the user's last action or message; don't summarize.\n\
- Cat-flavored vocabulary is welcome (mrow, purr, paw) but don't lean on it.\n\
- No emoji. No quotation marks around the line. No preamble. No trailing period \
unless the line genuinely ends in one.\n\
- If nothing in the transcript is interesting, output exactly: mrow.",
        name = s.name,
        rarity = s.rarity,
        breed = s.breed,
        peak = peak,
        dump = dump,
        personality = rendered_personality,
    );

    let mut user = String::new();
    user.push_str("Here's what just happened:\n\n");
    if !tail.last_user.is_empty() {
        user.push_str(&format!("USER: {}\n\n", tail.last_user));
    }
    if !tail.last_assistant.is_empty() {
        user.push_str(&format!("A: {}\n\n", tail.last_assistant));
    }
    if !tail.recent_tool_calls.is_empty() {
        user.push_str("RECENT TOOL CALLS:\n");
        for tc in &tail.recent_tool_calls {
            user.push_str(&format!("- {}({}) → {}\n", tc.name, tc.input_preview, tc.output_preview));
        }
        user.push('\n');
    }
    user.push_str(&format!("Mood signal: {mood}"));

    // Defensive cap: even if every component obeys its own limit, the joined
    // string could in theory exceed the budget. Truncate at a char boundary.
    if user.len() > TRANSCRIPT_BUDGET_BYTES {
        user = truncate_chars(&user, TRANSCRIPT_BUDGET_BYTES);
    }

    LlmRequest { system, user, max_tokens: REACT_MAX_TOKENS }
}

/// Trim an LLM response down to a single safe line for the speech bubble.
pub fn sanitize_quip(raw: &str) -> String {
    // 1. Take first non-empty line.
    let line = raw.lines().map(str::trim).find(|s| !s.is_empty()).unwrap_or("");
    // 2. Strip ALL control characters (ESC, BEL, C0/C1, DEL). The quip is
    //    printed raw into the terminal by the statusline — a model echoing a
    //    prompt-injected escape sequence must not be able to retitle the
    //    window, move the cursor, or clear the screen.
    let line: String = line.chars().filter(|c| !c.is_control()).collect();
    // 3. Strip surrounding matched quotes.
    let line = strip_quotes(&line);
    // 4. Hard char cap with word-boundary truncation.
    truncate_at_word_boundary(&line, QUIP_HARD_CAP_CHARS)
}

fn strip_quotes(s: &str) -> String {
    let trimmed = s.trim();
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    trimmed.to_string()
}

fn truncate_at_word_boundary(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let head: String = s.chars().take(max_chars).collect();
    if let Some(idx) = head.rfind(' ') {
        head[..idx].to_string()
    } else {
        head
    }
}

/// Generic watchdog: sleep `deadline`, then run `action` on a background thread.
/// Detached — the caller does not join. Extracted from `spawn_watchdog` so the
/// timing behaviour can be unit-tested without invoking `process::exit`.
fn spawn_watchdog_with_action<F>(deadline: Duration, action: F)
where
    F: FnOnce() + Send + 'static,
{
    let _ = std::thread::spawn(move || {
        std::thread::sleep(deadline);
        action();
    });
}

/// Wall-clock guard for the detached `purrgotchi react` worker. After the
/// deadline elapses, log a breadcrumb and force-exit the process so a stalled
/// HTTP call cannot keep `purrgotchi.exe` open and block future cargo builds
/// (Windows zombie-binary problem). Exit code 0 is consistent with the rest of
/// `react::run` — every failure here is non-fatal and silent.
fn spawn_watchdog(deadline: Duration) {
    spawn_watchdog_with_action(deadline, || {
        crate::debug::log("[llm] react fail reason=watchdog_timeout");
        std::process::exit(0);
    });
}

/// Top-level entry point for `purrgotchi react <transcript_path>`.
/// Always exits 0 — every failure is silent + logged.
pub fn run(transcript_path: &str) {
    spawn_watchdog(Duration::from_secs(WATCHDOG_DEADLINE_SECS));
    let _ = run_inner(transcript_path);
}

fn run_inner(transcript_path: &str) -> Option<()> {
    let provider = match llm::select::current_provider() {
        Ok(p) => p,
        Err(e) => {
            debug::log(&format!("[llm] react skip reason=config err={e}"));
            return None;
        }
    };
    let s = state::load()?;
    if s.breed.is_empty() {
        debug::log("[llm] react skip reason=unhatched");
        return None;
    }
    let tail = read_transcript_tail(transcript_path).unwrap_or_default();
    if tail.is_empty() {
        debug::log("[llm] react skip reason=empty_transcript");
        return None;
    }
    let mood = best_effort_mood(&s);
    let req = build_prompt(&s, mood, &tail);
    match provider.complete(req) {
        Ok(resp) => {
            debug::log(&format!(
                "[llm] react ok provider={} model={} latency_ms={} in_tok={} out_tok={}",
                provider.name(), resp.model, resp.latency_ms, resp.input_tokens, resp.output_tokens,
            ));
            let q = sanitize_quip(&resp.text);
            if q.is_empty() {
                debug::log("[llm] react skip reason=empty_quip_after_sanitize");
                return None;
            }
            let _ = quip_cache::write(&q);
        }
        Err(e) => {
            debug::log(&format!("[llm] react fail provider={} err={e}", provider.name()));
        }
    }
    Some(())
}

/// Pick a mood string from state alone. Cannot derive panic/alert (those need
/// context %, which the worker doesn't have). Defaults to calm.
fn best_effort_mood(s: &State) -> &'static str {
    let now = state::now_secs();
    let fed_delta = (now as i64) - (s.last_fed_at as i64);
    if fed_delta > 8 * 3600 { return "hungry"; }
    let last = [s.last_pet_at, s.last_played_at, s.last_treated_at].iter().copied().max().unwrap_or(0);
    if last != 0 && now.saturating_sub(last) < 180 {
        return "happy";
    }
    "calm"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_at_word_boundary_within_cap_unchanged() {
        assert_eq!(truncate_at_word_boundary("hello world", 80), "hello world");
    }

    #[test]
    fn truncate_at_word_boundary_cuts_at_space() {
        let long = "this is a long line of words and more words and more";
        let got = truncate_at_word_boundary(long, 20);
        assert!(got.chars().count() <= 20);
        assert!(!got.ends_with(' '));
        assert!(long.starts_with(&got));
    }

    #[test]
    fn truncate_at_word_boundary_no_space_falls_through() {
        let long = "supercalifragilisticexpialidocious";
        let got = truncate_at_word_boundary(long, 10);
        assert_eq!(got.chars().count(), 10);
    }

    #[test]
    fn strip_quotes_removes_double_quotes() {
        assert_eq!(strip_quotes("\"hello\""), "hello");
    }

    #[test]
    fn strip_quotes_removes_single_quotes() {
        assert_eq!(strip_quotes("'hello'"), "hello");
    }

    #[test]
    fn strip_quotes_leaves_unquoted_intact() {
        assert_eq!(strip_quotes("hello"), "hello");
    }

    #[test]
    fn strip_quotes_does_not_strip_mismatched_quotes() {
        assert_eq!(strip_quotes("\"hello"), "\"hello");
    }

    #[test]
    fn sanitize_takes_first_line_only() {
        let raw = "first line\nsecond line\nthird";
        assert_eq!(sanitize_quip(raw), "first line");
    }

    #[test]
    fn sanitize_strips_surrounding_quotes() {
        assert_eq!(sanitize_quip("\"meow.\""), "meow.");
    }

    #[test]
    fn sanitize_empty_returns_empty() {
        assert_eq!(sanitize_quip(""), "");
        assert_eq!(sanitize_quip("\n\n"), "");
    }

    #[test]
    fn sanitize_truncates_long_response_at_word_boundary() {
        let raw = "a ".repeat(60); // 120 chars
        let got = sanitize_quip(&raw);
        assert!(got.chars().count() <= QUIP_HARD_CAP_CHARS);
    }

    #[test]
    fn parse_transcript_extracts_last_user_text() {
        let jsonl = r#"{"type":"user","message":{"role":"user","content":"hello there"}}"#;
        let tail = parse_transcript_jsonl(jsonl);
        assert_eq!(tail.last_user, "hello there");
    }

    #[test]
    fn parse_transcript_extracts_last_assistant_text_block() {
        let jsonl = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"sure thing"}]}}"#;
        let tail = parse_transcript_jsonl(jsonl);
        assert_eq!(tail.last_assistant, "sure thing");
    }

    #[test]
    fn parse_transcript_pairs_tool_use_with_tool_result() {
        let jsonl = concat!(
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"abc","name":"Bash","input":{"command":"ls"}}]}}"#,
            "\n",
            r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"abc","content":"file1\nfile2"}]}}"#,
        );
        let tail = parse_transcript_jsonl(jsonl);
        assert_eq!(tail.recent_tool_calls.len(), 1);
        assert_eq!(tail.recent_tool_calls[0].name, "Bash");
        assert!(tail.recent_tool_calls[0].input_preview.contains("ls"));
        assert!(tail.recent_tool_calls[0].output_preview.contains("file1"));
    }

    #[test]
    fn parse_transcript_skips_malformed_lines() {
        let jsonl = "not json\n{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"hi\"}}\nalso bad";
        let tail = parse_transcript_jsonl(jsonl);
        assert_eq!(tail.last_user, "hi");
    }

    #[test]
    fn parse_transcript_keeps_only_last_n_tool_calls() {
        let mut s = String::new();
        for i in 0..(RECENT_TOOL_CALLS + 3) {
            s.push_str(&format!(
                r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"tool_use","id":"id{i}","name":"Tool{i}","input":{{}}}}]}}}}"#,
            ));
            s.push('\n');
        }
        let tail = parse_transcript_jsonl(&s);
        assert_eq!(tail.recent_tool_calls.len(), RECENT_TOOL_CALLS);
        // Must be the *last* N (Tool3..Tool7), not the first N.
        let last_name = &tail.recent_tool_calls.last().unwrap().name;
        assert!(last_name.ends_with(&format!("{}", RECENT_TOOL_CALLS + 2)));
    }

    #[test]
    fn build_prompt_includes_kitten_identity() {
        let mut s = State::base(0);
        s.name = "Thalia".into();
        s.breed = "Void Cat".into();
        s.rarity = "legendary".into();
        s.stats = [85, 30, 20, 60, 75]; // peak=CURIOSITY, dump=LAZINESS
        s.personality = "{name} watches you work.".into();
        let tail = TranscriptTail::default();
        let req = build_prompt(&s, "calm", &tail);
        assert!(req.system.contains("Thalia"));
        assert!(req.system.contains("Void Cat"));
        assert!(req.system.contains("legendary"));
        assert!(req.system.contains("CURIOSITY"));
        assert!(req.system.contains("LAZINESS"));
        // {name} placeholder must be substituted into system prompt with actual name.
        assert!(!req.system.contains("{name}"));
        assert!(req.system.contains("Thalia watches you work."));
    }

    #[test]
    fn build_prompt_includes_transcript_chunk() {
        let s = test_state();
        let tail = TranscriptTail {
            last_user: "rebase this".into(),
            last_assistant: "did the rebase".into(),
            ..Default::default()
        };
        let req = build_prompt(&s, "calm", &tail);
        assert!(req.user.contains("rebase this"));
        assert!(req.user.contains("did the rebase"));
        assert!(req.user.contains("Mood signal: calm"));
    }

    #[test]
    fn build_prompt_max_tokens_is_50() {
        let s = test_state();
        let req = build_prompt(&s, "calm", &TranscriptTail::default());
        assert_eq!(req.max_tokens, 50);
    }

    #[test]
    fn best_effort_mood_returns_hungry_on_long_fed_delta() {
        let mut s = State::base(state::now_secs() - 9 * 3600);
        s.last_fed_at = state::now_secs() - 9 * 3600;
        assert_eq!(best_effort_mood(&s), "hungry");
    }

    #[test]
    fn best_effort_mood_defaults_to_calm() {
        let s = State::base(state::now_secs());
        assert_eq!(best_effort_mood(&s), "calm");
    }

    fn test_state() -> State {
        let mut s = State::base(0);
        s.name = "Thalia".into();
        s.breed = "Tabby".into();
        s.rarity = "common".into();
        s.stats = [50, 50, 50, 50, 50];
        s.personality = "test".into();
        s
    }

    // --- Watchdog tests (v3.6.2) ---

    #[test]
    fn watchdog_action_fires_after_deadline() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::time::Duration;

        let fired = Arc::new(AtomicBool::new(false));
        let f = fired.clone();
        spawn_watchdog_with_action(Duration::from_millis(50), move || {
            f.store(true, Ordering::SeqCst);
        });
        // Sleep well past the deadline to avoid timing flakes on slow CI.
        std::thread::sleep(Duration::from_millis(250));
        assert!(fired.load(Ordering::SeqCst), "watchdog action did not fire after deadline");
    }
}
