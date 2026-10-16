#!/usr/bin/env python3
"""Running and debugging in RapidR Studio, walked on the real screen (macOS)
as a person does it — DEBUG-RUN's walk, Robert's Preview 2 report: ▶ greyed
while paused, the paused program's window unreachable after switching apps.

RapidR Studio (./rapidr) runs a copy of examples/gui/themes.rr with a
breakpoint at line 68 (SUB Switch); real keys and clicks (CGEvent) drive it,
and every check is what shows: which app is in front, which windows are on
the screen (CGWindowList), the tool bar's ▶ as VoiceOver reads it (its name
and enabled: tools/macos/ui_probe.swift), captures read back with OCR.

  paths: light   F5 (the program's window in front) · a click on Dark stops
                 at 68, Studio in front, ▶ "Continue (F5)" enabled · Finder
                 and back: the paused window still on the screen, painted ·
                 Cmd-Tab reaches it, it's raised, a click in it is dropped,
                 it moves · Debug ▸ Show Program Window · F5: in front, dark ·
                 a step (F10) · Shift+F5 while paused · run again · closed by
                 its close box
         error   tests/fixtures/studio_debug/split.rr: a division by zero
                 stops it (▶ Continue), F5 lets the error end it (exit code
                 1, ▶ Run), F5 runs it again, Shift+F5
         dark, contrast   the paused Studio in RapidR Dark / High Contrast

  python3 tools/macos/debug_walk.py [paths…]   (all by default)

The real screen is shared: take the screen lock first (the lanes' rules),
nothing else on the Mac moves meanwhile. Keys go only to this script's own
processes (it stops when another app is in front); every modifier is pressed
and released as a real key, and cleared at the end. Every process it starts
is closed. Captures and results.json: tests/results/debug-walk/. Needs
Accessibility and Screen Recording for the app running it.
"""
import json, os, signal, subprocess, sys, time
import AppKit, Quartz

W = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.environ.get("WALK_OUT") or f"{W}/tests/results/debug-walk"
WORK = f"{OUT}/work"
PROBE = os.environ.get("UI_PROBE") or f"{W}/target/ui_probe"
os.makedirs(WORK, exist_ok=True)
if not os.path.exists(PROBE):
    subprocess.run(["swiftc", "-O", f"{W}/tools/macos/ui_probe.swift", "-o", PROBE], check=True)
# (Studio's settings, the printer and the AI keys: the walk's own, never the user's)
ENV = dict(os.environ, RAPIDR_REGISTRY=f"{WORK}/registry", RAPIDR_PRINT_TO=f"{WORK}/print", RAPIDR_AI_KEYSTORE="memory",
           ANTHROPIC_API_KEY="", OPENAI_API_KEY="", DEEPSEEK_API_KEY="")
for k in ("RAPIDR_CAPTURE", "RAPIDR_TEST_EVENTS", "RAPIDR_TEST_DUMP", "RAPIDR_MENU"):
    ENV.pop(k, None)
F5, F9, F10, F11, TAB, CMD, ENTER = 96, 101, 109, 103, 48, 55, 36
results = []
started = []


def log(*a):
    print(*a, flush=True)


def check(name, ok, detail=""):
    results.append({"check": name, "ok": bool(ok), "detail": detail})
    log(("PASS " if ok else "FAIL ") + name + (f" — {detail}" if detail else ""))
    return ok


def wins(pid=None):
    out = []
    for i, w in enumerate(Quartz.CGWindowListCopyWindowInfo(Quartz.kCGWindowListOptionOnScreenOnly, Quartz.kCGNullWindowID)):
        if w.get("kCGWindowLayer") != 0:
            continue
        if pid is not None and w["kCGWindowOwnerPID"] != pid:
            continue
        b = w["kCGWindowBounds"]
        out.append({"z": i, "pid": w["kCGWindowOwnerPID"], "id": w["kCGWindowNumber"], "name": w.get("kCGWindowName"), "x": b["X"], "y": b["Y"], "w": b["Width"], "h": b["Height"]})
    return out


def top_pid():
    ws = wins()
    return ws[0]["pid"] if ws else None


