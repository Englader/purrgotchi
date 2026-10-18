#!/usr/bin/env python3
"""Budget awareness for Claude Code: plan usage + context size via hooks."""
import bisect
import glob
import json
import os
import re
import subprocess
import sys
import threading
import time
from datetime import datetime, timezone

HOME = os.environ.get("BUDGET_HOME") or os.path.expanduser("~/.claude")
BDIR = os.path.join(HOME, "hooks", "budget")
CACHE_DIR = os.path.join(BDIR, "cache")
STATE_DIR = os.path.join(BDIR, "state")
HANDOVER_DIR = os.path.join(HOME, "handovers")

FRESH = 300
ENDPOINT_TTL = 120
HOOK_BUDGET = 3.0   # wall-clock seconds a hook may spend on slow work (hook timeout is 5s)
FETCH_CAP = 2.0     # max seconds for the endpoint fetch
GIT_CAP = 1.5
_deadline = None    # monotonic time; set by hooks only (watch loops run unbounded)
_stuck = False      # a fetch thread outlived its deadline
CTX_CAP = 400_000
CTX_HANDOVER = 300_000
MAX_CTX_CHARS = 9500
FIVE_H = 5 * 3600
H5_BANDS = [50, 70, 80, 90, 95]
CTX_BANDS = [200_000, 300_000, 350_000, 400_000]
D7_BANDS = [70, 85, 95]
EMIT_INTERVAL = 600


def now():
    return time.time()


def read_json(path):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except Exception:
        return None


def write_json_atomic(path, obj):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = "%s.tmp%d" % (path, os.getpid())
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(obj, f)
    # Windows refuses to replace a file another process has open (a hook
    # reading the cache); retry briefly, and never leave the tmp behind.
    try:
        for i in range(20):
            try:
                os.replace(tmp, path)
                return
            except PermissionError:
                if i == 19:
                    raise
                time.sleep(0.025)
    finally:
        if os.path.exists(tmp):
            os.remove(tmp)


def safe_sid(sid):
    return "".join(c for c in str(sid) if c.isalnum() or c in "-_") or "unknown"


def slug_of(cwd):
    # Claude Code's own projects/ dir rule: every non-alphanumeric becomes "-".
    return re.sub(r"[^A-Za-z0-9]", "-", cwd)


def num(x):
    return x if isinstance(x, (int, float)) and not isinstance(x, bool) else None


# ---------- cache ----------
def cmd_cache(raw):
    d = json.loads(raw)
    sid = d.get("session_id")
    if not sid:
        return
    write_json_atomic(os.path.join(CACHE_DIR, safe_sid(sid) + ".json"), {
        "ts": now(),
        "rate_limits": d.get("rate_limits"),
        "context_window": d.get("context_window"),
    })


# ---------- context ----------
def usage_sum(u):
    if not isinstance(u, dict):
        return None
    tot = 0
    seen = False
    for k in ("input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens"):
        v = num(u.get(k))
        if v is not None:
            tot += v
            seen = True
    return int(tot) if seen else None


def ctx_from_transcript(path):
    try:
        if not path or not os.path.isfile(path):
            return None
        size = os.path.getsize(path)
        with open(path, "rb") as f:
            f.seek(max(0, size - 200_000))
            data = f.read()
        lines = data.split(b"\n")
        if size > 200_000:
            lines = lines[1:]  # first line likely partial
        for line in reversed(lines):
            if not line.strip():
                continue
            try:
                o = json.loads(line)
            except Exception:
                continue
            m = o.get("message") if isinstance(o, dict) else None
            if not isinstance(m, dict) or m.get("role") != "assistant":
                continue
            s = usage_sum(m.get("usage"))
            if s is not None:
                return s
    except Exception:
        pass
    return None


# ---------- plan ----------
def parse_iso(s):
    try:
        dt = datetime.fromisoformat(str(s).replace("Z", "+00:00"))
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=timezone.utc)
        return dt.timestamp()
    except Exception:
        return None


