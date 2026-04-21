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
# shellcheck disable=SC1091
source "$PURR_DIR/lib/speech.sh"

# ---- Stdin (using cat for reliability across spawn contexts) ---------------
# Note: $(</dev/stdin) was faster but proved unreliable when invoked by
# Claude Code on Windows. $(cat) adds one subprocess but always works.
INPUT=$(cat)

# ---- Optional debug capture (gated — off by default for latency) -----------
[ -n "${PURR_DEBUG:-}" ] && printf '%s\n' "$INPUT" > "$PURR_STATE_DIR/last-input.json" 2>/dev/null || true

# ---- Single jq call for session payload (only subprocess on this path) -----
# Uses \x01 (SOH) as field separator rather than \t: bash's POSIX read treats
# consecutive whitespace IFS chars as a single delimiter, collapsing empty
# fields. Non-whitespace separator preserves empty-string fields correctly.
model=Claude; cwd_primary=''; cwd_alt=''
ctx_pct=0; rate_5h=0; rate_7d=0; cost_usd=0
lines_added=0; lines_removed=0; duration_ms=0
reset_5h=0; reset_7d=0

if [ -n "$INPUT" ]; then
    IFS=$'\x01' read -r model cwd_primary cwd_alt ctx_pct rate_5h rate_7d \
        cost_usd lines_added lines_removed duration_ms reset_5h reset_7d < <(
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
            (.cost.total_duration_ms // 0),
            (.rate_limits.five_hour.resets_at // 0),
            (.rate_limits.seven_day.resets_at // 0)
        ] | map(tostring) | join("")' <<<"$INPUT" 2>/dev/null
    ) || true
fi

cwd="$cwd_primary"
[ -z "$cwd" ] && cwd="$cwd_alt"

# Normalize Windows paths (C:\... -> /c/...) so $HOME match in short_cwd works
# on Git Bash (where $HOME is in forward-slash form).
if [[ "$cwd" =~ ^([A-Za-z]):\\(.*)$ ]]; then
    _drive="${BASH_REMATCH[1],,}"
    _rest="${BASH_REMATCH[2]//\\//}"
    cwd="/${_drive}/${_rest}"
fi

# Normalize numeric fields (strip CR, tabs, spaces that can leak in)
ctx_pct="${ctx_pct//[^0-9.]/}";       [ -z "$ctx_pct" ] && ctx_pct=0
rate_5h="${rate_5h//[^0-9.]/}";       [ -z "$rate_5h" ] && rate_5h=0
rate_7d="${rate_7d//[^0-9.]/}";       [ -z "$rate_7d" ] && rate_7d=0
cost_usd="${cost_usd//[^0-9.]/}";     [ -z "$cost_usd" ] && cost_usd=0
lines_added="${lines_added//[^0-9]/}";     [ -z "$lines_added" ] && lines_added=0
lines_removed="${lines_removed//[^0-9]/}"; [ -z "$lines_removed" ] && lines_removed=0
duration_ms="${duration_ms//[^0-9]/}";     [ -z "$duration_ms" ] && duration_ms=0
reset_5h="${reset_5h//[^0-9]/}";           [ -z "$reset_5h" ] && reset_5h=0
reset_7d="${reset_7d//[^0-9]/}";           [ -z "$reset_7d" ] && reset_7d=0

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

# ---- Reset-time strings + speech (mood-contextual) -------------------------
time_until_reset reset_5h_str "$reset_5h" "$now"
time_until_reset reset_7d_str "$reset_7d" "$now"
compute_speech speech "$mood" "$now"

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

# ---- Per-turn cost: delta since the user's last prompt submit --------------
# The UserPromptSubmit hook (turn_hook.sh) deletes this file whenever the
# user sends a message. On the FIRST statusline render of a new turn the file
# is missing → we snapshot the current cost as the turn anchor. Subsequent
# renders during the same turn read the same anchor; Cost = current - anchor.
# Separate from state.json so the render path doesn't violate the "only
# feed_hook.py writes state" invariant. Pure-bash integer math on 4-decimal
# cents — no awk/bc subshell.
TURN_ANCHOR_FILE="$PURR_STATE_DIR/turn-start-cost.txt"
# Grace window (microseconds) — anchor keeps refreshing during this period after
# the hook, so Claude Code's async finalization of the prior turn gets absorbed.
# 500ms: plenty for finalization (~100ms), tight enough to miss real tool work (≥1s).
_TURN_GRACE_US=500000
# Anchor file is 2 lines: cost, then creation timestamp ($EPOCHREALTIME).
_now_real="${EPOCHREALTIME:-0}"
_now_us="${_now_real//./}"
if [ -f "$TURN_ANCHOR_FILE" ]; then
    { IFS= read -r _anchor_line1; IFS= read -r _anchor_line2; } < "$TURN_ANCHOR_FILE" 2>/dev/null || true
    turn_anchor="${_anchor_line1//[^0-9.]/}"; [ -z "$turn_anchor" ] && turn_anchor=0
    _anchor_ts="${_anchor_line2//[^0-9.]/}"; [ -z "$_anchor_ts" ] && _anchor_ts=0
    _anchor_us="${_anchor_ts//./}"
    _age_us=$(( 10#${_now_us:-0} - 10#${_anchor_us:-0} ))
    if [ "$_age_us" -ge 0 ] && [ "$_age_us" -le "$_TURN_GRACE_US" ]; then
        # In grace window — refresh anchor, keep original creation timestamp.
        turn_anchor="$cost_usd"
        printf '%s\n%s\n' "$cost_usd" "$_anchor_ts" > "$TURN_ANCHOR_FILE" 2>/dev/null || true
    fi
    # Past grace: anchor frozen, no write.
else
    # Hook cleared the file — this render begins the new turn's grace window.
    turn_anchor="$cost_usd"
    printf '%s\n%s\n' "$cost_usd" "$_now_real" > "$TURN_ANCHOR_FILE" 2>/dev/null || true
fi
printf -v _c_int "%.4f" "${cost_usd:-0}"; _c_int="${_c_int//./}"
printf -v _a_int "%.4f" "${turn_anchor:-0}"; _a_int="${_a_int//./}"
_turn_delta=$(( 10#${_c_int:-0} - 10#${_a_int:-0} ))
[ "$_turn_delta" -lt 0 ] && _turn_delta=0
_turn_whole=$(( _turn_delta / 10000 ))
_turn_cents=$(( ( (_turn_delta % 10000) + 50) / 100 ))
if [ "$_turn_cents" -ge 100 ]; then _turn_whole=$(( _turn_whole + 1 )); _turn_cents=0; fi
printf -v turn_cost_fmt '%d.%02d' "$_turn_whole" "$_turn_cents"

progress_bar ctx_bar  "$ctx_int"  12
progress_bar r5h_bar  "$rate_int" 5
progress_bar r7d_bar  "$rate_7d_int" 5
human_duration duration "$duration_ms"
human_elapsed  fed_ago  "$last_fed_delta"

# Pre-build DIR / GIT values (empty string if absent — pad_col handles blank cols).
short_path=""
[ -n "$cwd" ] && short_cwd short_path "$cwd"

# ---- Build HUD lines with closed box -----------------------------------------
# Three-column grid: ║ <W1> │ <W2> │ <W3> ║  — every row uses this grid so the
# two │ separators land at identical columns across H2/H3/H4/H5/H6.
# Widths accommodate: MODEL (27), DIR (29), TIME (17) / CTX-bar (21) /
# FED+│+MOOD combo (26).
# 1 leading + W1 + " │ " + W2 + " │ " + W3 = 89 → BOX_WIDTH 90 leaves 1 trailing.
W1=28
W2=30
W3=28
BOX_WIDTH=94

close_row() {
    # close_row out_var plain_content ansi_content
    local _out="$1" plain="$2" ansi="$3"
    local len=${#plain}
    local pad_n=$(( BOX_WIDTH - len ))
    [ "$pad_n" -lt 0 ] && pad_n=0
    printf -v "$_out" '%b%s%*s%b' "${C_PINK}║${C_RESET}" "$ansi" "$pad_n" '' "${C_PINK}║${C_RESET}"
}

# pad_col out_plain_var out_ansi_var plain ansi width — appends trailing spaces
# so the visible width (measured from plain) hits `width`. ANSI gets same pad.
pad_col() {
    local _p="$1" _a="$2" plain="$3" ansi="$4" width="$5"
    local len=${#plain}
    local pad_n=$(( width - len ))
    [ "$pad_n" -lt 0 ] && pad_n=0
    printf -v "$_p" '%s%*s' "$plain" "$pad_n" ''
    printf -v "$_a" '%s%*s' "$ansi" "$pad_n" ''
}

# center_col — same signature as pad_col but splits padding to center content.
center_col() {
    local _p="$1" _a="$2" plain="$3" ansi="$4" width="$5"
    local len=${#plain}
    local pad_total=$(( width - len ))
    [ "$pad_total" -lt 0 ] && pad_total=0
    local pad_l=$(( pad_total / 2 ))
    local pad_r=$(( pad_total - pad_l ))
    printf -v "$_p" '%*s%s%*s' "$pad_l" '' "$plain" "$pad_r" ''
    printf -v "$_a" '%*s%s%*s' "$pad_l" '' "$ansi" "$pad_r" ''
}

# split_col — two values anchored to column edges with a │ dead-center.
# Layout: <left><pad>│<pad><right>, fills `width` chars total.
split_col() {
    local _p="$1" _a="$2" lp="$3" la="$4" rp="$5" ra="$6" width="$7"
    local ll=${#lp} rl=${#rp}
    local half_l=$(( (width - 1) / 2 ))
    local half_r=$(( width - 1 - half_l ))
    local l_pad=$(( half_l - ll ))
    local r_pad=$(( half_r - rl ))
    [ "$l_pad" -lt 0 ] && l_pad=0
    [ "$r_pad" -lt 0 ] && r_pad=0
    printf -v "$_p" '%s%*s│%*s%s' "$lp" "$l_pad" '' "$r_pad" '' "$rp"
    printf -v "$_a" '%s%*s%b│%b%*s%s' "$la" "$l_pad" '' "$C_PINK" "$C_RESET" "$r_pad" '' "$ra"
}

# anchor_col — left content hugs left edge, right content hugs the middle │
# (separated by " │ "). Right side gets trailing pad instead of leading pad.
# Layout: <left><pad> │ <right><pad>, fills `width` chars total.
anchor_col() {
    local _p="$1" _a="$2" lp="$3" la="$4" rp="$5" ra="$6" width="$7"
    local ll=${#lp} rl=${#rp}
    local half_l=$(( (width - 3) / 2 ))   # left portion (content + trailing pad)
    local half_r=$(( width - 3 - half_l )) # right portion (content + trailing pad)
    local l_pad=$(( half_l - ll ))
    local r_pad=$(( half_r - rl ))
    [ "$l_pad" -lt 0 ] && l_pad=0
    [ "$r_pad" -lt 0 ] && r_pad=0
    printf -v "$_p" '%s%*s │ %s%*s' "$lp" "$l_pad" '' "$rp" "$r_pad" ''
    printf -v "$_a" '%s%*s %b│%b %s%*s' "$la" "$l_pad" '' "$C_PINK" "$C_RESET" "$ra" "$r_pad" ''
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

SEP_P=" │ "
SEP_A=" ${C_PINK}│${C_RESET} "

# H2: MODEL │ DIR │ GIT — columns padded to fixed widths so │ aligns with other rows.
c1p="MODEL: ${model}"
c1a="${C_DIM}MODEL:${C_RESET} ${C_CYAN}${model}${C_RESET}"
pad_col c1p c1a "$c1p" "$c1a" "$W1"
if [ -n "$cwd" ]; then
    c2p="DIR: ${short_path}"
    c2a="${C_DIM}DIR:${C_RESET} ${C_CYAN}${short_path}${C_RESET}"
else
    c2p=""; c2a=""
fi
pad_col c2p c2a "$c2p" "$c2a" "$W2"
if [ -n "$git_branch" ]; then
    c3p="GIT: ${git_branch}"
    c3a="${C_DIM}GIT:${C_RESET} ${C_CYAN}${git_branch}${C_RESET}"
else
    c3p=""; c3a=""
fi
pad_col c3p c3a "$c3p" "$c3a" "$W3"
p2=" ${c1p}${SEP_P}${c2p}${SEP_P}${c3p}"
a2=" ${c1a}${SEP_A}${c2a}${SEP_A}${c3a}"
close_row H2 "$p2" "$a2"

# H3: USAGE: 5h │ 7d │ CTX — "USAGE:" label parallels "RESET:" in H4, and
# also aligns 5h directly above H4's "RESET: 5h".
c1p="USAGE: 5h: ${r5h_bar} ${rate_int}%"
c1a="${C_DIM}USAGE:${C_RESET} ${C_DIM}5h:${C_RESET} ${C_YELLOW}${r5h_bar}${C_RESET} ${C_BOLD}${rate_int}%${C_RESET}"
pad_col c1p c1a "$c1p" "$c1a" "$W1"
c2p="7d: ${r7d_bar} ${rate_7d_int}%"
c2a="${C_DIM}7d:${C_RESET} ${C_YELLOW}${r7d_bar}${C_RESET} ${C_BOLD}${rate_7d_int}%${C_RESET}"
pad_col c2p c2a "$c2p" "$c2a" "$W2"
c3p="CTX: ${ctx_bar} ${ctx_int}%"
c3a="${C_DIM}CTX:${C_RESET} ${C_CYAN}${ctx_bar}${C_RESET} ${C_BOLD}${ctx_int}%${C_RESET}"
pad_col c3p c3a "$c3p" "$c3a" "$W3"
p3=" ${c1p}${SEP_P}${c2p}${SEP_P}${c3p}"
a3=" ${c1a}${SEP_A}${c2a}${SEP_A}${c3a}"
close_row H3 "$p3" "$a3"

# H4: RESET 5h │ 7d │ Cost (per-turn). Col 3 now tracks the delta since the
# last render so you can see the cost of whatever just happened.
c1p="RESET: 5h ${reset_5h_str}"
c1a="${C_DIM}RESET:${C_RESET} ${C_DIM}5h${C_RESET} ${C_YELLOW}${reset_5h_str}${C_RESET}"
pad_col c1p c1a "$c1p" "$c1a" "$W1"
c2p="7d ${reset_7d_str}"
c2a="${C_DIM}7d${C_RESET} ${C_YELLOW}${reset_7d_str}${C_RESET}"
pad_col c2p c2a "$c2p" "$c2a" "$W2"
c3p="Cost: \$${turn_cost_fmt}"
c3a="${C_DIM}Cost:${C_RESET} ${C_YELLOW}\$${turn_cost_fmt}${C_RESET}"
pad_col c3p c3a "$c3p" "$c3a" "$W3"
p4=" ${c1p}${SEP_P}${c2p}${SEP_P}${c3p}"
a4=" ${c1a}${SEP_A}${c2a}${SEP_A}${c3a}"
close_row H4 "$p4" "$a4"

# H5: LINES │ TIME │ Total  (session-total cost sits in col 3 under per-turn Cost).
# No decorative glyphs (⧖, ◆ caused width-ambiguity bugs that broke alignment).
c1p="LINES: +${lines_added}/-${lines_removed}"
c1a="${C_DIM}LINES:${C_RESET} ${C_GREEN}+${lines_added}${C_RESET}/${C_RED}-${lines_removed}${C_RESET}"
pad_col c1p c1a "$c1p" "$c1a" "$W1"
c2p="TIME: ${duration}"
c2a="${C_DIM}TIME:${C_RESET} ${duration}"
pad_col c2p c2a "$c2p" "$c2a" "$W2"
c3p="Total: \$${cost_fmt}"
c3a="${C_DIM}Total:${C_RESET} ${C_YELLOW}\$${cost_fmt}${C_RESET}"
pad_col c3p c3a "$c3p" "$c3a" "$W3"
p5=" ${c1p}${SEP_P}${c2p}${SEP_P}${c3p}"
a5=" ${c1a}${SEP_A}${c2a}${SEP_A}${c3a}"
close_row H5 "$p5" "$a5"

# H6: LV on left edge, │ centered, COINS on right edge (col1);
#     MOOD on left, │ centered, FED on right (col2);
#     col3 empty. Two main │ still align with rows above.
l1p="LV: ${level}"
l1a="${C_DIM}LV:${C_RESET} ${C_PINK}${C_BOLD}${level}${C_RESET}"
r1p="COINS: ${coins}"
r1a="${C_DIM}COINS:${C_RESET} ${C_YELLOW}${coins}${C_RESET}"
anchor_col c1p c1a "$l1p" "$l1a" "$r1p" "$r1a" "$W1"

l2p="MOOD: ${mood}"
l2a="${C_DIM}MOOD:${C_RESET} ${C_MAGENTA}${mood}${C_RESET}"
r2p="FED: ${fed_ago}"
r2a="${C_DIM}FED:${C_RESET} ${fed_ago}"
anchor_col c2p c2a "$l2p" "$l2a" "$r2p" "$r2a" "$W2"

c3p=""; c3a=""
pad_col c3p c3a "$c3p" "$c3a" "$W3"
p6=" ${c1p}${SEP_P}${c2p}${SEP_P}${c3p}"
a6=" ${c1a}${SEP_A}${c2a}${SEP_A}${c3a}"
close_row H6 "$p6" "$a6"

# H7 (bottom rail): ╚═══════╝
bottom_rail=''
for ((i=0; i<BOX_WIDTH; i++)); do bottom_rail+='═'; done
printf -v H7 '%b%s%b' "${C_PINK}╚${C_RESET}" "${C_PINK}${bottom_rail}" "╝${C_RESET}"

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

# Render: HUD on the LEFT, sprite on the RIGHT (v2.0 flip).
# Rows 0..5 = HUD top rail + 5 content rows, each with a cat sprite line to the
# right (6 sprite lines total). Row 6 = bottom rail + speech bubble — speech
# sits next to the cat's feet so it reads as the cat talking.
for i in 0 1 2 3 4 5 6; do
    case $i in
        0) HL="$H1" ;; 1) HL="$H2" ;; 2) HL="$H3" ;;
        3) HL="$H4" ;; 4) HL="$H5" ;; 5) HL="$H6" ;;
        6) HL="$H7" ;;
    esac
    if [ "$i" -eq 6 ]; then
        # Speech bubble — print raw (length varies, no pad); sits in sprite col.
        printf '%b   %b"%s"%b\n' "$HL" "$K_COLOR" "$speech" "$C_RESET"
    else
        pad padded "${sprite_lines[$i]:-}"
        printf '%b   %b%s%b\n' "$HL" "$K_COLOR" "$padded" "$C_RESET"
    fi
done
