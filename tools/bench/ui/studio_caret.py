#!/usr/bin/env python3
"""RapidR Studio's editor in a big file (docs/speed-audit-2026-11.md):
Studio on the desktop's headless host (a GUI test's run: RAPIDR_CAPTURE)
opens FILE in its code view, waits --wait ms, clicks in the text, then plays
a scenario; what it costs is the run's CPU and wall time against the same
run without the scenario (the baseline: Studio's start and the file's open).

  moves   N caret moves (the Down arrow)
  typing  N letters typed, one after the other (a word typed, a space,
          as at the script's pace), then a pause of 1.5 s
  edits   N rounds of: a letter typed and taken back, a pause of 1.5 s
          (Studio's idle work after an edit), a caret move

The test script plays an event at most every 50 ms, so a wall time per
event near 50 ms means "as fast as the script"; the CPU time per event is
the cost. The headless host doesn't paint: a real window adds its frames.

The UI thread on its own (RAPIDR_TEST_BUSY): its CPU per event, taken in
the scenario's run itself (from its last busy stretch before the scenario
to its last before a closing wait: Studio's start and exit left out), and
how long it stayed busy at a stretch after the lead's events — the longest
stretch is the longest a key pressed then would wait (p99 and max printed:
"UI busy"). The language service's worker isn't in either.

  python3 tools/bench/ui/studio_caret.py FILE [moves|edits|typing] [N] [--wait MS] [--sample OUT_PREFIX AFTER_S SECS]

A 5,000-line program to try: python3 tools/bench/ui/gen_source.py 5000 > big.rr
Nothing reaches the user's settings, registry, keychain or printer: Studio's
registry, data and prints are under tests/results/bench/studio/.
"""
import os
import re
import resource
import signal
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(HERE)))
RAPIDR = os.environ.get("RAPIDR") or os.path.join(ROOT, "rapidr")
LEAD_EVENTS = 3
WORK = os.path.join(ROOT, "tests", "results", "bench", "studio")


def env():
    e = dict(os.environ)
    os.makedirs(os.path.join(WORK, "prints"), exist_ok=True)
    e.update({
        "RAPIDR_REGISTRY": os.path.join(WORK, "studio.reg"),
        "RAPIDR_STUDIO_DATA": os.path.join(WORK, "studio-data"),
        "RAPIDR_AI_KEYSTORE": "memory",
        "RAPIDR_PRINT_TO": os.path.join(WORK, "prints"),
        "RAPIDR_MENU": "window",
        "RAPIDR_CAPTURE": os.path.join(WORK, "cap"),
        "RAPIDR_CAPTURE_DELAY": "1",
        "RAPIDR_TEST_DUMP": "codedoc(0).caretline",
    })
    # (SEC-24: a release build's test hooks need the test-mode token)
    token = os.path.expanduser("~/.rapidr/test-mode.token")
    if "RAPIDR_TEST_MODE" not in e and os.path.exists(token):
        e["RAPIDR_TEST_MODE"] = token
    return e


def busy_stats(path, after, end):
    """The UI thread's stretches from event `after` until event `end` starts
    (the closing wait: Studio's exit is after it), and its CPU between the
    last stretch before them and the last of them (ms; None without one)."""
    stretches, base, last = [], None, None
    try:
        with open(path) as f:
            for line in f:
                p = line.split()
                if not p or p[0] != "busy":
                    continue
                played, cpu = int(p[1]), float(p[4]) if len(p) > 4 else None
                if played < after:
                    base = cpu
                elif played < end:
                    stretches.append(float(p[3]))
                    last = cpu
    except OSError:
        pass
    cpu = last - base if base is not None and last is not None else None
    return sorted(stretches), cpu


def pct(v, p):
    return v[min(len(v) - 1, int(round((len(v) - 1) * p)))] if v else 0.0


