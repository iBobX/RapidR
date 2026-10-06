#!/usr/bin/env python3
"""RapidR's approved-image gallery (tests/visual/README.md): every case —
tests/visual/cases/*.bas, a few fixtures and the GUI examples — captured by
the UI kernel's headless host at 1× and 2× in each theme, compared with the
approved images (the goldens), and shown beside RapidQ's own screenshots in
an HTML contact sheet.

    tools/visual/gallery.py check [case …]     capture, compare with the goldens
                                              (exit 1 on a difference), write the sheet
    tools/visual/gallery.py capture [case …]   capture only (and the sheet)
    tools/visual/gallery.py approve case … | --all
                                              make the last captures the goldens
    tools/visual/gallery.py sheet              the sheet from what's there

Captures go to tests/visual/out/<theme>/<case>@<scale>x-<window>.png, with
a diff image beside each that differs from its golden (…-diff.png) and the
sheet, tests/visual/out/index.html. Goldens are tests/visual/golden/<theme>/
(classic at 1× and 2×, the other themes at 1×: GOLDEN_SCALES); RapidQ's
screenshots, tests/visual/rapidq/ (tools/visual/rq_shots.sh makes them).

Needs ./rapidr (cargo build --release -p rapidr-cli; copied to the
repository's root) and Pillow. Programs run with RAPIDR_PRINT_TO and
RAPIDR_REGISTRY pointed into tests/visual/out: nothing reaches a printer or
the user's registry.
"""
import concurrent.futures
import glob
import html
import os
import re
import shutil
import subprocess
import sys
import tempfile

try:
    from PIL import Image, ImageChops
except ImportError:  # pragma: no cover
    sys.exit("tools/visual/gallery.py needs Pillow (python3 -m pip install pillow)")

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
VIS = os.path.join(ROOT, "tests", "visual")
OUT = os.path.join(VIS, "out")
GOLDEN = os.path.join(VIS, "golden")
RAPIDQ = os.path.join(VIS, "rapidq")
RAPIDR = os.environ.get("RAPIDR_BIN", os.path.join(ROOT, "rapidr"))
THEMES = ["classic", "modern", "dark", "highcontrast"]
SCALES = [1, 2]
# What's approved (and checked): classic at both scales, the others at 1×.
GOLDEN_SCALES = {"classic": [1, 2], "modern": [1], "dark": [1], "highcontrast": [1]}
# A pixel is the same when no channel differs by more than this (the
# renderer is deterministic; this only forgives a platform's rounding).
TOLERANCE = 3

# Cases beyond tests/visual/cases: (name, path[, test hooks]) — fixtures
# and the GUI examples (run in their own folder, as `rapidr run` does).
EXTRA = [
    ("fixture-a11y_form", "tests/fixtures/a11y_form.bas"),
    ("fixture-themes", "tests/fixtures/themes.bas"),
    ("example-hello_form", "examples/gui/hello_form.rr"),
    ("example-dialogs", "examples/gui/dialogs.rr"),
    ("example-dialogs-font", "examples/gui/dialogs.rr", {"RAPIDR_TEST_EVENTS": "FontBtn.onclick", "RAPIDR_TEST_FONT_DIALOG": "", "RAPIDR_TEST_DIALOG_HOLD": "4000"}),
    ("example-menus", "examples/gui/menus.rr"),
    ("example-pantry", "examples/gui/pantry.rr"),
    ("example-stopwatch", "examples/gui/stopwatch.rr"),
    ("example-themes", "examples/gui/themes.rr"),
    ("example-canvas", "examples/graphics/canvas.rr"),
    ("example-notepad", "examples/rapidq/notepad.bas"),
    ("example-dataframe", "examples/data/dataframe.rr", {"RAPIDR_TEST_EVENTS": "ChartBtn.onclick"}),
    ("example-wave", "examples/media/wave.rr"),
    ("example-video", "examples/media/video.rr"),
]


def cases():
    out = []
    for p in sorted(glob.glob(os.path.join(VIS, "cases", "*.bas"))):
        if p.endswith(".rapidq.bas"):
            continue
        out.append((os.path.splitext(os.path.basename(p))[0], p))
    for name, rel, *env in EXTRA:
        p = os.path.join(ROOT, rel)
        if os.path.exists(p):
            EXTRA_ENV[name] = env[0] if env else {}
            out.append((name, p))
    return out


EXTRA_ENV = {}


def case_env(path):
    """`' rapidr-env: K=V …` lines in a case: its test hooks."""
    env = {}
    try:
        text = open(path, encoding="utf-8", errors="replace").read()
    except OSError:
        return env
    for m in re.finditer(r"(?im)^'\s*rapidr-env:\s*(.+?)\s*$", text):
        for kv in m.group(1).split():
            k, _, v = kv.partition("=")
            env[k] = v
    return env


