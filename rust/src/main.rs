mod bond;
mod card;
mod cli;
mod colors;
mod cost;
mod cost_acc;
mod debug;
mod events;
mod hatch;
mod hooks;
mod identity;
mod json;
mod llm;
mod mood;
mod paths;
mod payload;
mod playground;
mod quip_cache;
mod react;
mod render;
mod spawn;
mod span;
mod speech;
mod sprites;

mod state;
mod statusline;

use std::env;
use std::process;

fn main() {
    // Load .env ONLY from the trusted state dir (~/.claude/purrgotchi/.env),
    // never the inherited cwd. Hooks run with cwd = whatever project Claude
    // Code has open, so a cwd .env would let any repo silently inject HOME,
    // provider selection, or API keys. Process-set vars take precedence
    // (dotenvy default behaviour). Missing file is a no-op.
    let _ = dotenvy::from_path(paths::env_file());

    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("");

    debug::log(&format!("invoke cmd={cmd}"));

    match cmd {
        "statusline" => {
            statusline::run();
            process::exit(0);
        }
        "feed" => {
            let json = cli::has_flag(&args, "--json");
            let count = if cli::has_flag(&args, "--count") {
                match cli::flag_value(&args, "--count").and_then(cli::parse_count) {
                    Some(n) => n,
                    None => {
                        eprintln!("feed: --count must be an integer in 1..={}", cli::MAX_FEED_COUNT);
                        process::exit(2);
                    }
                }
            } else {
                1
            };
            hooks::feed_cmd(count, json);
            process::exit(0);
        }
        "turn" => {
            hooks::turn_cmd(cli::has_flag(&args, "--json"));
            process::exit(0);
        }
        "rename" => {
            let json = cli::has_flag(&args, "--json");
            let name = cli::positionals(&args, &["--json"], &[]).first().copied().unwrap_or("");
            process::exit(hooks::rename_cmd(name, json));
        }
        "card" => {
            card::run();
            process::exit(0);
        }
        "pet" => {
            hooks::pet_cmd(cli::has_flag(&args, "--json"));
            process::exit(0);
        }
        "play" => {
            hooks::play_cmd(cli::has_flag(&args, "--json"));
            process::exit(0);
        }
        "treat" => {
            hooks::treat_cmd(cli::has_flag(&args, "--json"));
            process::exit(0);
        }
        "stats" => {
            hooks::stats();
            process::exit(0);
        }
        "react" => {
            let transcript_path = args.get(2).map(String::as_str).unwrap_or("");
            react::run(transcript_path);
            process::exit(0);
        }
        "hatch" => {
            let force_rarity = args.iter().position(|a| a == "--rarity")
                .and_then(|i| args.get(i + 1))
                .map(|s| s.to_lowercase())
                .or_else(|| std::env::var("PURR_FORCE_RARITY").ok().map(|s| s.to_lowercase()));

            let force_breed = args.iter().position(|a| a == "--breed")
                .and_then(|i| args.get(i + 1))
                .map(|s| s.to_string())
                .or_else(|| std::env::var("PURR_FORCE_BREED").ok());

            let force_shiny = args.iter().any(|a| a == "--shiny")
                || std::env::var("PURR_FORCE_SHINY")
                    .map(|v| v == "1" || v == "true")
                    .unwrap_or(false);

            if let Some(r) = &force_rarity {
                const VALID: [&str; 5] = ["common", "uncommon", "rare", "epic", "legendary"];
                if !VALID.contains(&r.as_str()) {
                    eprintln!("hatch: unknown rarity '{r}'. Valid: common, uncommon, rare, epic, legendary");
                    process::exit(1);
                }
            }

            // Same whitelist treatment as rarity — an unknown breed would
            // otherwise reach identity::breed_global_idx and panic.
            if let Some(b) = &force_breed
                && !identity::is_known_breed(b)
            {
                eprintln!("hatch: unknown breed '{b}' (breed names are case-sensitive).");
                process::exit(1);
            }

            let opts = hatch::HatchOpts { force_rarity, force_breed, force_shiny };
            hatch::run(&opts);
            process::exit(0);
        }
        "--version" | "-V" => {
            println!("purrgotchi {}", env!("CARGO_PKG_VERSION"));
        }
        _ => {
            eprintln!("purrgotchi {}", env!("CARGO_PKG_VERSION"));
            eprintln!("usage: purrgotchi <statusline|feed|turn|rename|hatch|card|pet|play|treat|stats|react>");
            process::exit(2);
        }
    }
}
