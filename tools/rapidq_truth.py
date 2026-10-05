#!/usr/bin/env python3
"""RapidQ's own compiler as the ground truth (docs/rapidq-ground-truth.md):
programs compiled by RC.EXE in the Windows VM and run there
(tools/rc_probe.sh), then run by RapidR — the bytecode VM, and with
--native the native build too — and the outputs compared.

Usage (repo root, after building ./rapidr):
    tools/rapidq_truth.py probes <dir> [--native] [--cached] [--write-expected]
    tools/rapidq_truth.py conformance [--native] [--cached] [--write-expected] [filter ...]
    tools/rapidq_truth.py corpus [--native] [--cached] [--write-golden] [filter ...]
    tools/rapidq_truth.py golden [--native] [filter ...]

probes       every .bas in <dir> (one-off questions; <dir> under your home,
             e.g. tests/conformance/.work/…); a <name>.expected next to one
             is compared too.
conformance  the console cases of tests/conformance/cases (those with an
             .expected): RapidQ's output against the .expected file and
             against RapidR's.
corpus       the console programs of RapidQ's example corpus
             (RAPIDQ_DIR/examples, default ~/Downloads/Rapidq) that only use
             what RapidR runs (no DLL calls, no network, no printing, no
             registry, no SHELL): RapidQ's output against RapidR's.
             --write-golden saves RapidQ's output of each program that ran
             to tests/rapidq_golden/<name>.expected.
golden       no VM: RapidR's output of the corpus programs against the
             saved tests/rapidq_golden/*.expected (RapidQ's own output).

--write-expected (conformance, probes) writes RapidQ's output to the
.expected of each case it ran to the end (a new case without one included),
to pin RapidQ's behaviour; filter to the cases meant.

--cached reuses the last RC run of the set (its rc.json) instead of the VM.
Programs are staged (CRLF, `$APPTYPE CONSOLE` added when the program has
no $APPTYPE) in tests/conformance/.work/rqtruth/<set>/ and RapidR runs the
same staged files, in their own folder. Runs in the VM use
%USERPROFILE%\\rq\\<RQ_SUB> (default gt). RapidR's runs print to
RAPIDR_PRINT_TO and keep QREGISTRY in a file of their own, never the real
ones; RC's never run a program that prints or uses the registry (both
filtered here, and again by rc_probe.ps1).
"""

import argparse
import base64
import json
import os
import re
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXE = ".exe" if os.name == "nt" else ""
RAPIDR = os.path.join(ROOT, "rapidr" + EXE)
WORK = os.path.join(ROOT, "tests", "conformance", ".work", "rqtruth")
CASES = os.path.join(ROOT, "tests", "conformance", "cases")
GOLDEN = os.path.join(ROOT, "tests", "rapidq_golden")
RAPIDQ = os.path.expanduser(os.environ.get("RAPIDQ_DIR", "~/Downloads/Rapidq"))
SUB = os.environ.get("RQ_SUB", "gt")

