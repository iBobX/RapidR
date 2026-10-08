#!/usr/bin/env python3
"""Builds and runs the corpus programs that call DLLs (tools/dll_corpus_list.py),
the same way on the Mac and in the Windows VM (docs/windows-dll-calls.md §6).

    python3 tools/dll_breadth.py --rapidr R --corpus EXAMPLES --include INC --list LIST
                      --out OUT [--native] [--target CARGO_TARGET] [--filter S]

Each program is compiled for the interpreter (`build-bc`) and run
(`run-bc`); with --native also built natively (`rapidr build`) and run.
GUI programs run under the GUI test hook (RAPIDR_CAPTURE: a capture after
a few seconds, then exit; on Windows with real windows, so Form.Handle is a
real HWND). Programs that could shut the machine down, print, touch the
registry, inject input or delete files are only compiled (the reason is
recorded). Results go to OUT/results.jsonl, one line per program, as they
come.
"""
import argparse, json, os, re, shutil, subprocess, sys, time

ap = argparse.ArgumentParser()
ap.add_argument("--rapidr", required=True)
ap.add_argument("--corpus", required=True)
ap.add_argument("--include", required=True)
ap.add_argument("--list", required=True)
ap.add_argument("--out", required=True)
ap.add_argument("--native", action="store_true")
ap.add_argument("--target")
ap.add_argument("--filter")
ap.add_argument("--delay", default="3")
ap.add_argument("--timeout", type=int, default=25)
ap.add_argument("--classify", action="store_true")
args = ap.parse_args()

WIN = os.name == "nt"
EXE = ".exe" if WIN else ""
out = os.path.abspath(args.out)
os.makedirs(os.path.join(out, "caps"), exist_ok=True)
os.makedirs(os.path.join(out, "prints"), exist_ok=True)
work = os.path.join(out, "work")
if not os.path.isdir(work):
    shutil.copytree(args.corpus, work)

INC = re.compile(r'^\s*\$INCLUDE\s+["<]([^">]+)[">]', re.I | re.M)


def ci_find(d, rel):
    p = d
    for part in rel.replace("\\", "/").split("/"):
        if part in ("", "."):
            continue
        if part == "..":
            p = os.path.dirname(p)
            continue
        try:
            names = {n.lower(): n for n in os.listdir(p)}
        except OSError:
            return None
        n = names.get(part.lower())
        if n is None:
            return None
        p = os.path.join(p, n)
    return p if os.path.isfile(p) else None


def all_text(path, seen=None, own=False):
    """The program's text with its includes'; `own`: only its own files
    (not RapidQ's shared include folder)."""
    seen = seen if seen is not None else set()
    if path in seen:
        return ""
    seen.add(path)
    try:
        src = open(path, encoding="latin-1").read()
    except OSError:
        return ""
    texts = [src]
    for inc in INC.findall(src):
        f = ci_find(os.path.dirname(path), inc)
        if not f and not own:
            f = ci_find(args.include, inc)
        if f:
            texts.append(all_text(f, seen, own))
    return "\n".join(texts)


# What a run could do to the machine: compiled only.
UNSAFE = [
    (r"ExitWindows|InitiateSystemShutdown|SetSuspendState|LockWorkStation|SetSystemPowerState", "shuts down / locks / suspends the machine"),
    (r"\bLPRINT\b|QPRINTER|\bPrinter\.|PrintDlg|StartDoc|OpenPrinter|\bPRINTERS?\b\s*\(", "prints"),
    (r"QREGISTRY|Reg(Open|Create|Set|Delete)\w*Key|RegSetValue|RegDeleteValue", "writes the registry"),
    (r"keybd_event|SendInput|SetWindowsHookEx|BlockInput|SendKeys", "injects or hooks input"),
    (r"SHFileOperation|DeleteFile|RemoveDirectory|\bKILL\s|MoveFile", "deletes or moves files"),
    (r"WinExec|ShellExecute|CreateProcess|\bSHELL\b|RUN\s+\"", "starts other programs"),
    (r"SystemParametersInfo|ChangeDisplaySettings|SetSystemTime|SetLocalTime|SetComputerName", "changes system settings"),
    (r"URLDownloadToFile|InternetOpen|QSOCKET|WSAStartup|QDOWNLOAD|QSERVERSOCKET|\bsocket\b", None),
    (r"AddFontResource|RemoveFontResource", "installs fonts"),
    (r"QFILEDIALOG|QOPENDIALOG|QSAVEDIALOG", None),
]


