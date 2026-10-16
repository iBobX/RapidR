#!/usr/bin/env python3
"""RapidR Studio's look, walked on the real screen (macOS) as a person uses
it — THEME-LIVE, Robert's Preview 2 report: "Welcome screen is not
responding to theme changes, it's always light."

The real app (tools/studio_app.sh's bundle: target/RapidR Studio.app, its
own binary) with real clicks and keys (CGEvent), every check what shows:
the Welcome page's colour read from a capture of the real window (where the
accessibility tree says the page and its cards are), the window's title,
VoiceOver's tree (tools/macos/ui_probe.swift).

  paths: system   Studio started with the Mac as it is (never changed by
                  this walk): the Welcome page in the system's look, and
                  still so seconds later (no flash back) · View ▸ Theme ▸
                  RapidR Light / High Contrast / Dark / RapidR (as the
                  System) by the real menu, each recolouring the page and
                  its cards · the page scrolled by the wheel to its last
                  cards · Shift+Tab from the Project pane rings the last
                  card, Enter opens it · hello_form.rr's designer in each
                  look · Settings opened by its tool bar button
         flip     the system's look switched while Studio runs — through the
                  test hook (RAPIDR_TEST_SYSTEM_LOOK=file:…, the file this
                  walk writes; the Mac's own setting is never touched):
                  shop.rr paused at a breakpoint, Variables showing, then
                  dark, high contrast, light — Studio, its panes and the
                  program's own window follow; Settings open while it
                  switches

  python3 tools/macos/theme_walk.py [paths…]   (all by default)

The real screen is shared: take the screen lock first (the lanes' rules).
Keys and clicks go only to this walk's own processes (it stops when another
app is in front); every process it starts is closed. Studio runs with its
own registry, printer folder and AI keys in memory. Captures and
results.json: tests/results/theme-walk/. Needs Accessibility and Screen
Recording for the app running it, and Pillow.
"""
import json, os, subprocess, sys, time
import AppKit, Quartz
from PIL import Image

W = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.environ.get("WALK_OUT") or f"{W}/tests/results/theme-walk"
WORK = f"{OUT}/work"
PROBE = os.environ.get("UI_PROBE") or f"{W}/target/ui_probe"
APP = os.environ.get("STUDIO_APP") or f"{W}/target/RapidR Studio.app/Contents/MacOS/rapidr"
os.makedirs(WORK, exist_ok=True)
if not os.path.exists(PROBE):
    subprocess.run(["swiftc", "-O", f"{W}/tools/macos/ui_probe.swift", "-o", PROBE], check=True)
LOOK_FILE = f"{WORK}/system-look"
ENV = dict(os.environ, RAPIDR_HOME=W, RAPIDR_REGISTRY=f"{WORK}/registry", RAPIDR_PRINT_TO=f"{WORK}/print", RAPIDR_AI_KEYSTORE="memory",
           ANTHROPIC_API_KEY="", OPENAI_API_KEY="", DEEPSEEK_API_KEY="")
for k in ("RAPIDR_CAPTURE", "RAPIDR_TEST_EVENTS", "RAPIDR_TEST_DUMP", "RAPIDR_MENU", "RAPIDR_THEME", "RAPIDR_TEST_SYSTEM_LOOK"):
    ENV.pop(k, None)
F5, F9, ENTER, ESC, TAB = 96, 101, 36, 53, 48
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
    AppKit.NSRunLoop.currentRunLoop().runUntilDate_(AppKit.NSDate.dateWithTimeIntervalSinceNow_(0.05))
    return AppKit.NSWorkspace.sharedWorkspace().frontmostApplication().processIdentifier()


def activate(pid):
    subprocess.run(["osascript", "-e", f'tell application "System Events" to set frontmost of (first process whose unix id is {pid}) to true'], capture_output=True)
    time.sleep(0.8)


def mine_in_front(pid):
    return front_pid() == pid and top_pid() == pid


