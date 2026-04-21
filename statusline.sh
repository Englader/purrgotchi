#!/usr/bin/env bash
# Purrgotchi statusline — cyberpunk HUD + tamagotchi kitten.
# Aggressively optimized: 1 jq call total, bash regex for state.json,
# printf -v everywhere (no $(...) subshells on hot path).
set -uo pipefail

# Source location — resolves to wherever this script lives (libs + sprites).
# Lets the same code run from the dev repo or an installed copy without edits.
PURR_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
# Runtime state — always in Claude config dir, one kitten per machine.
PURR_STATE_DIR="$HOME/.claude/purrgotchi"
PURR_STATE="$PURR_STATE_DIR/state.json"

# shellcheck disable=SC1091
source "$PURR_DIR/lib/render.sh"
# shellcheck disable=SC1091
source "$PURR_DIR/lib/mood.sh"

# ---- Stdin (using cat for reliability across spawn contexts) ---------------
# Note: $(</dev/stdin) was faster but proved unreliable when invoked by
# Claude Code on Windows. $(cat) adds one subprocess but always works.
INPUT=$(cat)

# ---- Debug: capture last payload for schema inspection ---------------------
printf '%s\n' "$INPUT" > "$PURR_DIR/last-input.json" 2>/dev/null || true

# ---- Debug: append invocation details to rolling log -----------------------
# Caps at ~200 lines to prevent unbounded growth.
{
    printf '=== %s ===\n' "$(date '+%Y-%m-%dT%H:%M:%S')"
    printf 'pwd: %s\n' "$PWD"
    printf 'args: [%s]\n' "$*"
    printf 'stdin bytes: %d\n' "${#INPUT}"
    if [ -n "$INPUT" ]; then
        printf 'stdin (first 400 chars): %s\n' "${INPUT:0:400}"
    else
        printf 'stdin is EMPTY\n'
        printf 'env-claude-related:\n'
        env 2>/dev/null | grep -iE 'claude|cc_|anthropic' || true
    fi
    printf '\n'
} >> "$PURR_DIR/debug.log" 2>&1 || true
# Trim debug log if it grows past 400 lines
if [ -f "$PURR_DIR/debug.log" ]; then
    line_count=$(wc -l < "$PURR_DIR/debug.log" 2>/dev/null || echo 0)
    if [ "${line_count:-0}" -gt 400 ]; then
        tail -n 200 "$PURR_DIR/debug.log" > "$PURR_DIR/debug.log.tmp" 2>/dev/null && mv "$PURR_DIR/debug.log.tmp" "$PURR_DIR/debug.log"
    fi
fi

# ---- Single jq call for session payload (only subprocess on this path) -----
# Uses \x01 (SOH) as field separator rather than \t: bash's POSIX read treats
# consecutive whitespace IFS chars as a single delimiter, collapsing empty
# fields. Non-whitespace separator preserves empty-string fields correctly.
model=Claude; cwd_primary=''; cwd_alt=''
ctx_pct=0; rate_5h=0; rate_7d=0; cost_usd=0
lines_added=0; lines_removed=0; duration_ms=0

