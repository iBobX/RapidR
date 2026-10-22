#!/usr/bin/env python3
"""RapidQ's example corpus on Windows, three ways side by side: RapidQ's own
compiler (RC.EXE, the ground truth: docs/rapidq-ground-truth.md), RapidR
interpreted (`rapidr run`) and RapidR's native build (`rapidr build`), each
program run a few seconds with its windows captured
(docs/rapidq-corpus-status.md has what it found).

Runs ON Windows (tools/windows/corpus_win.sh drives it in the Parallels VM
from the Mac):

    python corpus_win.py --rapidr <an installed RapidR>\\bin\\rapidr.exe
        --examples <a copy of RapidQ's examples> --rq <a folder with RC.EXE,
        Lib\\ and include\\> --out <folder> [--no-native] [--no-rc]
        [--filter a,b] [--list FILE] [--delay 3] [--redo] [--keep-native]

Every program (.bas, .rqw, .rqb, .rq) is, in its own folder of a copy of the
examples (out\\work):

- compiled by RC.EXE (`-I include -L Lib`) and its program run: a console
  one with no input and its output kept, a GUI one until its first window
  shows and `--delay` seconds more, then every visible window of it captured
  (PrintWindow, the client area at 96 dpi) with its class, title and the
  text a message box shows (RapidQ's run-time errors are message boxes);
- run by RapidR (`rapidr run`) under the GUI tests' hook (RAPIDR_CAPTURE:
  real windows, captured after `--delay` seconds, then the program ends;
  message boxes, file / colour / font dialogs answered), in the classic
  theme (RapidQ's look, for the comparison);
- built natively (`rapidr build`) and that program run the same way.

Nothing a run can do reaches the machine: no program that prints, writes
the registry through Windows, starts other programs, injects input, moves or
deletes files, changes system settings, shuts the machine down, opens a
server socket or talks to serial ports / hardware is run — those are only
compiled, and the reason recorded. RapidR's QREGISTRY goes to a file
(RAPIDR_REGISTRY), its printing to a folder (RAPIDR_PRINT_TO). Every process
is ended by its own PID (and its children), never by name.

Results: out\\results.jsonl, one line per program as it ends (a rerun skips
the programs already there unless --redo); captures in out\\caps as PNGs at
1x (`<tag>-rc-<n>.png`, `-interp-`, `-native-`) and a side-by-side
`<tag>-cmp.png` (RC.EXE | interpreted | native). No RapidQ code is kept
anywhere but in out\\work.
"""
import argparse
import ctypes
import ctypes.wintypes as wt
import json
import os
import re
import shutil
import struct
import subprocess
import sys
import time
import zlib

ap = argparse.ArgumentParser()
ap.add_argument("--rapidr", required=True)
ap.add_argument("--examples", required=True)
ap.add_argument("--rq", required=True)
ap.add_argument("--out", required=True)
ap.add_argument("--list")
ap.add_argument("--filter")
ap.add_argument("--no-native", action="store_true")
ap.add_argument("--no-rc", action="store_true")
ap.add_argument("--no-interp", action="store_true")
ap.add_argument("--delay", type=float, default=3.0)
ap.add_argument("--timeout", type=int, default=25)
ap.add_argument("--scale", type=float, default=0, help="RapidR's capture scale (0: the screen's)")
ap.add_argument("--redo", action="store_true")
ap.add_argument("--merge", action="store_true", help="rerun only the parts asked for (--no-rc …), keeping the others' last results")
ap.add_argument("--keep-native", action="store_true")
ap.add_argument("--rc-from", help="RC.EXE's results (and captures) from another run's folder instead of running it again")
args = ap.parse_args()

EXTS = (".bas", ".rqw", ".rqb", ".rq")
out = os.path.abspath(args.out)
caps = os.path.join(out, "caps")
work = os.path.join(out, "work")
os.makedirs(caps, exist_ok=True)
if not os.path.isdir(work):
    shutil.copytree(args.examples, work)
rq = os.path.abspath(args.rq)
rcexe = os.path.join(rq, "RC.EXE")
rapidr = os.path.abspath(args.rapidr)

# ------------------------------------------------------------ safety --

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


def all_text(path, own=False, seen=None):
    """The program with its includes (own: only its own files, not RapidQ's
    shared include folder)."""
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
            f = ci_find(os.path.join(rq, "include"), inc)
        if f:
            texts.append(all_text(f, own, seen))
    return "\n".join(texts)


