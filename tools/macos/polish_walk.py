#!/usr/bin/env python3
"""POLISH-2's small Studio fixes, walked on the real screen (macOS) as a
person does it: real keys and clicks (CGEvent) into RapidR Studio (./rapidr),
every check what shows — what VoiceOver reads (tools/macos/ui_probe.swift),
captures read back with OCR, the clipboard.

  paths: editor     hello_form.rr: Cmd+G — the box keeps its ":", 12 typed
                    after it, Enter: the caret on 12 and the status bar's
                    "Ln 12, Col 1" · x typed: the tab "Not saved" · Cmd+Z:
                    saved again · Cmd+Shift+Z: not saved · Cmd+Z: saved
         designer   helpers.rr (and its logo.svg): the designer shows the
                    form in RGB(250, 246, 238) and the logo loaded after the
                    form — in light, dark and high contrast
         debug      runs_on.rr stopped at 12: F5, it runs on — a moment later
                    Call Stack and Variables say "Running…" · Debug ▸ Pause:
                    filled again · a value picked, Cmd+C: on the clipboard,
                    the status bar says so · Shift+F5 — light, dark, high
                    contrast (the copy in light)
         status     a_program_whose_name_is_far_too_long_for_the_status_bar.rr
                    stopped at 6: the status bar says all of it — light, dark,
                    high contrast
         ai         Tools ▸ Set Up RAI… with the AI mock as "Ollama" (no
                    network, no key): found, Next, Finish (the mouse): the
                    Assistant's own model list filled at once, ready — the
                    wizard's wrapped notes at the screen's scale (2× here)
                    whole — light and dark
         tree       Database Studio's tree (a root of 47 tables, then a CSV)
                    in a program of its own: the wheel to the end, the bar's
                    thumb dragged to the bottom, End — the last row whole
                    each time; its status bar's text about 10 pixels in —
                    light, dark, high contrast

  python3 tools/macos/polish_walk.py [paths…]   (all by default)

The real screen is shared: take the screen lock first (the lanes' rules).
Keys go only to Studio's own process (it stops when another app is in
front); every click first asks the accessibility hit test whose element is
under the point (a notification's banner there stops the walk). Every
process it starts is closed. Captures and results.json:
tests/results/polish-walk/. Needs Accessibility and Screen Recording for the
app running it.
"""
import json, os, signal, subprocess, sys, time
import AppKit, Quartz

W = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.environ.get("WALK_OUT") or f"{W}/tests/results/polish-walk"
WORK = f"{OUT}/work"
PROBE = os.environ.get("UI_PROBE") or f"{W}/target/ui_probe"
os.makedirs(WORK, exist_ok=True)
if not os.path.exists(PROBE):
    subprocess.run(["swiftc", "-O", f"{W}/tools/macos/ui_probe.swift", "-o", PROBE], check=True)
# (Studio's settings, the printer, the AI keys and RSecrets: the walk's own,
# never the user's)
ENV = dict(os.environ, RAPIDR_REGISTRY=f"{WORK}/registry", RAPIDR_PRINT_TO=f"{WORK}/print", RAPIDR_AI_KEYSTORE="memory",
           RAPIDR_KEYSTORE="memory", ANTHROPIC_API_KEY="", OPENAI_API_KEY="", DEEPSEEK_API_KEY="", ANTHROPIC_WORKSPACE_ID="")
for k in ("RAPIDR_CAPTURE", "RAPIDR_TEST_EVENTS", "RAPIDR_TEST_DUMP", "RAPIDR_MENU", "RAPIDR_SCALE"):
    ENV.pop(k, None)
CMD, ENTER, ESC, F5, F9 = 55, 36, 53, 96, 101
KEY = {"g": 5, "z": 6, "x": 7, "c": 8, "1": 18, "2": 19}
LOOKS = {"light": "rapidr-light", "dark": "rapidr-dark", "contrast": "rapidr-high-contrast"}
results = []
started = []


def log(*a):
    print(*a, flush=True)