# What a program must not do when RapidQ runs it in the VM (the VM shares
# the Mac's real printer; its registry is real), or what RapidR can't run
# alike (Windows DLLs, other machines, other programs).
PRINTING = re.compile(r'\blprint\b|\blflush\b|printer|printdialog|"\s*(lpt\d?|prn)\s*:?\s*"', re.I)
REGISTRY = re.compile(r"qregistry|advapi32|\bReg(Open|Create|Set|Delete|Query)\w*", re.I)
NETWORK = re.compile(r"qsocket|qserversocket|winsock|wsock32|ws2_32|wininet|urlmon|URLDownload|qhttp|rhttp|qftp|gethostby|qmysql|rmysql", re.I)
SHELL = re.compile(r"^\s*(shell|run|execute)\b|\bshell\s*\(|\bshellexecute|\bwinexec|\bcreateprocess", re.I | re.M)
# (`DECLARE FUNCTION f _` / `LIB "x.dll"` continued on the next line too)
DLL = re.compile(r"^\s*DECLARE\s+(SUB|FUNCTION)\s+\S+(\s|_\s*\r?\n)+LIB\b", re.I | re.M)
OLE_DX = re.compile(r"\bAS\s+Q(OLE|DX|D3D)\w*", re.I)
GUI = re.compile(r"\bQ(FORM|FORMEX|FORMMDI|DOCKFORM)\b", re.I)
APPTYPE = re.compile(r"^\s*\$APPTYPE\b", re.I | re.M)
APPTYPE_CONSOLE = re.compile(r"^\s*\$APPTYPE\s+CONSOLE\b", re.I | re.M)
INCLUDE = re.compile(r'^\s*\$INCLUDE\s+["<]([^">]+)[">]', re.I | re.M)
# (statements that wait for a key or a line from a real console)
WAITS = re.compile(r"\binkey\$|\binput\$|\bget\$|\bsleep\b|\bdoevents\b", re.I)


def read(path):
    with open(path, "rb") as f:
        return f.read().decode("latin-1")


def code(text):
    """BASIC source without its comments (`'` to the end of the line, REM),
    so the filters see what a program does, not what it says."""
    out = []
    for line in text.split("\n"):
        if re.match(r"\s*rem\b", line, re.I):
            continue
        in_str = False
        for i, c in enumerate(line):
            if c == '"':
                in_str = not in_str
            elif c == "'" and not in_str:
                line = line[:i]
                break
        out.append(line)
    return "\n".join(out)


def program_text(path):
    """A program's code and the includes it names (its own next to it,
    then RapidQ's), comments left out, so the filters see everything it
    runs."""
    src = read(path)
    texts = [src]
    for inc in INCLUDE.findall(src):
        inc = inc.replace("\\", "/")
        for d in (os.path.dirname(path), os.path.join(RAPIDQ, "include")):
            p = os.path.join(d, inc)
            if os.path.isfile(p):
                texts.append(read(p))
                break
    return code("\n".join(texts))


def unsafe(text):
    """Why RapidQ mustn't run a program in the VM, or None."""
    if PRINTING.search(text):
        return "printing"
    if REGISTRY.search(text):
        return "registry"
    if NETWORK.search(text):
        return "network"
    if SHELL.search(text):
        return "runs other programs"
    return None


ROUTINE_START = re.compile(r"^\s*(SUB|FUNCTION|SUBI|FUNCTIONI)\s+\w", re.I)
# ("Press any key": a wait for a key that never comes from a pipe)
ANY_KEY = re.compile(r'^\s*DO\s*:\s*LOOP\s+UNTIL\s+INKEY\$\s*<>\s*""\s*$', re.I)
ROUTINE_END = re.compile(r"^\s*END\s+(SUB|FUNCTION|SUBI|FUNCTIONI)\b", re.I)
PLAIN_END = re.compile(r"^\s*END\s*('.*)?$", re.I)


def staged_source(text):
    """CRLF line ends (RapidQ is a DOS-era tool) and `$APPTYPE CONSOLE`
    first unless the program says what it is. The main program's END
    becomes a jump to its last line: a RapidQ program's END drops what it
    printed while it went to a file or pipe (its output buffer isn't
    flushed), falling off the end doesn't. A "press any key" loop
    (`DO: LOOP UNTIL INKEY$ <> ""`) is left out: no key comes from a pipe."""
    text = text.replace("\r\n", "\n")
    if not APPTYPE.search(text):
        text = "$APPTYPE CONSOLE\n" + text
    lines, depth, ends = [], 0, False
    for line in text.split("\n"):
        if ROUTINE_END.match(line):
            depth = max(0, depth - 1)
        elif ROUTINE_START.match(line):
            depth += 1
        elif depth == 0 and PLAIN_END.match(line):
            line = "GOTO RQTRUTH_END"
            ends = True
        elif ANY_KEY.match(line):
            line = "' (rapidq_truth: the wait for a key left out)"
        lines.append(line)
    if ends:
        lines.append("RQTRUTH_END:")
    return "\n".join(lines).replace("\n", "\r\n")