def code_only(text):
    """Without comments, DECLAREs (and their `_` lines) and CONSTs: a
    declaration isn't a call."""
    kept, cont = [], False
    for line in text.splitlines():
        s = line.strip()
        if cont:
            cont = s.endswith("_")
            continue
        low = s.lower()
        if low.startswith(("'", "rem ", "const ", "$")):
            continue
        if low.startswith(("declare ", "declare\t")):
            cont = s.endswith("_")
            continue
        kept.append(line.split("'")[0] if '"' not in line else line)
    return "\n".join(kept)


# (what a run could do to the machine — or to the person using it: only compiled)
UNSAFE = [
    (r"ExitWindows|InitiateSystemShutdown|SetSuspendState|LockWorkStation|SetSystemPowerState|DoShutDown|sysHibernate|sysSleep", "shuts down / locks / suspends the machine", "own"),
    (r"\bLPRINT\b|\bLFLUSH\b|QPRINTER|\bPrinter\.|PrintDlg|PrintDialog|StartDoc|OpenPrinter|\bPRINTERS?\b\s*\(|\"\s*(lpt\d?|prn)\s*:?\s*\"", "prints", "all"),
    (r"Reg(Open|Create|Set|Delete)\w*Key|RegSetValue|RegDeleteValue|advapi32", "writes the registry through Windows", "all"),
    (r"keybd_event|SendInput|SetWindowsHookEx|BlockInput|SendKeys|journal", "injects or hooks input", "own"),
    (r"SHFileOperation|DeleteFile|RemoveDirectory|\bKILL\s|MoveFile|\bRMDIR\b|\bRENAME\s", "deletes or moves files", "own"),
    (r"WinExec|ShellExecute|CreateProcess|\bSHELL\b|^\s*RUN\b|[:\s]RUN\s+\"|\bRUN\s+\w", "starts other programs", "own"),
    (r"SystemParametersInfo|ChangeDisplaySettings|SetSystemTime|SetLocalTime|SetComputerName|SetSysColors|SwapMouseButton|SetDoubleClickTime|ClipCursor|SetSystemCursor", "changes system settings", "own"),
    (r"AddFontResource|RemoveFontResource", "installs fonts", "own"),
    (r"QSERVERSOCKET|\bbind\b\s*\(|\blisten\b\s*\(|ServerSocket|\.Listen\b", "opens a server socket", "own"),
    (r"\bAS\s+[QR]?COMPORT\b|\bCOM\d:|CreateFile\w*\s*\(\s*\"\\\\\\\\\.\\\\|\"COM\d\"", "serial port", "own"),
    (r"\bINP\s*\(|^\s*OUT\s+[^=]|\bio\.dll|inpout32|\bDlPortWrite", "port I/O (hardware)", "own"),
]
QREGISTRY = re.compile(r"\bQREGISTRY\b|\bRREGISTRY\b", re.I)


def unsafe_reason(path):
    allt = code_only(all_text(path))
    own = code_only(all_text(path, own=True))
    for pat, why, where in UNSAFE:
        if re.search(pat, allt if where == "all" else own, re.I | re.M):
            return why
    return None


# ----------------------------------------------------------- windows --

user32 = ctypes.WinDLL("user32", use_last_error=True)
gdi32 = ctypes.WinDLL("gdi32")
kernel32 = ctypes.WinDLL("kernel32")
try:
    # (RapidQ's programs aren't DPI-aware: seen from an unaware thread their
    # windows are 96-dpi, which PrintWindow returns as drawn)
    user32.SetThreadDpiAwarenessContext(ctypes.c_void_p(-1))
except Exception:
    pass