def front_pid():
    # (NSWorkspace learns of a switch through its notifications: the run
    # loop turned first, else the answer is the one from the script's start)
    AppKit.NSRunLoop.currentRunLoop().runUntilDate_(AppKit.NSDate.dateWithTimeIntervalSinceNow_(0.05))
    return AppKit.NSWorkspace.sharedWorkspace().frontmostApplication().processIdentifier()


def activate(pid):
    """Our own process to the front, as a click on its Dock icon would."""
    subprocess.run(["osascript", "-e", f'tell application "System Events" to set frontmost of (first process whose unix id is {pid}) to true'], capture_output=True)
    time.sleep(0.8)


def finder():
    """Away to Finder, as a person switching apps."""
    subprocess.run(["osascript", "-e", 'tell application "System Events" to set frontmost of process "Finder" to true'], capture_output=True)
    time.sleep(1.0)


def mine_in_front(pid):
    return front_pid() == pid and top_pid() == pid


def post_key(code, flags=0, down=True):
    # (the flags always set: an event created without them takes the
    # session's — a modifier left "held" by an earlier event would ride
    # along)
    e = Quartz.CGEventCreateKeyboardEvent(None, code, down)
    Quartz.CGEventSetFlags(e, flags)
    Quartz.CGEventPost(Quartz.kCGHIDEventTap, e)


MOD_KEYS = {"cmd": (55, Quartz.kCGEventFlagMaskCommand), "shift": (56, Quartz.kCGEventFlagMaskShift), "ctrl": (59, Quartz.kCGEventFlagMaskControl), "alt": (58, Quartz.kCGEventFlagMaskAlternate)}


def clear_modifiers():
    """The session's modifiers back to none (a real press and release of
    each), whatever the events before left."""
    for code, flag in MOD_KEYS.values():
        post_key(code, flag, True)
        post_key(code, 0, False)
    post_key(63, Quartz.kCGEventFlagMaskSecondaryFn, True)
    post_key(63, 0, False)
    time.sleep(0.05)


def key(pid, code, mods=()):
    """A real key — only when our process is in front; its modifiers
    pressed and released as real keys around it."""
    if not mine_in_front(pid):
        raise SystemExit(f"stopped: pid {pid} isn't in front (front {front_pid()}, top window {top_pid()}) — no key sent")
    flags = 0
    for m in mods:
        mcode, mflag = MOD_KEYS[m]
        flags |= mflag
        post_key(mcode, flags, True)
        time.sleep(0.03)
    post_key(code, flags, True)
    time.sleep(0.05)
    post_key(code, flags, False)
    for m in reversed(mods):
        mcode, mflag = MOD_KEYS[m]
        flags &= ~mflag
        post_key(mcode, flags, False)
        time.sleep(0.03)
    clear_modifiers()
    time.sleep(0.3)


def cmd_tab(times):
    """Cmd-Tab, Tab pressed `times` times (the app switcher's order)."""
    post_key(CMD, Quartz.kCGEventFlagMaskCommand, True)
    time.sleep(0.1)
    for _ in range(times):
        post_key(TAB, Quartz.kCGEventFlagMaskCommand, True)
        time.sleep(0.05)
        post_key(TAB, Quartz.kCGEventFlagMaskCommand, False)
        time.sleep(0.25)
    post_key(CMD, 0, False)
    clear_modifiers()
    time.sleep(1.0)


def mouse(kind, x, y):
    e = Quartz.CGEventCreateMouseEvent(None, kind, (x, y), Quartz.kCGMouseButtonLeft)
    Quartz.CGEventPost(Quartz.kCGHIDEventTap, e)


def click_at(pid, x, y, need_front=True):
    if need_front and not mine_in_front(pid):
        raise SystemExit(f"stopped: pid {pid} isn't in front — no click")
    mouse(Quartz.kCGEventMouseMoved, x, y)
    time.sleep(0.1)
    mouse(Quartz.kCGEventLeftMouseDown, x, y)
    time.sleep(0.06)
    mouse(Quartz.kCGEventLeftMouseUp, x, y)
    time.sleep(0.4)


