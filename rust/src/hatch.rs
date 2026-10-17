use crate::{identity, state};

#[derive(Default)]
pub struct HatchOpts {
    pub force_rarity: Option<String>,
    pub force_breed: Option<String>,
    pub force_shiny: bool,
}

impl HatchOpts {
    /// True if any dev-backdoor flag/env was set. Gates the re-hatch protection
    /// in `run`: unforced hatches refuse to overwrite an existing kitten.
    pub fn is_forced(&self) -> bool {
        self.force_rarity.is_some() || self.force_breed.is_some() || self.force_shiny
    }
}

/// Generate a complete State using environment-derived seed.
pub fn generate(opts: &HatchOpts) -> state::State {
    let now = state::now_secs();
    let username = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "user".to_string());
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "host".to_string());
    let seed = identity::fnv1a(&format!("{username}:{hostname}:{now}"));
    generate_from_seed(seed, now, opts)
}

/// Pure function: deterministic from `seed`. Used by tests and [`generate`].
///
/// **Determinism contract:** The order of `rng.range()` calls below is part of
/// the public contract. Adding, removing, or reordering calls silently changes
/// which kitten any given seed produces. Treat this as a breaking change.
pub fn generate_from_seed(seed: u64, now: u64, opts: &HatchOpts) -> state::State {
    let mut rng = identity::Lcg::new(seed);

    // Roll rarity
    let rarity = if let Some(r) = &opts.force_rarity {
        r.clone()
    } else {
        match rng.range(0, 99) {
            0..=59  => "common",
            60..=84 => "uncommon",
            85..=94 => "rare",
            95..=98 => "epic",
            _       => "legendary",   // 99 — arms above cover 0..=98, so this is the only remaining value
        }.to_string()
    };

    // Pick breed
    let breed = if let Some(b) = &opts.force_breed {
        b.clone()
    } else {
        let breeds = identity::breeds_for_rarity(&rarity);
        let idx = rng.range(0, (breeds.len() as u64) - 1) as usize;
        breeds[idx].to_string()
    };

    // Roll shiny
    let shiny = opts.force_shiny || rng.range(0, 99) == 0;

    // Roll stats
    let floor = identity::stat_floor(&rarity) as u64;
    let peak_idx = rng.range(0, 4) as usize;
    let dump_raw = rng.range(0, 3) as usize;
    // Map the 0..=3 draw onto the 4 non-peak stat slots:
    // if dump_raw would collide with peak_idx, bump it past.
    let dump_idx = if dump_raw >= peak_idx { dump_raw + 1 } else { dump_raw };

    let mut stats = [0u8; 5];
    for (i, stat) in stats.iter_mut().enumerate() {
        *stat = if i == peak_idx {
            rng.range(80, 100) as u8
        } else if i == dump_idx {
            rng.range(0, 20) as u8
        } else {
            rng.range(floor, 80) as u8
        };
    }

    // Look up personality blurb. Stored with literal `{name}` — render sites
    // (`card.rs`, future stats personality row) substitute at render time so
    // renames flow through automatically.
    let breed_idx = identity::breed_global_idx(&breed);
    let ds = identity::dump_slot(peak_idx, dump_idx);
    let personality = identity::BLURBS[breed_idx][peak_idx][ds].to_string();

    state::State {
        breed,
        rarity,
        shiny,
        stats,
        personality,
        ..state::State::base(now)
    }
}

/// Load existing state, or auto-hatch a fresh kitten and persist it.
///
/// Only a genuinely *missing* state.json hatches. A transient read failure
/// (Windows sharing violation from an AV scanner, etc.) is retried briefly
/// and then answered with an in-memory placeholder that is never saved — the
/// real kitten on disk must survive. A corrupt file is preserved aside
/// (state.json.corrupt) before the fresh hatch, so the data stays
/// recoverable.
pub fn load_or_hatch() -> state::State {
    use crate::state::LoadOutcome;
    match state::load_outcome_retry() {
        LoadOutcome::Loaded(s) => s,
        LoadOutcome::Unreadable => {
            crate::debug::log("state.json unreadable — serving placeholder, NOT saving");
            generate(&HatchOpts::default())
        }
        LoadOutcome::Missing | LoadOutcome::Corrupt => {
            // locked_update re-checks the outcome under the lock (so two
            // concurrent processes can't double-hatch), preserves a corrupt
            // file aside, hatches, and saves.
            state::locked_update(|s| s.clone())
                .unwrap_or_else(|_| generate(&HatchOpts::default()))
        }
    }
}