ANSI = re.compile(r"\x1b\[[0-9;?]*[A-Za-z]")


def norm(s):
    # (RapidR's ANSI sequences for CLS, COLOR, LOCATE: RapidQ's console
    # calls leave nothing in redirected output)
    s = ANSI.sub("", s)
    lines = [l.rstrip() for l in s.replace("\r\n", "\n").replace("\r", "\n").split("\n")]
    return "\n".join(lines).rstrip()


# --- the programs of each set -------------------------------------------

def conformance_programs(filters, new_cases=False):
    progs, skipped = [], []
    for f in sorted(os.listdir(CASES)):
        if not f.endswith(".bas"):
            continue
        name = f[:-4]
        if filters and not any(x in name for x in filters):
            continue
        expected = os.path.join(CASES, name + ".expected")
        # (a new case, given --write-expected: its .expected is RapidQ's output)
        if not os.path.exists(expected) and not (new_cases and not os.path.exists(os.path.join(CASES, name + ".expected-error"))):
            continue
        src = os.path.join(CASES, f)
        why = unsafe(program_text(src))
        if why:
            skipped.append((name, why))
            continue
        progs.append({"name": name, "src": src, "rel": f, "expected": expected,
                      "input": os.path.join(CASES, name + ".input")})
    return progs, skipped


def corpus_programs(filters):
    examples = os.path.join(RAPIDQ, "examples")
    progs, skipped = [], []
    for d, _, fs in sorted(os.walk(examples)):
        for f in sorted(fs):
            if not f.lower().endswith(".bas"):
                continue
            src = os.path.join(d, f)
            rel = os.path.relpath(src, examples)
            if filters and not any(x.lower() in rel.lower() for x in filters):
                continue
            text = program_text(src)
            # (console programs: they say so, or have no form at all — RapidQ
            # then decides by itself, and the .exe's subsystem tells)
            if not APPTYPE_CONSOLE.search(text) and (APPTYPE.search(read(src)) or GUI.search(text)):
                continue
            why = unsafe(text) or ("Windows DLL calls" if DLL.search(text) else None) \
                or ("OLE / DirectX" if OLE_DX.search(text) else None)
            if why:
                skipped.append((rel, why))
                continue
            name = re.sub(r"[^A-Za-z0-9]+", "_", os.path.splitext(rel)[0]).strip("_")
            progs.append({"name": name, "src": src, "rel": rel, "expected": os.path.join(GOLDEN, name + ".expected"),
                          "input": os.path.join(GOLDEN, name + ".input")})
    return progs, skipped


def probe_programs(folder):
    progs = []
    for f in sorted(os.listdir(folder)):
        if f.lower().endswith(".bas"):
            name = f[:-4]
            src = os.path.join(folder, f)
            why = unsafe(program_text(src))
            if why:
                print(f"skipped {f}: {why}")
                continue
            progs.append({"name": name, "src": src, "rel": f, "expected": os.path.join(folder, name + ".expected"),
                          "input": os.path.join(folder, name + ".input")})
    return progs


# --- staging and running --------------------------------------------------