def _fetch_remote():
    try:
        import urllib.request  # lazy: only the fallback path pays for it
        cred = read_json(os.path.join(HOME, ".credentials.json")) or {}
        tok = cred["claudeAiOauth"]["accessToken"]
        req = urllib.request.Request(
            "https://api.anthropic.com/api/oauth/usage",
            headers={"Authorization": "Bearer " + tok,
                     "anthropic-beta": "oauth-2025-04-20"})
        with urllib.request.urlopen(req, timeout=FETCH_CAP) as r:
            j = json.loads(r.read().decode())
        data = {}
        for k in ("five_hour", "seven_day"):
            w = j.get(k)
            if isinstance(w, dict) and num(w.get("utilization")) is not None:
                data[k] = {"used": float(w["utilization"]),
                           "resets": parse_iso(w.get("resets_at"))}
        return data if "five_hour" in data else None
    except Exception:
        return None


def fetch_endpoint():
    """Returns {'five_hour': {used, resets}, 'seven_day': {...}} or None."""
    cp = os.path.join(CACHE_DIR, "_endpoint.json")
    c = read_json(cp)
    if c and now() - c.get("ts", 0) < ENDPOINT_TTL:
        return c.get("data")
    if os.environ.get("BUDGET_NO_NET"):
        return None
    fp = os.path.join(CACHE_DIR, "_endpoint_fail.json")
    f = read_json(fp)
    if f and 0 <= now() - f.get("ts", 0) < ENDPOINT_TTL:
        return None  # recent failure: don't hammer the endpoint
    left = FETCH_CAP if _deadline is None else min(FETCH_CAP, _deadline - time.monotonic())
    if left <= 0.1:
        return None  # out of time budget; not the endpoint's fault, so no failure record
    box = []
    th = threading.Thread(target=lambda: box.append(_fetch_remote()), daemon=True)
    th.start()
    th.join(left)  # urlopen's timeout is per socket op; DNS/proxy lookup is unbounded
    data = box[0] if box else None
    if th.is_alive():
        global _stuck
        _stuck = True
    try:
        if data:
            write_json_atomic(cp, {"ts": now(), "data": data})
        else:
            write_json_atomic(fp, {"ts": now()})
    except Exception:
        pass
    return data


def plan_from_cache(rl):
    if not isinstance(rl, dict):
        return None
    out = {}
    for k in ("five_hour", "seven_day"):
        w = rl.get(k)
        if isinstance(w, dict) and num(w.get("used_percentage")) is not None:
            out[k] = {"used": float(w["used_percentage"]), "resets": num(w.get("resets_at"))}
    return out if "five_hour" in out else None


# ---------- status ----------
def pace(used, resets, t):
    """Returns (elapsed_pct or None, status)."""
    el = None
    if resets is not None:
        el = max(0.0, min(100.0, (t - (resets - FIVE_H)) / FIVE_H * 100))
    if used >= 90:
        return el, "WALL"
    if el is None:
        return el, "ON-PACE"
    return el, "ON-PACE" if used <= el + 5 else "AHEAD"


def get_status(sid, tpath):
    t = now()
    cache = read_json(os.path.join(CACHE_DIR, safe_sid(sid) + ".json")) if sid else None
    fresh = bool(cache) and t - (num(cache.get("ts")) or 0) < FRESH
    ctx = None
    if fresh:
        cw = cache.get("context_window")
        ctx = usage_sum(cw.get("current_usage")) if isinstance(cw, dict) else None
    if ctx is None:
        ctx = ctx_from_transcript(tpath)
    plan = plan_from_cache(cache.get("rate_limits")) if fresh else None
    if plan is None:
        plan = fetch_endpoint()
    st = {"ctx": ctx, "plan": plan, "now": t}
    if plan:
        f = plan["five_hour"]
        st["h5"] = f["used"]
        st["resets"] = f["resets"]
        st["elapsed"], st["pace"] = pace(f["used"], f["resets"], t)
        st["d7"] = plan.get("seven_day", {}).get("used")
    return st


def fmt_k(n):
    return "%dk" % round(n / 1000)