def check(name, ok, detail=""):
    results.append({"check": name, "ok": bool(ok), "detail": detail})
    log(("PASS " if ok else "FAIL ") + name + (f" — {detail}" if detail else ""))
    return ok


# ---------------------------------------------------------- the screen --

def wins(pid=None):
    out = []
    for w in Quartz.CGWindowListCopyWindowInfo(Quartz.kCGWindowListOptionOnScreenOnly, Quartz.kCGNullWindowID):
        if w.get("kCGWindowLayer") != 0 or (pid is not None and w["kCGWindowOwnerPID"] != pid):
            continue
        b = w["kCGWindowBounds"]
        out.append({"pid": w["kCGWindowOwnerPID"], "id": w["kCGWindowNumber"], "x": b["X"], "y": b["Y"], "w": b["Width"], "h": b["Height"]})
    return out


def front_pid():
    AppKit.NSRunLoop.currentRunLoop().runUntilDate_(AppKit.NSDate.dateWithTimeIntervalSinceNow_(0.05))
    return AppKit.NSWorkspace.sharedWorkspace().frontmostApplication().processIdentifier()


def top_pid():
    ws = wins()
    return ws[0]["pid"] if ws else None


def mine_in_front(pid):
    return front_pid() == pid and top_pid() == pid


def wait_front(pid, timeout=4.0):
    for _ in range(int(timeout / 0.1)):
        if front_pid() == pid:
            return True
        time.sleep(0.1)
    return False


def activate(pid):
    subprocess.run(["osascript", "-e", f'tell application "System Events" to set frontmost of (first process whose unix id is {pid}) to true'], capture_output=True)
    time.sleep(0.8)


def mine_under(pid, x, y):
    r = subprocess.run([PROBE, "hit", str(int(x)), str(int(y))], capture_output=True, text=True)
    try:
        hit = json.loads(r.stdout or "{}").get("pid")
    except json.JSONDecodeError:
        hit = None
    if hit != pid:
        log(f"  (the element under ({x:.0f}, {y:.0f}) is pid {hit}'s — not ours)")
    return hit == pid


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
    """A real key — only when Studio is in front; its modifiers pressed and
    released as real keys around it."""
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
    time.sleep(0.3)


def mouse(kind, x, y):
    Quartz.CGEventPost(Quartz.kCGHIDEventTap, Quartz.CGEventCreateMouseEvent(None, kind, (x, y), Quartz.kCGMouseButtonLeft))


def click_at(pid, x, y):
    if not mine_in_front(pid):
        raise SystemExit(f"stopped: pid {pid} isn't in front — no click")
    if not mine_under(pid, x, y):
        raise SystemExit(f"stopped: the point ({x:.0f}, {y:.0f}) isn't on pid {pid}'s window — no click")
    mouse(Quartz.kCGEventMouseMoved, x, y)
    time.sleep(0.1)
    mouse(Quartz.kCGEventLeftMouseDown, x, y)
    time.sleep(0.06)
    mouse(Quartz.kCGEventLeftMouseUp, x, y)
    time.sleep(0.4)


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
            # (AccessKit's description is the element's AXHelp)
            for f in ("title", "desc", "value", "help"):
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


def node_with(pid, start):
    for w in ax(pid):
        for n in walk(w):
            for f in ("title", "desc"):
                if str(n.get(f) or "").startswith(start) and "x" in n and n.get("h", 0) > 4:
                    return n
    return None


def ax_pos(script):
    r = subprocess.run(["osascript", "-e", script], capture_output=True, text=True)
    nums = [float(v) for v in r.stdout.replace(",", " ").split()]
    return nums if len(nums) == 4 else None


