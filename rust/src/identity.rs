//! Static identity data and pure lookup functions for Purrgotchi kittens.
//!
//! This module contains breed tables, stat constants, rarity-indexed star strings,
//! a deterministic FNV-1a hash, a deterministic LCG RNG, and the 320-entry BLURBS
//! lookup table indexed as `[breed_idx][peak_stat][dump_slot]`.
//!
//! No I/O, no mutable state — all pure. Consumed by `hatch.rs` and `card.rs`.

// Temporary: consumed by hatch.rs (Task 3), card.rs (Task 5), and statusline.rs (Task 6).
// Remove this allow once all three modules land.
#![allow(dead_code)]

// Stat indices
pub const CURIOSITY: usize = 0;
pub const MISCHIEF: usize = 1;
pub const LAZINESS: usize = 2;
pub const AFFECTION: usize = 3;
pub const WISDOM: usize = 4;

pub const STAT_NAMES: [&str; 5] = ["CURIOSITY", "MISCHIEF", "LAZINESS", "AFFECTION", "WISDOM"];

// Stat floors by rarity (applied to the 3 middle stats only)
pub fn stat_floor(rarity: &str) -> u8 {
    match rarity {
        "uncommon" => 15,
        "rare" => 25,
        "epic" => 35,
        "legendary" => 50,
        _ => 5,
    }
}

// Breed tables
pub const BREEDS_COMMON: [&str; 5] = ["Tabby", "Tuxedo", "Calico", "Ginger", "Grey Shorthair"];
pub const BREEDS_UNCOMMON: [&str; 3] = ["Siamese", "Russian Blue", "Bengal"];
pub const BREEDS_RARE: [&str; 3] = ["Maine Coon", "Norwegian Forest", "Turkish Van"];
pub const BREEDS_EPIC: [&str; 2] = ["Sphinx", "Scottish Fold"];
pub const BREEDS_LEGENDARY: [&str; 3] = ["Void Cat", "Snow Leopard Cat", "Chimera"];

pub fn breeds_for_rarity(rarity: &str) -> &'static [&'static str] {
    match rarity {
        "uncommon" => &BREEDS_UNCOMMON,
        "rare" => &BREEDS_RARE,
        "epic" => &BREEDS_EPIC,
        "legendary" => &BREEDS_LEGENDARY,
        _ => &BREEDS_COMMON,
    }
}

pub fn breed_global_idx(breed: &str) -> usize {
    let rarities: [&[&str]; 5] = [
        &BREEDS_COMMON,
        &BREEDS_UNCOMMON,
        &BREEDS_RARE,
        &BREEDS_EPIC,
        &BREEDS_LEGENDARY,
    ];
    let mut offset = 0;
    for group in rarities {
        if let Some(pos) = group.iter().position(|&b| b == breed) {
            return offset + pos;
        }
        offset += group.len();
    }
    panic!("breed_global_idx called with unknown breed: {breed}")
}

/// dump_slot returns the 0..=3 index of `dump` among the non-`peak` stats in ascending order.
pub fn dump_slot(peak: usize, dump: usize) -> usize {
    debug_assert_ne!(peak, dump, "peak and dump must differ");
    debug_assert!(peak < 5 && dump < 5, "stat indices must be 0..5");
    let mut slot = 0;
    for i in 0..5 {
        if i == peak { continue; }
        if i == dump { return slot; }
        slot += 1;
    }
    0
}

pub fn rarity_stars(rarity: &str, shiny: bool) -> String {
    let stars = match rarity {
        "uncommon" => "★★☆☆☆",
        "rare" => "★★★☆☆",
        "epic" => "★★★★☆",
        "legendary" => "★★★★★",
        _ => "★☆☆☆☆",
    };
    if shiny {
        format!("✦{stars}")
    } else {
        stars.to_string()
    }
}

// FNV-1a 64-bit hash
pub fn fnv1a(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in input.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x00000100000001b3);
    }
    hash
}

// Simple LCG RNG — deterministic, no external crate needed
pub struct Lcg(u64);
impl Lcg {
    pub fn new(seed: u64) -> Self { Self(seed) }
    pub fn next(&mut self) -> u64 {
        self.0 = self.0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    /// Returns value in [lo, hi] inclusive
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        debug_assert!(hi >= lo, "Lcg::range requires hi >= lo");
        lo + self.next() % (hi - lo + 1)
    }
}