def capture_one(name, path, theme, scale, work):
    """Runs the case once; its windows' captures as PNGs. Returns the PNG paths."""
    rrbc = os.path.join(work, name + ".rrbc")
    tmp = tempfile.mkdtemp(prefix=f"{name}-{theme}-{scale}-", dir=work)
    env = dict(os.environ)
    env.update({
        "RAPIDR_CAPTURE": os.path.join(tmp, "cap"),
        "RAPIDR_SCALE": str(scale),
        "RAPIDR_THEME": theme,
        "RAPIDR_PRINT_TO": os.path.join(work, "prints"),
        "RAPIDR_REGISTRY": os.path.join(tmp, "registry.reg"),
    })
    env.update(case_env(path))
    env.update(EXTRA_ENV.get(name, {}))
    try:
        subprocess.run([RAPIDR, "run-bc", rrbc], cwd=os.path.dirname(path), env=env,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=40)
    except subprocess.TimeoutExpired:
        pass
    shots = []
    for bmp in sorted(glob.glob(os.path.join(tmp, "cap-*.bmp")), key=lambda p: int(re.findall(r"(\d+)\.bmp$", p)[0])):
        n = re.findall(r"(\d+)\.bmp$", bmp)[0]
        dest = os.path.join(OUT, theme, f"{name}@{scale}x-{n}.png")
        Image.open(bmp).convert("RGB").save(dest, optimize=True)
        shots.append(dest)
    shutil.rmtree(tmp, ignore_errors=True)
    return shots


def capture(selected):
    if not os.path.exists(RAPIDR):
        sys.exit(f"{RAPIDR} not found: cargo build --release -p rapidr-cli && cp target/release/rapidr .")
    work = os.path.join(OUT, ".work")
    os.makedirs(work, exist_ok=True)
    for t in THEMES:
        os.makedirs(os.path.join(OUT, t), exist_ok=True)
    jobs = []
    for name, path in selected:
        for t in THEMES:
            for f in glob.glob(os.path.join(OUT, t, f"{name}@*")):
                os.remove(f)
        r = subprocess.run([RAPIDR, "build-bc", path, "-o", os.path.join(work, name + ".rrbc")],
                           cwd=os.path.dirname(path), capture_output=True, text=True)
        if r.returncode != 0:
            print(f"  {name}: build failed: {(r.stderr or r.stdout).strip()[:300]}")
            continue
        for t in THEMES:
            for s in SCALES:
                jobs.append((name, path, t, s))
    with concurrent.futures.ThreadPoolExecutor(max_workers=os.cpu_count() or 4) as pool:
        list(pool.map(lambda j: capture_one(*j, work), jobs))
    shutil.rmtree(work, ignore_errors=True)


def differs(a_path, b_path, diff_path):
    """None when the same (within TOLERANCE), else a reason; a diff image."""
    a, b = Image.open(a_path).convert("RGB"), Image.open(b_path).convert("RGB")
    if a.size != b.size:
        return f"size {b.size[0]}x{b.size[1]} -> {a.size[0]}x{a.size[1]}"
    d = ImageChops.difference(a, b)
    mask = d.convert("L").point(lambda v: 255 if v > TOLERANCE else 0)
    count = mask.histogram()[255]
    if not count:
        return None
    # (the capture, greyed, the changed pixels red)
    shown = Image.blend(a, Image.new("RGB", a.size, (255, 255, 255)), 0.6)
    shown.paste((255, 0, 0), mask=mask)
    shown.save(diff_path)
    return f"{count} pixels"


def check(selected):
    failed = []
    names = {n for n, _ in selected}
    for t in THEMES:
        for s in GOLDEN_SCALES[t]:
            for g in sorted(glob.glob(os.path.join(GOLDEN, t, f"*@{s}x-*.png"))):
                base = os.path.basename(g)
                name = base.split("@")[0]
                if name not in names:
                    continue
                cap = os.path.join(OUT, t, base)
                if not os.path.exists(cap):
                    failed.append((t, base, "not captured"))
                    continue
                why = differs(cap, g, cap[:-4] + "-diff.png")
                if why:
                    failed.append((t, base, why))
            for cap in sorted(glob.glob(os.path.join(OUT, t, f"*@{s}x-*.png"))):
                base = os.path.basename(cap)
                if base.endswith("-diff.png") or base.split("@")[0] not in names:
                    continue
                if not os.path.exists(os.path.join(GOLDEN, t, base)):
                    failed.append((t, base, "no golden (approve it)"))
    return failed


