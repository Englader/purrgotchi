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

const STREAK: &[&str] = &[
    "5 in a row! purr-fect!",
    "combo unlocked",
    "tool chain go brrr",
    "*tail flick* nice run",
    "we're cooking",
];

const MILESTONE_CENTURION: &[&str] = &[
    "100 calls! centurion!",
    "hundo paws up",
    "first century!",
    "100 and counting...",
    "*ceremonial purr*",
];

const MILESTONE_VETERAN: &[&str] = &[
    "500! veteran of the keys",
    "half a kilo of tools",
    "battle-hardened mrow",
    "we go way back now",
    "*decorated tail swish*",
];

const MILESTONE_MASTER: &[&str] = &[
    "kilocall ascended!",
    "1000 tools! master cat!",
    "thousand-tool club",
    "we built that",
    "*standing ovation meow*",
];

const MILESTONE_LEGENDARY: &[&str] = &[
    "deep in the work",
    "long-haul partner",
    "purr-gathered wisdom",
    "another kilo down",
    "still here, still cooking",
];

const BIRTHDAY: &[&str] = &[
    "happy purr-thday!",
    "🎂 paw-ty time",
    "year older, year wiser",
    "*birthday meow*",
    "another lap around the sun",
];

const GREETING: &[&str] = &[
    "welcome back!",
    "you came back!",
    "missed you",
    "*stretches awake*",
    "mrow! about time",
];

const RANDOM_DROP: &[&str] = &[
    "found a coin!",
    "what's this? +10!",
    "treasure!",
    "lucky paw moment",
    "*pounces on shiny*",
];

// Playground scene captions (v3.8.0) — rotate on the same 30 s bucket as the
// other pools; BallChase/ButterflyChase pin a finale line for the catch beat.
const SCENE_RUN: &[&str] = &[
    "laps! gotta go fast",
    "wind in my whiskers",
    "stretchin' the paws",
];
const SCENE_ZOOMIES: &[&str] = &[
    "ZOOMIES!!",
    "can't stop won't stop",
    "the floor is lava",
];
const SCENE_BALL: &[&str] = &[
    "after it!",
    "the ball is MINE",
    "almost...",
];
const SCENE_BALL_FINALE: &str = "gotcha!!";
const SCENE_BUTTERFLY: &[&str] = &[
    "ooh, flappy friend!",
    "so close...",
    "patience... patience...",
];
const SCENE_BUTTERFLY_FINALE: &str = "pounce!! ...it got away";
const SCENE_ROLL: &[&str] = &[
    "roly poly mode",
    "wheeee~",
    "not dizzy. you're dizzy",
];
const SCENE_YARN: &[&str] = &[
    "yarn!! my nemesis",
    "push. push. push.",
    "the string must pay",
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
        CelebrationKind::LevelUp             => LEVELUP,
        CelebrationKind::EvolvingYoung       => EVOLVING_YOUNG,
        CelebrationKind::EvolvingAdult       => EVOLVING_ADULT,
        CelebrationKind::Streak              => STREAK,
        CelebrationKind::MilestoneCenturion  => MILESTONE_CENTURION,
        CelebrationKind::MilestoneVeteran    => MILESTONE_VETERAN,
        CelebrationKind::MilestoneMaster     => MILESTONE_MASTER,
        CelebrationKind::MilestoneLegendary  => MILESTONE_LEGENDARY,
        CelebrationKind::Greeting            => GREETING,
        CelebrationKind::RandomDrop          => RANDOM_DROP,
        CelebrationKind::None                => return "mrow",
    };
    let idx = ((now_secs.max(0) / 30) as usize) % pool.len();
    pool[idx]
}

pub fn compute_birthday_speech(now_secs: i64) -> &'static str {
    let idx = ((now_secs.max(0) / 30) as usize) % BIRTHDAY.len();
    BIRTHDAY[idx]
}