/// Pure: should `hatch::run` emit the "set PURR_LLM_PROVIDER" hint?
///
/// True iff the provider-selection result is `Err` — meaning the user
/// will silently get static personality + no react quips. We point them
/// at the env var so they can opt in.
fn should_print_key_hint<T>(result: &Result<T, crate::llm::select::ConfigError>) -> bool {
    result.is_err()
}

/// Called by `purrgotchi hatch` CLI command.
///
/// A hatched kitten is permanent: an unforced `hatch` with an existing kitten
/// refuses and exits 1. Dev backdoors (`--rarity`/`--breed`/`--shiny` flags or
/// `PURR_FORCE_*` env vars) bypass the check.
pub fn run(opts: &HatchOpts) {
    if !opts.is_forced() {
        match state::load_outcome() {
            state::LoadOutcome::Loaded(existing) if !existing.breed.is_empty() => {
                eprintln!(
                    "You already have a {} ({}). Your kitten is permanent.",
                    existing.breed, existing.rarity
                );
                eprintln!("Run 'purrgotchi card' to see them.");
                std::process::exit(1);
            }
            state::LoadOutcome::Corrupt | state::LoadOutcome::Unreadable => {
                eprintln!("state.json exists but could not be read (locked or corrupt).");
                eprintln!(
                    "Refusing to hatch over it — check {}",
                    crate::paths::state_file().display()
                );
                std::process::exit(1);
            }
            _ => {}
        }
    }
    let mut s = generate(opts);
    println!("Hatched: {} {} ({}){}",
        if s.shiny { "✦" } else { "" },
        s.breed, s.rarity,
        if s.shiny { " [SHINY]" } else { "" }
    );
    println!("Stats: CUR={} MIS={} LAZ={} AFF={} WIS={}",
        s.stats[0], s.stats[1], s.stats[2], s.stats[3], s.stats[4]);
    if let Some(llm_blurb) = maybe_llm_blurb(&s) {
        s.personality = llm_blurb;
    }
    let _lock = state::acquire_lock();
    match state::save_atomic(&s) {
        Ok(()) => println!("Saved to: {}", crate::paths::state_file().display()),
        Err(e) => eprintln!("warning: could not write state: {e}"),
    }
    let provider_status = crate::llm::select::current_provider().map(|_| ());
    if should_print_key_hint(&provider_status) {
        eprintln!(
            "\nhint: set PURR_LLM_PROVIDER (and the matching API key in ~/.claude/purrgotchi/.env) to enable AI personality and react quips."
        );
    }
}

const HATCH_MAX_TOKENS: u32 = 80;
const HATCH_BLURB_HARD_CAP_CHARS: usize = 200;

