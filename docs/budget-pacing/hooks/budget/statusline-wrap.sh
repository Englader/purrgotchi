#!/bin/bash
# Wraps the purrgotchi statusline; caches budget data on the side.
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CMD="${BUDGET_WRAPPED_CMD:-\"/home/englader/Desktop/Work/Personal/purrgotchi/rust/target/release/purrgotchi\" statusline}"
input="$(cat)"
( printf '%s' "$input" | python3 "$DIR/budget.py" cache >/dev/null 2>&1 & ) >/dev/null 2>&1
printf '%s' "$input" | sh -c "$CMD"
exit $?