def post_key(code, flags=0, down=True):
    e = Quartz.CGEventCreateKeyboardEvent(None, code, down)
    Quartz.CGEventSetFlags(e, flags)
    Quartz.CGEventPost(Quartz.kCGHIDEventTap, e)


MOD_KEYS = {"cmd": (55, Quartz.kCGEventFlagMaskCommand), "shift": (56, Quartz.kCGEventFlagMaskShift), "ctrl": (59, Quartz.kCGEventFlagMaskControl), "alt": (58, Quartz.kCGEventFlagMaskAlternate)}


def clear_modifiers():
    for code, flag in MOD_KEYS.values():
        post_key(code, flag, True)
        post_key(code, 0, False)
    time.sleep(0.05)


def key(pid, code, mods=()):
    """A real key — only when our process is in front."""
    if not mine_in_front(pid):
        raise SystemExit(f"stopped: pid {pid} isn't in front (front {front_pid()}) — no key sent")
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
    time.sleep(0.4)


def mouse(kind, x, y):
    e = Quartz.CGEventCreateMouseEvent(None, kind, (x, y), Quartz.kCGMouseButtonLeft)
    Quartz.CGEventPost(Quartz.kCGHIDEventTap, e)


def click_at(pid, x, y):
    if not mine_in_front(pid):
        raise SystemExit(f"stopped: pid {pid} isn't in front — no click")
    mouse(Quartz.kCGEventMouseMoved, x, y)
    time.sleep(0.1)
    mouse(Quartz.kCGEventLeftMouseDown, x, y)
    time.sleep(0.06)
    mouse(Quartz.kCGEventLeftMouseUp, x, y)
    time.sleep(0.5)


def wheel(pid, x, y, lines, times):
    """The mouse wheel turned over (x, y): `lines` per notch (negative:
    down), `times` notches."""
    if not mine_in_front(pid):
        raise SystemExit(f"stopped: pid {pid} isn't in front — no scroll")
    mouse(Quartz.kCGEventMouseMoved, x, y)
    time.sleep(0.1)
    for _ in range(times):
        e = Quartz.CGEventCreateScrollWheelEvent(None, Quartz.kCGScrollEventUnitLine, 1, lines)
        Quartz.CGEventSetLocation(e, (x, y))
        Quartz.CGEventPost(Quartz.kCGHIDEventTap, e)
        time.sleep(0.08)
    time.sleep(0.6)


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
                if ((v == text) if exact else (text in v)) and "x" in n:
                    return n
    return None


def ax_largest(pid, text):
    """The largest element named `text` (the Welcome page, not its tab)."""
    found = [n for w in ax(pid) for n in walk(w) if text in (n.get("title"), n.get("desc")) and "x" in n]
    return max(found, key=lambda n: n["w"] * n["h"]) if found else None


def ax_click(pid, text, exact=True):
    n = ax_find(pid, text, exact)
    if not n:
        raise SystemExit(f"stopped: no {text!r} in pid {pid}'s windows")
    click_at(pid, n["x"] + n["w"] / 2, n["y"] + n["h"] / 2)
    return n


def ax_pos(script):
    r = subprocess.run(["osascript", "-e", script], capture_output=True, text=True)
    nums = [float(v) for v in r.stdout.replace(",", " ").split()]
    return nums if len(nums) == 4 else None