/// Try to generate an LLM personality blurb at hatch. Returns `None` on any
/// failure (including unconfigured provider or missing API key). Caller falls
/// back to whatever `generate_from_seed` already stored.
fn maybe_llm_blurb(s: &state::State) -> Option<String> {
    if std::env::var("PURR_FORCE_STATIC_BLURB").is_ok() {
        return None;
    }
    let provider = match crate::llm::select::current_provider() {
        Ok(p) => p,
        Err(e) => {
            crate::debug::log(&format!("[llm] hatch skip reason=config err={e}"));
            return None;
        }
    };
    let peak_idx = s.stats.iter().enumerate()
        .max_by_key(|&(_, &v)| v).map(|(i, _)| i)?;
    let dump_idx = s.stats.iter().enumerate()
        .min_by_key(|&(_, &v)| v).map(|(i, _)| i)?;
    let peak = identity::STAT_NAMES[peak_idx];
    let dump = identity::STAT_NAMES[dump_idx];

    let system = "You write 1-2 sentence personality blurbs for cats in a desktop pet game.\n\
The voice is dry, observational, character-rich — like a fortune cookie\n\
written by someone who knows this cat well.\n\n\
Output rules:\n\
- 1 to 2 sentences, max 200 characters total.\n\
- Use the literal token {name} wherever the cat's name appears. Do not\n\
  substitute a name. If the blurb doesn't naturally need the name, you may\n\
  omit it entirely — but then no other name either.\n\
- No emoji. No quotation marks. No preamble.".to_string();
    let user = format!(
        "Cat: {} {}\nPeak stat: {peak}\nDump stat: {dump}\n\nWrite the blurb.",
        s.rarity, s.breed,
    );
    let req = crate::llm::LlmRequest { system, user, max_tokens: HATCH_MAX_TOKENS };

    eprintln!("✨ summoning personality…");
    match provider.complete(req) {
        Ok(resp) => {
            crate::debug::log(&format!(
                "[llm] hatch ok provider={} model={} latency_ms={} in_tok={} out_tok={}",
                provider.name(), resp.model, resp.latency_ms, resp.input_tokens, resp.output_tokens,
            ));
            let cleaned = clean_hatch_blurb(&resp.text);
            if validate_blurb(&cleaned) {
                Some(cleaned)
            } else {
                crate::debug::log("[llm] hatch reject reason=validation_failed");
                None
            }
        }
        Err(e) => {
            crate::debug::log(&format!("[llm] hatch fail provider={} err={e}", provider.name()));
            None
        }
    }
}

fn clean_hatch_blurb(raw: &str) -> String {
    let trimmed = raw.trim();
    let bytes = trimmed.as_bytes();
    // len >= 2 so a degenerate single-quote-character response can't produce
    // the panicking slice range 1..0.
    let stripped = if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    let single = stripped.replace(['\n', '\r'], " ");
    single.chars().take(HATCH_BLURB_HARD_CAP_CHARS).collect()
}

