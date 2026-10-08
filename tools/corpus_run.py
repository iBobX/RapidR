#!/usr/bin/env python3
"""Runs every program of RapidQ's example corpus that compiles, on the
interpreter and as a native build, and sorts what goes wrong by cause.

Usage (repo root, after building ./rapidr):
    python3 tools/corpus_run.py [--backend vm,native] [--jobs N]
        [--corpus corpus.json] [--examples DIR] [--include DIR]
        [--golden DIR] [--report report.md] [--json out.json]
        [--label "after"] [--before before.json] [filter ...]

Which programs: those `tools/rapidq_corpus.py --json` lists as compiling
(`--corpus`; without one the compile check runs first). Each program's
folder is copied to tests/conformance/.work/corpus-run/<name>/ (RapidQ's
examples stay read-only) and run there with no input and a time limit:

- the interpreter: `rapidr run prog.bas`;
- native: `rapidr build prog.bas` (a Rust build), then the executable.

GUI programs run under the GUI tests' headless host (RAPIDR_CAPTURE): after
RAPIDR_CAPTURE_DELAY seconds every shown window is saved as a BMP and the
program ends; message boxes are answered with Escape, file / colour / font
dialogs with Cancel. Nothing prints on paper (RAPIDR_PRINT_TO) and QREGISTRY
writes a scratch store (RAPIDR_REGISTRY). Programs that use the network,
print, or start other programs are not run.

What counts as wrong, per run: a RapidR run-time error, a panic, a crash
(a signal), a hang (the time limit, unless the program waits for a key or a
line as RapidQ's would — or RC.EXE's build of it ran into the limit too, as
tools/rapidq_truth.py corpus found), RapidR's own warnings (`[rapidr] …`,
`[WARN] …`), a console program whose output isn't RapidQ's (when RC.EXE's
output of it is in --golden: tools/rapidq_truth.py corpus --write-golden), a
GUI program that shows no window, or one whose every window is a single
colour. Console programs also run in a terminal (a pseudo-terminal, typed
Enter, a number, q, Escape …): a run-time error, panic or warning there
counts too ("(terminal)").

The problems are grouped by cause (messages with their numbers and names
taken out) and written, with the counts, to --report. Programs that need
DirectX, OLE, Windows DLLs or hardware access are other lanes' work: run and
counted, but listed apart. With --before (an earlier --json) the report has
the before / after numbers.
"""

import argparse
import collections
import concurrent.futures
import datetime
import importlib.util
import json
import os
import re
import shutil
import struct
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXE = ".exe" if os.name == "nt" else ""
RAPIDR = os.path.join(ROOT, "rapidr" + EXE)
WORK = os.path.join(ROOT, "tests", "conformance", ".work", "corpus-run")
RAPIDQ = os.path.expanduser(os.environ.get("RAPIDQ_DIR", "~/Downloads/Rapidq"))