def drag(pid, x1, y1, x2, y2):
    if not mine_in_front(pid):
        raise SystemExit(f"stopped: pid {pid} isn't in front — no drag")
    mouse(Quartz.kCGEventMouseMoved, x1, y1)
    time.sleep(0.1)
    mouse(Quartz.kCGEventLeftMouseDown, x1, y1)
    time.sleep(0.1)
    for k in range(1, 13):
        mouse(Quartz.kCGEventLeftMouseDragged, x1 + (x2 - x1) * k / 12, y1 + (y2 - y1) * k / 12)
        time.sleep(0.03)
    mouse(Quartz.kCGEventLeftMouseUp, x2, y2)
    time.sleep(0.5)


def ax(pid):
    r = subprocess.run([PROBE, "tree", str(pid)], capture_output=True, text=True)
    try:
        return json.loads(r.stdout or "[]")
    except json.JSONDecodeError:
        return []


def walk(n):
    yield n
    for c in n.get("children", []):
        yield from walk(c)


def ax_find(pid, text, exact=False):
    for w in ax(pid):
        for n in walk(w):
            for f in ("title", "desc", "value"):
                v = str(n.get(f) or "")
                if (v == text) if exact else (text in v):
                    return n
    return None


def ax_click(pid, text, exact=True):
    n = ax_find(pid, text, exact)
    if not n or "x" not in n:
        raise SystemExit(f"stopped: no {text!r} in pid {pid}'s windows")
    click_at(pid, n["x"] + n["w"] / 2, n["y"] + n["h"] / 2)
    return n


def ax_pos(script):
    r = subprocess.run(["osascript", "-e", script], capture_output=True, text=True)
    nums = [float(v) for v in r.stdout.replace(",", " ").split()]
    return nums if len(nums) == 4 else None


def menu_click(pid, top, item):
    """A menu of the menu bar opened and its item picked with the real
    mouse (where macOS accessibility says they are)."""
    proc = f'(first process whose unix id is {pid})'
    t = ax_pos(f'tell application "System Events" to tell {proc} to get {{position, size}} of menu bar item "{top}" of menu bar 1')
    if not t:
        raise SystemExit(f"stopped: no menu {top}")
    click_at(pid, t[0] + t[2] / 2, t[1] + t[3] / 2)
    time.sleep(0.5)
    i = ax_pos(f'tell application "System Events" to tell {proc} to get {{position, size}} of menu item "{item}" of menu "{top}" of menu bar item "{top}" of menu bar 1')
    if not i:
        post_key(53, 0, True); post_key(53, 0, False)
        raise SystemExit(f"stopped: no menu item {item}")
    # (the menu is the system's: the front window is the menu's now)
    mouse(Quartz.kCGEventMouseMoved, i[0] + 30, i[1] + i[3] / 2)
    time.sleep(0.15)
    mouse(Quartz.kCGEventLeftMouseDown, i[0] + 30, i[1] + i[3] / 2)
    time.sleep(0.06)
    mouse(Quartz.kCGEventLeftMouseUp, i[0] + 30, i[1] + i[3] / 2)
    time.sleep(0.6)


def shot_window(pid, name):
    ws = wins(pid)
    if not ws:
        return None
    path = f"{OUT}/{name}.png"
    subprocess.run(["screencapture", "-x", "-o", f"-l{ws[0]['id']}", path])
    return path


def shot_screen_region(x, y, w, h, name):
    path = f"{OUT}/{name}.png"
    subprocess.run(["screencapture", "-x", f"-R{int(x)},{int(y)},{int(w)},{int(h)}", path])
    return path


def ocr(path):
    r = subprocess.run([PROBE, "ocr", path], capture_output=True, text=True)
    try:
        return " | ".join(l["text"] for l in json.loads(r.stdout or "[]"))
    except json.JSONDecodeError:
        return ""


def start_studio(path, theme="rapidr-light", do="wait,wait"):
    p = subprocess.Popen([f"{W}/rapidr", "ide", "--fresh", "--theme", theme, "--do", do, path], cwd=W, env=ENV,
                         stdout=open(f"{OUT}/studio-{theme}.log", "w"), stderr=subprocess.STDOUT)
    started.append(p)
    for _ in range(60):
        if wins(p.pid):
            break
        time.sleep(0.25)
    time.sleep(1.5)
    activate(p.pid)
    return p.pid