/// Validate: the blurb either uses `{name}` literal OR contains no
/// capitalized words at all. Without the placeholder we can't tell a
/// proper noun from a sentence-start capital, so we require the LLM to
/// have written the blurb in all-lowercase prose if it elected to omit
/// `{name}`. This is conservative but cheap and avoids the rename bug.
fn validate_blurb(b: &str) -> bool {
    if b.is_empty() {
        return false;
    }
    if b.contains("{name}") {
        return true;
    }
    // No placeholder — reject if any word starts with an uppercase letter.
    for word in b.split_whitespace() {
        let w = word.trim_matches(|c: char| !c.is_alphanumeric());
        if w.is_empty() {
            continue;
        }
        if w.chars().next().is_some_and(|c| c.is_uppercase()) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- v3.6.3: missing-key hint helper ---

    use crate::llm::select::ConfigError;

    #[test]
    fn key_hint_when_provider_unset() {
        let r: Result<(), ConfigError> = Err(ConfigError::ProviderNotSet);
        assert!(should_print_key_hint(&r));
    }

    #[test]
    fn key_hint_when_provider_ok() {
        let r: Result<(), ConfigError> = Ok(());
        assert!(!should_print_key_hint(&r));
    }

    fn hatch_with_seed(seed_suffix: u64) -> state::State {
        let seed = identity::fnv1a(&format!("testuser:testhost:{seed_suffix}"));
        generate_from_seed(seed, 0, &HatchOpts::default())
    }

    #[test]
    fn is_forced_flags() {
        assert!(!HatchOpts::default().is_forced());
        assert!(HatchOpts { force_rarity: Some("rare".into()), ..Default::default() }.is_forced());
        assert!(HatchOpts { force_breed: Some("Tabby".into()), ..Default::default() }.is_forced());
        assert!(HatchOpts { force_shiny: true, ..Default::default() }.is_forced());
    }

    #[test]
    fn same_seed_same_kitten() {
        let a = hatch_with_seed(1000);
        let b = hatch_with_seed(1000);
        assert_eq!(a.breed, b.breed);
        assert_eq!(a.rarity, b.rarity);
        assert_eq!(a.stats, b.stats);
    }

    #[test]
    fn different_seed_likely_different() {
        let a = hatch_with_seed(1000);
        let b = hatch_with_seed(9999);
        assert!(a.breed != b.breed || a.rarity != b.rarity || a.stats != b.stats);
    }

    #[test]
    fn peak_stat_in_range() {
        for seed in 0..50u64 {
            let s = hatch_with_seed(seed);
            let peak = s.stats.iter().copied().max().unwrap();
            assert!(peak >= 80, "peak {peak} < 80");
        }
    }

    #[test]
    fn dump_stat_in_range() {
        for seed in 0..50u64 {
            let s = hatch_with_seed(seed);
            let dump = s.stats.iter().copied().min().unwrap();
            assert!(dump <= 20, "dump {dump} > 20");
        }
    }

    #[test]
    fn rarity_valid() {
        let valid = ["common", "uncommon", "rare", "epic", "legendary"];
        for seed in 0..50u64 {
            let s = hatch_with_seed(seed);
            assert!(valid.contains(&s.rarity.as_str()), "invalid rarity: {}", s.rarity);
        }
    }

    #[test]
    fn stat_floor_respected_for_middle_stats() {
        let mut checked = 0;
        for seed in 0..200u64 {
            let s = hatch_with_seed(seed);
            if s.rarity != "legendary" { continue; }
            let peak_val = *s.stats.iter().max().unwrap();
            let dump_val = *s.stats.iter().min().unwrap();
            for &stat in &s.stats {
                if stat == peak_val || stat == dump_val { continue; }
                assert!(stat >= 50, "legendary middle stat {stat} < floor 50");
            }
            checked += 1;
            if checked >= 5 { break; }
        }
        assert!(checked > 0, "no legendary found in 200 seeds — check rarity distribution");
    }

    #[test]
    fn force_rarity_override() {
        let opts = HatchOpts {
            force_rarity: Some("legendary".to_string()),
            force_breed: None,
            force_shiny: false,
        };
        let seed = identity::fnv1a("u:h:1");
        let s = generate_from_seed(seed, 0, &opts);
        assert_eq!(s.rarity, "legendary");
    }

    #[test]
    fn force_breed_override() {
        let opts = HatchOpts {
            force_rarity: Some("legendary".to_string()),
            force_breed: Some("Void Cat".to_string()),
            force_shiny: true,
        };
        let seed = identity::fnv1a("u:h:1");
        let s = generate_from_seed(seed, 0, &opts);
        assert_eq!(s.breed, "Void Cat");
        assert!(s.shiny);
    }

    #[test]
    fn personality_non_empty_and_uses_placeholder() {
        // After v3.5.0: personality is stored with the literal `{name}` placeholder
        // and substituted at render time. This test asserts both that the static
        // table path produced something non-empty and that it used the placeholder
        // (the static blurbs all reference {name}).
        for seed in 0..20u64 {
            let s = hatch_with_seed(seed);
            assert!(!s.personality.is_empty());
            assert!(s.personality.contains("{name}"),
                "static blurb should retain {{name}} placeholder for render-time substitution: {}", s.personality);
        }
    }

    #[test]
    fn validate_blurb_accepts_with_placeholder() {
        assert!(super::validate_blurb("{name} watches the keys with quiet curiosity."));
    }

    #[test]
    fn validate_blurb_accepts_without_proper_nouns() {
        assert!(super::validate_blurb("a cat of few words and many opinions."));
    }

    #[test]
    fn validate_blurb_rejects_proper_noun_without_placeholder() {
        assert!(!super::validate_blurb("Mittens watches the keys."));
    }

    #[test]
    fn validate_blurb_rejects_empty() {
        assert!(!super::validate_blurb(""));
    }

    #[test]
    fn clean_hatch_blurb_strips_quotes_and_newlines() {
        let raw = "\"This is\nthe blurb.\"";
        assert_eq!(super::clean_hatch_blurb(raw), "This is the blurb.");
    }

    #[test]
    fn clean_hatch_blurb_caps_at_200_chars() {
        let raw = "a".repeat(500);
        let got = super::clean_hatch_blurb(&raw);
        assert_eq!(got.chars().count(), 200);
    }
}