def load_tool(name):
    spec = importlib.util.spec_from_file_location(name, os.path.join(ROOT, "tools", name + ".py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


corpus = load_tool("rapidq_corpus")
truth = load_tool("rapidq_truth")

NETWORK = re.compile(r"\b(qsocket|qclientsocket|qserversocket|qmysql|rmysql|qhttp|rhttp|qftp|qsmtp|qpop3|qwebbrowser|qdownload|inet\w*|winsock|wsock32|ws2_32|wininet|urlmon|urldownload\w*|gethostby\w+)\b", re.I)
PRINTING = truth.PRINTING
SHELL = truth.SHELL
GUI = re.compile(r"\bQ(FORM|FORMEX|FORMMDI|DOCKFORM)\b|\bR(FORM|FORMMDI)\b", re.I)
APPTYPE_CONSOLE = truth.APPTYPE_CONSOLE
# (a console program that waits for a key or a line: RapidQ's waits too)
WAITS = re.compile(r"\binkey\$|\binput\$|\bget\$|^\s*input\b|\bsleep\b|\bconsole\.(input|inkey)", re.I | re.M)
# (output that depends on random numbers or the clock: not compared)
VARIES = re.compile(r"\b(rnd|randomize|timer|time\$|date\$|tickcount|gettickcount)\b", re.I)
LANES = ("portable", "dll", "directx", "ole", "hardware")


def name_of(rel):
    return re.sub(r"[^A-Za-z0-9]+", "_", os.path.splitext(rel)[0]).strip("_")


def normalise(msg):
    msg = re.sub(r"\(at [^)]*\)", "", msg)
    msg = re.sub(r"\bline \d+(:\d+)?", "line N", msg)
    msg = re.sub(r"'[^']*'", "'X'", msg)
    msg = re.sub(r'"[^"]*"', '"X"', msg)
    msg = re.sub(r"`[^`]*`", "`X`", msg)
    msg = re.sub(r"/[^\s:]+", "<path>", msg)
    msg = re.sub(r"\b0x[0-9a-f]+\b", "N", msg, flags=re.I)
    msg = re.sub(r"\b\d+(\.\d+)?\b", "N", msg)
    return msg.strip()[:160]


def bmp_info(path):
    """(width, height, distinct colours up to 3) of a 24-bit bottom-up BMP
    (codec::encode_bmp's)."""
    try:
        b = open(path, "rb").read()
        off, w, h, bpp = struct.unpack_from("<I", b, 10)[0], *struct.unpack_from("<ii", b, 18), struct.unpack_from("<H", b, 28)[0]
        step = bpp // 8
        colours = set()
        for i in range(off, len(b) - step + 1, step * 7):
            colours.add(b[i:i + 3])
            if len(colours) > 2:
                break
        return w, abs(h), len(colours)
    except Exception:
        return 0, 0, 0


def png_colours(path):
    """How many colours (up to 2) a PNG has — RC.EXE's window captures
    (tools/corpus_rc_shots.sh: 8-bit RGB or RGBA, not interlaced)."""
    import zlib
    try:
        b = open(path, "rb").read()
        w, h, depth, ctype = struct.unpack(">IIBB", b[16:26])
        if depth != 8 or ctype not in (2, 6):
            return 0
        data, i = b"", 8
        while i < len(b):
            n, kind = struct.unpack(">I4s", b[i:i + 8])
            if kind == b"IDAT":
                data += b[i + 8:i + 8 + n]
            i += 12 + n
        raw = zlib.decompress(data)
        bpp = 3 if ctype == 2 else 4
        stride = w * bpp
        prev = bytearray(stride)
        colours = set()
        for y in range(h):
            f, line = raw[y * (stride + 1)], bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
            for x in range(stride):
                a = line[x - bpp] if x >= bpp else 0
                up = prev[x]
                c = prev[x - bpp] if x >= bpp else 0
                if f == 1:
                    line[x] = (line[x] + a) & 255
                elif f == 2:
                    line[x] = (line[x] + up) & 255
                elif f == 3:
                    line[x] = (line[x] + (a + up) // 2) & 255
                elif f == 4:
                    pa, pb, pc = abs(up - c), abs(a - c), abs(a + up - 2 * c)
                    line[x] = (line[x] + (a if pa <= pb and pa <= pc else up if pb <= pc else c)) & 255
            for x in range(0, stride, bpp * 5):
                colours.add(bytes(line[x:x + 3]))
            if len(colours) > 1:
                return 2
            prev = line
        return len(colours)
    except Exception:
        return 0


def rc_windows(name):
    """RC.EXE's captures of program `name` (tools/corpus_rc_shots.sh), or
    None when it wasn't run there."""
    d = os.path.join(WORK, "rc-shots")
    listed = os.path.join(d, "programs.txt")
    if not os.path.exists(listed) or name not in open(listed).read().split():
        return None
    return sorted(os.path.join(d, f) for f in os.listdir(d) if f.startswith(name + "-") and f.endswith(".png"))


def run(cmd, cwd, env, timeout, stdin=None):
    try:
        fin = open(stdin, "rb") if stdin else subprocess.DEVNULL
        try:
            p = subprocess.run(cmd, cwd=cwd, env=env, stdin=fin, capture_output=True, timeout=timeout)
        finally:
            if stdin:
                fin.close()
        return p.returncode, p.stdout.decode("utf-8", "replace"), p.stderr.decode("utf-8", "replace"), False
    except subprocess.TimeoutExpired as e:
        return None, (e.stdout or b"").decode("utf-8", "replace"), (e.stderr or b"").decode("utf-8", "replace"), True


# Keys a console program gets in a terminal (--pty): Enter, a number and
# Enter, then the usual ways out (q, Escape, n, x), one every 0.8 s.
PTY_KEYS = [b"\r", b"1\r", b"q", b"\x1b", b"n\r", b"x", b"\r", b"0\r", b"\r"]


def run_pty(cmd, cwd, env, timeout):
    """`cmd` in a pseudo-terminal, typed PTY_KEYS: (exit status or None,
    what the terminal showed)."""
    import pty
    import select
    import signal
    import time
    pid, fd = pty.fork()
    if pid == 0:
        try:
            os.chdir(cwd)
            os.execve(cmd[0], cmd, dict(env, TERM="xterm-256color"))
        finally:
            os._exit(127)
    out, status, t0, k = b"", None, time.time(), 0
    while time.time() - t0 < timeout and len(out) < 4_000_000:
        r, _, _ = select.select([fd], [], [], 0.1)
        if r:
            try:
                d = os.read(fd, 65536)
            except OSError:
                d = b""
            out += d
        if k < len(PTY_KEYS) and time.time() - t0 > 1.0 + 0.8 * k:
            try:
                os.write(fd, PTY_KEYS[k])
            except OSError:
                pass
            k += 1
        w, st = os.waitpid(pid, os.WNOHANG)
        if w:
            status = st
            break
    if status is None:
        os.kill(pid, signal.SIGKILL)
        os.waitpid(pid, 0)
    try:
        while select.select([fd], [], [], 0.1)[0]:
            d = os.read(fd, 65536)
            if not d:
                break
            out += d
    except OSError:
        pass
    os.close(fd)
    code = None if status is None else (os.waitstatus_to_exitcode(status) if hasattr(os, "waitstatus_to_exitcode") else status)
    return code, out.decode("utf-8", "replace")


def pty_problems(code, text):
    """What went wrong in a terminal run (a program still running at the
    end is fine: a game, a loop until a key RapidR wasn't given)."""
    problems = []
    lines = text.replace("\r", "").split("\n")
    for i, l in enumerate(lines):
        if "panicked at" in l:
            problems.append(("panic (terminal)", lines[i + 1].strip() if i + 1 < len(lines) else l))
        elif "run-time error" in l:
            problems.append(("run-time error (terminal)", l[l.index("run-time error") + len("run-time error:"):].strip()))
        elif "[WARN]" in l:
            problems.append(("warning (terminal)", l[l.index("[WARN]") + 6:].strip()))
        elif "[rapidr]" in l:
            problems.append(("warning (terminal)", l[l.index("[rapidr]") + 8:].strip()))
    if code is not None and code < 0:
        problems.append(("crash (terminal)", f"signal {-code}"))
    return problems


def base_env(args, work):
    env = dict(os.environ)
    env["RAPIDR_INCLUDE_PATH"] = args.include
    os.makedirs(os.path.join(WORK, "prints"), exist_ok=True)
    env["RAPIDR_PRINT_TO"] = os.environ.get("RAPIDR_PRINT_TO") or os.path.join(WORK, "prints")
    env["RAPIDR_REGISTRY"] = os.path.join(WORK, "registry.reg")
    env["RAPIDR_TEST_CLIPBOARD"] = "1"
    # (the runtime's own settings — the downloaded-file answers — kept apart)
    env["RAPIDR_CONFIG_DIR"] = os.path.join(WORK, "config")
    # (dialogs answered at once: Escape, Cancel)
    for hook in ("RAPIDR_TEST_MESSAGE_DIALOG", "RAPIDR_TEST_FILE_DIALOG", "RAPIDR_TEST_COLOR_DIALOG", "RAPIDR_TEST_FONT_DIALOG"):
        env[hook] = ""
    env["RAPIDR_CAPTURE_DELAY"] = str(args.delay)
    # (RapidQ's look, to set beside RC.EXE's windows; --theme another)
    env["RAPIDR_THEME"] = args.theme
    return env


def stage(p):
    """The program's folder copied to its own work folder; the copy's path."""
    d = os.path.join(WORK, p["name"])
    shutil.rmtree(d, ignore_errors=True)
    shutil.copytree(os.path.dirname(p["src"]), d, ignore=shutil.ignore_patterns("*.exe", "*.zip", "*.rar", "*.dll", "*.EXE", "*.DLL", "*.ZIP"))
    # (the copy is run on purpose: not a download that asks first)
    if sys.platform == "darwin":
        subprocess.run(["xattr", "-dr", "com.apple.quarantine", d], capture_output=True)
    return os.path.join(d, os.path.basename(p["src"]))


def judge(p, backend, code, out, err, timed_out, caps, golden):
    """The run's problems: [(kind, detail)]."""
    problems = []
    lines = err.splitlines()
    rt = next((l for l in lines + out.splitlines() if l.startswith("run-time error")), None)
    panic = next((l for l in lines if "panicked at" in l), None)
    if panic:
        i = lines.index(panic)
        why = lines[i + 1].strip() if i + 1 < len(lines) else panic
        problems.append(("panic", why))
    elif rt:
        problems.append(("run-time error", rt[len("run-time error:"):].strip()))
    elif timed_out:
        if p["console"] and (WAITS.search(p["text"]) or p.get("rc") == "timeout"):
            pass  # (waits for the keyboard, as RapidQ's does)
        else:
            problems.append(("hang", "no end within the time limit" + (" (a GUI program: the capture never came)" if not p["console"] else "")))
    elif code is not None and code < 0:
        problems.append(("crash", f"signal {-code}"))
    elif code not in (0, None) and not p["console"]:
        problems.append(("exit", f"exit code {code}: " + (lines[-1] if lines else "")))
    for l in lines:
        if l.startswith("[rapidr]") and "captured window" not in l and "capture" not in l:
            problems.append(("warning", l[len("[rapidr]"):].strip()))
        elif l.startswith("[WARN]"):
            problems.append(("warning", l[len("[WARN]"):].strip()))
        elif l.startswith("error") or ": error:" in l:
            problems.append(("error", l))
    if not p["console"] and not timed_out and not rt and not panic:
        if not caps:
            problems.append(("no window", "no window was showing when the capture came"))
        else:
            for c in caps:
                w, h, n = bmp_info(c)
                if n <= 1:
                    problems.append(("blank window", f"{w}x{h} all one colour"))
    if golden is not None and p["console"] and not timed_out and not rt and not panic and not VARIES.search(p["text"]):
        if truth.norm(out) != golden:
            problems.append(("wrong output", "differs from RC.EXE's"))
    return problems


def run_vm(p, args):
    staged = stage(p)
    d = os.path.dirname(staged)
    env = base_env(args, d)
    capdir = d + ".captures"
    shutil.rmtree(capdir, ignore_errors=True)
    os.makedirs(capdir)
    env["RAPIDR_CAPTURE"] = os.path.join(capdir, "vm")
    code, out, err, to = run([RAPIDR, "run", staged], d, env, args.timeout, p.get("input"))
    caps = sorted(os.path.join(capdir, f) for f in os.listdir(capdir) if f.startswith("vm-") and f.endswith(".bmp"))
    term = None
    if p["console"] and args.pty and os.name != "nt":
        env.pop("RAPIDR_CAPTURE", None)
        staged = stage(p)
        term = run_pty([RAPIDR, "run", staged], d, env, args.pty_timeout)
    return code, out, err, to, caps, term


def run_native(p, args):
    staged = stage(p)
    d = os.path.dirname(staged)
    env = base_env(args, d)
    env["CARGO_TARGET_DIR"] = args.cargo_target
    env["CARGO_INCREMENTAL"] = "0"
    stem = os.path.splitext(os.path.basename(staged))[0]
    proj = os.path.join(WORK, "native", p["name"])
    shutil.rmtree(proj, ignore_errors=True)
    # (a plain executable, not an app: it runs headless here)
    code, out, err, to = run([RAPIDR, "build", staged, proj, "--no-bundle"], d, env, 1800)
    exe = os.path.join(d, stem + EXE)
    if code != 0 or not os.path.exists(exe):
        truth.drop_build(args.cargo_target, stem)
        msg = next((l for l in (err + out).splitlines() if "error" in l.lower()), (err + out).strip()[-300:])
        return "build", msg, None
    capdir = d + ".captures"
    os.makedirs(capdir, exist_ok=True)
    env["RAPIDR_CAPTURE"] = os.path.join(capdir, "native")
    code, out, err, to = run([exe], d, env, args.timeout, p.get("input"))
    caps = sorted(os.path.join(capdir, f) for f in os.listdir(capdir) if f.startswith("native-") and f.endswith(".bmp"))
    term = None
    if p["console"] and args.pty and os.name != "nt":
        env.pop("RAPIDR_CAPTURE", None)
        term = run_pty([exe], d, env, args.pty_timeout)
    truth.drop_build(args.cargo_target, stem)
    shutil.rmtree(proj, ignore_errors=True)
    return None, None, (code, out, err, to, caps, term)


def programs(args):
    if args.corpus:
        results = json.load(open(args.corpus))["results"]
    else:
        env = dict(os.environ, RAPIDR_INCLUDE_PATH=args.include)
        results = {}
        for d, _, fs in os.walk(args.examples):
            for f in fs:
                if f.lower().endswith(corpus.PROGRAM_EXTS):
                    src = os.path.join(d, f)
                    results[os.path.relpath(src, args.examples)] = corpus.compile_one(src, env)[1]
    corpus.CORPUS_ROOT = os.path.abspath(args.examples)
    rc_json = os.path.join(truth.WORK, "corpus", "rc.json")
    rc_runs = json.load(open(rc_json)) if os.path.exists(rc_json) else {}
    progs = []
    for rel, errors in sorted(results.items()):
        if errors:
            continue
        if args.filters and not any(f.lower() in rel.lower() for f in args.filters):
            continue
        src = os.path.join(args.examples, rel)
        text = truth.program_text(src)
        p = {"rel": rel, "name": name_of(rel), "src": src, "text": text,
             "lane": corpus.category(src, []),
             "console": bool(APPTYPE_CONSOLE.search(text)) or not GUI.search(text)}
        if NETWORK.search(text):
            p["skip"] = "uses the network"
        elif PRINTING.search(text):
            p["skip"] = "prints"
        elif SHELL.search(text):
            p["skip"] = "starts other programs"
        g = os.path.join(args.golden, p["name"] + ".expected")
        p["golden"] = g if os.path.exists(g) else None
        # (how RC.EXE's build of it ran, when tools/rapidq_truth.py corpus
        # ran it: a console program that waits for input there too)
        p["rc"] = rc_runs.get(p["name"], {}).get("status")
        i = os.path.join(args.golden, p["name"] + ".input")
        p["input"] = i if os.path.exists(i) else None
        if args.kind and ("console" if p["console"] else "gui") != args.kind:
            continue
        progs.append(p)
    return progs


def sweep_one(p, args):
    entry = {"program": p["rel"], "lane": p["lane"], "kind": "console" if p["console"] else "gui"}
    if p.get("skip"):
        entry["skipped"] = p["skip"]
        return entry
    golden = truth.norm(open(p["golden"], encoding="utf-8").read()) if p["golden"] else None
    for backend in args.backends:
        if backend == "vm":
            code, out, err, to, caps, term = run_vm(p, args)
        else:
            kind, msg, res = run_native(p, args)
            if kind:
                entry[backend] = {"problems": [("native build fails", msg)]}
                continue
            code, out, err, to, caps, term = res
        problems = judge(p, backend, code, out, err, to, caps, golden)
        if term:
            seen = {(k, d) for k, d in problems}
            problems += [x for x in dict.fromkeys(pty_problems(*term)) if (x[0].replace(" (terminal)", ""), x[1]) not in seen]
        entry[backend] = {"problems": problems, "windows": len(caps), "captures": caps, "exit": code, "timed_out": to,
                          "stdout": out[-600:], "stderr": err[-600:], "terminal": term[1][-1500:] if term else None}
    return entry


def summary(report, backends):
    """{backend: {"ran": n, "ok": n, "by cause": Counter, "programs": {cause: [rel]}}} for each lane group."""
    out = {}
    for b in backends:
        for group in ("this lane", "other lanes"):
            s = {"ran": 0, "ok": 0, "causes": collections.Counter(), "programs": collections.defaultdict(list), "kinds": collections.Counter()}
            for e in report:
                if e.get("skipped") or b not in e:
                    continue
                if (e["lane"] == "portable") != (group == "this lane"):
                    continue
                s["ran"] += 1
                probs = e[b]["problems"]
                if not probs:
                    s["ok"] += 1
                    continue
                kinds = sorted({k for k, _ in probs})
                for k in kinds:
                    s["kinds"][k] += 1
                for key in sorted({f"{k}: {normalise(d)}" for k, d in probs}):
                    s["causes"][key] += 1
                    s["programs"][key].append(e["program"])
            out[(b, group)] = s
    return out


def write_report(path, report, backends, label, before, notes=None):
    now = datetime.date.today().isoformat()
    S = summary(report, backends)
    B = summary(before, backends) if before else None
    L = []
    L.append("# RapidQ's examples at run time")
    L.append("")
    L.append(f"*Generated by `tools/corpus_run.py` ({now}{', ' + label if label else ''}). Do not edit by hand: run it again.*")
    L.append("")
    L.append("Every program of RapidQ's example corpus (`~/Downloads/Rapidq/examples`, 386 programs, not in this repository) that RapidR compiles is run on the interpreter (`rapidr run`) and as a native build (`rapidr build`), with no input and a time limit; a GUI program under the GUI tests' headless host, whose windows are captured and closed. A run is **ok** when it has none of the problems below. Console programs are also compared with what RC.EXE's build of them prints (`tools/rapidq_truth.py corpus --write-golden`; the outputs stay on this machine).")
    L.append("")
    L.append("Problems: *run-time error* (RapidR stopped the program), *panic*, *crash* (a signal), *hang* (no end within the limit, for a program that doesn't wait for the keyboard), *warning* (RapidR's `[rapidr] …` messages), *wrong output* (a console program printing other than RC.EXE's build), *no window* / *blank window* (a GUI program showing nothing, or windows of one colour), *native build fails*.")
    L.append("")
    skipped = [e for e in report if e.get("skipped")]
    L.append("## Counts")
    L.append("")
    L.append("| Programs | Backend | Ran | ok | " + ("before ok | " if B else "") + "Problems |")
    L.append("|---|---|---:|---:|" + ("---:|" if B else "") + "---|")
    for (b, group), s in S.items():
        kinds = ", ".join(f"{k} {n}" for k, n in s["kinds"].most_common())
        prev = ""
        if B:
            bs = B.get((b, group))
            prev = f"{bs['ok']}/{bs['ran']} | " if bs else "— | "
        L.append(f"| {'portable (this lane)' if group == 'this lane' else 'DirectX / OLE / DLL / hardware'} | {b} | {s['ran']} | {s['ok']} | {prev}{kinds or '—'} |")
    L.append("")
    L.append(f"Not run: {len(skipped)} (" + ", ".join(f"{why} {n}" for why, n in collections.Counter(e['skipped'] for e in skipped).most_common()) + ").")
    L.append("")
    for b in backends:
        s = S[(b, "this lane")]
        L.append(f"## Causes, {b} (portable programs)")
        L.append("")
        if not s["causes"]:
            L.append("None.")
            L.append("")
            continue
        for cause, n in s["causes"].most_common():
            L.append(f"- **{n}** — {cause}: " + ", ".join(f"`{r}`" for r in s["programs"][cause][:12]) + (" …" if n > 12 else ""))
        L.append("")
    if notes:
        L.append(open(notes).read().rstrip())
        L.append("")
    L.append("## Other lanes (not fixed here)")
    L.append("")
    L.append("Programs that need DirectX, OLE, Windows DLL calls or hardware access (INP / OUT) belong to other lanes; how they run now:")
    L.append("")
    for lane in LANES[1:]:
        es = [e for e in report if e["lane"] == lane and not e.get("skipped")]
        if not es:
            continue
        L.append(f"### {lane} ({len(es)})")
        L.append("")
        for e in es:
            res = []
            for b in backends:
                if b in e:
                    ps = e[b]["problems"]
                    res.append(f"{b}: " + ("ok" if not ps else "; ".join(sorted({f'{k}: {normalise(d)}' for k, d in ps}))[:200]))
            L.append(f"- `{e['program']}` — " + " · ".join(res))
        L.append("")
    L.append("## Not run")
    L.append("")
    for e in skipped:
        L.append(f"- `{e['program']}` ({e['skipped']})")
    L.append("")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write("\n".join(L))


def merge(paths):
    """Sweeps (--json files) put together, program by program."""
    by = {}
    for path in paths:
        for e in json.load(open(path)):
            by.setdefault(e["program"], {}).update(e)
    # (a console program RC.EXE's build ran into the time limit too waits
    # for input: not a hang — sweeps made before RC.EXE's runs were known)
    rc_json = os.path.join(truth.WORK, "corpus", "rc.json")
    rc_runs = json.load(open(rc_json)) if os.path.exists(rc_json) else {}
    for e in by.values():
        # (RC.EXE's build of it shows no window, or only windows of one
        # colour, too: not a problem of RapidR's)
        rc = rc_windows(name_of(e["program"]))
        if rc is not None:
            e["rc_windows"] = len(rc)
            for b in ("vm", "native"):
                if b in e and not rc:
                    e[b]["problems"] = [x for x in e[b]["problems"] if x[0] != "no window"]
                if b in e and rc and all(png_colours(f) == 1 for f in rc):
                    e[b]["problems"] = [x for x in e[b]["problems"] if x[0] != "blank window"]
        varies = VARIES.search(truth.program_text(os.path.join(RAPIDQ, "examples", e["program"])))
        for b in ("vm", "native"):
            if varies and b in e:
                e[b]["problems"] = [x for x in e[b]["problems"] if x[0] != "wrong output"]
        if e.get("kind") == "console" and rc_runs.get(name_of(e["program"]), {}).get("status") == "timeout":
            for b in ("vm", "native"):
                if b in e:
                    e[b]["problems"] = [x for x in e[b]["problems"] if x[0] != "hang"]
    return [by[k] for k in sorted(by)]


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("filters", nargs="*")
    ap.add_argument("--backend", default="vm,native")
    ap.add_argument("--jobs", type=int, default=2)
    ap.add_argument("--corpus")
    ap.add_argument("--examples", default=os.path.join(RAPIDQ, "examples"))
    ap.add_argument("--include", default=os.path.join(RAPIDQ, "include"))
    ap.add_argument("--golden", default=truth.GOLDEN)
    ap.add_argument("--timeout", type=int, default=30)
    ap.add_argument("--delay", type=float, default=2.0)
    ap.add_argument("--theme", default="classic", help="the look programs run in (RAPIDR_THEME; classic: RapidQ's, as RC.EXE's windows)")
    ap.add_argument("--no-pty", dest="pty", action="store_false", help="console programs only with no input, not in a terminal too")
    ap.add_argument("--pty-timeout", type=float, default=9.0)
    ap.add_argument("--cargo-target", default=os.path.join(ROOT, "tests", "conformance", ".work", "cargo-target"))
    ap.add_argument("--json", default=os.path.join(WORK, "report.json"))
    ap.add_argument("--report")
    ap.add_argument("--label", default="")
    ap.add_argument("--before")
    ap.add_argument("--notes", default=os.path.join(ROOT, "tools", "corpus_run_notes.md"), help="a Markdown section the report carries (what was fixed, what stays)")
    ap.add_argument("--kind", choices=["console", "gui"], help="only console or only GUI programs")
    ap.add_argument("--merge", nargs="+", metavar="JSON", help="no runs: the report from these sweeps (e.g. one per backend) put together")
    args = ap.parse_args()
    args.backends = [b.strip() for b in args.backend.split(",") if b.strip()]
    if args.merge:
        report = merge(args.merge)
        before = merge(args.before.split(",")) if args.before else None
        write_report(args.report, report, args.backends, args.label, before, args.notes if os.path.exists(args.notes) else None)
        print(f"report: {args.report}")
        return
    if not os.path.exists(RAPIDR):
        sys.exit(f"{RAPIDR} not found: build it first")
    os.makedirs(WORK, exist_ok=True)
    progs = programs(args)
    print(f"{len(progs)} programs compile; running on {', '.join(args.backends)}", flush=True)
    report = []
    # (native builds share one cargo target: one at a time; the interpreter's in parallel)
    jobs = 1 if "native" in args.backends else args.jobs
    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
        for e in pool.map(lambda p: sweep_one(p, args), progs):
            report.append(e)
            if e.get("skipped"):
                print(f"- {e['program']}: not run ({e['skipped']})", flush=True)
                continue
            parts = []
            for b in args.backends:
                ps = e[b]["problems"]
                parts.append(f"{b}: " + ("ok" if not ps else "; ".join(f"{k}: {d}"[:140] for k, d in ps[:3])))
            bad = any(e[b]["problems"] for b in args.backends)
            print(f"{'✗' if bad else '✓'} [{e['lane']}/{e['kind']}] {e['program']} — " + " | ".join(parts), flush=True)
    with open(args.json, "w") as f:
        json.dump(report, f, indent=1)
    S = summary(report, args.backends)
    for (b, group), s in S.items():
        print(f"{b} {group}: ok {s['ok']}/{s['ran']}  " + ", ".join(f"{k} {n}" for k, n in s["kinds"].most_common()))
    if args.report:
        before = merge(args.before.split(",")) if args.before else None
        write_report(args.report, report, args.backends, args.label, before, args.notes if os.path.exists(args.notes) else None)
        print(f"report: {args.report}")


if __name__ == "__main__":
    main()