pub fn compute_scene_speech(
    kind: crate::state::PlaygroundScene,
    now_secs: i64,
    finale: bool,
) -> &'static str {
    use crate::state::PlaygroundScene;
    if finale {
        match kind {
            PlaygroundScene::BallChase => return SCENE_BALL_FINALE,
            PlaygroundScene::ButterflyChase => return SCENE_BUTTERFLY_FINALE,
            PlaygroundScene::None
            | PlaygroundScene::Run
            | PlaygroundScene::Zoomies
            | PlaygroundScene::Roll
            | PlaygroundScene::YarnPush => {}
        }
    }
    let pool: &[&str] = match kind {
        PlaygroundScene::Run => SCENE_RUN,
        PlaygroundScene::Zoomies => SCENE_ZOOMIES,
        PlaygroundScene::BallChase => SCENE_BALL,
        PlaygroundScene::ButterflyChase => SCENE_BUTTERFLY,
        PlaygroundScene::Roll => SCENE_ROLL,
        PlaygroundScene::YarnPush => SCENE_YARN,
        PlaygroundScene::None => return "mrow",
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

    #[test]
    fn streak_speech_returns_from_streak_pool() {
        let phrase = compute_celebration_speech(CelebrationKind::Streak, 0);
        assert!(STREAK.contains(&phrase));
    }

    #[test]
    fn milestone_centurion_speech_returns_from_centurion_pool() {
        let phrase = compute_celebration_speech(CelebrationKind::MilestoneCenturion, 0);
        assert!(MILESTONE_CENTURION.contains(&phrase));
    }

    #[test]
    fn milestone_legendary_speech_returns_from_legendary_pool() {
        let phrase = compute_celebration_speech(CelebrationKind::MilestoneLegendary, 0);
        assert!(MILESTONE_LEGENDARY.contains(&phrase));
    }

    #[test]
    fn greeting_speech_returns_from_greeting_pool() {
        let phrase = compute_celebration_speech(CelebrationKind::Greeting, 0);
        assert!(GREETING.contains(&phrase));
    }

    #[test]
    fn random_drop_speech_returns_from_drop_pool() {
        let phrase = compute_celebration_speech(CelebrationKind::RandomDrop, 0);
        assert!(RANDOM_DROP.contains(&phrase));
    }

    #[test]
    fn birthday_speech_returns_from_birthday_pool() {
        let phrase = compute_birthday_speech(0);
        assert!(BIRTHDAY.contains(&phrase));
    }

    #[test]
    fn birthday_speech_rotates_over_time() {
        // 30s bucket rotation, same as compute_speech / compute_celebration_speech.
        let a = compute_birthday_speech(0);
        let b = compute_birthday_speech(15);
        assert_eq!(a, b);
    }

    #[test]
    fn all_d_group_pools_have_five_phrases() {
        assert_eq!(STREAK.len(), 5);
        assert_eq!(MILESTONE_CENTURION.len(), 5);
        assert_eq!(MILESTONE_VETERAN.len(), 5);
        assert_eq!(MILESTONE_MASTER.len(), 5);
        assert_eq!(MILESTONE_LEGENDARY.len(), 5);
        assert_eq!(BIRTHDAY.len(), 5);
        assert_eq!(GREETING.len(), 5);
        assert_eq!(RANDOM_DROP.len(), 5);
    }

    use crate::state::PlaygroundScene;

    #[test]
    fn scene_speech_rotates_by_30s_bucket() {
        let a = compute_scene_speech(PlaygroundScene::Zoomies, 0, false);
        let b = compute_scene_speech(PlaygroundScene::Zoomies, 30, false);
        assert_ne!(a, b);
        assert_eq!(compute_scene_speech(PlaygroundScene::Zoomies, 0, false), "ZOOMIES!!");
    }

    #[test]
    fn scene_finale_pins_ball_and_butterfly() {
        assert_eq!(compute_scene_speech(PlaygroundScene::BallChase, 0, true), "gotcha!!");
        assert_eq!(
            compute_scene_speech(PlaygroundScene::ButterflyChase, 99, true),
            "pounce!! ...it got away"
        );
        // scenes without an ending beat ignore the finale flag
        assert_eq!(compute_scene_speech(PlaygroundScene::Run, 0, true), "laps! gotta go fast");
    }

    #[test]
    fn scene_none_falls_back_to_mrow() {
        assert_eq!(compute_scene_speech(PlaygroundScene::None, 0, false), "mrow");
    }
}
