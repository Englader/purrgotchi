import json, os, subprocess, sys, tempfile, threading, time, unittest

D = os.path.dirname(os.path.abspath(__file__))
PY = os.path.join(D, "budget.py")
WRAP = os.path.join(D, "statusline-wrap.sh")
sys.path.insert(0, D)
import budget  # noqa: E402

WIN = os.name == "nt"


class T(unittest.TestCase):
    def setUp(self):
        # realpath: Windows temp is an 8.3 short path (STEFAN~1.MAR) but git
        # reports the long one, so compare everything in long form.
        self.h = os.path.realpath(tempfile.mkdtemp())
        self.env = dict(os.environ, BUDGET_HOME=self.h, BUDGET_NO_NET="1")
        self.now = time.time()

    def run_b(self, *args, inp="", cwd=None, env=None):
        r = subprocess.run([sys.executable, PY, *args], input=inp, capture_output=True,
                           text=True, env=env or self.env, cwd=cwd)
        self.assertEqual(r.returncode, 0)
        return r.stdout

    def sl(self, sid="s1", h5=34, resets=None, ctx=54000, d7=16):
        return {"session_id": sid, "transcript_path": "/nonexistent",
                "rate_limits": {"five_hour": {"used_percentage": h5, "resets_at": resets or self.now + 4 * 3600},
                                "seven_day": {"used_percentage": d7, "resets_at": self.now + 86400}},
                "context_window": {"current_usage": {"input_tokens": 4000, "cache_creation_input_tokens": 0,
                                                     "cache_read_input_tokens": ctx - 4000},
                                   "context_window_size": 1000000, "used_percentage": 5}}

    def cache(self, **kw):
        d = self.sl(**kw)
        self.run_b("cache", inp=json.dumps(d))
        return d

    def prompt_ctx(self, sid="s1", **extra):
        out = self.run_b("hook-prompt", inp=json.dumps(dict(session_id=sid, **extra)))
        return json.loads(out)["hookSpecificOutput"]["additionalContext"]

    def test_cache_and_line(self):
        self.cache()
        p = os.path.join(self.h, "hooks/budget/cache/s1.json")
        c = json.load(open(p))
        self.assertEqual(c["rate_limits"]["five_hour"]["used_percentage"], 34)
        line = self.prompt_ctx()
        self.assertIn("5h: 34% used", line)
        self.assertIn("7d: 16%", line)
        self.assertIn("ctx: 54k/400k soft cap", line)
        self.assertIn("(AHEAD)", line)  # 34% used vs ~20% elapsed

    def test_transcript_fallback(self):
        tp = os.path.join(self.h, "t.jsonl")
        with open(tp, "w") as f:
            f.write(json.dumps({"type": "user", "message": {"role": "user", "content": "hi"}}) + "\n")
            f.write(json.dumps({"type": "assistant", "message": {"role": "assistant", "usage": {
                "input_tokens": 1, "cache_creation_input_tokens": 2, "cache_read_input_tokens": 3}}}) + "\n")
            f.write(json.dumps({"type": "assistant", "message": {"role": "assistant", "usage": {
                "input_tokens": 10, "cache_creation_input_tokens": 20000, "cache_read_input_tokens": 100000}}}) + "\n")
            f.write(json.dumps({"type": "user", "message": {"role": "user", "content": "x"}}) + "\n")
        line = self.prompt_ctx(sid="nocache", transcript_path=tp)
        self.assertIn("ctx: 120k/400k", line)
        self.assertIn("plan usage unavailable", line)

    def test_pace(self):
        # elapsed ~50%: resets in 2.5h
        r = self.now + 2.5 * 3600
        self.cache(sid="a", h5=52, resets=r) if False else None
        for h5, exp in ((52, "ON-PACE"), (60, "AHEAD"), (91, "WALL")):
            self.cache(h5=h5, resets=r)
            self.assertIn("(%s)" % exp, self.prompt_ctx())
        self.cache(ctx=310000, d7=90)
        l = self.prompt_ctx()
        self.assertIn("CTX HANDOVER ZONE", l)
        self.assertIn("7D CRITICAL", l)
        self.cache(ctx=410000)
        self.assertIn("CTX OVER CAP", self.prompt_ctx())

    def test_posttool_throttle(self):
        self.cache(h5=10, resets=self.now + 2.5 * 3600)
        hook = json.dumps({"session_id": "s1", "tool_name": "Bash"})
        self.assertIn("additionalContext", self.run_b("hook-posttool", inp=hook))
        self.assertEqual(self.run_b("hook-posttool", inp=hook), "")
        sub = json.dumps({"session_id": "s1", "agent_id": "ag1"})
        self.cache(h5=55, resets=self.now + 2.5 * 3600)  # band change
        self.assertEqual(self.run_b("hook-posttool", inp=sub), "")
        out = self.run_b("hook-posttool", inp=hook)
        self.assertIn("55%", out)
        self.assertEqual(self.run_b("hook-posttool", inp=hook), "")
        # time-based
        sp = os.path.join(self.h, "hooks/budget/state/s1.json")
        st = json.load(open(sp)); st["ts"] -= 700; json.dump(st, open(sp, "w"))
        self.assertIn("additionalContext", self.run_b("hook-posttool", inp=hook))

    def start(self, source, cwd):
        out = self.run_b("hook-sessionstart", inp=json.dumps(
            {"session_id": "s1", "source": source, "cwd": cwd}))
        return json.loads(out)["hookSpecificOutput"]["additionalContext"]

    def test_sessionstart(self):
        cwd = "/a/b.c/proj"
        hp = os.path.join(self.h, "handovers", "-a-b-c-proj.md")
        os.makedirs(os.path.dirname(hp))
        open(hp, "w").write("DO THE THING")
        self.assertNotIn("HANDOVER", self.start("resume", cwd))
        self.assertTrue(os.path.exists(hp))
        t = self.start("clear", cwd)
        self.assertIn("HANDOVER from previous session", t)
        self.assertIn("DO THE THING", t)
        self.assertIn("[budget]", t)
        self.assertFalse(os.path.exists(hp))
        arch = os.listdir(os.path.join(self.h, "handovers", "archive"))
        self.assertEqual(len(arch), 1)
        t2 = self.start("startup", cwd)
        self.assertNotIn("HANDOVER", t2)
        self.assertIn("[budget]", t2)

    def test_truncation(self):
        cwd = "/big"
        hp = os.path.join(self.h, "handovers", "-big.md")
        os.makedirs(os.path.dirname(hp))
        open(hp, "w").write("x" * 50000)
        t = self.start("startup", cwd)
        self.assertLessEqual(len(t), 9500)
        self.assertIn("truncated", t)

    def test_handover_path(self):
        d = os.path.join(self.h, "plain.dir")
        os.makedirs(d)
        out = self.run_b("handover-path", cwd=d).strip()
        self.assertNotIn("\\", out)  # forward slashes: safe to paste into Git Bash
        self.assertEqual(os.path.normpath(out), os.path.join(self.h, "handovers", budget.slug_of(d) + ".md"))

    def test_slug_matches_claude_projects_dirs(self):
        self.assertEqual(budget.slug_of("/home/u/Desktop/Work/purr.gotchi"),
                         "-home-u-Desktop-Work-purr-gotchi")
        self.assertEqual(budget.slug_of(r"C:\Users\Stefan.Markoski\Desktop\Work\purrgotchi"),
                         "C--Users-Stefan-Markoski-Desktop-Work-purrgotchi")
        self.assertEqual(budget.slug_of("C:/Users/a b/x_y"), "C--Users-a-b-x-y")

    def test_line_is_utf8(self):
        r = subprocess.run([sys.executable, PY, "line"], capture_output=True,
                           env=dict(self.env, CLAUDE_CODE_SESSION_ID="", PYTHONIOENCODING="", PYTHONUTF8="0"))
        self.assertIn("plan usage unavailable \u00b7 ctx".encode("utf-8"), r.stdout)

    def test_hook_reads_utf8_stdin(self):
        tdir = os.path.join(self.h, "St\u00e9fan \u0436")
        os.makedirs(tdir)
        tp = os.path.join(tdir, "t.jsonl")
        with open(tp, "w", encoding="utf-8") as f:
            f.write(json.dumps({"message": {"role": "assistant", "usage": {"input_tokens": 77000}}}) + "\n")
        inp = json.dumps({"session_id": "u1", "transcript_path": tp}, ensure_ascii=False).encode("utf-8")
        r = subprocess.run([sys.executable, PY, "hook-prompt"], input=inp, capture_output=True,
                           env=dict(self.env, PYTHONIOENCODING="", PYTHONUTF8="0"))
        self.assertIn("ctx: 77k/400k", json.loads(r.stdout)["hookSpecificOutput"]["additionalContext"])

    def test_atomic_write_survives_open_reader(self):
        # Windows: os.replace fails while another process holds the target open.
        p = os.path.join(self.h, "c", "x.json")
        budget.write_json_atomic(p, {"v": 1})
        f = open(p, encoding="utf-8")
        threading.Timer(0.15, f.close).start()
        budget.write_json_atomic(p, {"v": 2})
        self.assertEqual(budget.read_json(p), {"v": 2})
        self.assertEqual(os.listdir(os.path.dirname(p)), ["x.json"])

    def test_handover_path_resolves_repo_root(self):
        repo = os.path.join(self.h, "repo")
        sub = os.path.join(repo, "docs", "specs")
        os.makedirs(sub)
        git = lambda *a: subprocess.run(["git", "-C", repo, *a], check=True, capture_output=True)
        git("init", "-q")
        git("-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "x")
        wt = os.path.join(repo, ".worktrees", "w")
        git("worktree", "add", "-q", wt)
        want = os.path.join(self.h, "handovers", budget.slug_of(repo) + ".md")
        for cwd in (repo, sub, wt):
            self.assertEqual(os.path.normpath(self.run_b("handover-path", cwd=cwd).strip()), want, cwd)
        os.makedirs(os.path.dirname(want))
        open(want, "w").write("RESUME ME")
        self.assertIn("RESUME ME", self.start("clear", wt))

    def watch(self, first, later=None, delay=0.6, extra=("--max-minutes", "0.1"), env=None):
        """Run watch with cache `first`; after `delay` s rewrite cache with `later`."""
        self.cache(**first)
        p = subprocess.Popen([sys.executable, PY, "watch", "--interval", "0.05", *extra],
                             stdout=subprocess.PIPE, text=True, env=env or dict(self.env, CLAUDE_CODE_SESSION_ID="s1"))
        if later is not None:
            time.sleep(delay)
            self.cache(**later)
        out, _ = p.communicate(timeout=20)
        self.assertEqual(p.returncode, 0)
        lines = out.strip().split("\n")
        self.assertEqual(len(lines), 2, out)
        self.assertTrue(lines[1].startswith("[budget]"))
        return lines[0]

    def test_watch_pace_worse(self):
        r = self.now + 2.5 * 3600
        out = self.watch(dict(h5=50, resets=r), dict(h5=60, resets=r))
        self.assertEqual(out, "[budget-watch] pace worse: ahead")

    def test_watch_wall(self):
        r = self.now + 2.5 * 3600
        out = self.watch(dict(h5=60, resets=r), dict(h5=92, resets=r))
        self.assertIn("wall", out)  # AHEAD -> WALL

    def test_watch_wall_threshold(self):
        r = self.now + 2.5 * 3600
        out = self.watch(dict(h5=50, resets=r), dict(h5=76, resets=r))
        self.assertIn("pace worse", out)
        out = self.watch(dict(h5=60, resets=r), dict(h5=76, resets=r),
                         extra=("--max-minutes", "0.1", "--threshold", "75"))
        self.assertIn("wall: 5h at 76%", out)

    def test_watch_7d(self):
        r = self.now + 2.5 * 3600
        out = self.watch(dict(h5=50, resets=r, d7=80), dict(h5=50, resets=r, d7=86))
        self.assertEqual(out, "[budget-watch] 7d critical: 86%")

    def test_watch_reset(self):
        out = self.watch(dict(h5=95, resets=self.now + 2), dict(h5=3, resets=self.now + 5 * 3600), delay=0.3)
        self.assertEqual(out, "[budget-watch] reset")

    def test_watch_reset_by_time(self):
        out = self.watch(dict(h5=95, resets=self.now + 1.5))
        self.assertEqual(out, "[budget-watch] reset")

    def test_watch_wall_start_stays_quiet(self):
        r = self.now + 2.5 * 3600
        out = self.watch(dict(h5=95, resets=r), dict(h5=96, resets=r), extra=("--max-minutes", "0.02"))
        self.assertEqual(out, "[budget-watch] heartbeat")

    def test_watch_heartbeat(self):
        r = self.now + 2.5 * 3600
        t0 = time.time()
        out = self.watch(dict(h5=50, resets=r), extra=("--max-minutes", "0.03"))
        self.assertEqual(out, "[budget-watch] heartbeat")
        self.assertGreaterEqual(time.time() - t0, 1.7)

    def test_watch_unknown_keeps_looping(self):
        # no cache, no net: never exits early, only on heartbeat
        p = subprocess.run([sys.executable, PY, "watch", "--interval", "0.05", "--max-minutes", "0.03"],
                           capture_output=True, text=True, env=dict(self.env, CLAUDE_CODE_SESSION_ID="none"))
        self.assertEqual(p.stdout.split("\n")[0], "[budget-watch] heartbeat")
        self.assertIn("plan usage unavailable", p.stdout)

    def test_watch_stale_cache_uses_endpoint_cache(self):
        r = self.now + 2.5 * 3600
        self.cache(h5=10, resets=r)
        cp = os.path.join(self.h, "hooks/budget/cache")
        c = json.load(open(os.path.join(cp, "s1.json"))); c["ts"] -= 1000
        json.dump(c, open(os.path.join(cp, "s1.json"), "w"))
        json.dump({"ts": time.time(), "data": {"five_hour": {"used": 61.0, "resets": r},
                                               "seven_day": {"used": 5.0, "resets": r}}},
                  open(os.path.join(cp, "_endpoint.json"), "w"))
        out = self.run_b("watch", "--interval", "0.05", "--max-minutes", "0.02",
                         env=dict(self.env, CLAUDE_CODE_SESSION_ID="s1"))
        self.assertIn("5h: 61% used", out)

    @unittest.skipIf(WIN, "bash wrapper is Linux-only; Windows caches natively in purrgotchi")
    def test_wrapper(self):
        env = dict(self.env, BUDGET_WRAPPED_CMD="cat; echo EXTRA; exit 7")
        d = json.dumps(self.sl())
        r = subprocess.run([WRAP], input=d, capture_output=True, text=True, env=env)
        self.assertEqual(r.returncode, 7)
        self.assertEqual(r.stdout, d + "EXTRA\n")
        p = os.path.join(self.h, "hooks/budget/cache/s1.json")
        for _ in range(50):
            if os.path.exists(p):
                break
            time.sleep(0.1)
        self.assertTrue(os.path.exists(p))

    @unittest.skipIf(WIN, "bash wrapper is Linux-only; Windows caches natively in purrgotchi")
    def test_wrapper_garbage_stdin(self):
        env = dict(self.env, BUDGET_WRAPPED_CMD="cat")
        r = subprocess.run([WRAP], input="not json", capture_output=True, text=True, env=env)
        self.assertEqual((r.returncode, r.stdout), (0, "not json"))


if __name__ == "__main__":
    unittest.main(verbosity=1)