// BLURBS[breed_idx 0..=15][peak_idx 0..=4][dump_slot 0..=3]
// dump_slot ordering for each peak:
//   peak=CURIOSITY(0): slots → MISCHIEF, LAZINESS, AFFECTION, WISDOM
//   peak=MISCHIEF(1):  slots → CURIOSITY, LAZINESS, AFFECTION, WISDOM
//   peak=LAZINESS(2):  slots → CURIOSITY, MISCHIEF, AFFECTION, WISDOM
//   peak=AFFECTION(3): slots → CURIOSITY, MISCHIEF, LAZINESS, WISDOM
//   peak=WISDOM(4):    slots → CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
pub const BLURBS: [[[&str; 4]; 5]; 16] = [
    // 0: Tabby
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} is endlessly curious, sniffing out every corner of the codebase. Too cautious to cause trouble, but never too cautious to investigate.",
            "{name} is a tireless explorer, always pawing at something new. Chaos? No time — there are too many interesting things to examine first.",
            "{name} wanders through the work with sharp, restless curiosity. Attachments slow you down when there's a whole world to investigate.",
            "{name} chases every new thing with gleeful abandon, rarely stopping to reflect. Learning happens anyway — through sheer volume of experience.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} is a natural-born troublemaker with a very short attention span for boring things. Why understand it when you can just knock it over?",
            "{name} causes mischief with exhausting enthusiasm, never content to sit still. A whirlwind of chaos that somehow always lands on its feet.",
            "{name} makes trouble for its own sake, unbothered by social consequences. Friendships are fine, but a good prank is better.",
            "{name} wreaks havoc with pure instinct and no forethought whatsoever. The results are unpredictable — which is, honestly, half the fun.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} has perfected the art of doing absolutely nothing. Investigating things sounds exhausting — they'll catch up eventually.",
            "{name} conserves energy with expert precision, moving only when strictly necessary. Too relaxed to cause trouble; too comfortable to bother.",
            "{name} would make a great companion if getting up weren't so much effort. The warmth is there; the initiative is not.",
            "{name} has achieved a zen-like stillness by simply never thinking too hard about anything. Not a philosopher — just very, very comfortable.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} loves deeply and without reservation, content in the warmth of familiarity. Why wander when everything good is already right here?",
            "{name} is a devoted companion, all purrs and slow blinks. The idea of causing trouble is genuinely upsetting to them.",
            "{name} pours energy into every relationship, always showing up, always present. Love requires effort, and {name} gives it generously.",
            "{name} bonds fiercely and from the gut, driven by feeling rather than understanding. The heart is very full; the head asks fewer questions.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} has seen enough to stop being surprised. The answers are already known; the questions are just someone else's problem.",
            "{name} watches everything with calm, knowing eyes. Too wise to cause trouble — but quietly amused when others do.",
            "{name} holds hard-won knowledge and dispenses it with minimal effort. The insight is there; the demonstration is optional.",
            "{name} understands deeply but keeps a measured distance. Wisdom without warmth — sharp, clear, and just a little cold.",
        ],
    ],
    // 1: Tuxedo
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} investigates everything with fastidious interest, dressed impeccably for the occasion. Chaos would wrinkle the suit.",
            "{name} is relentlessly curious, attending every event with sharp eyes and sharper opinions. Rest is for lesser cats.",
            "{name} examines the world with critical interest, taking notes. Sentiment is distracting; analysis is not.",
            "{name} is a keen observer who leaps before looking and asks questions later. The instinct is good; the reflection will come eventually.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} causes trouble with great elegance and zero guilt. The investigation phase is skipped — the disruption is its own reward.",
            "{name} is a dramatic agent of chaos who operates on a very demanding schedule. There are standards to uphold, even in mayhem.",
            "{name} makes mischief with theatrical flair and absolutely no concern for what anyone thinks. The performance is the point.",
            "{name} acts on impulse with tremendous confidence. The reasoning comes after, if at all — and it's always convincing.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} is committed to an elegant, unhurried existence. Everything interesting will arrive eventually; there's no need to chase it.",
            "{name} reclines with great dignity, conserving energy for moments that matter. Disorder is beneath them.",
            "{name} maintains a distinguished repose, sociable in theory and horizontal in practice. The warmth is there — they're just busy resting.",
            "{name} has achieved a masterful indolence, requiring no justification. The reasons are unnecessary; the comfort is self-evident.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} shows up for everyone, always, dressed for the occasion. Wandering is less interesting than being exactly where the people are.",
            "{name} is devoted and proper, firmly opposed to disruption. Love is serious business.",
            "{name} works very hard at being charming and gives it considerable effort. Every interaction is a performance of genuine warmth.",
            "{name} loves with conviction and expects it returned in kind. The heart is enormous; the philosophical framework is improvised.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} has already figured out where this is going. The journey of discovery is a bit redundant when the answer is obvious.",
            "{name} observes with a knowing air and a slight sense of superiority. Too refined for pranks; too wise for surprise.",
            "{name} carries deep understanding and presents it with great effort. Getting there takes work; the wisdom, at least, is genuine.",
            "{name} is perceptive and principled, keeping a respectful distance. Understanding is personal; closeness is a matter of choice.",
        ],
    ],
    // 2: Calico
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} is all over the place in the best way — sniffing, poking, spinning around. The chaos is accidental; the curiosity is sincere.",
            "{name} charges headlong into every new thing with electric energy. Stopping to rest feels like missing out.",
            "{name} follows every lead wherever it goes, alone or otherwise. Connections happen — briefly, warmly, and then onward.",
            "{name} is drawn to everything new, rarely pausing to understand it. The experience accumulates; the understanding is optional.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} makes things happen without knowing quite why. Investigation comes second; disruption comes first.",
            "{name} bounces between schemes with chaotic enthusiasm, never stopping. The universe bends for those who keep moving.",
            "{name} is a whirlwind who means well but leaves a trail. The love is there — it just comes with collateral.",
            "{name} acts on pure impulse and spectacular luck. The method is unclear; the results are consistently surprising.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} achieves more by doing less than most cats manage with great effort. Lucky enough to let things come to them.",
            "{name} lounges through life with unconscious grace, trouble-free and unbothered. Serenity by temperament, not by discipline.",
            "{name} radiates warmth from a comfortable horizontal position. The affection is genuine; the energy to show it is rationed.",
            "{name} drifts through the day at a lucky pace, unstressed and unconcerned. Philosophy takes effort — napping does not.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} is loyal and warm, happiest in familiar company. Chasing new things is less appealing than staying where love already lives.",
            "{name} is genuinely sweet-natured, colorful in appearance and kind in action. Mischief is someone else's style.",
            "{name} shows up bright and warm for everyone, giving freely of energy and spirit. This cat moves.",
            "{name} connects from the heart, quickly and easily, without overthinking it. The feeling is real; the theory doesn't matter.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} has made peace with not knowing everything. Understanding what matters is enough.",
            "{name} has a quiet sense of how things work and keeps it to themselves. Lucky enough to see trouble coming; wise enough to step aside.",
            "{name} carries surprising depth beneath a colorful exterior. The insight is there — accessing it just requires some effort.",
            "{name} reads situations clearly and keeps appropriate emotional distance. Wise, perceptive, and just a little reserved.",
        ],
    ],
    // 3: Ginger
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} dives into everything with loud, fiery enthusiasm. The investigation is thorough; the aftermath is orderly by some definition.",
            "{name} tears through every new thing with unlimited energy. There is no rest — only the next thing.",
            "{name} is driven by curiosity above all, chasing answers without waiting for company. Bold enough to go alone.",
            "{name} acts first and understands later, loudly. The mistakes are colorful; the lessons eventually stick.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} causes chaos on instinct and defends it confidently afterward. There's a reason — it's just not important right now.",
            "{name} is an enthusiastic force of disruption with an impressive energy budget. Nothing is too much trouble.",
            "{name} makes trouble for the love of it, needing no audience or approval. Bold, fiery, and completely self-sufficient.",
            "{name} acts with spectacular confidence and minimal foresight. The wreckage is usually colorful; the lessons are secondary.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} has found that less is, in fact, more. Bold enough to stop trying.",
            "{name} conserves energy with impressive commitment for such a lively-looking cat. The fire is decorative.",
            "{name} is warm in principle, horizontal in practice. The love is there — it just runs on its own schedule.",
            "{name} has reached a bold conclusion: doing nothing is underrated. The philosophy is minimal and the conviction is total.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} loves big and stays close, trading adventure for belonging. Home is where the fire burns brightest.",
            "{name} is fiercely devoted, all warmth and no edge. Too genuinely fond of everyone to cause them grief.",
            "{name} is high-energy in love, giving loudly and abundantly. Rest can wait; the people cannot.",
            "{name} loves with raw instinct and total conviction. The analysis can wait — the feeling is overwhelming.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} has seen enough to stop chasing it. The fire inside burns steady now, not wild.",
            "{name} carries hard-won understanding and holds it like an ember. Too knowing for pranks; too warm to be cold.",
            "{name} has great depth and the boldness to show it. Expressing it takes energy — but that's never been the problem.",
            "{name} sees clearly and holds their ground. The wisdom is warm but not soft.",
        ],
    ],
    // 4: Grey Shorthair
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} investigates everything with methodical thoroughness and zero drama. There is a right way to explore, and {name} knows it.",
            "{name} never stops looking, steadily and without fanfare. The work is its own reward.",
            "{name} pursues answers with quiet determination, preferring facts to company. The data is more reliable anyway.",
            "{name} explores systematically and records everything. The understanding will come — patience is also a skill.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} causes trouble with grey, stoic efficiency. No explanation needed; no questions asked.",
            "{name} is a surprisingly determined agent of disruption who doesn't need anyone's approval. Reliable chaos.",
            "{name} operates on their own terms, mischief included. Social opinion is irrelevant.",
            "{name} acts with total commitment and no particular plan. The results are consistent if not always predictable.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} has decided that most things don't need investigating. The stillness is earned.",
            "{name} conserves energy with stoic discipline, content in the grey quiet. Causing trouble takes effort.",
            "{name} is comfortable and self-contained, warm in their own way. Demonstrations are optional.",
            "{name} rests without guilt or philosophy. Sometimes a nap is just a nap.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} is steadily, reliably devoted. Wandering is less interesting than being counted on.",
            "{name} is quietly loyal — solid, dependable, and thoroughly unobjectionable. Trouble is someone else's department.",
            "{name} shows up every time, without fail. Dependability is its own form of love.",
            "{name} bonds from the gut, steady and true. Overthinking it would just slow everything down.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} has seen enough not to need to see more. The world makes sense; further investigation is optional.",
            "{name} understands things thoroughly and behaves accordingly. Causing disruption would be beneath the analysis.",
            "{name} carries grey, heavy understanding and applies it with steady effort. The knowledge is real; the energy to use it is found.",
            "{name} sees clearly and keeps a comfortable distance. The insight is offered when asked; otherwise, it waits.",
        ],
    ],
    // 5: Siamese
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} demands answers to every question, loudly and with great ceremony. The investigation is a performance.",
            "{name} questions everything with theatrical urgency, never satisfied, always moving. Silence is for less interesting cats.",
            "{name} is driven by a relentless need to understand, regardless of what anyone else is doing. The questions come first.",
            "{name} asks everything immediately and loudly, before reflection has a chance. The answers are sorted out afterward.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} makes trouble with the confidence of someone who has been doing this for millennia. No investigation required.",
            "{name} is dramatically, exhaustingly, magnificently chaotic. There is no off switch.",
            "{name} operates on their own dramatic schedule, unbothered by what anyone thinks of it. The audience is irrelevant.",
            "{name} acts with ancient instinct and no modern second-guessing. The results are loud.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} demands the world come to them. Investigation is for the undignified.",
            "{name} reclines with aristocratic conviction, perfectly at peace. The drama will resume when ready.",
            "{name} accepts devotion graciously while contributing minimally to reciprocation. It's a lifestyle.",
            "{name} rests like someone who has already won. The ancient wisdom permits it.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} bonds with intense, focused devotion. Wandering means leaving — and leaving is not an option.",
            "{name} is dramatically, vocally, completely devoted. Trouble would upset the object of their affection.",
            "{name} gives love with exhausting energy, never resting in the giving. The theatrics are sincere.",
            "{name} loves loudly and immediately, from the deepest part of their ancient heart. Analysis is unnecessary.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} carries knowledge that predates the question. There is nothing new to discover — only things not yet announced.",
            "{name} sees through everything with the calm certainty of a very old lineage. Too knowing for pranks — though the temptation is noted.",
            "{name} holds ancient understanding and vocalizes it extensively. Demonstrating it requires slightly more effort.",
            "{name} understands much and bonds selectively. The wisdom is offered loudly; the warmth is rationed.",
        ],
    ],
    // 6: Russian Blue
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} investigates with quiet, precise attention — methodical and unhurried. Disruption is not the goal; understanding is.",
            "{name} is a careful, persistent observer who misses very little. The stillness is deceptive.",
            "{name} pursues knowledge with cool independence, preferring accuracy to company. The data does not disappoint.",
            "{name} explores with careful interest, building understanding piece by piece. Rushing would compromise the quality.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} causes trouble with quiet, precise intent. The investigation is skipped; the outcome is planned.",
            "{name} is an unexpectedly committed agent of disruption, operating with silver efficiency. There is a schedule.",
            "{name} creates chaos on their own terms, needing neither audience nor approval. Self-contained mischief.",
            "{name} acts with elegant certainty and revises the justification afterward. The precision is in the delivery.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} has determined that most things are not worth investigating. The quietness is by design.",
            "{name} is at peace in the stillness, silver and unbothered. Trouble requires energy better spent elsewhere.",
            "{name} maintains a cool, composed repose. Connection happens on its own schedule — ideally, not now.",
            "{name} rests with aristocratic precision. The wisdom is accessible; the exertion is not.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} bonds with measured, genuine warmth, preferring depth over breadth. The familiar is worth more than the new.",
            "{name} is carefully, quietly devoted — composed in expression, consistent in action. Chaos would complicate things.",
            "{name} is steadily attentive, always present, giving with quiet reliability. Love is not a passive activity.",
            "{name} connects from a place of genuine care, guided by feeling rather than analysis. The heart knows.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} already knows. The questions others ask are, at this point, formalities.",
            "{name} understands the system and has no interest in breaking it. The elegance is in how things hold together.",
            "{name} holds deep, quiet knowledge and applies it with careful effort. The silver mind does not idle.",
            "{name} perceives with clarity and maintains a precise emotional distance. Understanding is not the same as closeness.",
        ],
    ],
    // 7: Bengal
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} is driven by insatiable, kinetic curiosity — always moving, always investigating. The wildness is in the pursuit.",
            "{name} runs at full speed toward every new thing, tireless and electric. Resting is not currently scheduled.",
            "{name} is a solo explorer, following every trail wherever it leads. Company is optional when the trail is this good.",
            "{name} leaps into everything without reading the situation first. The instincts are wild; the landing is usually fine.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} causes mayhem with the confidence of a wild thing that does not need to understand what it's breaking. The chaos is the point.",
            "{name} is an endlessly energetic force of disruption who does not stop. There is no calm mode.",
            "{name} operates on pure animal instinct, needing no approval for the trail of chaos left behind. Free.",
            "{name} acts like a wild thing — fast, decisive, and completely unconcerned with consequences. The damage is spectacular.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} has found the sun patch and is done for the day. The wildness exhausted itself; the stillness won.",
            "{name} is surprisingly committed to doing nothing, given the markings. The energy went somewhere — probably last week.",
            "{name} stretches magnificently in the warmth, self-contained and content. Other cats are fine; the sun is better.",
            "{name} rests with wild, uncomplicated conviction. No philosophy needed — the body made the decision.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} bonds intensely, staying close rather than ranging wide. The wild heart chose a home.",
            "{name} is fiercely, physically devoted — all warmth and lean muscle. The energy goes into love now.",
            "{name} shows up fast and stays present, giving with athletic intensity. Resting can wait.",
            "{name} loves from the gut, fast and certain. The analysis never catches up, and that's fine.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} moves with the instinctive wisdom of something wild — knowing without needing to learn.",
            "{name} reads situations like a hunter: fast, quiet, and accurate. The mischief doesn't fit the profile.",
            "{name} carries wild, hard-won understanding and acts on it with energy. There is no sitting still when you can see this clearly.",
            "{name} sees everything and gives warmth carefully, on wild terms. Close enough to see you; far enough to stay free.",
        ],
    ],
    // 8: Maine Coon
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} explores everything with enormous, gentle enthusiasm. A giant with soft paws and the world's greatest nose for interesting things.",
            "{name} investigates tirelessly, which is impressive given the size. Everything is worth a look.",
            "{name} follows trails alone, curious and comfortable in the vastness. The size suits the scale of the curiosity.",
            "{name} dives into everything with enthusiastic openness. The understanding develops over time — there's a lot to cover.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} makes trouble with the subtlety of something very large. The approach is not delicate, but the intent is playful.",
            "{name} is a dedicated, energetic force of gentle chaos. This much cat contains a surprising amount of mischief.",
            "{name} wreaks good-natured havoc without needing anyone's company. Independent disruption, giant scale.",
            "{name} acts first and assesses later, with the cheerful confidence of something that cannot be stopped.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} has located the ideal sleeping position and sees no reason to update it. The world can wait.",
            "{name} is a massive, gentle, horizontal presence. Too comfortable to cause trouble; too relaxed to investigate.",
            "{name} radiates warmth from a stationary position. Moving toward you is a lot of effort; the feeling is no less real.",
            "{name} rests at scale, unbothered. The wisdom is there — it just doesn't require getting up to apply.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} is a loyal, constant presence. Wandering is less appealing than being exactly where the people are.",
            "{name} is devoted and gentle — dog-like in loyalty, cat-like in grace. Trouble would upset someone.",
            "{name} follows the people with great, energetic love. Rest is for when the people are accounted for.",
            "{name} bonds from the heart, immediately and completely. The size matches the love.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} understands more than they investigate. The answers are usually obvious once you're this large.",
            "{name} sees the whole picture with calm, giant clarity. Causing trouble would ruin a perfectly good moment.",
            "{name} carries patient, deep wisdom and applies it steadily. The energy is there — the size demands it.",
            "{name} understands gently and keeps a measured, respectful distance. The wisdom is offered; the closeness is earned.",
        ],
    ],
    // 9: Norwegian Forest
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} investigates with the patience of something that has lived through several winters. Thorough, unhurried, reliable.",
            "{name} never stops watching, never stops learning. The forest teaches those who pay attention.",
            "{name} follows every thread alone, into the deep places where answers live. The trail matters more than the company.",
            "{name} explores with steady, methodical attention, building understanding the slow way. Patience is its own intelligence.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} makes trouble with the calm efficiency of something accustomed to long winters. No investigation required.",
            "{name} is a surprisingly energetic agent of disruption — the forest reserves hide depths.",
            "{name} operates alone, in their own way, by their own schedule. The disruption is personal.",
            "{name} acts with mythic certainty and the confidence of something very old. The analysis comes later — if it comes at all.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} has wintered enough to know that most things resolve themselves. The stillness is earned wisdom.",
            "{name} rests through it all with the patience of stone. The forest is calm; {name} is calmer.",
            "{name} is warm in the deep way of old things — present, steady, not demonstrative. The feeling is vast.",
            "{name} rests with mythic patience. The insight is there; the effort to present it can wait until spring.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} bonds with the depth and steadiness of old roots. The familiar is where loyalty lives.",
            "{name} is quietly, profoundly devoted. The forest is gentle with those it loves.",
            "{name} shows up through every season, reliably and with great warmth. Love is not passive when it is this deep.",
            "{name} loves with the certainty of something ancient. The reasons are felt, not argued.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} has learned what the forest teaches: some things take a very long time to understand, and that is fine.",
            "{name} sees through the trees with old, quiet clarity. The mischief is for younger cats.",
            "{name} carries deep, weathered wisdom and expresses it with effort. The knowing is vast; the vocabulary for it is being found.",
            "{name} understands the long arc of things and loves carefully, from a patient distance.",
        ],
    ],
    // 10: Turkish Van
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} follows every strange current wherever it leads, alone and unworried. The eccentricity is just the curiosity in motion.",
            "{name} never stops investigating, even the wet and strange things other cats avoid. Especially those.",
            "{name} goes where the interesting things are, with or without company. The trail doesn't wait.",
            "{name} dives into everything headfirst, including things that turn out to be water. The reflection happens eventually.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} makes trouble by following instinct directly into the most unexpected places. The reason comes after.",
            "{name} is an eccentric force of disruption with unusual energy and highly specific interests. There is method in this.",
            "{name} causes chaos on their own schedule, by their own strange logic. Approval is irrelevant.",
            "{name} acts with the eccentric certainty of something that has never been fully explained. The results are original.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} floats through the day without urgency. Investigating things would mean getting up.",
            "{name} is calm in an unusual way — too settled for chaos, too strange for ordinary peace.",
            "{name} is comfortable in their own eccentric company. Warmth on request; solitude by default.",
            "{name} rests with the confidence of something that has its own understanding of things.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} bonds with eccentric intensity, faithful to those who match the strangeness. Wandering is for before you found them.",
            "{name} is devoted in an unusual way — specific, odd, and entirely genuine.",
            "{name} gives love with eccentric energy, in the specific ways that matter. The method is original.",
            "{name} bonds with strange, certain warmth. Understanding why is not the point.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} has an eccentric understanding of things that works, inexplicably, very well.",
            "{name} sees the strange logic in everything and keeps it. The mischief doesn't fit the pattern.",
            "{name} holds unusual wisdom and demonstrates it with eccentric effort. The energy is saved for the right moments.",
            "{name} understands things oddly and accurately. The warmth is there; the expression of it is its own thing.",
        ],
    ],
    // 11: Sphinx
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} observes everything with alien precision — seeing through surfaces, into structure. The chaos has already been catalogued.",
            "{name} investigates relentlessly, without fur to soften the intensity of the gaze. Nothing escapes.",
            "{name} pursues understanding alone, as is only logical. Connection is a variable; knowledge is the constant.",
            "{name} investigates everything with complete openness, the alien mind making no assumptions. Understanding takes time. That is fine.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} disrupts with surgical precision, having already assessed every possible outcome. The chaos is planned.",
            "{name} is a committed, unhurried agent of disruption. The schedule is internal and non-negotiable.",
            "{name} acts alone, in ways that are not easily explained. The mischief is private.",
            "{name} causes disruption with alien certainty. The justification exists — it is simply not available to others.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} sees no reason to investigate what can be understood from a seated position. The perception does not require motion.",
            "{name} reclines with an air of total comprehension. The stillness is a choice, not a limitation.",
            "{name} is warm in an alien way — perceivable only at close range, and not always then.",
            "{name} rests in perfect stillness, the mind fully occupied. The body is simply not relevant right now.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} bonds with alien intensity, completely and without reservation. The object of affection is everything.",
            "{name} is devoted with unsettling precision. Nothing about the care is casual.",
            "{name} gives love with alien energy — focused, direct, and constant. The effort is invisible because the need is not.",
            "{name} bonds with the entire force of a very unusual heart. The analysis is suspended when the feeling is this clear.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} already knows. The universe has been considered. Additional data is not currently required.",
            "{name} sees through everything with alien clarity. The mischief is already predicted; the choice not to intervene is deliberate.",
            "{name} holds alien wisdom of extraordinary depth and presents it from a stationary position. Getting up would change nothing.",
            "{name} understands everything and keeps a precise, considered distance. The perception is total; the warmth is selective.",
        ],
    ],
    // 12: Scottish Fold
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} investigates everything with round, attentive eyes and a peaceful manner. The universe is very interesting; hurting it would be wrong.",
            "{name} is a steady, gentle explorer, curious in a way that does not require urgency. Things reveal themselves.",
            "{name} investigates quietly and alone, the folded ears tilted toward every sound. The world is endlessly fascinating.",
            "{name} explores with open, round wonder, processing slowly and sincerely. The understanding arrives in its own time.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} causes trouble in a surprisingly gentle way. The disruption is soft; the intent is ambiguous.",
            "{name} is a committed, round-faced agent of mild chaos. Determined. Unhurried. Effective.",
            "{name} operates on their own gentle terms, needing no acknowledgment of the damage done.",
            "{name} acts with the calm confidence of something that has already forgiven itself.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} is perfectly, roundly comfortable. Investigating things sounds like getting up.",
            "{name} is gentle and still, the folded ears at peace. There is no reason to cause trouble from this position.",
            "{name} radiates warmth from a perfectly folded position. Moving toward you is effort; the intention is sincere.",
            "{name} rests with inexplicable wisdom. The comfort is not shallow — it is well-considered.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} is warmly, completely devoted. The world is interesting; the people in it are more interesting.",
            "{name} gives love gently and continuously. The disruption is not in {name}'s vocabulary.",
            "{name} shows up with round, warm consistency. Love is active, even for a cat this comfortable.",
            "{name} bonds immediately and sincerely, from a place of pure feeling. The analysis is not needed.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} understands more than is readily apparent. The round eyes have seen a great deal.",
            "{name} sees everything with gentle clarity. The mischief is noted, assessed, and declined.",
            "{name} carries surprising depth in a very comfortable form. The wisdom comes out when asked.",
            "{name} perceives with soft, certain clarity and keeps a gentle distance. Understanding is offered freely; closeness, carefully.",
        ],
    ],
    // 13: Void Cat
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} sees through the fabric of things, finding questions in the dark. The chaos is already catalogued; the investigation continues.",
            "{name} moves through the spaces between knowing, relentless and silent. There is no rest in the void.",
            "{name} follows every thread into the dark alone. The truth does not require company.",
            "{name} investigates everything with void-dark hunger, absorbing before understanding. The meaning surfaces later, from the depth.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} disrupts reality quietly and without explanation. The investigation was skipped; the result speaks for itself.",
            "{name} is a committed, silent agent of cosmic chaos. The energy is infinite; the rest is unnecessary.",
            "{name} operates alone, between worlds, according to rules not shared with others. The mischief is private.",
            "{name} acts with the certainty of something that exists outside cause and effect. The justification is not available to linear minds.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} does not investigate. The void already contains everything. Rest is not laziness — it is understanding.",
            "{name} is still as the deep dark, and just as absolute. Disruption requires motion; the void requires none.",
            "{name} radiates warmth from a place of absolute stillness, like a star that has gone quiet. It is still there.",
            "{name} rests in the void with complete comprehension. Motion would add nothing to what is already known.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} bonds across the dark with singular, absolute devotion. The familiar is worth more than the infinite.",
            "{name} loves with a void-depth that is impossible to disturb. The chaos is on the outside; the love is at the center.",
            "{name} reaches across the dark toward you with constant, quiet energy. Love does not rest.",
            "{name} loves with cosmic certainty, from the center of something vast and dark and warm.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} knows without needing to ask. The void has already answered.",
            "{name} sees from beyond the edge of things. The mischief was always going to happen — {name} simply watched.",
            "{name} holds the weight of everything known and carries it without effort. The wisdom is the void.",
            "{name} understands from a great, dark distance. The love is seen; the distance is respected.",
        ],
    ],
    // 14: Snow Leopard Cat
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} investigates with the focused intensity of something built for high altitudes and hard truths. Clean, precise, undistracted.",
            "{name} ranges wide and far, tracking every interesting thing. The cold does not discourage; neither does distance.",
            "{name} follows trails that lead far from others. The investigation is the destination.",
            "{name} moves toward everything with athletic openness, making sense of it in the long walk back.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} disrupts with the clean efficiency of something that does not need to understand what it's doing. Instinct is enough.",
            "{name} is a powerful, unhesitating force of disruption — rare enough to be unexpected.",
            "{name} operates alone at altitude, causing trouble on their own terms. No witnesses required.",
            "{name} acts with solitary certainty. The analysis is not the point; the action is.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} has chosen the high ledge and sees no reason to descend. The world can come to them.",
            "{name} rests with the stillness of something that is never truly still. The power is conserved, not gone.",
            "{name} is warmly solitary — present in a remote way, connected across great distance.",
            "{name} rests at altitude with perfect clarity. The body is still; the understanding is not.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} bonds with rare, powerful devotion — choosing once and completely. The trail ends here.",
            "{name} is fiercely protective and genuinely soft with those who earn it. The rare heart, opened.",
            "{name} moves for those it loves with the full force of something rarely seen. The energy is real.",
            "{name} loves from a rare, powerful certainty. The feeling does not require justification.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} sees from high ground what others miss in the valley. The questions have already been answered.",
            "{name} reads the terrain with rare, clear precision. The mischief is already predicted; the intervention is selective.",
            "{name} carries the wisdom of high places and expresses it with the effort of something accustomed to difficult terrain.",
            "{name} understands from a rare, solitary distance. The warmth is there — seen from the ridge.",
        ],
    ],
    // 15: Chimera
    [
        // peak CURIOSITY; dumps: MISCHIEF, LAZINESS, AFFECTION, WISDOM
        [
            "{name} investigates everything from both angles simultaneously. One side reasons; the other leaps first. The result is comprehensive.",
            "{name} is drawn in two directions at once, both curious, neither resting. The split makes for extraordinary range.",
            "{name} follows every question wherever it leads, the two halves in rare agreement on this. Understanding comes before belonging.",
            "{name} explores with the dual energy of two minds in one body — more territory covered, less time to reflect.",
        ],
        // peak MISCHIEF; dumps: CURIOSITY, LAZINESS, AFFECTION, WISDOM
        [
            "{name} causes spectacular, split-personality chaos. One half planned it; the other half is equally committed.",
            "{name} is a relentless force of disruption driven by both sides simultaneously. There is no rest in a divided soul.",
            "{name} wreaks havoc in stereo, needing no audience and offering no apology. The two halves agree on this.",
            "{name} acts with doubled conviction and minimal forethought. The reasoning varies by which side you ask.",
        ],
        // peak LAZINESS; dumps: CURIOSITY, MISCHIEF, AFFECTION, WISDOM
        [
            "{name} has found the rare point where both sides agree: this is a good place to stop.",
            "{name} is still, which is unusual — the two halves have, for now, reached equilibrium.",
            "{name} is warm in a divided way — one half reaching out, the other content to let things be.",
            "{name} rests with the peace of something that has made its peace with its own contradiction.",
        ],
        // peak AFFECTION; dumps: CURIOSITY, MISCHIEF, LAZINESS, WISDOM
        [
            "{name} bonds with the full force of two hearts, both choosing the same thing. The split resolves in love.",
            "{name} is devotedly, bilaterally warm. Both halves agree: this person is worth protecting.",
            "{name} gives love with the energy of two natures fully aligned. The split disappears when the feeling is this clear.",
            "{name} loves completely, from both sides at once. The analysis is suspended when the heart is unanimous.",
        ],
        // peak WISDOM; dumps: CURIOSITY, MISCHIEF, LAZINESS, AFFECTION
        [
            "{name} understands things from two angles at once. The conclusions converge.",
            "{name} sees everything from both sides and chooses not to act. The wisdom is in the restraint.",
            "{name} carries double the insight and expresses it with effort proportional to whichever side is currently dominant.",
            "{name} understands with split, overlapping clarity and keeps a distance calibrated by both halves independently.",
        ],
    ],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_is_deterministic() {
        assert_eq!(fnv1a("user:host:1000"), fnv1a("user:host:1000"));
        assert_ne!(fnv1a("user:host:1000"), fnv1a("user:host:1001"));
    }

    #[test]
    fn lcg_is_deterministic() {
        let mut a = Lcg::new(42);
        let mut b = Lcg::new(42);
        assert_eq!(a.range(0, 99), b.range(0, 99));
        assert_eq!(a.range(0, 99), b.range(0, 99));
    }

    #[test]
    fn breeds_for_rarity_correct_counts() {
        assert_eq!(breeds_for_rarity("common").len(), 5);
        assert_eq!(breeds_for_rarity("uncommon").len(), 3);
        assert_eq!(breeds_for_rarity("rare").len(), 3);
        assert_eq!(breeds_for_rarity("epic").len(), 2);
        assert_eq!(breeds_for_rarity("legendary").len(), 3);
    }

    #[test]
    fn all_blurbs_non_empty() {
        for b in 0..16 {
            for p in 0..5 {
                for d in 0..4 {
                    assert!(!BLURBS[b][p][d].is_empty(), "blurb [{b}][{p}][{d}] is empty");
                }
            }
        }
    }

    #[test]
    fn dump_slot_correct() {
        // peak=CURIOSITY(0), dump=WISDOM(4) → slot 3
        assert_eq!(dump_slot(0, 4), 3);
        // peak=WISDOM(4), dump=CURIOSITY(0) → slot 0
        assert_eq!(dump_slot(4, 0), 0);
        // peak=MISCHIEF(1), dump=LAZINESS(2) → slot 1
        assert_eq!(dump_slot(1, 2), 1);
    }

    #[test]
    fn rarity_stars_correct() {
        assert_eq!(rarity_stars("common", false), "★☆☆☆☆");
        assert_eq!(rarity_stars("legendary", false), "★★★★★");
        assert_eq!(rarity_stars("legendary", true), "✦★★★★★");
    }

    #[test]
    fn breed_global_idx_known_breeds() {
        assert_eq!(breed_global_idx("Tabby"), 0);
        assert_eq!(breed_global_idx("Grey Shorthair"), 4);
        assert_eq!(breed_global_idx("Siamese"), 5);
        assert_eq!(breed_global_idx("Sphinx"), 11);
        assert_eq!(breed_global_idx("Void Cat"), 13);
        assert_eq!(breed_global_idx("Chimera"), 15);
    }

    #[test]
    #[should_panic(expected = "unknown breed")]
    fn breed_global_idx_panics_on_unknown() {
        breed_global_idx("Mystery Cat");
    }
}
