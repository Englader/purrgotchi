mod bond;
mod card;
mod colors;
mod cost;
mod cost_acc;
mod debug;
mod events;
mod hatch;
mod hooks;
mod identity;
mod llm;
mod mood;
mod paths;
mod payload;
mod quip_cache;
mod react;
mod render;
mod spawn;
mod speech;
mod sprites;

mod state;
mod statusline;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("");

    debug::log(&format!("invoke cmd={cmd}"));

    match cmd {
        "statusline" => {
            statusline::run();
            process::exit(0);
        }
        "feed" => {
            hooks::feed();
            process::exit(0);
        }
        "turn" => {
            hooks::turn();
            process::exit(0);
        }
        "rename" => {
            let name = args.get(2).map(String::as_str).unwrap_or("");
            process::exit(hooks::rename(name));
        }
        "card" => {
            card::run();
            process::exit(0);
        }
        "pet" => {
            hooks::pet();
            process::exit(0);
        }
        "play" => {
            hooks::play();
            process::exit(0);
        }
        "treat" => {
            hooks::treat();
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
