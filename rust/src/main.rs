mod colors;
mod cost;
mod debug;
mod hatch;
mod hooks;
mod identity;
mod mood;
mod paths;
mod payload;
mod render;
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
        "hatch" => {
            let force_rarity = args.iter().position(|a| a == "--rarity")
                .and_then(|i| args.get(i + 1))
                .map(|s| s.to_lowercase());
            let force_breed = args.iter().position(|a| a == "--breed")
                .and_then(|i| args.get(i + 1))
                .map(|s| s.to_string());
            let force_shiny = args.iter().any(|a| a == "--shiny");
            let opts = hatch::HatchOpts { force_rarity, force_breed, force_shiny };
            hatch::run(&opts);
            process::exit(0);
        }
        "--version" | "-V" => {
            println!("purrgotchi {}", env!("CARGO_PKG_VERSION"));
        }
        _ => {
            eprintln!("purrgotchi {}", env!("CARGO_PKG_VERSION"));
            eprintln!("usage: purrgotchi <statusline|feed|turn|rename|hatch|card>");
            process::exit(2);
        }
    }
}
