mod bond;
mod card;
mod cli;
mod colors;
mod cost;
mod cost_acc;
mod debug;
mod events;
mod frame;
mod hatch;
mod hooks;
mod identity;
mod json;
mod llm;
mod mood;
mod paths;
mod payload;
mod pixel;
mod playground;
mod px;
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
    paths::load_trusted_env();

    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("");

    debug::log(&format!("invoke cmd={cmd}"));

    let json = cli::has_flag(&args, "--json");
    match cmd {
        "statusline" => {
            statusline::run();
            process::exit(0);
        }
        "feed" => {
            let count = match cli::opt_value(&args, "--count").map(|v| v.map_or(Some(1), cli::parse_count)) {
                Ok(Some(n)) => n,
                _ => {
                    bad_args("feed", &format!("--count must be an integer in 1..={}", cli::MAX_FEED_COUNT), json);
                    process::exit(2);
                }
            };
            process::exit(hooks::feed_cmd(count, json));
        }
        "turn" => process::exit(hooks::turn_cmd(json)),
        "rename" => {
            let name = cli::positionals(&args, &["--json"], &[]).first().copied().unwrap_or("");
            process::exit(hooks::rename_cmd(name, json));
        }
        "card" => process::exit(card::run_cmd(json, cli::has_flag(&args, "--plain"))),
        "pet" => process::exit(hooks::pet_cmd(json)),
        "play" => process::exit(hooks::play_cmd(json)),
        "treat" => process::exit(hooks::treat_cmd(json)),
        "stats" => process::exit(hooks::stats_cmd(json)),
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
        "frame" => match frame::parse_opts(&args) {
            Ok(opts) => process::exit(frame::run(&opts)),
            Err(msg) => {
                bad_args("frame", &msg, json);
                eprintln!("usage: purrgotchi frame [--json] [--width N] [--tick T] [--context-pct P] [--advance] [--pixels]");
                process::exit(2);
            }
        },
        "pixels" => match args.get(2).map(String::as_str) {
            Some("test") => {
                println!("{}", px::test_card().join("\n"));
                process::exit(0);
            }
            Some("sheet") => {
                let breed = cli::flag_value(&args, "--breed").unwrap_or("Ginger");
                println!("{}", px::sheet(breed, json));
                process::exit(0);
            }
            _ => {
                eprintln!("usage: purrgotchi pixels test|sheet [--breed B] [--json]");
                process::exit(2);
            }
        },
        "--version" | "-V" => {
            println!("purrgotchi {}", env!("CARGO_PKG_VERSION"));
        }
        _ => {
            eprintln!("purrgotchi {}", env!("CARGO_PKG_VERSION"));
            eprintln!("usage: purrgotchi <statusline|frame|pixels|feed|turn|rename|hatch|card|pet|play|treat|stats|react> [--json]");
            process::exit(2);
        }
    }
}

/// Bad CLI arguments: the classic `<cmd>: <detail>` line on stderr, plus a
/// `bad_args` error object on stdout under `--json`. Caller exits 2.
fn bad_args(cmd: &str, detail: &str, json: bool) {
    eprintln!("{cmd}: {detail}");
    if json {
        json::print_bad_args(&format!("{cmd}: {detail}"));
    }
}