def stage(set_name, progs, kind=None):
    """Each program in a folder of its own under WORK/<set>/p (its own
    includes and small data files beside it). Returns the staging folder."""
    base = os.path.join(WORK, set_name)
    shutil.rmtree(os.path.join(base, "p"), ignore_errors=True)
    for p in progs:
        d = os.path.join(base, "p", p["name"])
        os.makedirs(d, exist_ok=True)
        srcdir = os.path.dirname(p["src"])
        kind = kind or set_name
        if kind == "corpus":
            # (data files the program opens: what's next to it, when small)
            for f in os.listdir(srcdir):
                fp = os.path.join(srcdir, f)
                if os.path.isfile(fp) and os.path.getsize(fp) < 2_000_000 and not f.lower().endswith((".exe", ".dll")):
                    shutil.copy(fp, os.path.join(d, f))
        elif kind == "conformance":
            for folder in ("resource_files", "picture_files"):
                if os.path.isdir(os.path.join(CASES, folder)):
                    shutil.copytree(os.path.join(CASES, folder), os.path.join(d, folder))
        prog = os.path.join(d, p["name"] + ".bas")
        with open(prog, "wb") as f:
            f.write(staged_source(read(p["src"])).encode("latin-1"))
        if os.path.exists(p["input"]):
            # (CRLF: RapidQ's INPUT ends a line at CR — an LF-only file reads
            # as one long line)
            with open(p["input"], "rb") as f:
                data = f.read().replace(b"\r\n", b"\n").replace(b"\n", b"\r\n")
            with open(os.path.join(d, p["name"] + ".input"), "wb") as f:
                f.write(data)
        p["staged"] = prog
    with open(os.path.join(base, "list.txt"), "w") as f:
        f.write("".join(f"{p['name']}\\{p['name']}.bas\n" for p in progs))
    return base


def rc_run(base, progs, cached, timeout=10):
    cache = os.path.join(base, "rc.json")
    if cached and os.path.exists(cache):
        with open(cache) as f:
            return json.load(f)
    cmd = [os.path.join(ROOT, "tools", "rc_probe.sh"), "-s", SUB, "-l", os.path.join(base, "list.txt"), "-e",
           os.path.join(base, "p"), str(timeout)]
    proc = subprocess.run(cmd, capture_output=True, text=True, errors="replace")
    with open(os.path.join(base, "rc_report.txt"), "w") as f:
        f.write(proc.stdout + proc.stderr)
    results = parse_report(proc.stdout)
    if "== end" not in proc.stdout:
        print(f"warning: the RC run stopped early (see {base}/rc_report.txt; the VM keeps "
              f"%USERPROFILE%\\rq\\{SUB}.report.txt)", file=sys.stderr)
    with open(cache, "w") as f:
        json.dump(results, f, indent=1)
    return results


EXCEPTION = re.compile(r"Exception (E\w+) in module [^\r\n]* at [0-9A-F]+\.\r?\n([^\r\n]*)\r?\n?$")


def parse_report(text):
    """rc_probe.ps1 -Encode's report → {name: {status, rc, out, err}}."""
    results, cur = {}, None
    for line in text.split("\n"):
        if line.startswith("== "):
            if line == "== end":
                break
            name = line[3:].split("\\")[0].strip()
            if name.lower().endswith(".bas"):
                name = name[:-4]
            cur = results[name] = {"status": "compile error", "rc": [], "out": "", "err": []}
        elif cur is None:
            continue
        elif line.startswith("-- out64 "):
            out = base64.b64decode(line[9:].strip()).decode("cp1252", errors="replace")
            # (a run-time error: RapidQ prints the exception and its message
            # last — kept apart, compared with RapidR's run-time error)
            m = EXCEPTION.search(out)
            if m:
                cur["exception"] = [m.group(1), m.group(2).strip()]
                out = out[:m.start()]
            cur["out"] = out
        elif line.startswith("-- err "):
            cur["err"].append(line[7:])
        elif line == "-- run":
            cur["status"] = "ran"
        elif line.startswith("-- compiled"):
            cur["status"] = line[3:]
        elif line.startswith("TIMEOUT"):
            cur["status"] = "timeout"
        else:
            if line.strip() and not line.startswith("Compiling as"):
                cur["rc"].append(line)
            elif line.startswith("Compiling as"):
                cur["mode"] = line
    return results


