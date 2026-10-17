#!/usr/bin/env bash
# 1 fps statusline player for the pixel cat, against a throwaway state dir.
# usage: rust/tools/px_anim.sh [--mood calm|happy|alert|hungry|panic|sleep] [--secs N] [--fast] [--pose P] [--scene S]
#   --pose  PURR_FORCE_POSE (sit|loaf|sleep|groom|stretch|look|wander|zoomies)
#   --scene PURR_FORCE_SCENE (scenes stay off otherwise, so idle life shows)
#   --fast  no sleep between frames.  BIN=… overrides the binary (default: target/debug).
set -euo pipefail
BIN="${BIN:-$(dirname "$0")/../target/debug/purrgotchi}"
MOOD=calm SECS=60 FAST=
while [ $# -gt 0 ]; do
  case "$1" in
    --mood) MOOD="$2"; shift 2 ;;
    --secs) SECS="$2"; shift 2 ;;
    --fast) FAST=1; shift ;;
    --pose) export PURR_FORCE_POSE="$2"; shift 2 ;;
    --scene) export PURR_FORCE_SCENE="$2"; shift 2 ;;
    *) sed -n '2,6p' "$0"; exit 2 ;;
  esac
done
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
export HOME="$T" PURR_STATE_DIR="$T/state" PURR_PIXELS="${PURR_PIXELS:-1}" COLUMNS="${COLUMNS:-170}"
[ -n "${PURR_FORCE_SCENE:-}" ] || export PURR_PLAYGROUND=0
NOW0=1800000000 CTX=20
case "$MOOD" in
  calm) PATCH=".last_fed_at = $((NOW0 - 120))" ;;
  happy) PATCH=".last_fed_at = $((NOW0 - 120)) | .last_pet_at = $((NOW0 - 1))" ;;
  hungry) PATCH=".last_fed_at = $((NOW0 - 4000))" ;;
  sleep) PATCH=".last_fed_at = $((NOW0 - 30000))" ;;
  alert) PATCH=".last_fed_at = $((NOW0 - 120))"; CTX=55 ;;
  panic) PATCH=".last_fed_at = $((NOW0 - 120))"; CTX=92 ;;
  *) echo "unknown mood: $MOOD" >&2; exit 2 ;;
esac
PAYLOAD="{\"workspace\":{\"current_dir\":\"$T\"},\"context_window\":{\"used_percentage\":$CTX}}"
PURR_NOW=$NOW0 "$BIN" statusline <<<"$PAYLOAD" >/dev/null   # hatch
jq "$PATCH" "$PURR_STATE_DIR/state.json" >"$T/s.json" && mv "$T/s.json" "$PURR_STATE_DIR/state.json"
for ((i = 0; i < SECS; i++)); do
  printf '\033[H\033[2J'
  PURR_NOW=$((NOW0 + i)) "$BIN" statusline <<<"$PAYLOAD"
  [ -n "$FAST" ] || sleep 1
done
