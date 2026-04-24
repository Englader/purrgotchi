pub fn compute_mood(
    ctx_pct: f64,
    last_fed_delta_sec: i64,
    last_pet_delta_sec: i64,
    last_played_delta_sec: i64,
    last_treated_delta_sec: i64,
) -> &'static str {
    let ctx_int = ctx_pct.round() as i64;
    if ctx_int > 80 {
        "panic"
    } else if ctx_int > 40 {
        "alert"
    } else if (0..crate::bond::BOND_HAPPY_MOOD_WINDOW_SEC).contains(&last_pet_delta_sec)
        || (0..crate::bond::PLAY_HAPPY_MOOD_WINDOW_SEC).contains(&last_played_delta_sec)
        || (0..crate::bond::TREAT_HAPPY_MOOD_WINDOW_SEC).contains(&last_treated_delta_sec)
    {
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
    use crate::bond::{
        BOND_HAPPY_MOOD_WINDOW_SEC, PLAY_HAPPY_MOOD_WINDOW_SEC, TREAT_HAPPY_MOOD_WINDOW_SEC,
    };

    #[test]
    fn mood_happy_when_recently_petted() {
        assert_eq!(compute_mood(5.0, 0, 0, -1, -1), "happy");
        assert_eq!(compute_mood(5.0, 0, BOND_HAPPY_MOOD_WINDOW_SEC - 1, -1, -1), "happy");
    }

    #[test]
    fn mood_happy_when_recently_played() {
        assert_eq!(compute_mood(5.0, 200, -1, 0, -1), "happy");
        assert_eq!(compute_mood(5.0, 200, -1, PLAY_HAPPY_MOOD_WINDOW_SEC - 1, -1), "happy");
    }

    #[test]
    fn mood_happy_when_recently_treated() {
        assert_eq!(compute_mood(5.0, 200, -1, -1, 0), "happy");
        assert_eq!(compute_mood(5.0, 200, -1, -1, TREAT_HAPPY_MOOD_WINDOW_SEC - 1), "happy");
    }

    #[test]
    fn mood_falls_through_after_all_three_windows_elapse() {
        assert_eq!(
            compute_mood(5.0, 200, BOND_HAPPY_MOOD_WINDOW_SEC, PLAY_HAPPY_MOOD_WINDOW_SEC, TREAT_HAPPY_MOOD_WINDOW_SEC),
            "calm"
        );
    }

    #[test]
    fn mood_negative_deltas_never_trigger_happy() {
        assert_eq!(compute_mood(5.0, 200, -1, -1, -1), "calm");
    }

    #[test]
    fn mood_panic_beats_all_three_happy_signals() {
        assert_eq!(compute_mood(95.0, 0, 0, 0, 0), "panic");
    }

    #[test]
    fn mood_alert_beats_all_three_happy_signals() {
        assert_eq!(compute_mood(50.0, 0, 0, 0, 0), "alert");
    }

    #[test]
    fn mood_play_window_longer_than_pet_window() {
        // At delta > PET window but < PLAY window, play triggers happy but pet doesn't.
        let between = BOND_HAPPY_MOOD_WINDOW_SEC + 10;
        assert!(between < PLAY_HAPPY_MOOD_WINDOW_SEC);
        assert_eq!(compute_mood(5.0, 200, between, between, -1), "happy");
    }
}