def program_of(studio):
    r = subprocess.run(["pgrep", "-P", str(studio)], capture_output=True, text=True)
    pids = [int(x) for x in r.stdout.split()]
    for p in pids:
        if wins(p):
            return p
    return pids[0] if pids else None


def wait_program(studio, timeout=12):
    for _ in range(int(timeout / 0.25)):
        p = program_of(studio)
        if p and wins(p):
            return p
        time.sleep(0.25)
    return None


def wait_front(pid, timeout=3.0):
    for _ in range(int(timeout / 0.1)):
        if front_pid() == pid:
            return True
        time.sleep(0.1)
    return False


def studio_text(studio, name):
    path = shot_window(studio, name)
    return path, ocr(path) if path else ""


def run_button(studio):
    """The ▶ tool button as VoiceOver reads it: its name (hint) and enabled."""
    n = ax_find(studio, "Continue (F5)", exact=True) or ax_find(studio, "Run (F5)", exact=True)
    return (n.get("desc") or n.get("title"), n.get("enabled")) if n else (None, None)


def close_all():
    for p in started:
        if p.poll() is None:
            for c in subprocess.run(["pgrep", "-P", str(p.pid)], capture_output=True, text=True).stdout.split():
                try:
                    os.kill(int(c), signal.SIGTERM)
                except ProcessLookupError:
                    pass
            p.terminate()
            try:
                p.wait(5)
            except subprocess.TimeoutExpired:
                p.kill()


# ---------------------------------------------------------------- the paths --