def run_env(base):
    env = dict(os.environ)
    prints = os.path.join(base, "prints")
    os.makedirs(prints, exist_ok=True)
    env["RAPIDR_PRINT_TO"] = prints
    env["RAPIDR_REGISTRY"] = os.path.join(base, "registry.reg")
    env["RAPIDR_TEST_CLIPBOARD"] = "1"
    env.setdefault("RAPIDR_INCLUDE_PATH", os.path.join(RAPIDQ, "include"))
    return env


def run(cmd, cwd, env, stdin, timeout):
    try:
        fin = open(stdin, "rb") if stdin else subprocess.DEVNULL
        try:
            p = subprocess.run(cmd, cwd=cwd, env=env, stdin=fin, capture_output=True, timeout=timeout)
        finally:
            if stdin:
                fin.close()
        return p.returncode, p.stdout.decode("utf-8", errors="replace"), p.stderr.decode("utf-8", errors="replace")
    except subprocess.TimeoutExpired as e:
        return -1, (e.stdout or b"").decode("utf-8", errors="replace"), "timeout"


def rapidr_vm(p, base):
    d = os.path.dirname(p["staged"])
    env = run_env(base)
    rrbc = os.path.join(d, p["name"] + ".rrbc")
    if os.path.exists(rrbc):
        os.remove(rrbc)
    code, out, err = run([RAPIDR, "build-bc", p["staged"], "-o", rrbc], d, env, None, 60)
    if code != 0:
        return "compile error", (out + err).strip()
    stdin = os.path.join(d, p["name"] + ".input")
    code, out, err = run([RAPIDR, "run-bc", rrbc], d, env, stdin if os.path.exists(stdin) else None, 20)
    os.remove(rrbc)
    return ("ran" if code == 0 else f"exit {code} {err.strip()[:200]}"), out


def rapidr_native(p, base):
    d = os.path.dirname(p["staged"])
    env = run_env(base)
    target = os.path.join(ROOT, "tests", "conformance", ".work", "cargo-target")
    env["CARGO_TARGET_DIR"] = target
    code, out, err = run([RAPIDR, "build", p["staged"], os.path.join(base, "native", p["name"])], d, env, None, 900)
    exe = os.path.join(d, p["name"] + EXE)
    if code != 0 or not os.path.exists(exe):
        drop_build(target, p["name"])
        return "compile error", (out + err).strip()[-2000:]
    stdin = os.path.join(d, p["name"] + ".input")
    code, out, err = run([exe], d, env, stdin if os.path.exists(stdin) else None, 20)
    os.remove(exe)
    drop_build(target, p["name"])
    shutil.rmtree(os.path.join(base, "native", p["name"]), ignore_errors=True)
    return ("ran" if code == 0 else f"exit {code} {err.strip()[:200]}"), out


def drop_build(target, stem):
    """tests/cargo_builds.mjs dropBuild: the program's own build artifacts
    (~350 MB each) removed, the dependencies kept."""
    crate = re.sub(r"[^A-Za-z0-9_-]", "_", stem)
    if not crate or re.match(r"^[0-9-]", crate):
        crate = "rq_" + crate
    d = os.path.join(target, "debug")
    for f in (crate, crate + ".exe", crate + ".pdb", crate + ".d"):
        p = os.path.join(d, f)
        shutil.rmtree(p, ignore_errors=True) if os.path.isdir(p) else (os.path.exists(p) and os.remove(p))
    for sub in ("deps", "incremental", ".fingerprint"):
        sd = os.path.join(d, sub)
        if os.path.isdir(sd):
            for f in os.listdir(sd):
                if f.startswith(crate.replace("-", "_") + "-") or f.startswith(crate + "-"):
                    p = os.path.join(sd, f)
                    shutil.rmtree(p, ignore_errors=True) if os.path.isdir(p) else os.remove(p)


def diff(a, b, label_a, label_b):
    import difflib
    return "\n".join(difflib.unified_diff(a.split("\n"), b.split("\n"), label_a, label_b, lineterm="", n=1))


