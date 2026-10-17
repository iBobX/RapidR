#!/usr/bin/env python3
"""A walk through a RapidR program on the real screen (macOS), as a person
uses it: elements found through the accessibility tree VoiceOver gets
(tools/macos/ui_probe.swift), clicked and typed into with real input
(tools/real_input.py, strict: it never brings another app's window forward,
and stops if the program isn't in front — the person at the Mac may click
elsewhere), what shows read back (the tree's values, enabled states, a
dropped list's items) and captured. The lane rules' "USED like a user":
take the screen lock first, close every window after.

  real_walk.py front PID                  bring PID's window forward (the walk's start)
  real_walk.py ls PID [filter]            the front window's tree: role, title, value, enabled, frame
  real_walk.py get PID NAME [ROLE]        an element (by its title or accessible name) as JSON
  real_walk.py click PID NAME [ROLE] [n]  click the n-th element named NAME
  real_walk.py at PID X Y                 click a point of the screen
  real_walk.py text PID TEXT [DX] [n]     click the line of text TEXT on screen (macOS' Vision
                                          reads the window: a dropped list's item, a link)
  real_walk.py droplist PID NAME          open a combo box's list (its arrow)
  real_walk.py type PID TEXT              type TEXT (never a real key: a test's made-up one)
  real_walk.py key PID SPEC               a key (real_input's k: — cmd+0 is Cmd+A, 36 Enter, 53 Escape)
  real_walk.py shot PID NAME              the front window -> $WALK_OUT/NAME.png
  real_walk.py cropshot PID NAME OUT      the front window from combo NAME down (a dropped list,
                                          nothing above it — where a key's hint is)

UI_PROBE names the compiled probe (default: target/ui_probe, built with
swiftc on first use); WALK_OUT the captures' folder (default
tests/results/real-walk).
"""
import json, os, subprocess, sys

W = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROBE = os.environ.get("UI_PROBE") or os.path.join(W, "target", "ui_probe")
OUT = os.environ.get("WALK_OUT") or os.path.join(W, "tests", "results", "real-walk")
os.makedirs(OUT, exist_ok=True)
if not os.path.exists(PROBE):
    subprocess.run(["swiftc", "-O", os.path.join(W, "tools", "macos", "ui_probe.swift"), "-o", PROBE], check=True)
ENV = dict(os.environ, RAPIDR_REAL_INPUT_STRICT="1")


def ri(pid, *acts, strict=True):
    env = ENV if strict else dict(os.environ)
    r = subprocess.run(["python3", os.path.join(W, "tools", "real_input.py"), str(pid), *acts], env=env, capture_output=True, text=True)
    if r.returncode:
        sys.exit(f"real_input refused: {r.stderr.strip() or r.stdout.strip()}")


def tree(pid):
    r = subprocess.run([PROBE, "tree", str(pid)], capture_output=True, text=True)
    return json.loads(r.stdout or "[]")


def walk(n, d=0):
    yield n, d
    for c in n.get("children", []):
        yield from walk(c, d + 1)


def front_window(pid):
    ws = tree(pid)
    return ws[0] if ws else None


def find(pid, name, role=None, nth=0, everywhere=False):
    found = []
    for w in (tree(pid) if everywhere else [front_window(pid)]):
        if not w:
            continue
        for n, _ in walk(w):
            if (n.get("title") == name or n.get("desc") == name) and (role is None or n.get("role") == role) and "x" in n:
                found.append(n)
    return found[nth] if len(found) > nth else None


def center(n):
    return n["x"] + n["w"] / 2, n["y"] + n["h"] / 2


def norm(t):
    return t.lower().replace("l", "i").replace("0", "o").replace("—", "-").replace("…", "...")


def main():
    cmd, pid = sys.argv[1], int(sys.argv[2])
    a = sys.argv[3:]
    if cmd == "front":
        ri(pid, "w:0.2", strict=False)
    elif cmd == "ls":
        w = front_window(pid)
        flt = a[0].lower() if a else ""
        for n, d in walk(w):
            line = f"{'  ' * d}{n['role']} t={n.get('title')!r} d={n.get('desc')!r} v={n.get('value', '')[:70]!r} en={n.get('enabled')} kids={len(n.get('children', []))} @{int(n.get('x', -1))},{int(n.get('y', -1))} {int(n.get('w', 0))}x{int(n.get('h', 0))}"
            if not flt or flt in line.lower():
                print(line)
    elif cmd == "get":
        n = find(pid, a[0], a[1] if len(a) > 1 and a[1] else None)
        if not n:
            sys.exit(f"no {a[0]}")
        kids = n.pop("children", [])
        n["children"] = [k.get("value") or k.get("title") for k in kids]
        print(json.dumps(n))
    elif cmd == "click":
        n = find(pid, a[0], a[1] if len(a) > 1 and a[1] else None, int(a[2]) if len(a) > 2 else 0)
        if not n:
            sys.exit(f"no {a[0]}")
        x, y = center(n)
        ri(pid, f"C:{x:.0f},{y:.0f}")
        print(f"clicked {a[0]} at {x:.0f},{y:.0f}")
    elif cmd == "at":
        ri(pid, f"C:{a[0]},{a[1]}")
    elif cmd == "text":
        shot = f"{OUT}/_ocr.png"
        ri(pid, f"s:{shot}")
        w = front_window(pid)
        lines = json.loads(subprocess.run([PROBE, "ocr", shot], capture_output=True, text=True).stdout)
        # (the whole window is in it — a key's last characters too: not kept)
        os.remove(shot)
        want = norm(a[0])
        exact = [l for l in lines if norm(l["text"]).strip(" ✓✔×x") == want]
        hits = exact or sorted([l for l in lines if want in norm(l["text"])], key=lambda l: l["y"])
        if not hits:
            sys.exit(f"no text {a[0]!r} on screen: {[l['text'] for l in lines][:40]}")
        h = hits[int(a[2]) if len(a) > 2 else 0]
        dx = float(a[1]) if len(a) > 1 and a[1] else 0
        # (the capture is the window's frame at 2x)
        sx = w["x"] + (h["x"] + 12) / 2 + dx
        sy = w["y"] + (h["y"] + h["h"] / 2) / 2
        ri(pid, f"C:{sx:.0f},{sy:.0f}")
        print(f"clicked text {h['text']!r} at {sx:.0f},{sy:.0f}")
    elif cmd == "droplist":
        n = find(pid, a[0], "AXPopUpButton")
        ri(pid, f"C:{n['x'] + n['w'] - 10:.0f},{n['y'] + n['h'] / 2:.0f}")
    elif cmd == "cropshot":
        # (the front window from element a[0]'s row down, 380 points: what a
        # dropped list shows — nothing above it, where a key's hint is)
        n = find(pid, a[0], "AXPopUpButton")
        w = front_window(pid)
        full = f"{OUT}/_full.png"
        ri(pid, "w:0.3", f"s:{full}")
        top = int((n["y"] - w["y"] - 25) * 2)
        subprocess.run(["sips", "-c", "760", str(int(w["w"] * 2)), "--cropOffset", str(top), "0", full, "--out", f"{OUT}/{a[1]}.png"], capture_output=True)
        os.remove(full)
        print(f"{OUT}/{a[1]}.png")
    elif cmd == "type":
        ri(pid, f"t:{a[0]}")
    elif cmd == "key":
        ri(pid, f"k:{a[0]}")
    elif cmd == "shot":
        path = f"{OUT}/{a[0]}.png"
        ri(pid, "w:0.3", f"s:{path}")
        print(path)
    else:
        sys.exit("unknown " + cmd)


main()