def themes_walk(theme, full):
    src = f"{WORK}/themes-{theme}.rr"
    subprocess.run(["cp", f"{W}/examples/gui/themes.rr", src])
    studio = start_studio(src, theme, "wait,wait,line:68")
    check(f"[{theme}] Studio in front at the start", wait_front(studio))
    key(studio, F9)
    key(studio, F5)
    prog = wait_program(studio)
    check(f"[{theme}] F5: the program's window shows", prog is not None)
    if prog is None:
        return
    check(f"[{theme}] F5: the program's window comes up in front of Studio", wait_front(prog) and top_pid() == prog, f"front {front_pid()}, top {top_pid()}, program {prog}")
    shot_window(prog, f"{theme}-01-program-started")
    # (a real click on Dark: the handler's breakpoint)
    if front_pid() != prog:
        activate(prog)
    ax_click(prog, "Dark")
    ok = wait_front(studio, 4)
    check(f"[{theme}] the click stops at line 68 and Studio comes to the front", ok, f"front {front_pid()}")
    time.sleep(0.8)
    name, enabled = run_button(studio)
    check(f"[{theme}] paused: the ▶ button reads 'Continue (F5)' and is enabled", name == "Continue (F5)" and enabled in (True, 1, "1"), f"{name!r} enabled={enabled!r}")
    path, text = studio_text(studio, f"{theme}-02-paused")
    # (the status bar: "Breakpoint: <file>:68" — a long file name cut at
    # its panel's end — and the caret's "Ln 68")
    check(f"[{theme}] paused: the status bar says Breakpoint, at line 68", "Breakpoint:" in text and "Ln 68" in text, text[-200:])
    if not full:
        key(studio, F5)
        time.sleep(1.0)
        return studio, prog
    # (away to Finder and back, as Robert did)
    finder()
    check("away: Finder in front", front_pid() != studio and front_pid() != prog, f"front {front_pid()}")
    check("away: the paused program's window is still on the screen", bool(wins(prog)), json.dumps(wins(prog)))
    activate(studio)
    check("back: Studio in front again", wait_front(studio))
    pw = wins(prog)
    check("back: the paused program's window is still on the screen", bool(pw), json.dumps(pw))
    img = shot_window(prog, "03-paused-program-window")
    t = ocr(img) if img else ""
    check("back: the paused program's window is painted (its own image reads Themes … Theme:)", "Theme" in t and "Classic" in t, t[:200])
    # (Cmd-Tab: Studio, Finder, the program — twice to reach it)
    cmd_tab(2)
    check("Cmd-Tab reaches the paused program: it's in front", wait_front(prog), f"front {front_pid()}")
    check("Cmd-Tab: its window is the topmost on the screen", top_pid() == prog, f"top {top_pid()}")
    w = wins(prog)[0]
    shot_screen_region(w["x"], w["y"], w["w"], w["h"], "04-paused-program-raised-by-cmd-tab")
    t = ocr(f"{OUT}/04-paused-program-raised-by-cmd-tab.png")
    check("raised: the screen shows it painted", "Theme" in t, t[:200])
    # (a click on Classic while paused: dropped — no second stop, Studio stays paused at 68)
    n = ax_find(prog, "Classic", exact=True)
    click_at(prog, n["x"] + n["w"] / 2, n["y"] + n["h"] / 2)
    time.sleep(1.2)
    check("paused: a click in the program is dropped (no stop, the program stays in front)", front_pid() == prog, f"front {front_pid()}")
    # (moved by its title bar)
    before = wins(prog)[0]
    drag(prog, before["x"] + before["w"] / 2, before["y"] + 12, before["x"] + before["w"] / 2 - 120, before["y"] + 12 + 60)
    after = wins(prog)[0]
    check("paused: its window moves (dragged by the title bar)", (after["x"], after["y"]) != (before["x"], before["y"]), f"{before['x']},{before['y']} -> {after['x']},{after['y']}")
    shot_screen_region(after["x"], after["y"], after["w"], after["h"], "05-paused-program-moved")
    t = ocr(f"{OUT}/05-paused-program-moved.png")
    check("moved: painted at its new place", "Theme" in t, t[:200])
    # (back to Studio: Cmd-Tab once)
    cmd_tab(1)
    check("Cmd-Tab back: Studio in front", wait_front(studio), f"front {front_pid()}")
    path, text = studio_text(studio, "06-still-paused")
    check("still paused at 68 after the dropped click", "themes-rapidr-light.rr:68" in text.replace(" ", ""), text[-160:])
    # (Debug > Show Program Window, through the menu bar as VoiceOver presses it)
    menu_click(studio, "Debug", "Show Program Window")
    ok = wait_front(prog)
    r = subprocess.CompletedProcess([], 0, "", "")
    check("Debug > Show Program Window: the paused program in front", ok and top_pid() == prog, f"front {front_pid()} top {top_pid()} {r.stderr.strip()}")
    w = wins(prog)[0]
    shot_screen_region(w["x"], w["y"], w["w"], w["h"], "07-show-program-window")
    activate(studio)
    check("back to Studio", wait_front(studio))
    # (Continue: F5 — the program in front, dark)
    key(studio, F5)
    ok = wait_front(prog, 4)
    check("F5 (Continue): the program comes to the front", ok and top_pid() == prog, f"front {front_pid()} top {top_pid()}")
    time.sleep(1.0)
    img = shot_window(prog, "08-continued-dark")
    t = ocr(img)
    check("Continue: the theme changed (Theme: … dark)", "dark" in t.lower(), t[:200])
    # ---- a step: click Classic → paused; F10 → still paused, a line on; F5 → classic
    ax_click(prog, "Classic")
    check("step: the click on Classic stops at 68, Studio in front", wait_front(studio, 4))
    time.sleep(0.6)
    key(studio, F10)
    time.sleep(1.0)
    path, text = studio_text(studio, "09-stepped")
    t2 = text.replace(" ", "")
    check("F10: Studio stays in front, paused a line on (69 or 70)", front_pid() == studio and ("themes-rapidr-light.rr:69" in t2 or "themes-rapidr-light.rr:70" in t2), text[-160:])
    key(studio, F5)
    ok = wait_front(prog, 4)
    time.sleep(1.0)
    t = ocr(shot_window(prog, "10-continued-classic"))
    check("F5 after a step: the program in front, the theme classic", ok and "classic" in t.lower(), t[:200])
    # ---- Stop while paused
    ax_click(prog, "High contrast")
    check("stop: the click on High contrast stops at 68, Studio in front", wait_front(studio, 4))
    time.sleep(0.6)
    key(studio, F5, ("shift",))
    time.sleep(1.5)
    gone = not wins(prog)
    name, enabled = run_button(studio)
    path, text = studio_text(studio, "11-stopped")
    check("Shift+F5 while paused: the program's window goes, ▶ is Run (F5) and enabled", gone and name == "Run (F5)" and enabled in (True, 1, "1"), f"gone={gone} {name!r} {enabled!r}")
    # ---- run again after it ended; close its window while it runs
    key(studio, F5)
    prog2 = wait_program(studio)
    ok = prog2 is not None and wait_front(prog2)
    check("F5 again after a Stop: the program runs, in front", ok, f"program {prog2} front {front_pid()}")
    if prog2:
        shot_window(prog2, "12-run-again")
        w = wins(prog2)[0]
        # (the window's close button, the first of its title bar's three)
        click_at(prog2, w["x"] + 15, w["y"] + 15)
        time.sleep(2.0)
        name, enabled = run_button(studio)
        activate(studio)
        path, text = studio_text(studio, "13-closed-by-the-user")
        check("closed by its close button while running: it ends, ▶ Run (F5) enabled", not wins(prog2) and name == "Run (F5)" and enabled in (True, 1, "1") and "ended" in text, f"{name!r} {enabled!r} {text[-200:]}")
    return studio, None