EnumProc = ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)
user32.EnumWindows.argtypes = [EnumProc, wt.LPARAM]
user32.EnumChildWindows.argtypes = [wt.HWND, EnumProc, wt.LPARAM]
user32.GetWindowThreadProcessId.argtypes = [wt.HWND, ctypes.POINTER(wt.DWORD)]
user32.GetClientRect.argtypes = [wt.HWND, ctypes.POINTER(wt.RECT)]
user32.GetWindowRect.argtypes = [wt.HWND, ctypes.POINTER(wt.RECT)]
user32.IsWindowVisible.argtypes = [wt.HWND]
user32.GetClassNameW.argtypes = [wt.HWND, wt.LPWSTR, ctypes.c_int]
user32.GetWindowTextW.argtypes = [wt.HWND, wt.LPWSTR, ctypes.c_int]
user32.PrintWindow.argtypes = [wt.HWND, wt.HDC, wt.UINT]
user32.GetMenu.argtypes = [wt.HWND]
user32.GetMenu.restype = wt.HMENU
user32.GetDC.argtypes = [wt.HWND]
user32.GetDC.restype = wt.HDC
user32.ReleaseDC.argtypes = [wt.HWND, wt.HDC]
gdi32.CreateCompatibleDC.argtypes = [wt.HDC]
gdi32.CreateCompatibleDC.restype = wt.HDC
gdi32.SelectObject.argtypes = [wt.HDC, wt.HGDIOBJ]
gdi32.SelectObject.restype = wt.HGDIOBJ
gdi32.DeleteObject.argtypes = [wt.HGDIOBJ]
gdi32.DeleteDC.argtypes = [wt.HDC]
gdi32.CreateDIBSection.argtypes = [wt.HDC, ctypes.c_void_p, wt.UINT, ctypes.POINTER(ctypes.c_void_p), wt.HANDLE, wt.DWORD]
gdi32.CreateDIBSection.restype = wt.HBITMAP


def win_text(h):
    b = ctypes.create_unicode_buffer(512)
    user32.GetWindowTextW(h, b, 512)
    return b.value


def win_class(h):
    b = ctypes.create_unicode_buffer(256)
    user32.GetClassNameW(h, b, 256)
    return b.value


def windows_of(pid):
    """The process's visible top-level windows (not RapidQ's hidden 0 × 0
    TApplication), oldest first."""
    found = []

    def cb(h, _):
        p = wt.DWORD()
        user32.GetWindowThreadProcessId(h, ctypes.byref(p))
        if p.value == pid and user32.IsWindowVisible(h):
            r = wt.RECT()
            user32.GetWindowRect(h, ctypes.byref(r))
            if r.right - r.left > 2 and r.bottom - r.top > 2 and win_class(h) != "TApplication":
                found.append(h)
        return True

    user32.EnumWindows(EnumProc(cb), 0)
    found.reverse()
    return found


def child_texts(h):
    texts = []

    def cb(c, _):
        t = win_text(c)
        if t and win_class(c).lower() in ("static", "button", "tlabel", "tbutton", "directuihwnd"):
            texts.append(t)
        return True

    user32.EnumChildWindows(h, EnumProc(cb), 0)
    return texts


class BITMAPINFOHEADER(ctypes.Structure):
    _fields_ = [("biSize", wt.DWORD), ("biWidth", wt.LONG), ("biHeight", wt.LONG), ("biPlanes", wt.WORD),
                ("biBitCount", wt.WORD), ("biCompression", wt.DWORD), ("biSizeImage", wt.DWORD),
                ("biXPelsPerMeter", wt.LONG), ("biYPelsPerMeter", wt.LONG), ("biClrUsed", wt.DWORD), ("biClrImportant", wt.DWORD)]


def capture(h, aware=False):
    """(w, h, RGB rows) of the window's client area as drawn; `aware`: in
    physical pixels (RapidR's windows are per-monitor DPI-aware)."""
    if aware:
        user32.SetThreadDpiAwarenessContext(ctypes.c_void_p(-4))
    try:
        return capture_now(h)
    finally:
        if aware:
            user32.SetThreadDpiAwarenessContext(ctypes.c_void_p(-1))


def capture_now(h):
    r = wt.RECT()
    user32.GetClientRect(h, ctypes.byref(r))
    w, hh = r.right, r.bottom
    if w <= 0 or hh <= 0:
        return None
    screen = user32.GetDC(None)
    dc = gdi32.CreateCompatibleDC(screen)
    bi = BITMAPINFOHEADER()
    bi.biSize = ctypes.sizeof(BITMAPINFOHEADER)
    bi.biWidth, bi.biHeight, bi.biPlanes, bi.biBitCount = w, -hh, 1, 32
    bits = ctypes.c_void_p()
    bmp = gdi32.CreateDIBSection(dc, ctypes.byref(bi), 0, ctypes.byref(bits), None, 0)
    old = gdi32.SelectObject(dc, bmp)
    # (PW_CLIENTONLY | PW_RENDERFULLCONTENT)
    user32.PrintWindow(h, dc, 3)
    raw = ctypes.string_at(bits, w * hh * 4)
    gdi32.SelectObject(dc, old)
    gdi32.DeleteObject(bmp)
    gdi32.DeleteDC(dc)
    user32.ReleaseDC(None, screen)
    rows = []
    for y in range(hh):
        line = raw[y * w * 4:(y + 1) * w * 4]
        rgb = bytearray(w * 3)
        rgb[0::3] = line[2::4]
        rgb[1::3] = line[1::4]
        rgb[2::3] = line[0::4]
        rows.append(bytes(rgb))
    return w, hh, rows