if [ -n "$INPUT" ]; then
    IFS=$'\x01' read -r model cwd_primary cwd_alt ctx_pct rate_5h rate_7d \
        cost_usd lines_added lines_removed duration_ms < <(
        jq -r '[
            (.model.display_name // "Claude"),
            (.workspace.current_dir // ""),
            (.cwd // ""),
            (.context_window.used_percentage // 0),
            (.rate_limits.five_hour.used_percentage // 0),
            (.rate_limits.seven_day.used_percentage // 0),
            (.cost.total_cost_usd // 0),
            (.cost.total_lines_added // 0),
            (.cost.total_lines_removed // 0),
            (.cost.total_duration_ms // 0)
        ] | map(tostring) | join("")' <<<"$INPUT" 2>/dev/null
    ) || true
fi

cwd="$cwd_primary"
[ -z "$cwd" ] && cwd="$cwd_alt"

# Normalize numeric fields (strip CR, tabs, spaces that can leak in)
ctx_pct="${ctx_pct//[^0-9.]/}";       [ -z "$ctx_pct" ] && ctx_pct=0
rate_5h="${rate_5h//[^0-9.]/}";       [ -z "$rate_5h" ] && rate_5h=0
rate_7d="${rate_7d//[^0-9.]/}";       [ -z "$rate_7d" ] && rate_7d=0
cost_usd="${cost_usd//[^0-9.]/}";     [ -z "$cost_usd" ] && cost_usd=0
lines_added="${lines_added//[^0-9]/}";     [ -z "$lines_added" ] && lines_added=0
lines_removed="${lines_removed//[^0-9]/}"; [ -z "$lines_removed" ] && lines_removed=0
duration_ms="${duration_ms//[^0-9]/}";     [ -z "$duration_ms" ] && duration_ms=0

# ---- State.json parsed with bash regex (no jq subprocess) ------------------
total_tc=0; coins=0; level=1; last_fed_at=0; kitten_name="Purrgotchi"
if [ -f "$PURR_STATE" ]; then
    state_content=$(<"$PURR_STATE")
    [[ "$state_content" =~ \"total_tool_calls\"[[:space:]]*:[[:space:]]*([0-9]+) ]] && total_tc="${BASH_REMATCH[1]}"
    [[ "$state_content" =~ \"coins\"[[:space:]]*:[[:space:]]*([0-9]+) ]]             && coins="${BASH_REMATCH[1]}"
    [[ "$state_content" =~ \"level\"[[:space:]]*:[[:space:]]*([0-9]+) ]]             && level="${BASH_REMATCH[1]}"
    [[ "$state_content" =~ \"last_fed_at\"[[:space:]]*:[[:space:]]*([0-9]+) ]]       && last_fed_at="${BASH_REMATCH[1]}"
    [[ "$state_content" =~ \"name\"[[:space:]]*:[[:space:]]*\"([^\"]+)\" ]]          && kitten_name="${BASH_REMATCH[1]}"
fi

# ---- Wall clock via bash builtin -------------------------------------------
printf -v now '%(%s)T' -1

if [ "${last_fed_at:-0}" -gt 0 ] 2>/dev/null; then
    last_fed_delta=$(( now - last_fed_at ))
else
    last_fed_delta=999999
fi

# ---- Mood + frame ----------------------------------------------------------
compute_mood mood "$ctx_pct" "$cost_usd" "$last_fed_delta"
frame=$(( (now / 2) % 2 ))

# ---- Git branch via .git/HEAD file read (no git subprocess) ----------------
git_branch=""
if [ -n "$cwd" ] && [ -f "$cwd/.git/HEAD" ]; then
    head_contents=$(<"$cwd/.git/HEAD")
    if [[ "$head_contents" == "ref: refs/heads/"* ]]; then
        git_branch="${head_contents#ref: refs/heads/}"
    fi
fi

# ---- Format helpers (all printf -v; zero subshells) ------------------------
printf -v ctx_int  "%.0f" "${ctx_pct:-0}"
printf -v rate_int "%.0f" "${rate_5h:-0}"
printf -v cost_fmt "%.2f" "${cost_usd:-0}"

printf -v rate_7d_int "%.0f" "${rate_7d:-0}"

progress_bar ctx_bar  "$ctx_int"  12
progress_bar r5h_bar  "$rate_int" 5
progress_bar r7d_bar  "$rate_7d_int" 5
human_duration duration "$duration_ms"
human_elapsed  fed_ago  "$last_fed_delta"

# Build DIR segment — omit entirely if cwd unknown (instead of showing "?")
dir_seg_plain=""
dir_seg_ansi=""
if [ -n "$cwd" ]; then
    short_cwd short_path "$cwd"
    dir_seg_plain=" │ DIR ${short_path}"
    dir_seg_ansi=" ${C_PINK}│${C_RESET} ${C_DIM}DIR${C_RESET} ${C_CYAN}${short_path}${C_RESET}"
fi

# Build GIT segment — omit if not a repo
git_seg_plain=""
git_seg_ansi=""
if [ -n "$git_branch" ]; then
    git_seg_plain=" │ GIT ${git_branch}"
    git_seg_ansi=" ${C_PINK}│${C_RESET} ${C_DIM}GIT${C_RESET} ${C_CYAN}${git_branch}${C_RESET}"
fi

# ---- Build HUD lines with closed box -----------------------------------------
# Box total width = 62 (2 corners + 60 content cells).
# For each content row we build (a) the plain text — for length math — and
# (b) the ANSI-decorated version. Pad right side with spaces + closing ║.
BOX_WIDTH=60

close_row() {
    # close_row out_var plain_content ansi_content
    local _out="$1" plain="$2" ansi="$3"
    local len=${#plain}
    local pad_n=$(( BOX_WIDTH - len ))
    [ "$pad_n" -lt 0 ] && pad_n=0
    printf -v "$_out" '%b%s%*s%b' "${C_PINK}║${C_RESET}" "$ansi" "$pad_n" '' "${C_PINK}║${C_RESET}"
}

# H1 (top rail): ╔═╡ NETRUNNER // <name> ╞══════╗
title_plain="═╡ NETRUNNER // ${kitten_name} ╞"
title_ansi="${C_PINK}═╡${C_RESET} ${C_BOLD}${C_CYAN}NETRUNNER${C_RESET} ${C_MAGENTA}//${C_RESET} ${C_PINK}${kitten_name}${C_RESET} ${C_PINK}╞${C_RESET}"
title_len=${#title_plain}
rail_count=$(( BOX_WIDTH - title_len ))
[ "$rail_count" -lt 0 ] && rail_count=0
rail=''
for ((i=0; i<rail_count; i++)); do rail+='═'; done
printf -v H1 '%b%s%b%s%b' "${C_PINK}╔${C_RESET}" "$title_ansi" "${C_PINK}" "$rail" "╗${C_RESET}"

# H2: MODEL <name> │ DIR <path> │ GIT <branch>
p2=" MODEL ${model}${dir_seg_plain}${git_seg_plain}"
a2=" ${C_DIM}MODEL${C_RESET} ${C_CYAN}${model}${C_RESET}${dir_seg_ansi}${git_seg_ansi}"
close_row H2 "$p2" "$a2"

# H3: CTX <bar> pct% │ 5h <bar> pct% │ 7d <bar> pct%
p3=" CTX ${ctx_bar} ${ctx_int}% │ 5h ${r5h_bar} ${rate_int}% │ 7d ${r7d_bar} ${rate_7d_int}%"
a3=" ${C_DIM}CTX${C_RESET} ${C_CYAN}${ctx_bar}${C_RESET} ${C_BOLD}${ctx_int}%${C_RESET} ${C_PINK}│${C_RESET} ${C_DIM}5h${C_RESET} ${C_YELLOW}${r5h_bar}${C_RESET} ${C_BOLD}${rate_int}%${C_RESET} ${C_PINK}│${C_RESET} ${C_DIM}7d${C_RESET} ${C_YELLOW}${r7d_bar}${C_RESET} ${C_BOLD}${rate_7d_int}%${C_RESET}"
close_row H3 "$p3" "$a3"

# H4: COST $x │ LINES +a/-b │ TIME duration
# No decorative glyphs (⧖, ◆ caused width-ambiguity bugs that broke alignment).
p4=" COST \$${cost_fmt} │ LINES +${lines_added}/-${lines_removed} │ TIME ${duration}"
a4=" ${C_DIM}COST${C_RESET} ${C_YELLOW}\$${cost_fmt}${C_RESET} ${C_PINK}│${C_RESET} ${C_DIM}LINES${C_RESET} ${C_GREEN}+${lines_added}${C_RESET}/${C_RED}-${lines_removed}${C_RESET} ${C_PINK}│${C_RESET} ${C_DIM}TIME${C_RESET} ${duration}"
close_row H4 "$p4" "$a4"

# H5: LV <n> │ COINS <c> │ FED <ago> │ MOOD <mood>
p5=" LV ${level} │ COINS ${coins} │ FED ${fed_ago} │ MOOD ${mood}"
a5=" ${C_DIM}LV${C_RESET} ${C_PINK}${C_BOLD}${level}${C_RESET} ${C_PINK}│${C_RESET} ${C_DIM}COINS${C_RESET} ${C_YELLOW}${coins}${C_RESET} ${C_PINK}│${C_RESET} ${C_DIM}FED${C_RESET} ${fed_ago} ${C_PINK}│${C_RESET} ${C_DIM}MOOD${C_RESET} ${C_MAGENTA}${mood}${C_RESET}"
close_row H5 "$p5" "$a5"

# H6 (bottom rail): ╚═══════╝
bottom_rail=''
for ((i=0; i<BOX_WIDTH; i++)); do bottom_rail+='═'; done
printf -v H6 '%b%s%b' "${C_PINK}╚${C_RESET}" "${C_PINK}${bottom_rail}" "╝${C_RESET}"

# ---- Sprite (single file read via mapfile) ---------------------------------
mood_file="$PURR_DIR/sprites/${mood}_${frame}.txt"
[ ! -f "$mood_file" ] && mood_file="$PURR_DIR/sprites/${mood}_0.txt"
[ ! -f "$mood_file" ] && mood_file="$PURR_DIR/sprites/calm_0.txt"
mapfile -t sprite_lines < "$mood_file"
while [ "${#sprite_lines[@]}" -lt 6 ]; do sprite_lines+=(""); done

SW=16
pad() {
    local _out="$1" s="$2" len pad_n
    len=${#s}
    pad_n=$(( SW - len ))
    [ "$pad_n" -lt 0 ] && pad_n=0
    printf -v "$_out" '%s%*s' "$s" "$pad_n" ''
}

case "$mood" in
    panic)  K_COLOR="$C_RED" ;;
    alert)  K_COLOR="$C_YELLOW" ;;
    happy)  K_COLOR="$C_GREEN" ;;
    sleep)  K_COLOR="$C_MAGENTA" ;;
    hungry) K_COLOR="${C_DIM}${C_CYAN}" ;;
    *)      K_COLOR="$C_CYAN" ;;
esac

for i in 0 1 2 3 4 5; do
    case $i in
        0) HL="$H1" ;; 1) HL="$H2" ;; 2) HL="$H3" ;;
        3) HL="$H4" ;; 4) HL="$H5" ;; 5) HL="$H6" ;;
    esac
    pad padded "${sprite_lines[$i]:-}"
    printf '%b%s%b   %b\n' "$K_COLOR" "$padded" "$C_RESET" "$HL"
done