def error_walk():
    src = f"{WORK}/split.rr"
    subprocess.run(["cp", f"{W}/tests/fixtures/studio_debug/split.rr", src])
    studio = start_studio(src, "rapidr-light")
    check("[error] Studio in front", wait_front(studio))
    key(studio, F5)
    prog = wait_program(studio)
    check("[error] F5: the program in front", prog is not None and wait_front(prog), f"program {prog} front {front_pid()}")
    ax_click(prog, "Split")
    check("[error] Split divides by zero: Studio in front at the error", wait_front(studio, 4))
    time.sleep(0.8)
    name, enabled = run_button(studio)
    path, text = studio_text(studio, "20-runtime-error")
    check("[error] paused at the error: Continue (F5) enabled, the error shown", name == "Continue (F5)" and enabled in (True, 1, "1") and "Run-time error" in text and "run-time error: Divi" in text, f"{name!r} {enabled!r} {text[-200:]}")
    check("[error] the program's window is still on the screen", bool(wins(prog)))
    key(studio, F5)
    time.sleep(2.0)
    name, enabled = run_button(studio)
    activate(studio)
    path, text = studio_text(studio, "21-after-error-continue")
    check("[error] Continue lets the error end it (exit code 1); ▶ Run (F5) enabled", not wins(prog) and name == "Run (F5)" and enabled in (True, 1, "1"), f"{name!r} {enabled!r} {text[-200:]}")
    key(studio, F5)
    prog = wait_program(studio)
    check("[error] F5 runs it again after the failed run", prog is not None and wait_front(prog), f"program {prog}")
    ax_click(prog, "Split")
    check("[error] stopped at the error again", wait_front(studio, 4))
    time.sleep(0.6)
    key(studio, F5, ("shift",))
    time.sleep(1.5)
    name, enabled = run_button(studio)
    path, text = studio_text(studio, "22-error-stopped")
    check("[error] Shift+F5 ends it: ▶ Run (F5) enabled", not wins(prog) and name == "Run (F5)" and enabled in (True, 1, "1"), f"{name!r} {enabled!r}")


try:
    paths = sys.argv[1:] or ["light", "error", "dark", "contrast"]
    t0 = time.time()
    if "light" in paths:
        themes_walk("rapidr-light", True)
        close_all(); started.clear(); time.sleep(1)
    if "error" in paths:
        error_walk()
        close_all(); started.clear(); time.sleep(1)
    if "dark" in paths:
        themes_walk("rapidr-dark", False)
        close_all(); started.clear(); time.sleep(1)
    if "contrast" in paths:
        themes_walk("rapidr-high-contrast", False)
        close_all(); started.clear()
    log(f"walk took {time.time() - t0:.0f} s")
finally:
    close_all()
    clear_modifiers()
    log("modifiers left:", hex(Quartz.CGEventSourceFlagsState(Quartz.kCGEventSourceStateCombinedSessionState)))
    json.dump(results, open(f"{OUT}/results.json", "w"), indent=1)
    fails = [r for r in results if not r["ok"]]
    log(f"{len(results) - len(fails)} passed, {len(fails)} failed")