def code_only(text):
    """The text without comments, DECLARE statements (with their `_`
    continuation lines) and CONST lines: a declaration in an include isn't
    a call."""
    out, cont = [], False
    for line in text.splitlines():
        s = line.strip()
        if cont:
            cont = s.endswith("_")
            continue
        low = s.lower()
        if low.startswith("'") or low.startswith("rem ") or low.startswith("const ") or low.startswith("$"):
            continue
        if low.startswith("declare ") or low.startswith("declare\t"):
            cont = s.endswith("_")
            continue
        out.append(line.split("'")[0] if '"' not in line else line)
    return "\n".join(out)


# What a program's own code calls to shut the machine down through
# RapidQ's shared include (rapidq2.inc wraps ExitWindowsEx and
# SetSuspendState in methods; defining them is no call).
OWN_SHUTDOWN = r"ExitWindows|InitiateSystemShutdown|SetSuspendState|LockWorkStation|SetSystemPowerState|DoShutDown|PowerStatus|sysHibernate|sysSleep"


def unsafe_reason(path):
    text = code_only(all_text(path))
    own = code_only(all_text(path, own=True))
    if re.search(OWN_SHUTDOWN, own, re.I):
        return "shuts down / locks / suspends the machine"
    for pat, why in UNSAFE[1:]:
        # (printing and the registry: anywhere, the shared includes'
        # wrappers too — the VM has the Mac's printer and its own registry;
        # the rest only when the program's own code calls it)
        where = text if why in ("prints", "writes the registry") else own
        if why and re.search(pat, where, re.I):
            return why
    return None


def env_for(cap):
    e = dict(os.environ)
    e["RAPIDR_INCLUDE_PATH"] = os.path.abspath(args.include)
    e["RAPIDR_PRINT_TO"] = os.path.join(out, "prints")
    e["RAPIDR_REGISTRY"] = os.path.join(out, "registry.reg")
    e["RAPIDR_TEST_CLIPBOARD"] = "1"
    e["RAPIDR_CAPTURE"] = cap
    e["RAPIDR_CAPTURE_DELAY"] = args.delay
    # (message boxes answered at once: Escape)
    e["RAPIDR_TEST_MESSAGE_DIALOG"] = ";;;;;;;;;;;;;;;;;;;;"
    e["RAPIDR_TEST_FILE_DIALOG"] = ";;;;;;;;"
    if WIN:
        e["RAPIDR_CAPTURE_WINDOWS"] = "1"
    if args.target:
        e["CARGO_TARGET_DIR"] = os.path.abspath(args.target)
    return e


def run(cmd, cwd, env, timeout):
    t0 = time.time()
    try:
        p = subprocess.run(cmd, cwd=cwd, env=env, stdin=subprocess.DEVNULL, capture_output=True, timeout=timeout)
        return {"code": p.returncode, "out": p.stdout.decode("latin-1")[-4000:], "err": p.stderr.decode("latin-1")[-4000:], "secs": round(time.time() - t0, 1)}
    except subprocess.TimeoutExpired as e:
        if WIN:
            subprocess.run(["taskkill", "/F", "/T", "/IM", os.path.basename(cmd[0])], capture_output=True)
        return {"code": None, "out": (e.stdout or b"").decode("latin-1")[-4000:], "err": (e.stderr or b"").decode("latin-1")[-4000:], "secs": timeout, "timeout": True}


ERR = re.compile(r"^(?:.*?:\d+:\d+: )?error: (.*)$", re.M)


def outcome(r, built=True):
    """ok / runtime-error / crash / timeout, and the message."""
    text = (r.get("err") or "") + "\n" + (r.get("out") or "")
    m = re.search(r"run-time error[^:\n]*: (.*)", text)
    if m:
        return "runtime-error", m.group(1).strip()[:300]
    if r.get("timeout"):
        return "timeout", ""
    if r["code"] not in (0, None):
        last = [l for l in text.splitlines() if l.strip()]
        return "crash", (f"exit {r['code']}: " + (last[-1] if last else ""))[:300]
    return "ok", ""


def crate_name(stem):
    n = re.sub(r"[^A-Za-z0-9_-]", "_", stem)
    return f"rq_{n}" if n == "" or re.match(r"^[0-9-]", n) else n