def menu_click(pid, top, item):
    """A menu of the menu bar opened and its item picked with the real mouse."""
    proc = f'(first process whose unix id is {pid})'
    t = ax_pos(f'tell application "System Events" to tell {proc} to get {{position, size}} of menu bar item "{top}" of menu bar 1')
    if not t:
        raise SystemExit(f"stopped: no menu {top}")
    click_at(pid, t[0] + t[2] / 2, t[1] + t[3] / 2)
    time.sleep(0.5)
    i = ax_pos(f'tell application "System Events" to tell {proc} to get {{position, size}} of menu item "{item}" of menu "{top}" of menu bar item "{top}" of menu bar 1')
    if not i:
        post_key(ESC, 0, True); post_key(ESC, 0, False)
        raise SystemExit(f"stopped: no menu item {item}")
    mouse(Quartz.kCGEventMouseMoved, i[0] + 30, i[1] + i[3] / 2)
    time.sleep(0.15)
    mouse(Quartz.kCGEventLeftMouseDown, i[0] + 30, i[1] + i[3] / 2)
    time.sleep(0.06)
    mouse(Quartz.kCGEventLeftMouseUp, i[0] + 30, i[1] + i[3] / 2)
    time.sleep(0.6)


def shot_window(pid, name, index=0):
    ws = wins(pid)
    if len(ws) <= index:
        return None
    path = f"{OUT}/{name}.png"
    subprocess.run(["screencapture", "-x", "-o", f"-l{ws[index]['id']}", path])
    return path


def ocr(path):
    r = subprocess.run([PROBE, "ocr", path], capture_output=True, text=True)
    try:
        return " | ".join(l["text"] for l in json.loads(r.stdout or "[]"))
    except json.JSONDecodeError:
        return ""


def start_studio(path, theme, do="wait,wait"):
    p = subprocess.Popen([f"{W}/rapidr", "ide", "--fresh", "--theme", theme, "--do", do, path], cwd=W, env=ENV,
                         stdout=open(f"{OUT}/studio-{os.path.basename(path)}-{theme}.log", "w"), stderr=subprocess.STDOUT)
    started.append(p)
    for _ in range(80):
        if wins(p.pid):
            break
        time.sleep(0.25)
    time.sleep(2.0)
    activate(p.pid)
    return p.pid


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
    started.clear()
    time.sleep(1.0)


def until(f, timeout=6.0):
    t0 = time.time()
    while time.time() - t0 < timeout:
        v = f()
        if v:
            return v
        time.sleep(0.3)
    return None


# ------------------------------------------------------------- the paths --

def editor_walk():
    """b, d: Go to Line and the changed mark, light."""
    src = f"{WORK}/hello_form.rr"
    subprocess.run(["cp", f"{W}/examples/gui/hello_form.rr", src])
    studio = start_studio(src, "rapidr-light", "wait,wait,view.code,wait")
    check("[editor] Studio in front", wait_front(studio))
    key(studio, KEY["g"], ("cmd",))
    time.sleep(0.6)
    shot_window(studio, "b1-go-to-line-open")
    check("[editor] Cmd+G: the palette's box says ':'", ax_find(studio, ":", exact=True) is not None)
    key(studio, KEY["1"])
    key(studio, KEY["2"])
    time.sleep(0.4)
    shot_window(studio, "b2-go-to-line-typed")
    check("[editor] 12 typed after the ':' — the box says ':12', the row 'Go to line 12'", ax_find(studio, ":12", exact=True) is not None and ax_find(studio, "Go to line 12") is not None)
    key(studio, ENTER)
    time.sleep(0.8)
    shot_window(studio, "b3-go-to-line-done")
    check("[editor] Enter: the status bar says Ln 12, Col 1", ax_find(studio, "Ln 12, Col 1", exact=True) is not None)
    # d: a change, undone back to the saved text, redone
    key(studio, KEY["x"])
    # (the completion list that comes up, closed)
    key(studio, ESC)
    time.sleep(0.6)
    check("[editor] x typed: the tab says Not saved", until(lambda: ax_find(studio, "Not saved", exact=True), 3) is not None)
    shot_window(studio, "d1-changed")
    key(studio, KEY["z"], ("cmd",))
    time.sleep(0.8)
    shot_window(studio, "d2-undone")
    check("[editor] Cmd+Z back to the saved text: no longer Not saved", ax_find(studio, "Not saved", exact=True) is None)
    key(studio, KEY["z"], ("cmd", "shift"))
    time.sleep(0.8)
    shot_window(studio, "d3-redone")
    check("[editor] Cmd+Shift+Z: Not saved again", ax_find(studio, "Not saved", exact=True) is not None)
    key(studio, KEY["z"], ("cmd",))
    time.sleep(0.8)
    check("[editor] Cmd+Z: saved again", ax_find(studio, "Not saved", exact=True) is None)
    close_all()


