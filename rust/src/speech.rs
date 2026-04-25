const CALM: &[&str] = &[
    "just vibing",
    "mrow.",
    "watching you work",
    "purring softly",
    "keeping an eye out",
    "code code code...",
];

const HAPPY: &[&str] = &[
    "you got this!",
    "yesss tool call!",
    "nice one!",
    "mrow! productive",
    "work work work <3",
];

const ALERT: &[&str] = &[
    "getting busy...",
    "context filling",
    "watching the bars",
    "keep it tight",
    "gentle: context",
];

const PANIC: &[&str] = &[
    "LOSING CONTEXT!!",
    "CONDENSE!!",
    "aaa too much!!",
    "/clear plz...",
    "walls closing in",
];

const HUNGRY: &[&str] = &[
    "feed me tools",
    "so empty...",
    "starving for work",
    "where are tools?",
];

const SLEEP: &[&str] = &[
    "zzz 5 more min...",
    "*snoring*",
    "don't wake me",
    "dreaming of fish",
];

const LEVELUP: &[&str] = &[
    "mrow! level up!",
    "+1 purr",
    "feeling stronger",
    "*stretches proudly*",
    "level +1 nya~",
];

const EVOLVING_YOUNG: &[&str] = &[
    "not a kitten anymore...",
    "mrrreow! i've grown!",
    "look at me now!",
    "bigger paws!",
    "young cat arc \u{2726}",
];

const EVOLVING_ADULT: &[&str] = &[
    "all grown up",
    "dignified now, obviously",
    "an adult cat regards you",
    "fully realized meow",
    "adult arc unlocked",
];

pub fn compute_speech(mood: &str, now_secs: i64) -> &'static str {
    let pool: &[&str] = match mood {
        "calm" => CALM,
        "happy" => HAPPY,
        "alert" => ALERT,
        "panic" => PANIC,
        "hungry" => HUNGRY,
        "sleep" => SLEEP,
        _ => return "mrow",
    };
    let idx = ((now_secs.max(0) / 30) as usize) % pool.len();
    pool[idx]
}

pub fn compute_celebration_speech(
    kind: crate::state::CelebrationKind,
    now_secs: i64,
) -> &'static str {
    use crate::state::CelebrationKind;
    let pool: &[&str] = match kind {
        CelebrationKind::LevelUp       => LEVELUP,
        CelebrationKind::EvolvingYoung => EVOLVING_YOUNG,
        CelebrationKind::EvolvingAdult => EVOLVING_ADULT,
        CelebrationKind::None          => return "mrow",
        // D-group variants (v3.4.8) — speech pools land in T4; fall back to levelup pool.
        _ => LEVELUP,
    };
    let idx = ((now_secs.max(0) / 30) as usize) % pool.len();
    pool[idx]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CelebrationKind;

    #[test]
    fn levelup_returns_from_levelup_pool() {
        let phrase = compute_celebration_speech(CelebrationKind::LevelUp, 0);
        assert!(LEVELUP.contains(&phrase));
    }

    #[test]
    fn evolving_young_returns_from_young_pool() {
        let phrase = compute_celebration_speech(CelebrationKind::EvolvingYoung, 0);
        assert!(EVOLVING_YOUNG.contains(&phrase));
    }

    #[test]
    fn evolving_adult_returns_from_adult_pool() {
        let phrase = compute_celebration_speech(CelebrationKind::EvolvingAdult, 0);
        assert!(EVOLVING_ADULT.contains(&phrase));
    }

    #[test]
    fn none_returns_fallback_mrow() {
        assert_eq!(compute_celebration_speech(CelebrationKind::None, 0), "mrow");
    }

    #[test]
    fn celebration_speech_rotates_over_time() {
        // 30s bucket rotation — same bucket gives same phrase
        let a = compute_celebration_speech(CelebrationKind::LevelUp, 0);
        let b = compute_celebration_speech(CelebrationKind::LevelUp, 15);
        assert_eq!(a, b);
    }
}