def drop_build(target, stem):
    crate = crate_name(stem)
    d = os.path.join(target, "debug")
    for f in (crate, crate + ".exe", crate + ".pdb", crate + ".d"):
        p = os.path.join(d, f)
        if os.path.isdir(p):
            shutil.rmtree(p, ignore_errors=True)
        elif os.path.exists(p):
            os.remove(p)
    prefixes = (crate.replace("-", "_") + "-", crate + "-")
    for sub in ("deps", "incremental", ".fingerprint"):
        sd = os.path.join(d, sub)
        if not os.path.isdir(sd):
            continue
        for f in os.listdir(sd):
            if f.startswith(prefixes):
                p = os.path.join(sd, f)
                shutil.rmtree(p, ignore_errors=True) if os.path.isdir(p) else os.remove(p)


progs = [l.strip() for l in open(args.list, encoding="utf-8") if l.strip()]
if args.classify:
    for rel in progs:
        print((unsafe_reason(os.path.join(args.corpus, rel)) or "-")[:30].ljust(31), rel)
    sys.exit(0)
if args.filter:
    progs = [p for p in progs if args.filter.lower() in p.lower()]
done = set()
res_path = os.path.join(out, "results.jsonl")
if os.path.exists(res_path):
    for l in open(res_path, encoding="utf-8"):
        try:
            done.add(json.loads(l)["prog"])
        except Exception:
            pass
rapidr = os.path.abspath(args.rapidr)
for i, rel in enumerate(progs):
    if rel in done:
        continue
    src = os.path.join(work, rel)
    d, f = os.path.split(src)
    stem = os.path.splitext(f)[0]
    tag = f"{i:03d}"
    rec = {"prog": rel}
    why = unsafe_reason(src)
    rec["compile_only"] = why
    rrbc = os.path.join(out, "work", f"__{tag}.rrbc")
    c = run([rapidr, "build-bc", f, "-o", rrbc], d, env_for(""), 120)
    errs = ERR.findall(c["err"] + "\n" + c["out"])
    rec["bc_build"] = "ok" if c["code"] == 0 and os.path.exists(rrbc) else "fail"
    if rec["bc_build"] == "fail":
        rec["bc_build_error"] = (errs[0] if errs else (c["err"] or c["out"]).strip()[-300:])[:300]
    elif not why:
        r = run([rapidr, "run-bc", rrbc], d, env_for(os.path.join(out, "caps", f"{tag}-vm")), args.timeout)
        rec["bc_run"], rec["bc_msg"] = outcome(r)
        rec["bc_out"] = (r["out"] or "")[-600:]
    if args.native and rec["bc_build"] == "ok":
        nd = os.path.join(out, "native", tag)
        os.makedirs(nd, exist_ok=True)
        rr = os.path.join(d, f"__{tag}_{stem}.bas")
        shutil.copyfile(src, rr)
        nstem = f"__{tag}_{stem}"
        b = run([rapidr, "build", os.path.basename(rr), os.path.join(nd, "proj")], d, env_for(""), 1500)
        exe = None
        for cand in (os.path.join(nd, nstem + EXE), os.path.join(d, nstem + EXE), os.path.join(nd, "proj", nstem + EXE)):
            if os.path.exists(cand):
                exe = cand
                break
        if not exe:
            for root_, _, fs in os.walk(nd):
                for g in fs:
                    if g.lower().endswith(EXE or "") and g.startswith(crate_name(nstem)) and os.access(os.path.join(root_, g), os.X_OK):
                        exe = os.path.join(root_, g)
        rec["native_build"] = "ok" if (b["code"] == 0 and exe) else "fail"
        if rec["native_build"] == "fail":
            txt = b["err"] + "\n" + b["out"]
            m = re.search(r"^error(\[E\d+\])?: .*$", txt, re.M)
            rec["native_build_error"] = (m.group(0) if m else txt.strip()[-300:])[:300]
        elif not why:
            r = run([exe], d, env_for(os.path.join(out, "caps", f"{tag}-native")), args.timeout)
            rec["native_run"], rec["native_msg"] = outcome(r)
        if args.target:
            drop_build(os.path.abspath(args.target), nstem)
        shutil.rmtree(nd, ignore_errors=True)
        try:
            os.remove(rr)
        except OSError:
            pass
    with open(res_path, "a", encoding="utf-8") as fh:
        fh.write(json.dumps(rec) + "\n")
    print(tag, rel, rec.get("bc_build"), rec.get("bc_run"), rec.get("native_build"), rec.get("native_run"), (rec.get("bc_msg") or rec.get("bc_build_error") or "")[:120], flush=True)
