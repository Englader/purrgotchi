#!/usr/bin/env python3
"""Purrgotchi feed hook — runs on Claude Code PostToolUse.

Bumps state.json atomically so parallel sessions don't corrupt the file.
Fails silently; a broken hook must never block Claude Code.
"""
import json
import os
import sys
import tempfile
import time
from pathlib import Path

STATE_DIR = Path(os.path.expanduser("~/.claude/purrgotchi"))
STATE_FILE = STATE_DIR / "state.json"


def fresh_state():
    now = int(time.time())
    return {
        "version": 1,
        "name": "Purrgotchi",
        "birth_time": now,
        "last_fed_at": now,
        "total_tool_calls": 0,
        "coins": 0,
        "level": 1,
        "lifetime_cost_usd": 0.0,
    }


def load_state():
    if not STATE_FILE.exists():
        return fresh_state()
    try:
        with open(STATE_FILE, "r", encoding="utf-8") as f:
            state = json.load(f)
        for k, v in fresh_state().items():
            state.setdefault(k, v)
        return state
    except (json.JSONDecodeError, OSError):
        return fresh_state()


def save_atomic(state):
    STATE_DIR.mkdir(parents=True, exist_ok=True)
    fd, tmp_path = tempfile.mkstemp(
        dir=str(STATE_DIR), prefix=".state_", suffix=".json.tmp"
    )
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            json.dump(state, f, indent=2)
        os.replace(tmp_path, STATE_FILE)
    except Exception:
        try:
            os.unlink(tmp_path)
        except OSError:
            pass
        raise


def main():
    try:
        sys.stdin.read()
    except Exception:
        pass

    state = load_state()
    state["total_tool_calls"] += 1
    state["coins"] += 1
    state["last_fed_at"] = int(time.time())
    state["level"] = 1 + state["total_tool_calls"] // 50
    save_atomic(state)


if __name__ == "__main__":
    try:
        main()
    except Exception:
        sys.exit(0)
