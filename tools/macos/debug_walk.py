#!/usr/bin/env python3
"""Running and debugging in RapidR Studio, walked on the real screen (macOS)
as a person does it — DEBUG-RUN's walk, Robert's Preview 2 report: ▶ greyed
while paused, the paused program's window unreachable after switching apps.

RapidR Studio (./rapidr) runs a copy of examples/gui/themes.rr with a
breakpoint at line 94 (SUB Switch); real keys and clicks (CGEvent) drive it,
and every check is what shows: which app is in front, which windows are on
the screen (CGWindowList), the tool bar's ▶ as VoiceOver reads it (its name
and enabled: tools/macos/ui_probe.swift), captures read back with OCR.

  paths: light   F5 (the program's window in front) · a click on Dark stops
                 at 94, Studio in front, ▶ "Continue (F5)" enabled · Finder
                 and back: the paused window still on the screen, painted ·
                 Cmd-Tab reaches it, it's raised, a click in it is dropped,
                 it moves · Debug ▸ Show Program Window · F5: in front, dark ·
                 a step (F10) · Shift+F5 while paused · run again · closed by
                 its close box
         error   tests/fixtures/studio_debug/split.rr: a division by zero in
                 a handler stops it (▶ Continue); F5 ends the handler there
                 and the program runs on (RapidQ's way) — Split again stops
                 again, three times —; 4 people: each gets 25; Shift+F5
         dark, contrast   the paused Studio in RapidR Dark / High Contrast
         events  examples/gui/hello_form.rr, a breakpoint in the name's
                 one-line OnChange (49): each letter typed into the program
                 stops there, Studio in front — three times with F5 between
                 (the program in front again) · a letter typed into the
                 stopped program is dropped · F10 out of the handler, the
                 next letter stops again · its window closed by its close
                 button while it's stopped: it ends, ▶ Run (F5)

         focus   (DEBUG-2) hello_form.rr with OnExit on the name and OnEnter
                 on Greet, a breakpoint in each: Tab stops in OnExit, then in
                 OnEnter — three rounds —; then Enter greets
         loops   steps.rr: a FOR loop's body stops at every pass, a recursive
                 FUNCTION at every level, the Call Stack listing them
         values  counter.rr: a data tip on i, total set to 100 in Variables,
                 the program goes on with it (total 112)
         include a breakpoint in tally.inc (opened from the project tree,
                 closed again): F5 stops there and opens it
         look    (S10, also look-light / look-dark / look-contrast) shop.rr
                 paused in AddUp in each look: Variables' values with their
                 kinds' icons, read by VoiceOver as sentences; a data tip;
                 a watch typed; the Call Stack; Breakpoints; the panes'
                 Close tip above their header

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
END, KEY_A, KEY_B, KEY_C, KEY_X = 119, 0, 11, 8, 7
DOWN, KEY_4, KEY_1, KEY_0, ESC, F2 = 125, 21, 18, 29, 53, 120
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


def double_click_at(pid, x, y):
    if not mine_in_front(pid):
        raise SystemExit(f"stopped: pid {pid} isn't in front — no double click")
    mouse(Quartz.kCGEventMouseMoved, x, y)
    time.sleep(0.1)
    for n in (1, 2):
        for kind in (Quartz.kCGEventLeftMouseDown, Quartz.kCGEventLeftMouseUp):
            e = Quartz.CGEventCreateMouseEvent(None, kind, (x, y), Quartz.kCGMouseButtonLeft)
            Quartz.CGEventSetIntegerValueField(e, Quartz.kCGMouseEventClickState, n)
            Quartz.CGEventPost(Quartz.kCGHIDEventTap, e)
            time.sleep(0.04)
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
    studio = start_studio(src, theme, "wait,wait,line:94")
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
    check(f"[{theme}] the click stops at line 94 and Studio comes to the front", ok, f"front {front_pid()}")
    time.sleep(0.8)
    name, enabled = run_button(studio)
    check(f"[{theme}] paused: the ▶ button reads 'Continue (F5)' and is enabled", name == "Continue (F5)" and enabled in (True, 1, "1"), f"{name!r} enabled={enabled!r}")
    path, text = studio_text(studio, f"{theme}-02-paused")
    # (the status bar: "Breakpoint: <file>:94" — a long file name cut at
    # its panel's end — and the caret's "Ln 94")
    check(f"[{theme}] paused: the status bar says Breakpoint, at line 94", "Breakpoint:" in text and "Ln 94" in text, text[-200:])
    # (the two together: Show Program Window brings the paused program over
    # Studio, whose ▶ shows Continue beside it — Studio's frame captured)
    menu_click(studio, "Debug", "Show Program Window")
    if wait_front(prog):
        sw = wins(studio)[0]
        shot_screen_region(sw["x"], sw["y"], sw["w"], sw["h"], f"{theme}-03-paused-with-program")
    activate(studio)
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
    # (a click on Classic while paused: dropped — no second stop, Studio stays paused at 94)
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
    check("still paused at 94 after the dropped click", "themes-rapidr-light.rr:94" in text.replace(" ", ""), text[-160:])
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
    check("step: the click on Classic stops at 94, Studio in front", wait_front(studio, 4))
    time.sleep(0.6)
    key(studio, F10)
    time.sleep(1.0)
    path, text = studio_text(studio, "09-stepped")
    t2 = text.replace(" ", "")
    # (the SELECT CASE's lines: its CASE tests, then Classic's CASE ELSE, 95 to 106)
    check("F10: Studio stays in front, paused a line on (in the SELECT CASE, 95 to 106)", front_pid() == studio and any(f"themes-rapidr-light.rr:{n}" in t2 for n in range(95, 107)), text[-160:])
    key(studio, F5)
    ok = wait_front(prog, 4)
    time.sleep(1.0)
    t = ocr(shot_window(prog, "10-continued-classic"))
    check("F5 after a step: the program in front, the theme classic", ok and "classic" in t.lower(), t[:200])
    # ---- Stop while paused
    ax_click(prog, "High contrast")
    check("stop: the click on High contrast stops at 94, Studio in front", wait_front(studio, 4))
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
    # (Continue: what the program does without the debugger — RapidQ's, RC.EXE:
    # the handler ends at the error, the program runs on; three times)
    for k in (1, 2, 3):
        key(studio, F5)
        ok = wait_front(prog, 4)
        time.sleep(1.0)
        name, enabled = run_button(studio)
        path = shot_window(prog, f"21-after-error-continue-{k}")
        check(f"[error] Continue {k}: the handler ends, the program runs on — its window in front, ▶ off while it runs", ok and bool(wins(prog)) and enabled not in (True, 1, "1"), f"{name!r} {enabled!r} front {front_pid()}")
        if k == 3:
            break
        ax_click(prog, "Split")
        ok = wait_front(studio, 4)
        time.sleep(0.6)
        path, text = studio_text(studio, f"21-error-again-{k}")
        check(f"[error] Split again: it stops at the error again ({k + 1})", ok and "run-time error: Divi" in text, text[-160:])
    # (the program answers: 4 people now — the answer shows)
    ax_click(prog, "0")
    key(prog, 51)
    key(prog, KEY_4)
    ax_click(prog, "Split")
    time.sleep(1.0)
    check("[error] after the errors the program works: 4 people, each gets 25", ax_find(prog, "each gets 25") is not None and front_pid() == prog, f"front {front_pid()}")
    shot_window(prog, "21-error-program-works")
    activate(studio)
    key(studio, F5, ("shift",))
    time.sleep(1.5)
    name, enabled = run_button(studio)
    path, text = studio_text(studio, "22-error-stopped")
    check("[error] Shift+F5 ends it: ▶ Run (F5) enabled", not wins(prog) and name == "Run (F5)" and enabled in (True, 1, "1"), f"{name!r} {enabled!r}")


def ocr_boxes(path):
    """What Vision reads in a capture, with where (the capture's pixels)."""
    r = subprocess.run([PROBE, "ocr", path], capture_output=True, text=True)
    try:
        return json.loads(r.stdout or "[]")
    except json.JSONDecodeError:
        return []


def screen_point(pid, path, box, fx=0.5):
    """A point of an OCR box of window `pid`'s capture `path`, on the screen
    (the capture is in the screen's pixels: Retina's are 2 a point)."""
    w = wins(pid)[0]
    scale = AppKit.NSImage.alloc().initWithContentsOfFile_(path).representations()[0].pixelsWide() / w["w"]
    return (w["x"] + (box["x"] + box["w"] * fx) / scale, w["y"] + (box["y"] + box["h"] / 2) / scale)


def stops_at(studio, name, where):
    """Studio in front, stopped where `where` (file:line) says: its status
    line read back from a capture."""
    # (Studio may be in front still from the key that went on: the stop
    # comes a moment later — read again until it shows, a few seconds)
    text = ""
    for _ in range(10):
        ok = wait_front(studio, 5)
        time.sleep(0.6)
        path, text = studio_text(studio, name)
        if ok and where in text.replace(" ", ""):
            return True, text
    return False, text


def focus_walk():
    """DEBUG-2: hello_form.rr with an OnExit on the name and an OnEnter on
    Greet, a breakpoint in each: Tab stops in OnExit, then (F5) in OnEnter —
    three rounds, Shift+Tab back between them —; then the program works."""
    src = f"{WORK}/hello_focus.rr"
    text = open(f"{W}/examples/gui/hello_form.rr").read()
    text = text.replace("DECLARE SUB NameChanged\n", "DECLARE SUB NameChanged\nDECLARE SUB NameLeft\nDECLARE SUB GreetEntered\n")
    text = text.replace("        OnChange = NameChanged\n", "        OnChange = NameChanged\n        OnExit = NameLeft\n")
    text = text.replace("        OnClick = Greet\n", "        OnClick = Greet\n        OnEnter = GreetEntered\n")
    text = text.replace("NameEdit.SetFocus\n", 'SUB NameLeft\n    Answer.Caption = "Left the name: " + NameEdit.Text\nEND SUB\n\nSUB GreetEntered\n    Answer.Caption = Answer.Caption + ", on Greet"\nEND SUB\n\nNameEdit.SetFocus\n')
    open(src, "w").write(text)
    lines = text.split("\n")
    a = lines.index('    Answer.Caption = "Left the name: " + NameEdit.Text') + 1
    b = lines.index('    Answer.Caption = Answer.Caption + ", on Greet"') + 1
    studio = start_studio(src, "rapidr-light", f"wait,wait,line:{a}")
    check("[focus] Studio in front", wait_front(studio))
    key(studio, F9)
    for _ in range(b - a):
        key(studio, DOWN)
    key(studio, F9)
    path, text = studio_text(studio, "40-focus-breakpoints")
    key(studio, F5)
    prog = wait_program(studio)
    check("[focus] F5: the program in front", prog is not None and wait_front(prog), f"program {prog} front {front_pid()}")
    if prog is None:
        return
    for r in (1, 2, 3):
        key(prog, TAB)
        ok, text = stops_at(studio, f"41-focus-onexit-{r}", f"hello_focus.rr:{a}")
        check(f"[focus] round {r}: Tab from the name stops in its OnExit ({a})", ok, text[-160:])
        key(studio, F5)
        ok, text = stops_at(studio, f"42-focus-onenter-{r}", f"hello_focus.rr:{b}")
        check(f"[focus] round {r}: F5, then Greet's OnEnter ({b})", ok, text[-160:])
        key(studio, F5)
        ok = wait_front(prog, 4)
        check(f"[focus] round {r}: F5, the program in front again", ok, f"front {front_pid()}")
        if not ok:
            return
        key(prog, TAB, ("shift",))
        time.sleep(0.8)
        check(f"[focus] round {r}: Shift+Tab back to the name: no stop (no breakpoint there)", front_pid() == prog, f"front {front_pid()}")
    # (the program goes on working: Greet pressed — Enter, its Default —
    # from the name: the stops again, then, focused, Greet's OnClick)
    key(prog, ENTER)
    time.sleep(1.0)
    shot_window(prog, "43-focus-program-works")
    n = ax_find(prog, "Hello, World!")
    check("[focus] the program works: Enter greets (Hello, World! (1))", n is not None, str(n)[:120] if n else "no Hello")
    path = shot_window(studio, "44-focus-studio-after")


def loops_walk():
    """DEBUG-2: steps.rr — a breakpoint on a FOR loop's body stops at every
    pass (i = 1, 2, 3); one in the recursive Fact stops at every level, the
    Call Stack listing them all at the deepest."""
    src = f"{WORK}/steps.rr"
    for f in ("steps.rr", "tally.inc"):
        subprocess.run(["cp", f"{W}/tests/fixtures/studio_debug/{f}", WORK])
    studio = start_studio(src, "rapidr-light", "wait,wait,line:11")
    check("[loops] Studio in front", wait_front(studio))
    key(studio, F9)
    for _ in range(40 - 11):
        key(studio, DOWN)
    key(studio, F9)
    key(studio, F5)
    for k in (1, 2, 3):
        ok, text = stops_at(studio, f"50-loop-pass-{k}", "steps.rr:40")
        # (Vision reads the single letter i badly: total says which pass —
        # 0, 1, 3 before the 1st, 2nd and 3rd)
        t = (0, 1, 3)[k - 1]
        check(f"[loops] the loop's body stops at pass {k} (total = {t})", ok and f"total={t}" in text.replace(" ", ""), text[-200:])
        key(studio, F5)
    for level, n in enumerate((4, 3, 2, 1), 1):
        ok, text = stops_at(studio, f"51-recursion-level-{level}", "steps.rr:11")
        detail = text[-260:]
        good = ok and f"n={n}" in text.replace(" ", "")
        if n == 1:
            good = good and text.count("Fact") >= 4
        check(f"[loops] Fact stops at level {level} (n = {n})" + (", the Call Stack lists every level" if n == 1 else ""), good, detail)
        key(studio, F5)
    prog = wait_program(studio)
    check("[loops] then the program runs on: its window", prog is not None and bool(wins(prog)), f"program {prog}")
    activate(studio)
    key(studio, F5, ("shift",))


def values_walk():
    """DEBUG-2: counter.rr at line 6 — the mouse resting on `i` shows its
    data tip; total set to 100 in Variables (F2); after F5 the program uses
    it (total 112)."""
    for f in ("counter.rr", "tally.inc"):
        subprocess.run(["cp", f"{W}/tests/fixtures/studio_debug/{f}", WORK])
    studio = start_studio(f"{WORK}/counter.rr", "rapidr-light", "wait,wait,line:6")
    check("[values] Studio in front", wait_front(studio))
    key(studio, F9)
    key(studio, F5)
    ok, text = stops_at(studio, "60-values-paused", "counter.rr:6")
    check("[values] F5: paused at 6", ok, text[-160:])
    path = shot_window(studio, "60-values-paused")
    box = next((b for b in ocr_boxes(path) if b["text"].strip().startswith("AddUp i")), None)
    if box is None:
        check("[values] the line 'AddUp i' found on the screen", False)
        return
    x, y = screen_point(studio, path, box, 0.97)
    mouse(Quartz.kCGEventMouseMoved, x, y)
    time.sleep(2.0)
    path, text = studio_text(studio, "61-values-datatip")
    # (the tip: the value and its type — Variables shows no type)
    check("[values] the mouse resting on i: its data tip (i = 1, Integer)", "i=1" in text.replace(" ", "") and "Integer" in text, text[-200:])
    mouse(Quartz.kCGEventMouseMoved, x, y + 120)
    time.sleep(0.5)
    path = shot_window(studio, "62-values-variables")
    row = next((b for b in ocr_boxes(path) if "total = 0" in b["text"]), None)
    if row is None:
        check("[values] Variables shows total = 0", False)
        return
    x, y = screen_point(studio, path, row, 0.3)
    click_at(studio, x, y)
    key(studio, F2)
    for k in (KEY_1, KEY_0, KEY_0):
        key(studio, k)
    key(studio, ENTER)
    time.sleep(0.8)
    path, text = studio_text(studio, "63-values-edited")
    check("[values] F2, 100, Enter: Variables shows total = 100", "total=100" in text.replace(" ", ""), text[-200:])
    # (the breakpoint off — F9 on the editor's line 6 — and on: the program
    # adds 2 + 4 + 6 to the 100)
    click_at(studio, *screen_point(studio, path, box, 0.5))
    key(studio, F9)
    key(studio, F5)
    time.sleep(2.0)
    path, text = studio_text(studio, "64-values-used")
    check("[values] after F5 the program used the new value: total 112", "total112" in text.replace(" ", ""), text[-200:])


KEY_T, KEY_O, KEY_L = 17, 31, 37


def ax_all(pid, text, exact=False):
    """Every node whose title, description or value has `text`."""
    out = []
    for w in ax(pid):
        for n in walk(w):
            for f in ("title", "desc", "value"):
                v = str(n.get(f) or "")
                if (v == text) if exact else (text in v):
                    out.append(n)
                    break
    return out


def look_walk(theme):
    """S10: the debugger's look, used as a person does — shop.rr paused at
    76 (in AddUp's loop) in `theme`: Variables' values drawn with their
    kinds' icons and read by VoiceOver as sentences; the mouse resting on
    `orders`: its data tip, the array's count; a watch typed into Watch;
    the Call Stack's current call; the Breakpoints pane; the mouse resting
    on the panes' Close button: its tip above the header, the first row
    clear."""
    tag = {"rapidr-light": "light", "rapidr-dark": "dark", "rapidr-high-contrast": "contrast"}[theme]
    src = f"{WORK}/shop-{tag}.rr"
    subprocess.run(["cp", f"{W}/examples/debugging/shop.rr", src])
    studio = start_studio(src, theme, "wait,wait,line:76")
    check(f"[look {tag}] Studio in front", wait_front(studio))
    key(studio, F9)
    key(studio, F5)
    ok, text = stops_at(studio, f"70-look-{tag}-paused", f"shop-{tag}.rr:76")
    check(f"[look {tag}] F5: paused at 76", ok, text[-160:])
    if not ok:
        return
    ax_click(studio, "Variables")
    time.sleep(0.8)
    path = shot_window(studio, f"71-look-{tag}-variables")
    for said in ("Locals", "Globals", "i, whole number 1", "count, whole number 3", "orders, array 1 to 10, 10 items"):
        check(f"[look {tag}] VoiceOver reads Variables' {said!r}", ax_find(studio, said, exact=True) is not None)
    # the data tip: the mouse resting on `orders` in line 76
    box = next((b for b in ocr_boxes(path) if "LineTotal(orders" in b["text"].replace(" ", "")), None)
    if box is None:
        check(f"[look {tag}] line 76 found on the screen", False)
        return
    t = box["text"]
    at = t.find("orders")
    x, y = screen_point(studio, path, box, (at + 3) / max(1, len(t)))
    mouse(Quartz.kCGEventMouseMoved, x, y)
    time.sleep(2.0)
    path, text = studio_text(studio, f"72-look-{tag}-datatip")
    flat = text.replace(" ", "")
    check(f"[look {tag}] the mouse resting on orders: its data tip, the array and its count", "Array(1TO10)" in flat and "10items" in flat, text[-240:])
    mouse(Quartz.kCGEventMouseMoved, x, y + 260)
    time.sleep(0.6)
    # a watch typed into Watch
    ax_click(studio, "Watch")
    time.sleep(0.6)
    box_node = ax_find(studio, "Add a watch")
    if box_node and "x" in box_node:
        click_at(studio, box_node["x"] + box_node["w"] / 2, box_node["y"] + box_node["h"] / 2)
        for k in (KEY_T, KEY_O, KEY_T, KEY_A, KEY_L):
            key(studio, k)
        key(studio, ENTER)
        time.sleep(1.5)
    path, text = studio_text(studio, f"73-look-{tag}-watch")
    # (total is a DOUBLE: a decimal number)
    check(f"[look {tag}] the watch typed: VoiceOver reads 'total, decimal number …'", bool(ax_all(studio, "total, decimal number")), text[-200:])
    # the Call Stack: the current call first
    ax_click(studio, "Call Stack")
    time.sleep(0.6)
    shot_window(studio, f"74-look-{tag}-callstack")
    check(f"[look {tag}] the Call Stack: AddUp, where the program is", bool(ax_all(studio, f"AddUp   shop-{tag}.rr:76")))
    # the Breakpoints pane: the breakpoint and its state
    ax_click(studio, "Breakpoints", exact=False)
    time.sleep(0.6)
    shot_window(studio, f"75-look-{tag}-breakpoints")
    check(f"[look {tag}] Breakpoints: VoiceOver reads 'shop-{tag}.rr:76, breakpoint'", ax_find(studio, f"shop-{tag}.rr:76, breakpoint", exact=True) is not None)
    # the panes' Close button: its tip above the header (the first row clear)
    tab = ax_find(studio, "Variables", exact=True)
    # (the group's buttons: "Close Variables" …, on its header's row)
    closes = [n for n in ax_all(studio, "Close ") if "y" in n and tab and str(n.get("title") or n.get("desc") or "").startswith("Close ") and abs(n["y"] - tab["y"]) < 20]
    if not closes:
        check(f"[look {tag}] the panes' Close button found (VoiceOver)", False, json.dumps(tab))
        return
    # (the Variables group's own: the first right of its tab)
    c = min((n for n in closes if n["x"] > tab["x"]), key=lambda n: n["x"], default=closes[0])
    mouse(Quartz.kCGEventMouseMoved, c["x"] + c["w"] / 2, c["y"] + c["h"] / 2)
    time.sleep(1.6)
    # (the tip is drawn in the window's popup layer: the screen around the
    # header captured, not the window alone)
    top = tab["y"] - 70
    path = shot_screen_region(c["x"] - 220, top, 300, 120, f"76-look-{tag}-close-tip")
    scale = AppKit.NSImage.alloc().initWithContentsOfFile_(path).representations()[0].pixelsWide() / 300
    tips = [b for b in ocr_boxes(path) if b["text"].strip().startswith("Close")]
    header_top = tab["y"] - 4
    above = [b for b in tips if top + (b["y"] + b["h"]) / scale <= header_top + 2]
    check(f"[look {tag}] the Close button's tip sits above the panes' header, not over the first row", bool(above), f"tips at {[round(top + b['y'] / scale) for b in tips]} ({[b['text'] for b in tips]}), header at {round(tab['y'])}")
    mouse(Quartz.kCGEventMouseMoved, c["x"] - 200, c["y"] + 80)
    time.sleep(0.3)


def include_walk():
    """DEBUG-2: a breakpoint in an $INCLUDEd file — tally.inc opened from the
    project tree, a breakpoint on its line 5, the file closed; F5 stops there
    and opens it at the line."""
    for f in ("counter.rr", "tally.inc"):
        subprocess.run(["cp", f"{W}/tests/fixtures/studio_debug/{f}", WORK])
    studio = start_studio(f"{WORK}/counter.rr", "rapidr-light", "wait,wait")
    check("[include] Studio in front", wait_front(studio))
    path = shot_window(studio, "70-include-project")
    item = next((b for b in ocr_boxes(path) if b["text"].strip() == "tally.inc"), None)
    if item is None:
        check("[include] tally.inc in the project tree", False)
        return
    x, y = screen_point(studio, path, item)
    double_click_at(studio, x, y)
    time.sleep(1.2)
    path = shot_window(studio, "71-include-open")
    line = next((b for b in ocr_boxes(path) if b["text"].strip().startswith("total = total + k")), None)
    if line is None:
        check("[include] tally.inc opened (its line 5 on the screen)", False)
        return
    click_at(studio, *screen_point(studio, path, line, 0.5))
    key(studio, F9)
    menu_click(studio, "File", "Close Document")
    time.sleep(0.8)
    path, text = studio_text(studio, "72-include-closed")
    key(studio, F5)
    ok, text = stops_at(studio, "73-include-stopped", "tally.inc:5")
    check("[include] F5: it stops in tally.inc at 5 and the file is open there", ok and "SUBAddUp" in text.replace(" ", "") and "AddUptally.inc:5" in text.replace(" ", ""), text[-200:])
    key(studio, F5, ("shift",))


def events_walk():
    src = f"{WORK}/hello_form.rr"
    subprocess.run(["cp", f"{W}/examples/gui/hello_form.rr", src])
    studio = start_studio(src, "rapidr-light", "wait,wait,line:49")
    check("[events] Studio in front", wait_front(studio))
    key(studio, F9)
    key(studio, F5)
    prog = wait_program(studio)
    check("[events] F5: the program in front", prog is not None and wait_front(prog), f"program {prog} front {front_pid()}")
    if prog is None:
        return
    # (into the name: a click on its edit, the caret to its end)
    ax_click(prog, "World")
    key(prog, END)
    for k, letter in enumerate((KEY_A, KEY_B, KEY_C), 1):
        key(prog, letter)
        ok = wait_front(studio, 4)
        time.sleep(0.6)
        path, text = studio_text(studio, f"30-events-stop-{k}")
        check(f"[events] letter {k} typed: it stops at the OnChange line 49, Studio in front", ok and "hello_form.rr:49" in text.replace(" ", ""), text[-160:])
        if k < 3:
            key(studio, F5)
            check(f"[events] F5: the program in front again", wait_front(prog, 4), f"front {front_pid()}")
    # (stopped: a letter typed into the program is dropped — no other stop)
    menu_click(studio, "Debug", "Show Program Window")
    if wait_front(prog):
        before = ocr(shot_window(prog, "31-events-paused-program"))
        key(prog, KEY_X)
        time.sleep(1.0)
        after = ocr(shot_window(prog, "32-events-typed-while-paused"))
        check("[events] typed while stopped: dropped (its window unchanged, still in front: no stop)", front_pid() == prog and "Worldabcx" not in after.replace(" ", "") and before == after, f"{before[:80]!r} -> {after[:80]!r}")
    activate(studio)
    # (out of the handler: F10 — its END SUB, then on — the program runs on)
    key(studio, F10)
    time.sleep(0.8)
    if front_pid() == studio:
        key(studio, F10)
    ok = wait_front(prog, 4)
    check("[events] F10 out of the handler: the program runs on, in front", ok, f"front {front_pid()}")
    if ok:
        key(prog, KEY_A)
        ok = wait_front(studio, 4)
        time.sleep(0.6)
        path, text = studio_text(studio, "33-events-after-step")
        check("[events] after the step, the next letter stops at 49 again", ok and "hello_form.rr:49" in text.replace(" ", ""), text[-160:])
    # (stopped: its window closed by its close button — it ends, as Stop)
    menu_click(studio, "Debug", "Show Program Window")
    if wait_front(prog):
        w = wins(prog)[0]
        click_at(prog, w["x"] + 15, w["y"] + 15)
        time.sleep(2.0)
    activate(studio)
    name, enabled = run_button(studio)
    path, text = studio_text(studio, "34-events-closed-while-stopped")
    check("[events] its window closed while stopped: it ends, ▶ Run (F5) enabled", not wins(prog) and name == "Run (F5)" and enabled in (True, 1, "1") and "ended" in text, f"{name!r} {enabled!r} {text[-200:]}")


try:
    paths = sys.argv[1:] or ["light", "error", "dark", "contrast", "events", "focus", "loops", "values", "include"]
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
        close_all(); started.clear(); time.sleep(1)
    if "events" in paths:
        events_walk()
        close_all(); started.clear(); time.sleep(1)
    # (DEBUG-2: focus events, loops and recursion, values, an included file)
    for name, fn in (("focus", focus_walk), ("loops", loops_walk), ("values", values_walk), ("include", include_walk)):
        if name in paths:
            fn()
            close_all(); started.clear(); time.sleep(1)
    # (S10: the debugger's look, in the three looks)
    for name, theme in (("look-light", "rapidr-light"), ("look-dark", "rapidr-dark"), ("look-contrast", "rapidr-high-contrast")):
        if name in paths or "look" in paths:
            look_walk(theme)
            close_all(); started.clear(); time.sleep(1)
    log(f"walk took {time.time() - t0:.0f} s")
finally:
    close_all()
    clear_modifiers()
    log("modifiers left:", hex(Quartz.CGEventSourceFlagsState(Quartz.kCGEventSourceStateCombinedSessionState)))
    json.dump(results, open(f"{OUT}/results.json", "w"), indent=1)
    fails = [r for r in results if not r["ok"]]
    log(f"{len(results) - len(fails)} passed, {len(fails)} failed")
