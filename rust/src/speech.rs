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