def menu_click(pid, top, item, sub=None):
    """A menu of the menu bar opened with the real mouse and its item
    picked (through a submenu `sub` when given)."""
    proc = f'(first process whose unix id is {pid})'
    bar = f'menu bar item "{top}" of menu bar 1'
    t = ax_pos(f'tell application "System Events" to tell {proc} to get {{position, size}} of {bar}')
    if not t:
        raise SystemExit(f"stopped: no menu {top}")
    click_at(pid, t[0] + t[2] / 2, t[1] + t[3] / 2)
    time.sleep(0.5)
    if sub:
        s = ax_pos(f'tell application "System Events" to tell {proc} to get {{position, size}} of menu item "{sub}" of menu "{top}" of {bar}')
        if not s:
            post_key(ESC, 0, True); post_key(ESC, 0, False)
            raise SystemExit(f"stopped: no submenu {sub}")
        mouse(Quartz.kCGEventMouseMoved, s[0] + 30, s[1] + s[3] / 2)
        time.sleep(0.2)
        mouse(Quartz.kCGEventMouseMoved, s[0] + s[2] - 12, s[1] + s[3] / 2)
        time.sleep(0.6)
        path = f'menu item "{item}" of menu "{sub}" of menu item "{sub}" of menu "{top}" of {bar}'
    else:
        path = f'menu item "{item}" of menu "{top}" of {bar}'
    i = ax_pos(f'tell application "System Events" to tell {proc} to get {{position, size}} of {path}')
    if not i:
        for _ in range(2):
            post_key(ESC, 0, True); post_key(ESC, 0, False)
        raise SystemExit(f"stopped: no menu item {item}")
    # (across to the submenu along its row, then down to the item)
    mouse(Quartz.kCGEventMouseMoved, i[0] + 20, i[1] + i[3] / 2)
    time.sleep(0.2)
    mouse(Quartz.kCGEventLeftMouseDown, i[0] + 20, i[1] + i[3] / 2)
    time.sleep(0.06)
    mouse(Quartz.kCGEventLeftMouseUp, i[0] + 20, i[1] + i[3] / 2)
    time.sleep(1.2)


def shot_window(pid, name, nth=0):
    ws = wins(pid)
    if len(ws) <= nth:
        return None, None
    w = ws[nth]
    path = f"{OUT}/{name}.png"
    subprocess.run(["screencapture", "-x", "-o", f"-l{w['id']}", path])
    return path, w


def luma(path, win, rect):
    """The mean relative luminance (0 black – 1 white) of `rect` (screen
    points) in a window's capture."""
    img = Image.open(path).convert("RGB")
    s = img.width / win["w"]
    x0, y0 = (rect[0] - win["x"]) * s, (rect[1] - win["y"]) * s
    box = (int(x0), int(y0), int(x0 + rect[2] * s), int(y0 + rect[3] * s))
    px = list(img.crop(box).get_flattened_data()) if hasattr(Image.Image, "get_flattened_data") else list(img.crop(box).getdata())
    lin = lambda c: (c / 255 / 12.92) if c <= 10 else (((c / 255 + 0.055) / 1.055) ** 2.4)
    return sum(0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b) for r, g, b in px) / max(1, len(px))


def window_luma(path):
    """The mean relative luminance of a whole capture."""
    img = Image.open(path).convert("RGB").resize((200, 120))
    lin = lambda c: (c / 255 / 12.92) if c <= 10 else (((c / 255 + 0.055) / 1.055) ** 2.4)
    px = list(img.get_flattened_data()) if hasattr(img, "get_flattened_data") else list(img.getdata())
    return sum(0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b) for r, g, b in px) / len(px)


# (a whole window in each look: mostly white, mostly ink, mostly black)
WINDOW = {"light": lambda l: l > 0.5, "dark": lambda l: l < 0.12, "contrast": lambda l: l < 0.09}


def page_and_card(pid, name):
    """The Welcome page's ground (its bottom left, where nothing's drawn)
    and the first example card's (its middle, right of the text), as
    luminances from a capture of the real window."""
    path, win = shot_window(pid, name)
    page = ax_largest(pid, "Welcome")
    card = ax_find(pid, "Example csv_explorer", exact=False)
    if not (path and page and card):
        return path, None, None
    p = luma(path, win, (page["x"] + 12, page["y"] + page["h"] - 40, 120, 24))
    c = luma(path, win, (card["x"] + card["w"] - 30, card["y"] + 6, 20, 10))
    return path, p, c


# (each look: the page's ground and a card's, as luminances)
LOOKS = {
    "light": lambda p, c: p > 0.85 and c > 0.85,
    "dark": lambda p, c: p < 0.05 and c < 0.08 and c > p,
    "contrast": lambda p, c: p < 0.005 and c < 0.005,
}


