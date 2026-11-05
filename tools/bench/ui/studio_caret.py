#!/usr/bin/env python3
"""RapidR Studio's editor in a big file (docs/speed-audit-2026-11.md):
Studio on the desktop's headless host (a GUI test's run: RAPIDR_CAPTURE)
opens FILE in its code view, waits --wait ms, clicks in the text, then plays
a scenario; what it costs is the run's CPU and wall time against the same
run without the scenario (the baseline: Studio's start and the file's open).

  moves   N caret moves (the Down arrow)
  edits   N rounds of: a letter typed and taken back, a pause of 1.5 s
          (Studio's idle work after an edit), a caret move

The test script plays an event at most every 50 ms, so a wall time per
event near 50 ms means "as fast as the script"; the CPU time per event is
the cost. The headless host doesn't paint: a real window adds its frames.

  python3 tools/bench/ui/studio_caret.py FILE [moves|edits] [N] [--wait MS] [--sample OUT_PREFIX AFTER_S SECS]

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


def run(file, events, sample=None):
    e = env()
    e["RAPIDR_TEST_EVENTS"] = events
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
    return wall, cpu, (line[-1] if line else "?")


def main():
    a = sys.argv[1:]
    if not a:
        sys.exit(__doc__)
    file = os.path.abspath(a[0])
    rest = [x for x in a[1:] if not x.startswith("--")]
    kind = rest[0] if rest and rest[0] in ("moves", "edits") else "moves"
    n = int(rest[1]) if len(rest) > 1 else (40 if kind == "moves" else 4)
    wait = int(a[a.index("--wait") + 1]) if "--wait" in a else 6000
    sample = None
    if "--sample" in a:
        i = a.index("--sample")
        sample = (a[i + 1], float(a[i + 2]), a[i + 3])
    lead = f"studio.__wait_{wait},codedoc(0).__mousedown_300_200,codedoc(0).__mouseup_300_200"
    if kind == "moves":
        play = ",".join(["codedoc(0).__key_40"] * n)
    else:
        play = ",".join(["codedoc(0).__key_88,codedoc(0).__key_8,studio.__wait_1500,codedoc(0).__key_40"] * n)
    w0, c0, l0 = run(file, lead)
    w1, c1, l1 = run(file, lead + "," + play, sample)
    name = os.path.basename(file)
    print(f"{name}: open (Studio's start + the file, then a {wait} ms wait): wall {w0 * 1000:.0f} ms, CPU {c0 * 1000:.0f} ms (caret line {l0})")
    print(f"{name}: + {n} x {kind}: wall {w1 * 1000:.0f} ms, CPU {c1 * 1000:.0f} ms (caret line {l1})")
    print(f"{name}: each {kind[:-1]}: wall {(w1 - w0) * 1000 / n:.1f} ms, CPU {(c1 - c0) * 1000 / n:.1f} ms")


if __name__ == "__main__":
    main()