def compare(set_name, progs, rc, native, write_golden=False):
    base = os.path.join(WORK, set_name)
    tally = {}
    report = []
    for p in progs:
        r = rc.get(p["name"])
        line = f"{p['name']}"
        if r is None:
            status = "not run by RC"
        elif r["status"] != "ran":
            status = f"RC: {r['status']}" + (f" — {r['rc'][0]}" if r["rc"] else "")
        else:
            status = None
        if status:
            tally[status.split(" —")[0]] = tally.get(status.split(" —")[0], 0) + 1
            report.append(f"-- {line}: {status}")
            continue
        truth = norm(r["out"])
        if write_golden and not r.get("exception"):
            os.makedirs(os.path.dirname(p["expected"]), exist_ok=True)
            with open(p["expected"], "w", encoding="utf-8") as f:
                f.write(truth + "\n")
        notes = []
        # RapidQ stopped with an exception: RapidR must stop with the same
        # message (as its run-time error) after the same output.
        exc = r.get("exception")

        def check_run(label, status, out):
            if status == "compile error":
                notes.append(f"{label}: compile error: " + out.split("\n")[0])
                return
            if exc and (status == "ran" or exc[1].rstrip(".") not in status):
                notes.append(f"{label}: RapidQ stopped with {exc[0]} ({exc[1]}), RapidR {status}")
            elif not exc and status != "ran":
                notes.append(f"{label} {status}")
            if norm(out) != truth:
                notes.append(f"{label} differs\n" + diff(truth, norm(out), "rapidq", f"rapidr-{label}"))

        check_run("vm", *rapidr_vm(p, base))
        if native:
            check_run("native", *rapidr_native(p, base))
        if set_name not in ("corpus", "golden") and os.path.exists(p["expected"]):
            exp = norm(read(p["expected"]))
            if exp != truth:
                notes.append(".expected differs\n" + diff(truth, exp, "rapidq", ".expected"))
        key = "same" if not notes else "differs"
        tally[key] = tally.get(key, 0) + 1
        report.append(f"{'==' if notes else 'ok'} {line}" + ("".join("\n   " + n.replace("\n", "\n   ") for n in notes)))
    print("\n".join(report))
    print()
    print(", ".join(f"{k}: {v}" for k, v in sorted(tally.items())))
    with open(os.path.join(base, "report.txt"), "w") as f:
        f.write("\n".join(report) + "\n")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("set", choices=["probes", "conformance", "corpus", "golden"])
    ap.add_argument("args", nargs="*")
    ap.add_argument("--native", action="store_true")
    ap.add_argument("--cached", action="store_true")
    ap.add_argument("--write-golden", action="store_true")
    ap.add_argument("--write-expected", action="store_true")
    ap.add_argument("--timeout", type=int, default=10)
    a = ap.parse_args()
    a.write_golden = a.write_golden or a.write_expected
    if not os.path.exists(RAPIDR):
        sys.exit(f"{RAPIDR} not found: build it first (./build.sh)")

    if a.set == "probes":
        folder = os.path.abspath(a.args[0])
        set_name = "probes-" + os.path.basename(folder.rstrip("/"))
        progs = probe_programs(folder)
        skipped = []
    elif a.set == "conformance":
        set_name = "conformance"
        progs, skipped = conformance_programs(a.args, a.write_expected)
    else:
        set_name = "corpus"
        progs, skipped = corpus_programs(a.args)
    for name, why in skipped:
        print(f"skipped {name}: {why}")
    if a.set == "golden":
        progs = [p for p in progs if os.path.exists(p["expected"])]
        stage("golden", progs, "corpus")
        rc = {p["name"]: {"status": "ran", "rc": [], "out": read(p["expected"])} for p in progs}
        compare("golden", progs, rc, a.native)
        return
    base = stage(set_name, progs)
    rc = rc_run(base, progs, a.cached, a.timeout)
    compare(set_name, progs, rc, a.native, a.write_golden)


if __name__ == "__main__":
    main()