def designer_walk(look):
    """c: the form's RGB colour and the logo loaded after the form."""
    d = f"{WORK}/helpers-{look}"
    os.makedirs(d, exist_ok=True)
    for f in ("helpers.rr", "logo.svg"):
        subprocess.run(["cp", f"{W}/examples/studio/{f}", f"{d}/{f}"])
    studio = start_studio(f"{d}/helpers.rr", LOOKS[look], "wait,wait,view.designer,wait")
    check(f"[designer {look}] Studio in front", wait_front(studio))
    time.sleep(1.0)
    path = shot_window(studio, f"c-designer-{look}")
    # (the designed form's label "Colours, pictures and fonts", read in the
    # capture: the form's face just above it, the logo 60 points left of it
    # — the capture's own pixels)
    ok_face = ok_logo = False
    detail = ""
    boxes = []
    if path:
        out = subprocess.run([PROBE, "ocr", path], capture_output=True, text=True)
        try:
            boxes = json.loads(out.stdout or "[]")
        except json.JSONDecodeError:
            boxes = []
    label = next((b for b in boxes if b["text"].startswith("Colours, pictures")), None)
    if path and label:
        from PIL import Image
        img = Image.open(path).convert("RGB")
        scale = img.width / wins(studio)[0]["w"]
        px = lambda x, y: img.getpixel((int(x), int(y)))
        face = px(label["x"] + label["w"] / 2, label["y"] - 6 * scale)
        lx, ly = label["x"] - 60 * scale, label["y"] + label["h"] / 2 - 24 * scale
        colours = {px(lx + 48 * scale * i / 8, ly + 48 * scale * j / 8) for i in range(1, 8) for j in range(1, 8)}
        # (the capture is in the display's colour space: the cream's
        # signature — warm, light — rather than its exact numbers; the
        # themes' clBtnFace is neutral or cool)
        ok_face = face[0] >= 230 and face[0] - face[2] >= 6
        ok_logo = len(colours) >= 3
        detail = f"face {face}, the logo's area {len(colours)} colours"
    check(f"[designer {look}] the form in RGB(250, 246, 238)", ok_face, detail)
    check(f"[designer {look}] the logo loaded after the form shows", ok_logo, detail)
    close_all()


