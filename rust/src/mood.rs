pub fn compute_mood(
    ctx_pct: f64,
    last_fed_delta_sec: i64,
    last_pet_delta_sec: i64,
) -> &'static str {
    let ctx_int = ctx_pct.round() as i64;
    if ctx_int > 80 {
        "panic"
    } else if ctx_int > 40 {
        "alert"
    } else if (0..crate::bond::BOND_HAPPY_MOOD_WINDOW_SEC).contains(&last_pet_delta_sec) {
        "happy"
    } else if last_fed_delta_sec > 21600 {
        "sleep"
    } else if last_fed_delta_sec > 3600 {
        "hungry"
    } else if last_fed_delta_sec < 30 {
        "happy"
    } else {
        "calm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bond::BOND_HAPPY_MOOD_WINDOW_SEC;

    #[test]
    fn mood_happy_when_recently_petted() {
        // Within the window, mood is happy regardless of feed time.
        assert_eq!(compute_mood(5.0, 0, 0), "happy");
        assert_eq!(compute_mood(5.0, 0, BOND_HAPPY_MOOD_WINDOW_SEC - 1), "happy");
    }

    #[test]
    fn mood_falls_through_after_pet_window_elapses() {
        // last_pet_delta >= window → happy does not apply; existing rules win.
        // At delta=0s fed, this should be happy-by-feed (< 30s), but the new
        // rule is about pet, not feed. Pass large feed delta to avoid the feed
        // "happy" rule and land on "calm".
        assert_eq!(
            compute_mood(5.0, 200, BOND_HAPPY_MOOD_WINDOW_SEC),
            "calm"
        );
    }

    #[test]
    fn mood_negative_last_pet_delta_means_never_petted_no_happy() {
        // Convention: negative last_pet_delta means "never petted" (caller
        // passes -1 or similar sentinel). Must NOT trigger happy override.
        assert_eq!(compute_mood(5.0, 200, -1), "calm");
    }

    #[test]
    fn mood_panic_beats_happy_even_when_recently_petted() {
        assert_eq!(compute_mood(95.0, 0, 0), "panic");
    }

    #[test]
    fn mood_alert_beats_happy_even_when_recently_petted() {
        assert_eq!(compute_mood(50.0, 0, 0), "alert");
    }
}
