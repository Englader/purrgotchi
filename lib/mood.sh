#!/usr/bin/env bash
# compute_mood out_var ctx_pct cost_usd last_fed_delta_sec
# Writes result to out_var via printf -v (no subshell on call site).

compute_mood() {
    local _out="$1"
    local ctx_pct="${2:-0}" cost_usd="${3:-0}" last_fed_delta="${4:-999999}"
    local ctx_int cost_int result
    printf -v ctx_int  "%.0f" "$ctx_pct"
    printf -v cost_int "%.0f" "$cost_usd"

    if   [ "$ctx_int" -gt 80 ] 2>/dev/null || [ "$cost_int" -gt 5 ] 2>/dev/null; then result="panic"
    elif [ "$ctx_int" -gt 40 ]              2>/dev/null; then result="alert"
    elif [ "$last_fed_delta" -gt 21600 ]    2>/dev/null; then result="sleep"
    elif [ "$last_fed_delta" -gt 3600 ]     2>/dev/null; then result="hungry"
    elif [ "$last_fed_delta" -lt 30 ]       2>/dev/null; then result="happy"
    else                                                      result="calm"
    fi
    printf -v "$_out" '%s' "$result"
}