def debug_walk(look):
    """e (and j in light): runs on — Running…; Pause — filled; Cmd+C copies."""
    src = f"{WORK}/runs_on.rr"
    subprocess.run(["cp", f"{W}/tests/fixtures/studio_debug/runs_on.rr", src])
    # (the breakpoint by Studio's own step — F9 itself is the editor path's
    # and the S11 walk's —; F5 and the rest real keys)
    studio = start_studio(src, LOOKS[look], "wait,wait,view.code,line:12,debug.toggleBreakpoint,wait")
    check(f"[debug {look}] Studio in front", wait_front(studio))
    time.sleep(1.5)
    key(studio, F5)
    ok = until(lambda: node_with(studio, "Locals"), 10)
    activate(studio)
    shot_window(studio, f"e1-paused-{look}")
    check(f"[debug {look}] F5: paused at 12, Variables shows Locals", ok is not None)
    key(studio, F5)
    time.sleep(1.5)
    activate(studio)
    path = shot_window(studio, f"e2-running-{look}")
    check(f"[debug {look}] F5, it runs on: Variables says Running… the values show when it pauses", ax_find(studio, "Running… the values show when it pauses") is not None)
    # (the Call Stack's tab: of the nodes named so, the short one)
    tab = min((n for w in ax(studio) for n in walk(w) if (n.get("title") or n.get("desc")) == "Call Stack" and n.get("h", 0) > 4 and "x" in n), key=lambda n: n["h"], default=None)
    if tab:
        click_at(studio, tab["x"] + tab["w"] / 2, tab["y"] + tab["h"] / 2)
        time.sleep(0.6)
    shot_window(studio, f"e2-running-callstack-{look}")
    check(f"[debug {look}] … and Call Stack (its tab clicked): Running…", ax_find(studio, "Running…", exact=True) is not None)
    menu_click(studio, "Debug", "Pause")
    ok = until(lambda: node_with(studio, "Locals"), 8)
    activate(studio)
    shot_window(studio, f"e3-paused-again-{look}")
    check(f"[debug {look}] Debug ▸ Pause: filled again (Locals), no Running…", ok is not None and ax_find(studio, "Running…", exact=True) is None)
    if look == "light":
        # j: a value picked, Cmd+C
        k = until(lambda: node_with(studio, "k, ") or node_with(studio, "k = "), 4)
        if not check("[debug light] Variables shows k", k is not None):
            close_all()
            return
        click_at(studio, k["x"] + 40, k["y"] + k["h"] / 2)
        key(studio, KEY["c"], ("cmd",))
        time.sleep(1.2)
        clip = subprocess.run(["pbpaste"], capture_output=True, text=True).stdout.strip()
        shot_window(studio, "j-copied")
        check("[debug light] Cmd+C in Variables: k's value on the clipboard", clip.isdigit() and int(clip) > 0, repr(clip))
        check("[debug light] the status bar: Copied the value", ax_find(studio, "Copied the value") is not None)
    key(studio, F5, ("shift",))
    time.sleep(1.0)
    close_all()


def status_walk(look):
    """k: a long message whole in the status bar."""
    name = "a_program_whose_name_is_far_too_long_for_the_status_bar.rr"
    src = f"{WORK}/{name}"
    subprocess.run(["cp", f"{W}/tests/fixtures/studio_debug/{name}", src])
    studio = start_studio(src, LOOKS[look], "wait,wait,view.code,line:6")
    check(f"[status {look}] Studio in front", wait_front(studio))
    key(studio, F9)
    key(studio, F5)
    want = "Breakpoint: " + name + ":6"
    ok = until(lambda: ax_find(studio, want, exact=True) or ax_find(studio, "  " + want, exact=True), 10)
    activate(studio)
    shot_window(studio, f"k-status-{look}")
    check(f"[status {look}] the status bar says all of it: {want}", ok is not None)
    key(studio, F5, ("shift",))
    time.sleep(1.0)
    close_all()


