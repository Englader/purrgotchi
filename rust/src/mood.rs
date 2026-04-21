pub fn compute_mood(ctx_pct: f64, last_fed_delta_sec: i64) -> &'static str {
    let ctx_int = ctx_pct.round() as i64;
    if ctx_int > 80 {
        "panic"
    } else if ctx_int > 40 {
        "alert"
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