def look_check(pid, name, look, what):
    path, p, c = page_and_card(pid, name)
    ok = p is not None and LOOKS[look](p, c)
    check(f"{what}: the Welcome page and its cards are {look}", ok, f"page {p if p is None else round(p, 4)}, card {c if c is None else round(c, 4)} — {path}")


def start_studio(args, extra_env=None, log_name="studio"):
    env = dict(ENV, **(extra_env or {}))
    p = subprocess.Popen([APP, "ide", "--fresh", *args], cwd=W, env=env, stdout=open(f"{OUT}/{log_name}.log", "w"), stderr=subprocess.STDOUT)
    started.append(p)
    for _ in range(80):
        if wins(p.pid):
            break
        time.sleep(0.25)
    time.sleep(1.5)
    activate(p.pid)
    return p.pid


def children(pid):
    r = subprocess.run(["pgrep", "-P", str(pid)], capture_output=True, text=True)
    return [int(x) for x in r.stdout.split()]


def close_all():
    for p in started:
        for c in children(p.pid):
            try:
                os.kill(c, 15)
            except OSError:
                pass
        if p.poll() is None:
            p.terminate()
            try:
                p.wait(5)
            except subprocess.TimeoutExpired:
                p.kill()
    started.clear()


def system_path():
    system_dark = subprocess.run(["defaults", "read", "-g", "AppleInterfaceStyle"], capture_output=True, text=True).stdout.strip() == "Dark"
    look = "dark" if system_dark else "light"
    log(f"[system] the Mac is {look} (this walk never changes it)")
    studio = start_studio([])
    look_check(studio, "01-startup", look, f"[system] Studio opened with the Mac in {look}")
    time.sleep(3)
    look_check(studio, "02-startup-later", look, "[system] three seconds later: no flash back")
    for item, want, n in [("RapidR Light", "light", "03-menu-light"), ("RapidR High Contrast", "contrast", "04-menu-contrast"),
                          ("RapidR Dark", "dark", "05-menu-dark"), ("RapidR (as the System)", look, "06-menu-system")]:
        menu_click(studio, "View", item, sub="Theme")
        look_check(studio, n, want, f"[system] View ▸ Theme ▸ {item}")
    # (the page at its default size: the last cards below its edge)
    page = ax_largest(studio, "Welcome")
    last = ax_find(studio, "Example hello:", exact=False)
    hidden = last and page and last["y"] + last["h"] > page["y"] + page["h"]
    check("[system] at the default size the last card is below the page's edge", bool(hidden), f"page {page and (page['y'], page['h'])} card {last and (last['y'], last['h'])}")
    # (the wheel's sign: a notch down, whichever way this Mac scrolls)
    for lines in (-5, 5):
        wheel(studio, page["x"] + page["w"] / 2, page["y"] + page["h"] / 2, lines, 8)
        last = ax_find(studio, "Example hello:", exact=False)
        shown = last and last["y"] >= page["y"] and last["y"] + last["h"] <= page["y"] + page["h"] + 1
        if shown:
            break
    path, _ = shot_window(studio, "07-wheel-to-last-card")
    check("[system] the wheel scrolls the page: the last card (hello.rr) shows whole", bool(shown), f"card {last and (last['y'], last['h'])} — {path}")
    wheel(studio, page["x"] + page["w"] / 2, page["y"] + page["h"] / 2, -lines, 8)
    # (the keyboard: the Project pane clicked, Shift+Tab onto the last card)
    proj = ax_largest(studio, "Project")
    click_at(studio, proj["x"] + proj["w"] / 2, proj["y"] + 60)
    key(studio, TAB, ("shift",))
    last = ax_find(studio, "Example hello:", exact=False)
    path, _ = shot_window(studio, "08-shift-tab-last-card")
    shown = last and last["y"] >= page["y"] and last["y"] + last["h"] <= page["y"] + page["h"] + 1
    # (the focus ring is in the capture; Enter below opening it proves the
    # focus — AccessKit's card node doesn't report AXFocused)
    check("[system] Shift+Tab from the Project pane reaches the last card and scrolls to it", bool(shown), f"card {last and (last['y'], last['h'])} — {path}")
    key(studio, ENTER)
    time.sleep(1.5)
    title = (wins(studio) or [{}])[0].get("name") or ""
    path, _ = shot_window(studio, "09-enter-opens-hello")
    check("[system] Enter opens it: hello.rr", "hello" in title, f"title {title!r} — {path}")
    # (a form's designer in each look)
    menu_click(studio, "View", "Welcome Page")
    ax_click(studio, "Example hello_form: A form, a button, its OnClick")
    time.sleep(2.5)
    for item, want, n in [("RapidR Light", "light", "10-designer-light"), ("RapidR High Contrast", "contrast", "11-designer-contrast"), ("RapidR (as the System)", look, "12-designer-system")]:
        menu_click(studio, "View", item, sub="Theme")
        path, _ = shot_window(studio, n)
        l = window_luma(path) if path else None
        check(f"[system] hello_form.rr's designer after View ▸ Theme ▸ {item}: {want}", l is not None and WINDOW[want](l), f"window {l and round(l, 3)} — {path}")
    # (Settings, by its tool bar button)
    ax_click(studio, "Settings")
    time.sleep(1.5)
    path, win = shot_window(studio, "13-settings", 0)
    l = window_luma(path) if path else None
    check(f"[system] Settings opens over Studio, {look}", len(wins(studio)) >= 2 and l is not None and WINDOW[look](l), f"window {l and round(l, 3)} — {path}")
    key(studio, ESC)
    close_all()


