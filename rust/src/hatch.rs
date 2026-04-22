use crate::{identity, state};

#[derive(Default)]
pub struct HatchOpts {
    pub force_rarity: Option<String>,
    pub force_breed: Option<String>,
    pub force_shiny: bool,
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

    // Look up personality blurb
    let breed_idx = identity::breed_global_idx(&breed);
    let ds = identity::dump_slot(peak_idx, dump_idx);
    let blurb = identity::BLURBS[breed_idx][peak_idx][ds];
    let personality = blurb.replace("{name}", "Purrgotchi");

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
/// Used by statusline and hook entry points so a missing/incompatible
/// state.json transparently becomes a fresh v2 hatch.
pub fn load_or_hatch() -> state::State {
    state::load().unwrap_or_else(|| {
        let fresh = generate(&HatchOpts::default());
        let _ = state::save_atomic(&fresh);
        fresh
    })
}

/// Called by `purrgotchi hatch` CLI command.
pub fn run(opts: &HatchOpts) {
    let s = generate(opts);
    println!("Hatched: {} {} ({}){}",
        if s.shiny { "✦" } else { "" },
        s.breed, s.rarity,
        if s.shiny { " [SHINY]" } else { "" }
    );
    println!("Stats: CUR={} MIS={} LAZ={} AFF={} WIS={}",
        s.stats[0], s.stats[1], s.stats[2], s.stats[3], s.stats[4]);
    let _ = state::save_atomic(&s);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hatch_with_seed(seed_suffix: u64) -> state::State {
        let seed = identity::fnv1a(&format!("testuser:testhost:{seed_suffix}"));
        generate_from_seed(seed, 0, &HatchOpts::default())
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
    fn personality_non_empty() {
        for seed in 0..20u64 {
            let s = hatch_with_seed(seed);
            assert!(!s.personality.is_empty());
            assert!(!s.personality.contains("{name}"),
                "name substitution failed in blurb: {}", s.personality);
        }
    }
}