def ai_walk(look):
    """i (and g): Set Up RAI with the mock as Ollama; Finish fills the
    Assistant's own list at once."""
    mock = subprocess.Popen(["node", f"{W}/tests/ai_mock_server.mjs", "0"], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    started.append(mock)
    url = mock.stdout.readline().strip()
    if not check(f"[ai {look}] the AI mock (no network, no key) at {url}", url.startswith("http://127.0.0.1:")):
        close_all()
        return
    src = f"{WORK}/hello_form.rr"
    subprocess.run(["cp", f"{W}/examples/gui/hello_form.rr", src])
    studio = start_studio(src, LOOKS[look], f"wait,wait,ai:setupprobes={url}|http://127.0.0.1:9/v1|http://127.0.0.1:9/v1")
    check(f"[ai {look}] Studio in front", wait_front(studio))
    # (Studio's start may still be settling: the menu tried again once when
    # the wizard isn't there)
    for _ in range(2):
        activate(studio)
        try:
            menu_click(studio, "Tools", "Set Up RAI...")
        except SystemExit:
            menu_click(studio, "Tools", "Set Up RAI…")
        if until(lambda: ax_find(studio, "Found on this computer"), 5):
            break
    time.sleep(1.0)
    shot_window(studio, f"i1-setup-{look}")
    check(f"[ai {look}] Set Up RAI: Ollama found (the mock)", until(lambda: ax_find(studio, "Found on this computer: Ollama"), 6) is not None)
    ax_click(studio, "Next >")
    time.sleep(1.0)
    shot_window(studio, f"i2-setup-model-{look}")
    ax_click(studio, "Finish")
    time.sleep(2.5)
    activate(studio)
    path = shot_window(studio, f"i3-finished-{look}")
    check(f"[ai {look}] Finish: RAI is ready: mock-small", ax_find(studio, "RAI is ready: mock-small.") is not None)
    box = ax_find(studio, "mock-small", exact=True)
    check(f"[ai {look}] the Assistant's model list filled at Finish (mock-large listed without ↻)", ax_find(studio, "mock-large") is not None or "mock-large" in (ocr(path) if path else ""), str(box)[:120])
    close_all()


TREE_PROGRAM = '''$THEME rapidr
DIM i AS INTEGER
CREATE Form AS RForm
    Caption = "Tree scroll": Width = 420: Height = 520
    CREATE Bottom AS RPanel
        Align = alBottom: Height = 26: BevelOuter = 0
        CREATE Status AS RStatusBar
            Align = alClient: SimplePanel = 1
        END CREATE
    END CREATE
    CREATE LeftPane AS RPanel
        Align = alLeft: Width = 270: BevelOuter = 0
        CREATE Head AS RPanel
            Align = alTop: Height = 36: BevelOuter = 0
        END CREATE
        CREATE Tree AS RTreeView
            Align = alClient
            ReadOnly = 1
        END CREATE
    END CREATE
END CREATE
Tree.AddItems "bookshop.db"
FOR i = 1 TO 47
    Tree.AddChildItems 0, "table_" + STR$(i)
NEXT
Tree.AddItems "shop.csv (CSV)"
Tree.Item(0).Expanded = 1
Status.SimpleText = "Opened BPAnalytics DB: 47 tables. Press Run (F5) to run the query."
Form.ShowModal
'''


def tree_walk(look):
    """n, o: Database Studio's tree (a root of 47, then a CSV) — the wheel,
    the bar's thumb dragged to its end, End: the last row whole; the status
    bar's text 10 pixels in."""
    src = f"{WORK}/tree-{look}.rr"
    theme = {"light": "rapidr light", "dark": "rapidr dark", "contrast": "rapidr high contrast"}[look]
    open(src, "w").write(TREE_PROGRAM.replace("$THEME rapidr", f"$THEME {theme}", 1))
    p = subprocess.Popen([f"{W}/rapidr", "run", src], cwd=WORK, env=ENV, stdout=open(f"{OUT}/tree-{look}.log", "w"), stderr=subprocess.STDOUT)
    started.append(p)
    for _ in range(60):
        if wins(p.pid):
            break
        time.sleep(0.25)
    time.sleep(1.0)
    activate(p.pid)
    prog = p.pid
    check(f"[tree {look}] the program in front", wait_front(prog))
    # (the tree itself, by its role: an outline)
    tree = until(lambda: next((n for w in ax(prog) for n in walk(w) if n.get("role") == "AXOutline" and n.get("h", 0) > 40), None), 4)
    last = lambda: node_with(prog, "shop.csv (CSV)")

    def whole():
        t, r = tree, last()
        return bool(t and r and r.get("h", 0) > 4 and r["y"] >= t["y"] and r["y"] + r["h"] <= t["y"] + t["h"])
    if not check(f"[tree {look}] the tree found (VoiceOver)", tree is not None, str(tree)[:120]):
        close_all()
        return
    log(f"  (the tree at {tree['x']:.0f}, {tree['y']:.0f}, {tree['w']:.0f} x {tree['h']:.0f})")

    def wheel(lines):
        x, y = tree["x"] + tree["w"] / 2, tree["y"] + tree["h"] / 2
        if not mine_under(prog, x, y):
            raise SystemExit("stopped: the tree isn't under the mouse")
        for _ in range(12):
            e = Quartz.CGEventCreateScrollWheelEvent(None, Quartz.kCGScrollEventUnitLine, 1, lines)
            Quartz.CGEventSetLocation(e, (x, y))
            Quartz.CGEventPost(Quartz.kCGHIDEventTap, e)
            time.sleep(0.05)
        time.sleep(0.6)
    # the wheel, to the end
    wheel(-5)
    path1 = shot_window(prog, f"n1-tree-wheel-{look}")
    check(f"[tree {look}] the wheel to the end: shop.csv (CSV) whole in the tree", whole(), str(last())[:160])
    # back to the top by the wheel, then the bar's thumb dragged to the bottom
    wheel(5)
    bx, by = tree["x"] + tree["w"] - 10, tree["y"] + 30
    if check(f"[tree {look}] the bar's thumb is ours under the mouse", mine_in_front(prog) and mine_under(prog, bx, by)):
        mouse(Quartz.kCGEventMouseMoved, bx, by)
        time.sleep(0.1)
        mouse(Quartz.kCGEventLeftMouseDown, bx, by)
        for k in range(1, 13):
            mouse(Quartz.kCGEventLeftMouseDragged, bx, by + (tree["h"] + 10 - 30) * k / 12)
            time.sleep(0.04)
        mouse(Quartz.kCGEventLeftMouseUp, bx, tree["y"] + tree["h"] + 10)
        time.sleep(0.6)
    shot_window(prog, f"n2-tree-thumb-{look}")
    check(f"[tree {look}] the thumb dragged to the bottom: shop.csv (CSV) whole", whole(), str(last())[:160])
    # a row clicked (the tree has the keys), Home, End: the selection's row in view
    r = last()
    if r:
        click_at(prog, r["x"] + 40, r["y"] + r["h"] / 2)
    key(prog, 115)  # Home
    time.sleep(0.4)
    shot_window(prog, f"n3-tree-home-{look}")
    check(f"[tree {look}] Home: the first row in view again", node_with(prog, "bookshop.db") is not None and node_with(prog, "bookshop.db")["y"] >= tree["y"])
    key(prog, 119)  # End
    time.sleep(0.6)
    shot_window(prog, f"n4-tree-end-{look}")
    check(f"[tree {look}] End: shop.csv (CSV) whole in the tree", whole(), str(last())[:160])
    # o: the status bar's text about 10 pixels in (the window's capture: Retina's 2 a point)
    boxes = []
    if path1:
        out = subprocess.run([PROBE, "ocr", path1], capture_output=True, text=True)
        try:
            boxes = json.loads(out.stdout or "[]")
        except json.JSONDecodeError:
            boxes = []
    line = next((b for b in boxes if b["text"].startswith("Opened")), None)
    ws = wins(prog)
    left = None
    if line and ws:
        scale = AppKit.NSImage.alloc().initWithContentsOfFile_(path1).representations()[0].pixelsWide() / ws[0]["w"]
        left = round(line["x"] / scale, 1)
    check(f"[status {look}] the status bar's text about 10 pixels in (OCR: {left})", left is not None and 8 <= left <= 14)
    close_all()


try:
    paths = sys.argv[1:] or ["editor", "designer", "debug", "status", "ai", "tree"]
    t0 = time.time()
    if "editor" in paths:
        editor_walk()
    for look in ("light", "dark", "contrast"):
        if "designer" in paths or f"designer-{look}" in paths:
            designer_walk(look)
        if "debug" in paths or f"debug-{look}" in paths:
            debug_walk(look)
        if "status" in paths or f"status-{look}" in paths:
            status_walk(look)
        if "tree" in paths or f"tree-{look}" in paths:
            tree_walk(look)
    for look in ("light", "dark"):
        if "ai" in paths or f"ai-{look}" in paths:
            ai_walk(look)
    log(f"walk took {time.time() - t0:.0f} s")
finally:
    close_all()
    clear_modifiers()
    json.dump(results, open(f"{OUT}/results.json", "w"), indent=1)
    fails = [r for r in results if not r["ok"]]
    log(f"{len(results) - len(fails)} passed, {len(fails)} failed")