def flip_path():
    def flip(look):
        open(LOOK_FILE, "w").write(look + "\n")
        time.sleep(1.8)

    open(LOOK_FILE, "w").write("light\n")
    studio = start_studio(["--do", "line:147,debug.toggleBreakpoint,run.start,wait,wait,wait,view.variables", "examples/debugging/shop.rr"],
                          {"RAPIDR_TEST_SYSTEM_LOOK": f"file:{LOOK_FILE}"}, "studio-flip")
    for _ in range(40):
        if children(studio):
            break
        time.sleep(0.25)
    time.sleep(4)
    activate(studio)
    path, _ = shot_window(studio, "20-flip-paused-light")
    check("[flip] shop.rr paused at its breakpoint, Studio light (the system says light)", path is not None, path or "")
    for look, n in [("dark", "21-flip-paused-dark"), ("contrast", "22-flip-paused-contrast"), ("light", "23-flip-paused-light-again")]:
        flip(look)
        path, win = shot_window(studio, n)
        prog = [c for c in children(studio) if wins(c)]
        if prog:
            shot_window(prog[0], n + "-program")
        l = window_luma(path) if path else None
        check(f"[flip] the system switched to {look} while paused: Studio and its panes follow", l is not None and WINDOW[look](l), f"window {l and round(l, 3)} — {path}")
    # (Settings open while the system switches)
    activate(studio)
    ax_click(studio, "Settings")
    time.sleep(1.5)
    for look, n in [("dark", "24-flip-settings-dark"), ("contrast", "25-flip-settings-contrast"), ("light", "26-flip-settings-light")]:
        flip(look)
        path, win = shot_window(studio, n, 0)
        l = window_luma(path) if path else None
        check(f"[flip] Settings open while the system switches to {look}: it follows", len(wins(studio)) >= 2 and l is not None and WINDOW[look](l), f"window {l and round(l, 3)} — {path}")
    key(studio, ESC)
    close_all()


try:
    paths = sys.argv[1:] or ["system", "flip"]
    t0 = time.time()
    if "system" in paths:
        system_path()
    if "flip" in paths:
        flip_path()
    log(f"walk took {time.time() - t0:.0f} s")
finally:
    close_all()
    clear_modifiers()
    json.dump(results, open(f"{OUT}/results.json", "w"), indent=1)
    fails = [r for r in results if not r["ok"]]
    log(f"{len(results) - len(fails)} passed, {len(fails)} failed")