def screen_scale():
    try:
        user32.SetThreadDpiAwarenessContext(ctypes.c_void_p(-4))
        dpi = user32.GetDpiForSystem()
        user32.SetThreadDpiAwarenessContext(ctypes.c_void_p(-1))
        return dpi / 96.0
    except Exception:
        return 1.0


# ------------------------------------------------------------ images --

def write_png(path, w, h, rows):
    raw = b"".join(b"\x00" + r for r in rows)

    def chunk(kind, data):
        c = struct.pack(">I", len(data)) + kind + data
        return c + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))


def read_bmp(path):
    """(w, h, RGB rows top-down) of a 24- or 32-bit BMP."""
    b = open(path, "rb").read()
    off = struct.unpack_from("<I", b, 10)[0]
    w, h = struct.unpack_from("<ii", b, 18)
    bpp = struct.unpack_from("<H", b, 28)[0]
    step = bpp // 8
    stride = (w * step + 3) & ~3
    rows = []
    for y in range(abs(h)):
        line = b[off + y * stride: off + y * stride + w * step]
        rgb = bytearray(w * 3)
        rgb[0::3] = line[2::step]
        rgb[1::3] = line[1::step]
        rgb[2::3] = line[0::step]
        rows.append(bytes(rgb))
    if h > 0:
        rows.reverse()
    return w, abs(h), rows


