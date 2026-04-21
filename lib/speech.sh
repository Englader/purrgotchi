#!/usr/bin/env bash
# Speech bubble phrases for Purrgotchi — mood-contextual, rotates every ~30s.
# Pure bash associative arrays; no subshells. Phrases kept ~16–18 cols so the
# speech row fits alongside the sprite column without breaking the 81-char budget.

declare -A _SPEECH_COUNTS=(
    [calm]=6
    [happy]=5
    [alert]=5
    [panic]=5
    [hungry]=4
    [sleep]=4
)

declare -A _SPEECH=(
    [calm_0]="just vibing"
    [calm_1]="mrow."
    [calm_2]="watching you work"
    [calm_3]="purring softly"
    [calm_4]="keeping an eye out"
    [calm_5]="code code code..."

    [happy_0]="you got this!"
    [happy_1]="yesss tool call!"
    [happy_2]="nice one!"
    [happy_3]="mrow! productive"
    [happy_4]="work work work <3"

    [alert_0]="getting busy..."
    [alert_1]="context filling"
    [alert_2]="watching the bars"
    [alert_3]="keep it tight"
    [alert_4]="gentle: context"

    [panic_0]="LOSING CONTEXT!!"
    [panic_1]="CONDENSE!!"
    [panic_2]="aaa too much!!"
    [panic_3]="/clear plz..."
    [panic_4]="walls closing in"

    [hungry_0]="feed me tools"
    [hungry_1]="so empty..."
    [hungry_2]="starving for work"
    [hungry_3]="where are tools?"

    [sleep_0]="zzz 5 more min..."
    [sleep_1]="*snoring*"
    [sleep_2]="don't wake me"
    [sleep_3]="dreaming of fish"
)

# compute_speech out_var mood now_epoch
compute_speech() {
    local _out="$1" mood="$2" now="$3"
    local n="${_SPEECH_COUNTS[$mood]:-1}"
    local idx=$(( (now / 30) % n ))
    printf -v "$_out" '%s' "${_SPEECH[${mood}_${idx}]:-mrow}"
}
