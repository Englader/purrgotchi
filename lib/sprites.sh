#!/usr/bin/env bash
# get_sprite(mood, frame) -> cats the sprite file (6 lines)
# Falls back: mood_frame -> mood_0 -> calm_0

PURR_SPRITES="${PURR_SPRITES:-$HOME/.claude/purrgotchi/sprites}"

get_sprite() {
    local mood="${1:-calm}"
    local frame="${2:-0}"
    local file="$PURR_SPRITES/${mood}_${frame}.txt"

    if [ ! -f "$file" ]; then
        file="$PURR_SPRITES/${mood}_0.txt"
    fi
    if [ ! -f "$file" ]; then
        file="$PURR_SPRITES/calm_0.txt"
    fi

    if [ -f "$file" ]; then
        cat "$file"
    fi
}