def fmt_line(st):
    parts = []
    if st.get("plan"):
        s = "5h: %d%% used" % round(st["h5"])
        if st.get("elapsed") is not None:
            s += " / %d%% elapsed" % round(st["elapsed"])
        s += " (%s)" % st["pace"]
        parts.append(s)
        r = st.get("resets")
        if r is not None:
            rem = max(0, int(r - st["now"]))
            hhmm = datetime.fromtimestamp(r).strftime("%H:%M")
            parts.append("resets %s (%dh%02dm)" % (hhmm, rem // 3600, rem % 3600 // 60))
        if st.get("d7") is not None:
            parts.append("7d: %d%%" % round(st["d7"]))
    else:
        parts.append("plan usage unavailable")
    ctx = st.get("ctx")
    parts.append("ctx: %s/%dk soft cap" % (fmt_k(ctx) if ctx is not None else "?", CTX_CAP // 1000))
    if ctx is not None:
        if ctx >= CTX_CAP:
            parts.append("CTX OVER CAP")
        elif ctx >= CTX_HANDOVER:
            parts.append("CTX HANDOVER ZONE")
    if st.get("d7") is not None and st["d7"] >= 85:
        parts.append("7D CRITICAL")
    return "[budget] " + " · ".join(parts)


def bands_of(st):
    def b(v, th):
        return None if v is None else bisect.bisect_right(th, v)
    return {"h5": b(st.get("h5"), H5_BANDS), "ctx": b(st.get("ctx"), CTX_BANDS),
            "d7": b(st.get("d7"), D7_BANDS), "pace": st.get("pace")}


def state_path(sid):
    return os.path.join(STATE_DIR, safe_sid(sid) + ".json")


def save_state(sid, st):
    write_json_atomic(state_path(sid), {"ts": now(), "bands": bands_of(st)})


def emit(event, text):
    sys.stdout.write(json.dumps({"hookSpecificOutput": {
        "hookEventName": event, "additionalContext": text}}))


def read_stdin_json():
    try:
        d = json.loads(sys.stdin.read())
        return d if isinstance(d, dict) else {}
    except Exception:
        return {}


# ---------- transcript path for manual mode ----------
def find_transcript(sid, cwd):
    p = os.path.join(HOME, "projects", slug_of(cwd), sid + ".jsonl")
    if os.path.isfile(p):
        return p
    g = glob.glob(os.path.join(HOME, "projects", "*", sid + ".jsonl"))
    return g[0] if g else None


# ---------- commands ----------
def cmd_line():
    sid = os.environ.get("CLAUDE_CODE_SESSION_ID", "")
    tp = find_transcript(sid, os.getcwd()) if sid else None
    print(fmt_line(get_status(sid, tp)))


PACE_RANK = {"ON-PACE": 0, "AHEAD": 1, "WALL": 2}


def watch_reason(st0, st, t):
    """Why a watcher started at st0 should exit now, or None. Only crossings
    trigger, so a watcher started already at the wall waits for the reset."""
    if not st.get("plan"):
        return None
    r0, r = st0.get("resets"), st.get("resets")
    if r0 is not None and (t >= r0 or (r is not None and r > r0 + 600)):
        return "reset"
    if PACE_RANK.get(st.get("pace"), 0) > PACE_RANK.get(st0.get("pace"), 0):
        return "pace worse: %s" % st["pace"].lower()
    if st["h5"] >= WATCH["wall"] and st0["h5"] < WATCH["wall"]:
        return "wall: 5h at %d%%" % round(st["h5"])
    d, d0 = st.get("d7"), st0.get("d7")
    if d is not None and d >= 85 and (d0 is None or d0 < 85):
        return "7d critical: %d%%" % round(d)
    return None


WATCH = {"wall": 90.0}


def cmd_watch(argv):
    interval, max_min, WATCH["wall"] = 180.0, 50.0, 90.0
    it = iter(argv)
    for a in it:
        v = next(it, None)
        if a == "--interval":
            interval = float(v)
        elif a == "--max-minutes":
            max_min = float(v)
        elif a == "--threshold":
            WATCH["wall"] = min(90.0, float(v))
    sid = os.environ.get("CLAUDE_CODE_SESSION_ID", "")
    tp = find_transcript(sid, os.getcwd()) if sid else None
    deadline = now() + max_min * 60
    st0 = None
    while True:
        st = get_status(sid, tp)
        if st.get("plan"):
            if st0 is None:
                st0 = st
            else:
                reason = watch_reason(st0, st, st["now"])
                if reason:
                    break
        left = deadline - now()
        if left <= 0:
            reason = "heartbeat"
            break
        time.sleep(min(max(interval, 0.01), left))
    print("[budget-watch] %s" % reason)
    print(fmt_line(get_status(sid, tp)))


def start_deadline():
    global _deadline
    _deadline = time.monotonic() + HOOK_BUDGET


def cmd_hook_prompt():
    start_deadline()
    d = read_stdin_json()
    sid = d.get("session_id")
    st = get_status(sid, d.get("transcript_path"))
    emit("UserPromptSubmit", fmt_line(st))
    if sid:
        try:
            save_state(sid, st)
        except Exception:
            pass


def cmd_hook_posttool():
    start_deadline()
    d = read_stdin_json()
    if d.get("agent_id"):
        return
    sid = d.get("session_id")
    if not sid:
        return
    st = get_status(sid, d.get("transcript_path"))
    prev = read_json(state_path(sid))
    cur = bands_of(st)
    if prev and now() - (num(prev.get("ts")) or 0) < EMIT_INTERVAL and prev.get("bands") == cur:
        return
    emit("PostToolUse", fmt_line(st))
    save_state(sid, st)


def project_root(cwd):
    """Main repo root for any subdir or worktree of it, else cwd itself."""
    try:
        r = subprocess.run(["git", "-C", cwd, "rev-parse", "--path-format=absolute", "--git-common-dir"],
                           capture_output=True, text=True, timeout=GIT_CAP)
        d = r.stdout.strip()
        if r.returncode == 0 and os.path.basename(d) == ".git":
            return os.path.dirname(d)
    except Exception:
        pass
    return cwd


def handover_path(cwd):
    return os.path.join(HANDOVER_DIR, slug_of(project_root(cwd)) + ".md")


def cmd_hook_sessionstart():
    start_deadline()
    d = read_stdin_json()
    cwd = project_root(d.get("cwd") or os.getcwd())
    sid = d.get("session_id")
    line = fmt_line(get_status(sid, d.get("transcript_path")))
    text = line
    hp = handover_path(cwd)
    if d.get("source") in ("startup", "clear") and os.path.isfile(hp):
        try:
            with open(hp, encoding="utf-8", errors="replace") as f:
                content = f.read()
            arch = os.path.join(HANDOVER_DIR, "archive",
                                "%s-%s.md" % (slug_of(cwd), datetime.now().strftime("%Y%m%d-%H%M%S")))
            head = ("HANDOVER from previous session (file: %s, archived to %s). "
                    "Resume this work:\n\n" % (hp, arch))
            tail_line = "\n\n" + line
            note = "\n\n[handover truncated; read the archived file for the rest: %s]" % arch
            budget = MAX_CTX_CHARS - len(head) - len(tail_line)
            if len(content) > budget:
                content = content[:max(0, budget - len(note))] + note
            text = head + content + tail_line
            os.makedirs(os.path.dirname(arch), exist_ok=True)
            os.replace(hp, arch)
        except Exception:
            text = line
    emit("SessionStart", text)


def utf8_stdio():
    """Windows Python defaults to cp1252: hook JSON with non-ASCII paths would
    mis-decode and the "·" separator would print as a bare 0xB7 byte."""
    for stream, kw in ((sys.stdin, {"errors": "replace"}), (sys.stdout, {})):
        try:
            stream.reconfigure(encoding="utf-8", **kw)
        except Exception:
            pass


def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    utf8_stdio()
    try:
        if cmd == "cache":
            cmd_cache(sys.stdin.read())
        elif cmd == "line":
            cmd_line()
        elif cmd == "watch":
            cmd_watch(sys.argv[2:])
        elif cmd == "hook-prompt":
            cmd_hook_prompt()
        elif cmd == "hook-posttool":
            cmd_hook_posttool()
        elif cmd == "hook-sessionstart":
            cmd_hook_sessionstart()
        elif cmd == "handover-path":
            # forward slashes so the path survives being pasted into Git Bash
            print(handover_path(os.getcwd()).replace(os.sep, "/"))
    except Exception:
        pass
    sys.stdout.flush()
    if _stuck:
        os._exit(0)  # don't let a hung fetch thread delay interpreter shutdown


if __name__ == "__main__":
    main()
