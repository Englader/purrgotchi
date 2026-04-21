mod colors;
mod cost;
mod hooks;
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
        "--version" | "-V" => {
            println!("purrgotchi {}", env!("CARGO_PKG_VERSION"));
        }
        _ => {
            eprintln!("purrgotchi {}", env!("CARGO_PKG_VERSION"));
            eprintln!("usage: purrgotchi <statusline|feed|turn|rename>");
            process::exit(2);
        }
    }
}
