#!/usr/bin/env python3
"""Live check of the cat band: run interactive claude in a pty with the mod, measure CPU and shown frames.

usage: live-check.py <label> <outdir> [--rows R] [--fps F] [--resize-at S] [--no-mod]

The binary is $PURR_BIN when set (e.g. a release wrapper that pins
PURR_STATE_DIR), else the dev-sandbox wrapper under /tmp/claude-1000/purr-band-smoke.
claude keeps the real HOME and runs with --setting-sources project,local, so
user hooks and the user statusline do not run. PURR_STATE_DIR is passed
through when set. The mod writes its counters to <outdir>/<label>.stats.json
(PURR_BAND_STATS).
"""
import argparse, fcntl, json, os, pty, re, select, signal, struct, subprocess, sys, termios, time

WT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
MOD = WT + '/mod'
COLS, LINES = 140, 40
DUR, W0, W1 = 22.0, 8.0, 18.0
HZ = os.sysconf('SC_CLK_TCK')

ap = argparse.ArgumentParser()
ap.add_argument('label')
ap.add_argument('outdir')
ap.add_argument('--rows', default='4')
ap.add_argument('--fps', default='30')
ap.add_argument('--resize-at', type=float, default=None)
ap.add_argument('--no-mod', action='store_true')
opt = ap.parse_args()
label, outdir = opt.label, opt.outdir
if not os.path.realpath(outdir).startswith('/tmp/claude-1000/'):
    sys.exit('live-check: outdir must be under /tmp/claude-1000/')
os.makedirs(outdir, exist_ok=True)
counters = f'{outdir}/{label}.stats.json'
if os.path.exists(counters):
    os.remove(counters)
purr_bin = os.environ.get('PURR_BIN')
if not purr_bin:
    sb = '/tmp/claude-1000/purr-band-smoke'
    subprocess.run(['bash', WT + '/mod/scripts/dev-sandbox.sh', sb], check=True, stdout=subprocess.DEVNULL)
    purr_bin = sb + '/bin/purrgotchi'

argv = ['claude', '--setting-sources', 'project,local', '--debug-file', f'{outdir}/{label}.debug.log']
if os.environ.get('SPIKE_FULLSCREEN'):
    argv += ['--settings', '{"tui":"fullscreen"}']
if not opt.no_mod:
    argv += ['--plugin-dir', MOD]
env = dict(os.environ, COLUMNS=str(COLS), LINES=str(LINES), TERM='xterm-256color', COLORTERM='truecolor',
           PURR_BIN=purr_bin, PURR_BAND_ROWS=opt.rows, PURR_BAND_FPS=opt.fps, PURR_BAND_STATS=counters)
for k in list(env):
    if k.startswith(('CLAUDE', 'VSCODE')) or k in ('TERM_PROGRAM', 'TERM_PROGRAM_VERSION'):
        del env[k]

pid, fd = pty.fork()
if pid == 0:
    os.chdir(WT)
    os.execvpe(argv[0], argv, env)
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', LINES, COLS, 0, 0))


def tree(root):
    kids = {}
    for p in os.listdir('/proc'):
        if not p.isdigit():
            continue
        try:
            st = open(f'/proc/{p}/stat').read()
            ppid = int(st.rsplit(')', 1)[1].split()[1])
            kids.setdefault(ppid, []).append(int(p))
        except OSError:
            pass
    out, stack = [], [root]
    while stack:
        q = stack.pop()
        out.append(q)
        stack += kids.get(q, [])
    return out


def cpu(p):
    try:
        f = open(f'/proc/{p}/stat').read().rsplit(')', 1)[1].split()
        cmd = open(f'/proc/{p}/cmdline').read().replace('\0', ' ')
        return int(f[11]) + int(f[12]), cmd
    except OSError:
        return None, ''


def sample():
    tot, prod, procs = 0, 0, {}
    for p in tree(pid):
        t, cmd = cpu(p)
        if t is None:
            continue
        procs[p] = (t, cmd[:80])
        if 'purrgotchi' in cmd and ' band ' in cmd:
            prod += t
        else:
            tot += t
    return tot, prod, procs


def stats():
    try:
        return json.load(open(counters))
    except (OSError, ValueError):
        return None


t0 = time.monotonic()
chunks, s0, s1 = [], None, None
st0 = st1 = None
answered = resized = False
while True:
    now = time.monotonic() - t0
    if now >= DUR:
        break
    if s0 is None and now >= W0:
        s0 = (now, sample())
        st0 = stats()
    if s1 is None and now >= W1:
        s1 = (now, sample())
        st1 = stats()
    if opt.resize_at is not None and not resized and now >= opt.resize_at:
        resized = True
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', LINES, 120, 0, 0))
        os.kill(pid, signal.SIGWINCH)
        print('resized at', round(now, 2))
    r, _, _ = select.select([fd], [], [], 0.05)
    if r:
        try:
            data = os.read(fd, 65536)
        except OSError:
            break
        chunks.append((now, data))
        if not answered and re.search(rb'trust', data, re.I) and now < 6:
            answered = True
            print('NOTE: trust-like text seen at', round(now, 2))

try:
    os.kill(pid, signal.SIGTERM)
    time.sleep(1)
    os.kill(pid, signal.SIGKILL)
except OSError:
    pass
for p in tree(pid):
    try:
        os.kill(p, signal.SIGKILL)
    except OSError:
        pass

raw = b''.join(d for _, d in chunks)
open(f'{outdir}/{label}.tty', 'wb').write(raw)
win_bytes = sum(len(d) for t, d in chunks if W0 <= t < W1)

# frames shown: repaint bursts (each starts with cursor-hide ESC[?25l) that carry a half-block cell
import bisect
offs, acc = [], 0
for t, d in chunks:
    offs.append(acc)
    acc += len(d)
starts = [m.start() for m in re.finditer(rb'\x1b\[\?25l', raw)] + [len(raw)]
bursts_win = band_bursts_win = 0
for a, b in zip(starts, starts[1:]):
    t = chunks[bisect.bisect_right(offs, a) - 1][0]
    if W0 <= t < W1:
        bursts_win += 1
        if b'\xe2\x96\x80' in raw[a:b] or b'\x1b[3' in raw[a:b]:
            band_bursts_win += 1
ids_all = set()
(ta, (ca, pa, procs_a)), (tb, (cb, pb, procs_b)) = s0, s1
span = tb - ta
res = {
    'label': label, 'rows': opt.rows, 'fps': opt.fps, 'resize_at': opt.resize_at, 'bin': purr_bin,
    'claude_cpu_pct': round(100 * (cb - ca) / HZ / span, 1),
    'stream_cpu_pct': round(100 * (pb - pa) / HZ / span, 1),
    'tty_bytes_per_s': round(win_bytes / (W1 - W0)),
    'shown_fps': round(band_bursts_win / (W1 - W0), 1),
    'bursts_per_s': round(bursts_win / (W1 - W0), 1),
    'procs': {p: v[1] for p, v in procs_b.items()},
}
c = stats()
if st0 and st1 and st1['t'] > st0['t']:
    dt = (st1['t'] - st0['t']) / 1000
    for k in ('sent', 'ok', 'lines', 'frames'):
        res[f'mod_{k}_per_s'] = round((st1[k] - st0[k]) / dt, 2)
res['mod'] = c if c is not None else 'no stats'
json.dump(res, open(f'{outdir}/{label}.result.json', 'w'), indent=1)
print(json.dumps(res, indent=1))
