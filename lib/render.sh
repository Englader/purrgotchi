#!/usr/bin/env bash
# Rendering helpers for Purrgotchi.
# All functions write their result to a named variable via printf -v
# (no subshells). Usage pattern:
#   progress_bar myvar 42 12   # sets $myvar to "█████░░░░░░░"

C_CYAN='\033[38;2;10;189;198m'
C_PINK='\033[38;2;234;0;217m'
C_MAGENTA='\033[38;2;113;28;145m'
C_YELLOW='\033[38;2;243;243;27m'
C_BLUE='\033[38;2;19;62;124m'
C_GREEN='\033[38;2;87;236;166m'
C_RED='\033[38;2;255;84;104m'
C_DIM='\033[2m'
C_BOLD='\033[1m'
C_RESET='\033[0m'

# progress_bar out_var pct width
progress_bar() {
    local _out="$1" pct="${2:-0}" width="${3:-12}"
    local pct_int filled empty bar='' i
    printf -v pct_int "%.0f" "$pct"
    [ "$pct_int" -lt 0 ]   2>/dev/null && pct_int=0
    [ "$pct_int" -gt 100 ] 2>/dev/null && pct_int=100
    filled=$(( pct_int * width / 100 ))
    empty=$(( width - filled ))
    for ((i=0; i<filled; i++)); do bar+='█'; done
    for ((i=0; i<empty;  i++)); do bar+='░'; done
    printf -v "$_out" '%s' "$bar"
}

# human_duration out_var ms
human_duration() {
    local _out="$1" ms="${2:-0}"
    local s=$(( ms / 1000 ))
    local h=$(( s / 3600 ))
    local m=$(( (s % 3600) / 60 ))
    local rs=$(( s % 60 ))
    if   [ "$h" -gt 0 ]; then printf -v "$_out" "%dh %dm %ds" "$h" "$m" "$rs"
    elif [ "$m" -gt 0 ]; then printf -v "$_out" "%dm %ds" "$m" "$rs"
    else                      printf -v "$_out" "%ds" "$rs"
    fi
}

# time_until_reset out_var reset_epoch now_epoch — pure bash, no subshells.
# Drops leading units when they're zero: "in 20h 00m" (no 0d prefix),
# "in 45m" (no 0h prefix). Minutes always zero-padded when hours are present.
time_until_reset() {
    local _out="$1" reset_at="${2:-0}" now_ts="${3:-0}"
    if [ "${reset_at:-0}" -le 0 ] 2>/dev/null || [ "$reset_at" -le "$now_ts" ] 2>/dev/null; then
        printf -v "$_out" "%s" "---"
    else
        local delta=$(( reset_at - now_ts ))
        local d=$(( delta / 86400 ))
        local h=$(( (delta % 86400) / 3600 ))
        local m=$(( (delta % 3600) / 60 ))
        if   [ "$d" -gt 0 ]; then printf -v "$_out" "in %dd %02dh %02dm" "$d" "$h" "$m"
        elif [ "$h" -gt 0 ]; then printf -v "$_out" "in %dh %02dm" "$h" "$m"
        else                      printf -v "$_out" "in %dm" "$m"
        fi
    fi
}

# human_elapsed out_var seconds_ago
human_elapsed() {
    local _out="$1" d="${2:-0}"
    if   [ "$d" -lt 60 ]    2>/dev/null; then printf -v "$_out" "%ds ago" "$d"
    elif [ "$d" -lt 3600 ]  2>/dev/null; then printf -v "$_out" "%dm ago" "$(( d / 60 ))"
    elif [ "$d" -lt 86400 ] 2>/dev/null; then printf -v "$_out" "%dh ago" "$(( d / 3600 ))"
    else                                      printf -v "$_out" "%dd ago" "$(( d / 86400 ))"
    fi
}

# short_cwd out_var full_path — collapses $HOME to ~, left-ellipsizes at 38 cols.
short_cwd() {
    local _out="$1" cwd="$2" home="$HOME" result
    case "$cwd" in
        "$home")   result="~" ;;
        "$home"/*) result="~${cwd#$home}" ;;
        "")        result="?" ;;
        *)         result="$cwd" ;;
    esac
    if [ "${#result}" -gt 25 ]; then
        result="...${result: -22}"
    fi
    printf -v "$_out" "%s" "$result"
}