def approve(names):
    n = 0
    for t in THEMES:
        os.makedirs(os.path.join(GOLDEN, t), exist_ok=True)
        for s in GOLDEN_SCALES[t]:
            for name in names:
                for f in glob.glob(os.path.join(GOLDEN, t, f"{name}@{s}x-*.png")):
                    os.remove(f)
                for cap in glob.glob(os.path.join(OUT, t, f"{name}@{s}x-*.png")):
                    if cap.endswith("-diff.png"):
                        continue
                    shutil.copy(cap, os.path.join(GOLDEN, t, os.path.basename(cap)))
                    n += 1
    print(f"approved {n} images")


def sheet(failed=()):
    bad = {(t, b): why for t, b, why in failed}
    rel = lambda p: os.path.relpath(p, OUT)
    names = [n for n, _ in cases()]
    rows = []
    for name in names:
        windows = sorted({re.findall(r"-(\d+)\.png$", p)[0] for p in glob.glob(os.path.join(OUT, "*", f"{name}@*x-*.png")) + glob.glob(os.path.join(RAPIDQ, f"{name}@*x-*.png")) if not p.endswith("-diff.png")}, key=int)
        for w in windows:
            cells = []
            for s in SCALES:
                rq = os.path.join(RAPIDQ, f"{name}@{s}x-{w}.png")
                cells.append(("RapidQ", s, rq if os.path.exists(rq) else None, None))
                for t in THEMES:
                    cap = os.path.join(OUT, t, f"{name}@{s}x-{w}.png")
                    why = bad.get((t, os.path.basename(cap)))
                    diff = cap[:-4] + "-diff.png"
                    cells.append((t, s, cap if os.path.exists(cap) else None, (why, diff if os.path.exists(diff) else None) if why else None))
            tds = []
            for label, s, img, fail in cells:
                if not img:
                    tds.append(f"<td class=empty><div class=lab>{label} {s}×</div>—</td>")
                    continue
                im = Image.open(img)
                width = im.size[0] // s
                cls = " class=fail" if fail else ""
                extra = ""
                if fail:
                    why, diff = fail
                    extra = f"<div class=why>{html.escape(why)}</div>" + (f"<a href='{rel(diff)}'>diff</a>" if diff else "")
                tds.append(f"<td{cls}><div class=lab>{label} {s}×</div><a href='{rel(img)}'><img src='{rel(img)}' width={width}></a>{extra}</td>")
            rows.append(f"<tr><th>{html.escape(name)}<br><small>window {w}</small></th>{''.join(tds)}</tr>")
    status = f"{len(failed)} differences from the goldens" if failed else "every capture matches its golden"
    page = f"""<!doctype html><meta charset=utf-8><title>RapidR visual gallery</title>
<style>
body {{ font: 13px system-ui, sans-serif; margin: 16px; background: #fafafa; color: #222 }}
table {{ border-collapse: collapse }} td, th {{ border: 1px solid #ddd; padding: 6px; vertical-align: top; background: #fff }}
th {{ text-align: left; position: sticky; left: 0; background: #f3f3f3 }}
.lab {{ font-size: 11px; color: #666; margin-bottom: 4px }} td.fail {{ background: #ffecec }} .why {{ color: #b00; font-size: 11px }}
img {{ image-rendering: auto; display: block }} td.empty {{ color: #aaa }}
</style>
<h1>RapidR visual gallery</h1>
<p>{status}. Each row: RapidQ's own window (RC.EXE on Windows 11, unthemed: the classic look) beside RapidR's in every theme, at 1× and 2× (2× shown at half size: crisp on a high-DPI screen). Goldens: classic at 1× and 2×, the other themes at 1×.</p>
<table>{''.join(rows)}</table>"""
    os.makedirs(OUT, exist_ok=True)
    with open(os.path.join(OUT, "index.html"), "w") as f:
        f.write(page)
    print(f"sheet: {os.path.join(OUT, 'index.html')}")


def main():
    args = sys.argv[1:]
    if not args or args[0] not in ("check", "capture", "approve", "sheet"):
        sys.exit(__doc__)
    cmd, rest = args[0], args[1:]
    all_cases = cases()
    if cmd == "approve":
        names = [n for n, _ in all_cases] if rest == ["--all"] else rest
        if not names:
            sys.exit("approve which cases? (names, or --all)")
        approve(names)
        return
    if cmd == "sheet":
        sheet()
        return
    selected = [c for c in all_cases if not rest or c[0] in rest]
    unknown = set(rest) - {n for n, _ in all_cases}
    if unknown:
        sys.exit(f"no such case: {', '.join(sorted(unknown))}")
    capture(selected)
    if cmd == "capture":
        sheet()
        return
    failed = check(selected)
    sheet(failed)
    for t, b, why in failed:
        print(f"  ✗ {t}/{b}: {why}")
    print(f"visual: {len(failed)} differences" if failed else f"visual: {len(selected)} cases match their goldens")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
