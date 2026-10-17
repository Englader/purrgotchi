//! Minimal argv helpers — the binary deliberately has no CLI-parsing dependency.
//! `args` is the full `env::args()` vector: `[exe, subcommand, rest…]`.

pub const MAX_FEED_COUNT: u32 = 10_000;

pub fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().skip(2).any(|a| a == flag)
}

/// The arg after `flag`, when both are present.
pub fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let i = args.iter().skip(2).position(|a| a == flag)? + 2;
    args.get(i + 1).map(String::as_str)
}

/// Positional args after the subcommand. Only the listed flags count as flags
/// (value flags also swallow the next arg); any other dash-word stays
/// positional, so `rename -Kitty` keeps working as it always has.
// Used by `rename` now; the value-flag form is for later commands (cost, frame).
#[allow(dead_code)]
pub fn positionals<'a>(args: &'a [String], bool_flags: &[&str], value_flags: &[&str]) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut i = 2;
    while i < args.len() {
        let a = args[i].as_str();
        if bool_flags.contains(&a) {
            i += 1;
        } else if value_flags.contains(&a) {
            i += 2;
        } else {
            out.push(a);
            i += 1;
        }
    }
    out
}

pub fn parse_count(v: &str) -> Option<u32> {
    v.trim().parse::<u32>().ok().filter(|n| (1..=MAX_FEED_COUNT).contains(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn has_flag_ignores_exe_and_subcommand() {
        assert!(has_flag(&a(&["p", "feed", "--json"]), "--json"));
        assert!(!has_flag(&a(&["--json", "feed"]), "--json"));
    }

    #[test]
    fn flag_value_reads_next_arg() {
        let v = a(&["p", "feed", "--json", "--count", "7"]);
        assert_eq!(flag_value(&v, "--count"), Some("7"));
        assert_eq!(flag_value(&a(&["p", "feed", "--count"]), "--count"), None);
        assert_eq!(flag_value(&v, "--width"), None);
    }

    #[test]
    fn positionals_skip_only_known_flags() {
        let v = a(&["p", "rename", "--json", "-Kitty"]);
        assert_eq!(positionals(&v, &["--json"], &[]), vec!["-Kitty"]);
        let v = a(&["p", "cost", "--session", "s1", "x"]);
        assert_eq!(positionals(&v, &[], &["--session"]), vec!["x"]);
    }

    #[test]
    fn parse_count_bounds() {
        assert_eq!(parse_count("1"), Some(1));
        assert_eq!(parse_count("10000"), Some(10_000));
        for bad in ["0", "10001", "-1", "abc", ""] {
            assert_eq!(parse_count(bad), None, "{bad}");
        }
    }
}