def downscale(img, k):
    """Box-filtered by an integer factor k."""
    w, h, rows = img
    if k <= 1:
        return img
    nw, nh = w // k, h // k
    out_rows = []
    for y in range(nh):
        acc = [0] * (nw * 3)
        for dy in range(k):
            r = rows[y * k + dy]
            for x in range(nw):
                for dx in range(k):
                    i = ((x * k) + dx) * 3
                    acc[x * 3] += r[i]
                    acc[x * 3 + 1] += r[i + 1]
                    acc[x * 3 + 2] += r[i + 2]
        n = k * k
        out_rows.append(bytes(v // n for v in acc))
    return nw, nh, out_rows


def difference(a, b, dy=0):
    """Mean absolute grey difference (0-255) over the overlap, and the
    share of the overlap differing by more than 48; `dy`: rows of b above
    a's first (RapidR's main menu, inside its window)."""
    w, h = min(a[0], b[0]), min(a[1], b[1] - dy)
    if w <= 0 or h <= 0:
        return None, None
    tot, big, n = 0, 0, 0
    for y in range(0, h, 2):
        ra, rb = a[2][y], b[2][y + dy]
        for x in range(0, w, 2):
            i = x * 3
            ga = (ra[i] * 3 + ra[i + 1] * 6 + ra[i + 2]) // 10
            gb = (rb[i] * 3 + rb[i + 1] * 6 + rb[i + 2]) // 10
            d = abs(ga - gb)
            tot += d
            big += d > 48
            n += 1
    return round(tot / n, 1), round(big / n, 3)


def side_by_side(path, imgs):
    imgs = [i for i in imgs if i]
    if not imgs:
        return
    gap = 6
    w = sum(i[0] for i in imgs) + gap * (len(imgs) - 1)
    h = max(i[1] for i in imgs)
    rows = []
    for y in range(h):
        parts = []
        for k, (iw, ih, ir) in enumerate(imgs):
            parts.append(ir[y] if y < ih else b"\xff\x00\xff" * iw)
            if k < len(imgs) - 1:
                parts.append(b"\x40\x40\x40" * gap)
        rows.append(b"".join(parts))
    write_png(path, w, h, rows)


# --------------------------------------------------------------- runs --

def kill_tree(pid):
    subprocess.run(["taskkill", "/PID", str(pid), "/T", "/F"], capture_output=True)


def subsystem(exe):
    try:
        b = open(exe, "rb").read(4096)
        pe = struct.unpack_from("<I", b, 0x3C)[0]
        return struct.unpack_from("<H", b, pe + 0x5C)[0]
    except Exception:
        return 0


EMPTY = os.path.join(out, "empty.txt")
open(EMPTY, "wb").close()


def tail(s, n=1500):
    return s[-n:] if s else ""


def rc_build(folder, fname, stem, tag):
    exe = os.path.join(folder, stem + ".exe")
    for f in os.listdir(folder):
        if f.lower() == (stem + ".exe").lower():
            os.remove(os.path.join(folder, f))
    with open(EMPTY, "rb") as fin:
        try:
            p = subprocess.run([rcexe, "-I" + os.path.join(rq, "include"), "-L" + os.path.join(rq, "Lib"), fname],
                               cwd=folder, stdin=fin, capture_output=True, timeout=90)
            text = (p.stdout + p.stderr).decode("latin-1")
        except subprocess.TimeoutExpired:
            return None, "RC.EXE timed out"
    lines = [l for l in text.replace("\r", "").split("\n") if l.strip() and not re.match(
        r"^(Rapid-Q Compiler|Compiling .* to |Number of statements|Numeric Variables|String Variables|Resource data|Bytes global data|Bytes sub data|Copyright|\s*$)", l)]
    if not os.path.exists(exe):
        return None, " | ".join(lines)[:400] or "no executable"
    return exe, " | ".join(lines)[:300]


def shots(wins, tag):
    """RC.EXE's windows `wins` captured: class, title, a message box's text."""
    out_ = []
    for n, h in enumerate(wins, 1):
        info = {"cls": win_class(h), "title": win_text(h), "menu": bool(user32.GetMenu(h))}
        if info["cls"] == "#32770":
            info["text"] = " | ".join(child_texts(h))[:300]
        img = capture(h)
        if img:
            info["w"], info["h"] = img[0], img[1]
            info["png"] = f"{tag}-rc-{n}.png"
            write_png(os.path.join(caps, info["png"]), *img)
        out_.append(info)
    return out_


def rc_run(exe, folder, tag):
    """RC.EXE's program run: its windows (or its console output)."""
    rec = {}
    if subsystem(exe) == 3:
        rec["kind"] = "console"
        so = os.path.join(out, "rc.out")
        with open(EMPTY, "rb") as fin, open(so, "wb") as fo:
            p = subprocess.Popen([exe], cwd=folder, stdin=fin, stdout=fo, stderr=subprocess.STDOUT)
            # (a console program can show windows too — QFORMs, message boxes:
            # captured as a GUI program's are, `--delay` seconds after the first)
            t0, seen, wins = time.time(), None, []
            while time.time() - t0 < 12 and p.poll() is None:
                time.sleep(0.2)
                if seen is None and windows_of(p.pid):
                    seen = time.time()
                if seen is not None and time.time() - seen >= args.delay:
                    wins = windows_of(p.pid)
                    break
            if wins:
                rec["windows"] = shots(wins, tag)
            try:
                p.wait(timeout=max(0.1, 12 - (time.time() - t0)))
                rec["run"], rec["exit"] = "ok", p.returncode
            except subprocess.TimeoutExpired:
                kill_tree(p.pid)
                rec["run"] = "timeout"
        rec["out"] = tail(open(so, "rb").read().decode("latin-1"))
        return rec
    rec["kind"] = "gui"
    p = subprocess.Popen([exe], cwd=folder, stdin=subprocess.DEVNULL)
    wins, t0 = [], time.time()
    while time.time() - t0 < 10 and not wins and p.poll() is None:
        time.sleep(0.2)
        wins = windows_of(p.pid)
    if p.poll() is not None and not wins:
        rec["run"], rec["exit"], rec["windows"] = "ended", p.returncode, []
        return rec
    time.sleep(args.delay)
    rec["windows"] = shots(windows_of(p.pid), tag)
    rec["run"] = "ok" if p.poll() is None else "ended"
    kill_tree(p.pid)
    try:
        p.wait(timeout=5)
    except subprocess.TimeoutExpired:
        pass
    return rec


def rr_env(prefix):
    e = dict(os.environ)
    e["RAPIDR_INCLUDE_PATH"] = os.path.join(rq, "include")
    e["RAPIDR_PRINT_TO"] = os.path.join(out, "prints")
    e["RAPIDR_REGISTRY"] = os.path.join(out, "registry.reg")
    e["RAPIDR_CONFIG_DIR"] = os.path.join(out, "config")
    e["RAPIDR_TEST_CLIPBOARD"] = "1"
    e["RAPIDR_CAPTURE"] = prefix
    # (the harness captures the windows from the screen first — what a
    # program put on them through Windows itself shows there too —, then
    # RapidR saves its own and the program ends)
    e["RAPIDR_CAPTURE_DELAY"] = str(args.delay + 4)
    e["RAPIDR_CAPTURE_WINDOWS"] = "1"
    e["RAPIDR_THEME"] = "classic"
    # (dialogs shown, then answered — Escape / Cancel — once the capture is past)
    e["RAPIDR_TEST_MESSAGE_DIALOG"] = ";" * 40
    e["RAPIDR_TEST_FILE_DIALOG"] = ";" * 20
    e["RAPIDR_TEST_COLOR_DIALOG"] = ";" * 20
    e["RAPIDR_TEST_FONT_DIALOG"] = ";" * 20
    e["RAPIDR_TEST_DIALOG_HOLD"] = str(int(args.delay * 1000 + 4000))
    e.pop("RAPIDR_HOME", None)
    return e


RUNERR = re.compile(r"run-time error[^:\n]*:\s*(.*)", re.I)
COMPERR = re.compile(r"^(?:.*?:\d+:\d+: )?error(?:\[E\d+\])?: (.*)$", re.M)


def rr_run(cmd, folder, tag, which):
    prefix = os.path.join(caps, f"{tag}-{which}")
    for f in os.listdir(caps):
        if f.startswith(f"{tag}-{which}-"):
            os.remove(os.path.join(caps, f))
    rec = {}
    so = os.path.join(out, f"{which}.out")
    t0 = time.time()
    screens = []
    with open(EMPTY, "rb") as fin, open(so, "wb") as fo:
        p = subprocess.Popen(cmd, cwd=folder, env=rr_env(prefix), stdin=fin, stdout=fo, stderr=subprocess.STDOUT)
        # (the windows as the screen shows them, `--delay` seconds after the
        # first one appeared — as RC.EXE's are taken)
        try:
            wins, w0 = [], time.time()
            while time.time() - w0 < args.delay + 3 and not wins and p.poll() is None:
                time.sleep(0.2)
                wins = windows_of(p.pid)
            if wins and p.poll() is None:
                time.sleep(args.delay)
                k = int(round(scale)) if scale >= 1.5 else 1
                for n, h in enumerate(windows_of(p.pid), 1):
                    img = capture(h, aware=True)
                    if img:
                        img = downscale(img, k)
                        png = f"{tag}-{which}-s{n}.png"
                        write_png(os.path.join(caps, png), *img)
                        screens.append({"w": img[0], "h": img[1], "png": png, "cls": win_class(h), "title": win_text(h)})
        except Exception as e:
            screens.append({"error": str(e)[:200]})
        try:
            p.wait(timeout=args.timeout)
            rec["exit"] = p.returncode
        except subprocess.TimeoutExpired:
            kill_tree(p.pid)
            rec["timeout"] = True
    rec["secs"] = round(time.time() - t0, 1)
    rec["screens"] = screens
    text = open(so, "rb").read().decode("utf-8", "replace")
    rec["out"] = tail(text)
    m = RUNERR.search(text)
    rec["windows"] = []
    n = 1
    while os.path.exists(f"{prefix}-{n}.bmp"):
        try:
            img = read_bmp(f"{prefix}-{n}.bmp")
            k = int(round(scale)) if scale >= 1.5 else 1
            img = downscale(img, k)
            png = f"{tag}-{which}-{n}.png"
            write_png(os.path.join(caps, png), *img)
            rec["windows"].append({"w": img[0], "h": img[1], "png": png})
        except Exception as e:
            rec["windows"].append({"error": str(e)})
        os.remove(f"{prefix}-{n}.bmp")
        n += 1
    titles = re.findall(r"captured window '([^']*)'", text)
    for w, t in zip(rec["windows"], titles):
        w["title"] = t
    if m:
        rec["run"], rec["msg"] = "error", m.group(1).strip()[:300]
    elif "panicked at" in text:
        rec["run"], rec["msg"] = "panic", text[text.index("panicked at"):][:300]
    elif rec.get("timeout"):
        rec["run"] = "timeout"
    elif rec.get("exit") not in (0, None):
        rec["run"], rec["msg"] = "crash", f"exit {rec['exit']}"
    else:
        rec["run"] = "ok"
    warns = [l.strip() for l in text.splitlines() if ("[rapidr]" in l or "[WARN]" in l) and "captured window" not in l]
    if warns:
        rec["warnings"] = warns[:5]
    return rec


def first_error(text):
    m = COMPERR.search(text)
    if m:
        return m.group(1).strip()[:300]
    lines = [l for l in text.splitlines() if l.strip()]
    return (lines[-1] if lines else "failed")[:300]


def compile_check(folder, fname):
    """RapidR's compile (to bytecode): {"compile": "ok"} or the error."""
    rrbc = os.path.join(out, "check.rrbc")
    p = subprocess.run([rapidr, "build-bc", fname, "-o", rrbc], cwd=folder, env=rr_env(""), stdin=subprocess.DEVNULL, capture_output=True, timeout=120)
    if p.returncode != 0 or not os.path.exists(rrbc):
        return {"compile": "error", "msg": first_error((p.stderr + p.stdout).decode("utf-8", "replace"))}
    os.remove(rrbc)
    return {"compile": "ok"}


def interp(folder, fname, tag):
    # (a compile check first: its error is the compiler's message)
    rec = compile_check(folder, fname)
    if rec["compile"] != "ok" or skip:
        return rec
    rec.update(rr_run([rapidr, "run", fname], folder, tag, "interp"))
    return rec


def native(folder, fname, stem, tag):
    rec = {}
    nd = os.path.join(out, "native", tag)
    shutil.rmtree(nd, ignore_errors=True)
    os.makedirs(nd)
    t0 = time.time()
    try:
        p = subprocess.run([rapidr, "build", fname, nd], cwd=folder, env=rr_env(""), stdin=subprocess.DEVNULL, capture_output=True, timeout=1800)
        text = (p.stderr + p.stdout).decode("utf-8", "replace")
        code = p.returncode
    except subprocess.TimeoutExpired:
        text, code = "native build timed out", None
    rec["build_secs"] = round(time.time() - t0, 1)
    exes = [os.path.join(d, f) for d, _, fs in os.walk(nd) for f in fs if f.lower().endswith(".exe")]
    if code != 0 or not exes:
        rec["compile"] = "error"
        rec["msg"] = first_error(text)
        shutil.rmtree(nd, ignore_errors=True)
        return rec
    rec["compile"] = "ok"
    if not skip:
        # (run from beside the program, where RC.EXE's build is: a program
        # finds its files through Command$(0) / Application.Path)
        beside = os.path.join(folder, f"{stem}.exe")
        shutil.copyfile(exes[0], beside)
        try:
            rec.update(rr_run([beside], folder, tag, "native"))
        finally:
            for _ in range(10):
                try:
                    os.remove(beside)
                    break
                except OSError:
                    time.sleep(0.5)
    if not args.keep_native:
        shutil.rmtree(nd, ignore_errors=True)
    return rec


# --------------------------------------------------------------- main --

# (every program, sorted: its place is its tag — the captures' names —, the
# same whatever --list or --filter picks)
every = sorted(os.path.relpath(os.path.join(d, f), work) for d, _, fs in os.walk(work) for f in fs
               if f.lower().endswith(EXTS) and not f.startswith("__"))
tags = {p.lower(): f"{i:03d}" for i, p in enumerate(every)}
if args.list:
    progs = [l.strip().replace("/", os.sep) for l in open(args.list, encoding="utf-8") if l.strip()]
else:
    progs = every
if args.filter:
    keys = [k.lower() for k in args.filter.split(",")]
    progs = [p for p in progs if any(k in p.lower() for k in keys)]
scale = args.scale or screen_scale()
res_path = os.path.join(out, "results.jsonl")
done = set()
last = {}
if os.path.exists(res_path):
    for l in open(res_path, encoding="utf-8"):
        try:
            r = json.loads(l)
        except Exception:
            continue
        last[r["prog"]] = r
        if not (args.redo or args.merge):
            done.add(r["prog"])
rc_recs, rc_caps = {}, caps
if args.rc_from:
    rc_caps = os.path.join(os.path.abspath(args.rc_from), "caps")
    for l in open(os.path.join(os.path.abspath(args.rc_from), "results.jsonl"), encoding="utf-8"):
        try:
            r = json.loads(l)
        except Exception:
            continue
        if r.get("rc"):
            rc_recs[r["prog"]] = r["rc"]
print(f"{len(progs)} programs, {len(done)} done before; RapidR's capture scale {scale}", flush=True)
for i, rel in enumerate(progs):
    key = rel.replace(os.sep, "/")
    if key in done:
        continue
    tag = tags.get(rel.lower(), f"x{i:03d}")
    src = os.path.join(work, rel)
    if not os.path.isfile(src):
        print(f"(no {rel})", flush=True)
        continue
    folder, fname = os.path.split(src)
    stem = os.path.splitext(fname)[0]
    rec = dict(last.get(key, {})) if args.merge else {}
    rec.update({"prog": key, "tag": tag})
    skip = unsafe_reason(src)
    rec["skip"] = skip
    rc_skip = skip or (QREGISTRY.search(code_only(all_text(src))) and "writes the registry")
    t0 = time.time()
    if args.rc_from and key in rc_recs:
        rec["rc"] = rc_recs[key]
    elif not args.no_rc and os.path.exists(rcexe):
        exe, msg = rc_build(folder, fname, stem, tag)
        rc = {"compile": "ok" if exe else "error", "msg": msg}
        if exe and not rc_skip:
            try:
                rc.update(rc_run(exe, folder, tag))
            except Exception as e:
                rc["run"], rc["msg"] = "harness", str(e)[:200]
        elif exe:
            rc["run"] = "not run: " + rc_skip
        if exe:
            try:
                os.remove(exe)
            except OSError:
                pass
        rec["rc"] = rc
    if not args.no_interp:
        try:
            rec["interp"] = interp(folder, fname, tag)
        except Exception as e:
            rec["interp"] = {"compile": "harness", "msg": str(e)[:200]}
    if not args.no_native and args.no_interp and not rec.get("interp"):
        rec["check"] = compile_check(folder, fname)
    if not args.no_native and (rec.get("interp") or rec.get("check") or {}).get("compile") == "ok":
        try:
            rec["native"] = native(folder, fname, stem, tag)
        except Exception as e:
            rec["native"] = {"compile": "harness", "msg": str(e)[:200]}
    # (the windows side by side, the first of each)
    def first(which):
        part = rec.get(which, {}) or {}
        for ws in (part.get("screens") or [], part.get("windows") or []):
            for w in ws:
                if w.get("png"):
                    return w
        return None

    def load_png_rows(name, where):
        # (a PNG write_png made: 8-bit RGB, filter 0)
        import zlib as z
        b = open(os.path.join(where, name), "rb").read()
        w, h = struct.unpack(">II", b[16:24])
        data, j = b"", 8
        while j < len(b):
            n2, kind = struct.unpack(">I4s", b[j:j + 8])
            if kind == b"IDAT":
                data += b[j + 8:j + 8 + n2]
            j += 12 + n2
        raw = z.decompress(data)
        rows = [raw[y * (w * 3 + 1) + 1:(y + 1) * (w * 3 + 1)] for y in range(h)]
        return w, h, rows

    a, b_, c = first("rc"), first("interp"), first("native")
    imgs = {}
    for k, wv in (("rc", a), ("interp", b_), ("native", c)):
        if wv:
            try:
                imgs[k] = load_png_rows(wv["png"], rc_caps if k == "rc" else caps)
            except Exception:
                pass
    if "rc" in imgs:
        # (RC.EXE's client area is below its menu bar; RapidR's window has the
        # menu inside, MAIN_MENU_HEIGHT 28)
        dy = 28 if a and a.get("menu") else 0
        for k in ("interp", "native"):
            if k in imgs:
                rec[k]["diff"] = difference(imgs["rc"], imgs[k], dy)
    if imgs:
        side_by_side(os.path.join(caps, f"{tag}-cmp.png"), [imgs.get("rc"), imgs.get("interp"), imgs.get("native")])
    rec["secs"] = round(time.time() - t0, 1)
    with open(res_path, "a", encoding="utf-8") as fh:
        fh.write(json.dumps(rec) + "\n")
    short = lambda r: (r or {}).get("run") or (r or {}).get("compile") or "-"
    print(tag, key, "rc:", short(rec.get("rc")), "interp:", short(rec.get("interp")), "native:", short(rec.get("native")),
          (skip or ""), flush=True)
print("== end", flush=True)
