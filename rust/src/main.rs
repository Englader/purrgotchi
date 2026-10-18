mod band;
mod budget_cache;
mod bond;
mod card;
mod cli;
mod colors;
mod cost;
mod cost_acc;
mod debug;
mod events;
mod frame;
mod heartbeat;
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
mod speech_box;
mod sprites;

mod state;
mod term_write;
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
        // Hidden: not in usage or README. Used by the mod backdrop hook.
        "term-write" => process::exit(term_write::run()),
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
                eprintln!("usage: purrgotchi frame [--json] [--width N] [--tick T] [--context-pct P] [--advance]");
                process::exit(2);
            }
        },
        "band" => match band::parse_opts(&args) {
            Ok(opts) => process::exit(band::run(&opts)),
            Err(msg) => {
                bad_args("band", &msg, false);
                eprintln!("{}", band::USAGE);
                process::exit(2);
            }
        },
        "pixels" => match args.get(2).map(String::as_str) {
            Some("test") => {
                println!("{}", px::test_card().join("\n"));
                process::exit(0);
            }
            Some("sheet") => {
                let (breed, _) = dev_look(&args);
                println!("{}", px::sheet(breed, dev_height(&args), json));
                process::exit(0);
            }
            Some("downscale") => match dev_height(&args) {
                px::PxHeight::H12 => {
                    eprintln!("usage: purrgotchi pixels downscale --height 10");
                    process::exit(2);
                }
                h => {
                    print!("{}", px::art10::downscale_source(h));
                    process::exit(0);
                }
            },
            Some("scene") => {
                let Some(kind) = playground::force_scene_from(args.get(3).map(String::as_str)) else {
                    eprintln!("usage: purrgotchi pixels scene run|zoomies|ball|butterfly|roll|yarn [--width W] [--breed B] [--stage S] [--json]");
                    process::exit(2);
                };
                let width = dev_width(&args);
                let (breed, stage) = dev_look(&args);
                println!("{}", px::scene_sheet(kind, width, breed, stage, dev_height(&args), json));
                process::exit(0);
            }
            Some("celebrate") => {
                let width = dev_width(&args);
                let (breed, stage) = dev_look(&args);
                let Some(sheet) = args.get(3).and_then(|n| px::celebrate_sheet(n, width, breed, stage, dev_height(&args), json)) else {
                    eprintln!("usage: purrgotchi pixels celebrate levelup|young|adult|streak|centurion|veteran|master|legendary|greeting|drop|birthday [--width W] [--breed B] [--stage S] [--json]");
                    process::exit(2);
                };
                println!("{sheet}");
                process::exit(0);
            }
            _ => {
                eprintln!("usage: purrgotchi pixels test|sheet|scene|celebrate|downscale [--breed B] [--height 12|10] [--json]");
                process::exit(2);
            }
        },
        "--version" | "-V" => {
            println!("purrgotchi {}", env!("CARGO_PKG_VERSION"));
        }
        _ => {
            eprintln!("purrgotchi {}", env!("CARGO_PKG_VERSION"));
            eprintln!("usage: purrgotchi <statusline|frame|band|pixels|feed|turn|rename|hatch|card|pet|play|treat|stats|react> [--json]");
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

/// `--width` of the `pixels` dev verbs, clamped like the statusline's cat column.
fn dev_width(args: &[String]) -> usize {
    let w = cli::flag_value(args, "--width").and_then(|w| w.parse().ok()).unwrap_or(playground::DEFAULT_STRIP);
    playground::clamp_strip(w).unwrap_or_else(|| {
        eprintln!("--width must be at least {} (capped at {})", playground::MIN_STRIP, playground::MAX_STRIP);
        process::exit(2);
    })
}

/// `--height 12|10` of the `pixels` dev verbs (default 12).
fn dev_height(args: &[String]) -> px::PxHeight {
    let Some(v) = cli::flag_value(args, "--height") else { return px::PxHeight::H12 };
    v.parse().ok().and_then(px::PxHeight::from_px).unwrap_or_else(|| {
        eprintln!("--height must be 12 or 10, got {v:?}");
        process::exit(2);
    })
}

/// `--breed` / `--stage` of the `pixels` dev verbs.
fn dev_look(args: &[String]) -> (&str, &str) {
    (cli::flag_value(args, "--breed").unwrap_or("Ginger"), cli::flag_value(args, "--stage").unwrap_or("adult"))
}