def run(file, events, sample=None):
    e = env()
    # (a closing wait: what the scenario's last event set going is done
    # before it, Studio's exit after it)
    events += ",studio.__wait_300"
    e["RAPIDR_TEST_EVENTS"] = events
    busy = os.path.join(WORK, "busy.txt")
    if os.path.exists(busy):
        os.remove(busy)
    e["RAPIDR_TEST_BUSY"] = busy
    cmd = [RAPIDR, "run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", "rapidr-light", "--do", "wait,view.code,wait", file]
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    t0 = time.perf_counter()
    p = subprocess.Popen(cmd, cwd=ROOT, env=e, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    if sample:
        prefix, after, secs = sample
        time.sleep(after)
        subprocess.run(["sample", str(p.pid), str(secs), "-mayDie", "-file", prefix + ".sample.txt"], capture_output=True)
    try:
        out, _ = p.communicate(timeout=600)
    except subprocess.TimeoutExpired:
        os.kill(p.pid, signal.SIGKILL)
        out, _ = p.communicate()
    wall = time.perf_counter() - t0
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = (after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime)
    line = re.findall(r"codedoc\(0\)\.caretline=(\S*)", out)
    return wall, cpu, (line[-1] if line else "?"), busy_stats(busy, LEAD_EVENTS, events.count(",") + 1)


def main():
    a = sys.argv[1:]
    if not a:
        sys.exit(__doc__)
    file = os.path.abspath(a[0])
    rest = [x for x in a[1:] if not x.startswith("--")]
    kind = rest[0] if rest and rest[0] in ("moves", "edits", "typing") else "moves"
    n = int(rest[1]) if len(rest) > 1 else (40 if kind == "moves" else 4)
    wait = int(a[a.index("--wait") + 1]) if "--wait" in a else 6000
    sample = None
    if "--sample" in a:
        i = a.index("--sample")
        sample = (a[i + 1], float(a[i + 2]), a[i + 3])
    lead = f"studio.__wait_{wait},codedoc(0).__mousedown_300_200,codedoc(0).__mouseup_300_200"
    global LEAD_EVENTS
    LEAD_EVENTS = lead.count(",") + 1
    if kind == "moves":
        play = ",".join(["codedoc(0).__key_40"] * n)
    elif kind == "typing":
        # (letters, a space after every fifth: words, completion opening and
        # narrowing as a person's typing makes it)
        keys = ["codedoc(0).__key_32" if i % 6 == 5 else "codedoc(0).__key_" + str(65 + i % 26) for i in range(n)]
        play = ",".join(keys) + ",studio.__wait_1500"
    else:
        play = ",".join(["codedoc(0).__key_88,codedoc(0).__key_8,studio.__wait_1500,codedoc(0).__key_40"] * n)
    w0, c0, l0, _ = run(file, lead)
    w1, c1, l1, (stretches, ui) = run(file, lead + "," + play, sample)
    name = os.path.basename(file)
    print(f"{name}: open (Studio's start + the file, then a {wait} ms wait): wall {w0 * 1000:.0f} ms, CPU {c0 * 1000:.0f} ms (caret line {l0})")
    print(f"{name}: + {n} x {kind}: wall {w1 * 1000:.0f} ms, CPU {c1 * 1000:.0f} ms (caret line {l1})")
    print(f"{name}: each {kind[:-1]}: wall {(w1 - w0) * 1000 / n:.1f} ms, CPU {(c1 - c0) * 1000 / n:.1f} ms")
    # (the UI thread's CPU from the one run: a difference of two runs is
    # mostly how differently Studio's start went in them)
    ui_each = f"{ui / n:.1f}" if ui is not None else "?"
    print(f"{name}: UI thread: CPU {ui_each} ms each; UI busy p50 {pct(stretches, 0.5):.1f} ms, p99 {pct(stretches, 0.99):.1f} ms, max {pct(stretches, 1.0):.1f} ms ({len(stretches)} stretches)")


if __name__ == "__main__":
    main()
