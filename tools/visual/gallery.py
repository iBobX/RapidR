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

Studio's own goldens (tests/visual/README.md): the scenes of
tools/visual/studio_scenes.py — what a person sees in RapidR Studio —
captured in RapidR light, dark and high contrast and compared, pixel for
pixel, with tests/visual/studio/<look>/<scene>@<s>x.png:

    tools/visual/gallery.py studio check [scene …]     capture, compare (exit 1 on a difference)
    tools/visual/gallery.py studio capture [scene …]   capture only
    tools/visual/gallery.py studio approve scene … | --all
                                              make the last captures the approved images
    tools/visual/gallery.py studio list        the scenes and what each guards
    (narrow a run with --theme light|dark|contrast and --scale 1|2; --jobs N
    paces it; RAPIDR_STUDIO_TOLERANCE=n forgives n levels of a colour channel)

Captures go to tests/visual/out/<theme>/<case>@<scale>x-<window>.png, with
a diff image beside each that differs from its golden (…-diff.png) and the
sheet, tests/visual/out/index.html. Goldens are tests/visual/golden/<theme>/
(classic at 1× and 2×, the other themes at 1×: GOLDEN_SCALES); RapidQ's
screenshots, tests/visual/rapidq/ (tools/visual/rq_shots.sh makes them).

Studio's captures go to tests/visual/out/studio/<look>/<scene>@<s>x.png;
a scene that differs also leaves -compare.png (approved | now | what
changed), -zoom.png (the changed part, enlarged) and -diff.png beside it,
named in the output, and tests/visual/out/studio/index.html shows them all.

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
    from PIL import Image, ImageChops, ImageDraw
except ImportError:  # pragma: no cover
    sys.exit("tools/visual/gallery.py needs Pillow (python3 -m pip install pillow)")

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
VIS = os.path.join(ROOT, "tests", "visual")
OUT = os.path.join(VIS, "out")
GOLDEN = os.path.join(VIS, "golden")
RAPIDQ = os.path.join(VIS, "rapidq")
RAPIDR = os.environ.get("RAPIDR_BIN", os.path.join(ROOT, "rapidr"))
THEMES = ["classic", "rapidr-light", "rapidr-dark", "rapidr-high-contrast"]
SCALES = [1, 2]
# What's approved (and checked): classic at both scales, the others at 1×.
GOLDEN_SCALES = {"classic": [1, 2], "rapidr-light": [1], "rapidr-dark": [1], "rapidr-high-contrast": [1]}
# A pixel is the same when no channel differs by more than this (the
# renderer is deterministic; this only forgives a platform's rounding).
TOLERANCE = 3

# Cases beyond tests/visual/cases: (name, path[, test hooks]) — fixtures
# and the GUI examples (run in their own folder, as `rapidr run` does).
EXTRA = [
    ("fixture-a11y_form", "tests/fixtures/a11y_form.bas"),
    ("fixture-themes", "tests/fixtures/themes.bas"),
    ("fixture-rplot_on_form", "tests/fixtures/rplot_on_form.bas", {"RAPIDR_TEST_EVENTS": "btn.onclick"}),
    ("example-hello_form", "examples/gui/hello_form.rr"),
    ("example-dialogs", "examples/gui/dialogs.rr"),
    ("example-dialogs-font", "examples/gui/dialogs.rr", {"RAPIDR_TEST_EVENTS": "FontBtn.onclick", "RAPIDR_TEST_FONT_DIALOG": "", "RAPIDR_TEST_DIALOG_HOLD": "4000"}),
    ("example-menus", "examples/gui/menus.rr"),
    ("example-pantry", "examples/gui/pantry.rr"),
    ("example-richtext", "examples/gui/richtext.rr"),
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


def example_leftovers():
    """The files the examples write as they run (examples/**/*.tmp.*; ignored by the repository)."""
    return set(glob.glob(os.path.join(ROOT, "examples", "**", "*.tmp.*"), recursive=True))


def capture(selected):
    if not os.path.exists(RAPIDR):
        sys.exit(f"{RAPIDR} not found: cargo build --release -p rapidr-cli && cp target/release/rapidr .")
    # (a capture ends its program before the program's own clean-up — it
    # KILLs the files it extracted when it ends —, so the examples leave
    # their *.tmp.* files in their folders: the ones this run caused go at
    # its end, not before — the runs of an example share its folder)
    before = example_leftovers()
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
    for f in example_leftovers() - before:
        os.remove(f)


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


# ---- RapidR Studio's own goldens ------------------------------------------------------
# tests/visual/studio/<look>/<scene>@<s>x.png: what a person sees in Studio,
# approved. The scenes are tools/visual/studio_scenes.py's; each is Studio
# started as the tests start it and captured by the headless host. (The web
# build's captures are byte-identical to the desktop's — tests/studio_shell.mjs
# — so the desktop's are the ones kept.)
STUDIO_LOOKS = ["rapidr-light", "rapidr-dark", "rapidr-high-contrast"]
STUDIO_SHORT = {"light": "rapidr-light", "dark": "rapidr-dark", "contrast": "rapidr-high-contrast", "high-contrast": "rapidr-high-contrast"}
STUDIO_GOLDEN = os.path.join(VIS, "studio")
STUDIO_OUT = os.path.join(OUT, "studio")
# (the renderer is deterministic, to the byte, on every platform — the CPU
# renderer's portable SIMD level —, so nothing is forgiven unless asked)
STUDIO_TOLERANCE = int(os.environ.get("RAPIDR_STUDIO_TOLERANCE", "0"))
# A capture that differs is taken again (a loaded machine can make a step
# of the script late): it passes if a retake matches — and says so, so a
# scene that needs it is seen and fixed, not hidden.
STUDIO_RETAKES = 2


def studio_scenes():
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import studio_scenes
    return studio_scenes.SCENES


def studio_golden_path(look, scene, scale):
    return os.path.join(STUDIO_GOLDEN, look, f"{scene}@{scale}x.png")


def studio_out_path(look, scene, scale):
    return os.path.join(STUDIO_OUT, look, f"{scene}@{scale}x.png")


def studio_jobs(scenes, looks, scales):
    """(scene, look, scale) for what's asked: a scene's own scales, narrowed by `scales`."""
    for sc in scenes:
        for look in looks:
            for s in sc.get("scales", [1]):
                if not scales or s in scales:
                    yield sc, look, s


# (the AI scenes' provider has an address on the screen — Settings, the panel's
# footer, the steps' echo in Output —: the same one every run)
STUDIO_MOCK_PORT = 18695


def start_mock():
    """The AI mock provider (tests/ai_mock_server.mjs) in a process of its own: it and its URL."""
    proc = subprocess.Popen(["node", os.path.join(ROOT, "tests", "ai_mock_server.mjs"), str(STUDIO_MOCK_PORT)], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    url = proc.stdout.readline().strip()
    if not url.startswith("http"):
        proc.kill()
        sys.exit(f"tests/ai_mock_server.mjs didn't start: node is needed for the AI scenes, and port {STUDIO_MOCK_PORT} free")
    return proc, url


def studio_capture_one(scene, look, scale, mock_url, work_root):
    """Studio run once for `scene`: the window that is its image, as an RGB image (None: there is none)."""
    name = scene["name"]
    tmp = os.path.join(work_root, f"{name}-{look}@{scale}x")
    shutil.rmtree(tmp, ignore_errors=True)
    os.makedirs(tmp)
    steps = scene.get("do", "").replace("{mock}", mock_url or "")
    rel = ""
    if scene.get("work"):
        # (a folder of its own, relative to the repository: no machine's path is on the screen)
        rel = os.path.join("tests", "visual", "out", "studio", ".work", "scenes", os.path.basename(tmp))
        shutil.rmtree(os.path.join(ROOT, rel), ignore_errors=True)
        shutil.copytree(os.path.join(ROOT, scene["work"]), os.path.join(ROOT, rel))
        steps = steps.replace("{work}", rel)
    args = [RAPIDR, "run", "ide/studio.rr", "--home", ".", "--fresh", "--theme", look]
    if scene.get("open"):
        args.append(scene["open"].replace("{work}", rel) if scene.get("work") else scene["open"])
    if steps:
        args += ["--do", steps]
    env = dict(os.environ)
    env.update({
        "RAPIDR_CAPTURE": os.path.join(tmp, "window"),
        "RAPIDR_CAPTURE_DELAY": str(scene.get("delay", 5)),
        "RAPIDR_SCALE": str(scale),
        "RAPIDR_MENU": "window",
        "RAPIDR_PRINT_TO": os.path.join(work_root, "prints"),
        "RAPIDR_REGISTRY": os.path.join(tmp, "registry.reg"),
        # (the AI assistant: keys in memory only, none from this shell, its log the run's)
        "RAPIDR_AI_KEYSTORE": "memory",
        "RAPIDR_AI_AUDIT": os.path.join(tmp, "ai-audit.jsonl"),
        "ANTHROPIC_API_KEY": "", "OPENAI_API_KEY": "", "DEEPSEEK_API_KEY": "", "ANTHROPIC_WORKSPACE_ID": "",
    })
    env.update(scene.get("env", {}))
    if scene.get("events"):
        env["RAPIDR_TEST_EVENTS"] = scene["events"]
    else:
        env.pop("RAPIDR_TEST_EVENTS", None)
    try:
        subprocess.run(args, cwd=ROOT, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                       timeout=60 + 2 * scene.get("delay", 5))
    except subprocess.TimeoutExpired:
        pass
    bmp = os.path.join(tmp, f"window-{scene.get('window', 1)}.bmp")
    img = Image.open(bmp).convert("RGB") if os.path.exists(bmp) else None
    shutil.rmtree(tmp, ignore_errors=True)
    return img


def studio_diff_mask(a, b):
    """White where any colour channel of the two images differs by more than the tolerance."""
    masks = [c.point(lambda v: 255 if v > STUDIO_TOLERANCE else 0) for c in ImageChops.difference(a, b).split()]
    return ImageChops.lighter(ImageChops.lighter(masks[0], masks[1]), masks[2])


def studio_compare(img, golden_path):
    """None when `img` is the approved image, else (why, a mask of what changed — or None)."""
    if not os.path.exists(golden_path):
        return "no approved image yet", None
    g = Image.open(golden_path).convert("RGB")
    if g.size != img.size:
        return f"size {g.size[0]}x{g.size[1]} -> {img.size[0]}x{img.size[1]}", None
    mask = studio_diff_mask(img, g)
    count = mask.histogram()[255]
    if not count:
        return None
    x0, y0, x1, y1 = mask.getbbox()
    return f"{count} pixels differ, within x {x0}..{x1}, y {y0}..{y1}", mask


def studio_save_png(img, path):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    img.save(path, optimize=True)


def studio_panels(panels, path):
    """The (title, image) panels side by side on grey, each under its title: a PNG at `path`."""
    gap, band = 12, 22
    w = sum(p.size[0] for _, p in panels) + gap * (len(panels) - 1)
    h = max(p.size[1] for _, p in panels) + band
    side = Image.new("RGB", (w, h), (128, 128, 128))
    draw = ImageDraw.Draw(side)
    x = 0
    for title, p in panels:
        draw.text((x + 4, 5), title, fill=(255, 255, 255))
        side.paste(p, (x, band))
        x += p.size[0] + gap
    side.save(path)


def studio_write_compare(img, golden_path, mask, look, scene, scale):
    """What a failing scene leaves to look at, in tests/visual/out/studio/<look>/:
    <scene>@<s>x-compare.png (approved | now | what changed, red on the greyed image),
    -zoom.png (the changed part of the approved and the new image, enlarged), -diff.png.
    Returns the paths: (compare, zoom, diff)."""
    base = os.path.join(STUDIO_OUT, look, f"{scene}@{scale}x")
    approved = Image.open(golden_path).convert("RGB") if os.path.exists(golden_path) else None
    changed = zoom = None
    if mask is not None:
        changed = Image.blend(img, Image.new("RGB", img.size, (255, 255, 255)), 0.6)
        changed.paste((255, 0, 0), mask=mask)
        changed.save(base + "-diff.png")
        if approved is not None:
            x0, y0, x1, y1 = mask.getbbox()
            pad = 24 * scale
            box = (max(0, x0 - pad), max(0, y0 - pad), min(img.size[0], x1 + pad), min(img.size[1], y1 + pad))
            f = max(1, min(8, 700 // max(1, box[2] - box[0])))
            size = ((box[2] - box[0]) * f, (box[3] - box[1]) * f)
            studio_panels([("approved", approved.crop(box).resize(size, Image.NEAREST)), ("now", img.crop(box).resize(size, Image.NEAREST))], base + "-zoom.png")
            zoom = base + "-zoom.png"
    studio_panels([(t, p) for t, p in (("approved", approved), ("now", img), ("changed", changed)) if p is not None], base + "-compare.png")
    return base + "-compare.png", zoom, (base + "-diff.png" if changed else None)


def studio_run_job(job, mock_url, work_root):
    """One scene in one look at one scale: captured (taken again if it differs), compared with its approved image."""
    scene, look, scale = job
    name = scene["name"]
    golden = studio_golden_path(look, name, scale)
    shown = None  # the last capture that was made, and why it isn't the approved image
    for attempt in range(STUDIO_RETAKES + 1):
        img = studio_capture_one(scene, look, scale, mock_url, work_root)
        if img is None:
            continue
        why = studio_compare(img, golden)
        if why is None:
            studio_save_png(img, studio_out_path(look, name, scale))
            for stale in ("-compare.png", "-zoom.png", "-diff.png"):
                f = os.path.join(STUDIO_OUT, look, f"{name}@{scale}x{stale}")
                if os.path.exists(f):
                    os.remove(f)
            return dict(job=job, ok=True, retakes=attempt)
        shown = (img, why)
        if not os.path.exists(golden):
            break  # (nothing to be steadier against: it only needs approving)
    if shown is None:
        return dict(job=job, ok=False, why="no capture (Studio didn't start, or that window isn't there)", retakes=STUDIO_RETAKES)
    img, (reason, mask) = shown
    studio_save_png(img, studio_out_path(look, name, scale))
    compare, zoom, diff = studio_write_compare(img, golden, mask, look, name, scale)
    return dict(job=job, ok=False, why=reason, retakes=STUDIO_RETAKES, compare=compare, zoom=zoom, diff=diff)


def studio_run(scenes, looks, scales, jobs, capture_only=False):
    """Every job, `jobs` at a time: the results."""
    todo = list(studio_jobs(scenes, looks, scales))
    if not todo:
        sys.exit("nothing to do: no scene at those looks and scales")
    if not os.path.exists(RAPIDR):
        sys.exit(f"{RAPIDR} not found: cargo build --release -p rapidr-cli && cp target/release/rapidr .")
    before = example_leftovers()
    work_root = os.path.join(STUDIO_OUT, ".work")
    shutil.rmtree(work_root, ignore_errors=True)
    os.makedirs(os.path.join(work_root, "prints"))
    mock, mock_url = None, ""
    if any(sc.get("mock") for sc, _, _ in todo):
        mock, mock_url = start_mock()

    def capture(job):
        scene, look, scale = job
        img = studio_capture_one(scene, look, scale, mock_url, work_root)
        if img is not None:
            studio_save_png(img, studio_out_path(look, scene["name"], scale))
        return dict(job=job, ok=img is not None, why="" if img is not None else "no capture", retakes=0)

    try:
        with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
            return list(pool.map(capture if capture_only else (lambda j: studio_run_job(j, mock_url, work_root)), todo))
    finally:
        if mock:
            mock.kill()
            mock.wait()
        shutil.rmtree(work_root, ignore_errors=True)
        for f in example_leftovers() - before:
            os.remove(f)


def studio_orphans():
    """Approved images no scene (at that look and scale) makes any more."""
    want = {studio_golden_path(look, sc["name"], s) for sc, look, s in studio_jobs(studio_scenes(), STUDIO_LOOKS, None)}
    return sorted(f for f in glob.glob(os.path.join(STUDIO_GOLDEN, "*", "*.png")) if f not in want)


def studio_sheet(results):
    """tests/visual/out/studio/index.html: each scene in each look as drawn now, the failing ones marked and linked to their comparison."""
    rel = lambda p: os.path.relpath(p, STUDIO_OUT)
    by = {}
    for r in results:
        sc, look, scale = r["job"]
        by.setdefault((sc["name"], scale), {})[look] = r
    rows = []
    for (name, scale), cells in sorted(by.items()):
        tds = []
        for look in STUDIO_LOOKS:
            r = cells.get(look)
            img = studio_out_path(look, name, scale)
            if r is None or not os.path.exists(img):
                tds.append("<td class=empty>—</td>")
                continue
            w = Image.open(img).size[0] // scale // 2
            extra = ""
            if not r["ok"] and r.get("why"):
                extra = f"<div class=why>{html.escape(r['why'])}</div>"
                if r.get("compare"):
                    extra += f"<a href='{rel(r['compare'])}'>approved | now | changed</a>"
                if r.get("zoom"):
                    extra += f" · <a href='{rel(r['zoom'])}'>enlarged</a>"
            tds.append(f"<td{'' if r['ok'] else ' class=fail'}><div class=lab>{look}</div><a href='{rel(img)}'><img src='{rel(img)}' width={w}></a>{extra}</td>")
        rows.append(f"<tr><th>{html.escape(name)}<br><small>{scale}×</small></th>{''.join(tds)}</tr>")
    bad = sum(1 for r in results if not r["ok"])
    status = f"{bad} differences from the approved images" if bad else "every capture matches its approved image"
    page = f"""<!doctype html><meta charset=utf-8><title>RapidR Studio goldens</title>
<style>
body {{ font: 13px system-ui, sans-serif; margin: 16px; background: #fafafa; color: #222 }}
table {{ border-collapse: collapse }} td, th {{ border: 1px solid #ddd; padding: 6px; vertical-align: top; background: #fff }}
th {{ text-align: left; position: sticky; left: 0; background: #f3f3f3 }}
.lab {{ font-size: 11px; color: #666; margin-bottom: 4px }} td.fail {{ background: #ffecec }} .why {{ color: #b00; font-size: 11px }}
td.empty {{ color: #aaa }}
</style>
<h1>RapidR Studio goldens</h1>
<p>{status}. Each scene as Studio draws it now, in each look (shown at half size).</p>
<table>{''.join(rows)}</table>"""
    os.makedirs(STUDIO_OUT, exist_ok=True)
    with open(os.path.join(STUDIO_OUT, "index.html"), "w") as f:
        f.write(page)
    return os.path.join(STUDIO_OUT, "index.html")


def studio_approve(scenes, looks, scales):
    """The last captures of these scenes become the approved images."""
    n, missing = 0, []
    for sc, look, s in studio_jobs(scenes, looks, scales):
        cap = studio_out_path(look, sc["name"], s)
        if not os.path.exists(cap):
            missing.append(f"{look}/{sc['name']}@{s}x")
            continue
        dest = studio_golden_path(look, sc["name"], s)
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        shutil.copy(cap, dest)
        n += 1
    if missing:
        print(f"not captured yet (run `studio check` or `studio capture` first and look at the images): {', '.join(missing)}")
    print(f"approved {n} images")
    return 1 if missing else 0


def studio_main(rest):
    if not rest or rest[0] not in ("check", "capture", "approve", "list"):
        sys.exit(__doc__)
    cmd, rest = rest[0], rest[1:]
    looks, scales, jobs, names = list(STUDIO_LOOKS), None, None, []
    i = 0
    while i < len(rest):
        a = rest[i]
        if a in ("--theme", "--look") and i + 1 < len(rest):
            i += 1
            looks = [STUDIO_SHORT.get(rest[i], rest[i])]
        elif a == "--scale" and i + 1 < len(rest):
            i += 1
            scales = [int(rest[i])]
        elif a == "--jobs" and i + 1 < len(rest):
            i += 1
            jobs = int(rest[i])
        elif a == "--all":
            names = ["--all"]
        else:
            names.append(a)
        i += 1
    for look in looks:
        if look not in STUDIO_LOOKS:
            sys.exit(f"no such look: {look} (light, dark, contrast)")
    all_scenes = studio_scenes()
    if cmd == "list":
        for sc in all_scenes:
            print(f"{sc['name']:<20} {','.join(str(s) + 'x' for s in sc.get('scales', [1])):<5} {sc.get('why', '')}")
        return 0
    if names == ["--all"] or (not names and cmd != "approve"):
        scenes = all_scenes
    else:
        unknown = set(names) - {sc["name"] for sc in all_scenes}
        if unknown:
            sys.exit(f"no such scene: {', '.join(sorted(unknown))} (`studio list` names them)")
        scenes = [sc for sc in all_scenes if sc["name"] in names]
    if cmd == "approve":
        if not scenes:
            sys.exit("approve which scenes? (names, or --all)")
        return studio_approve(scenes, looks, scales)
    # (Studio's scripts are timed by the clock: a few runs at a time keep the steps on time)
    jobs = jobs or int(os.environ.get("RAPIDR_STUDIO_JOBS", "0")) or max(1, min(4, (os.cpu_count() or 4) // 2))
    results = studio_run(scenes, looks, scales, jobs, capture_only=(cmd == "capture"))
    sheet_path = studio_sheet(results)
    if cmd == "capture":
        print(f"studio: {sum(1 for r in results if r['ok'])} of {len(results)} captured")
        print(f"sheet: {sheet_path}")
        return 0
    failed = [r for r in results if not r["ok"]]
    for r in results:
        sc, look, s = r["job"]
        if r["ok"] and r["retakes"]:
            print(f"  ! {look}/{sc['name']}@{s}x: matched only on retake {r['retakes']} (an unsteady scene: give it more delay or waits)")
    for r in failed:
        sc, look, s = r["job"]
        print(f"  ✗ {look}/{sc['name']}@{s}x: {r['why']}")
        for label, key in (("side by side", "compare"), ("the change, enlarged", "zoom"), ("diff", "diff")):
            if r.get(key):
                print(f"      {label + ':':<22}{os.path.relpath(r[key], ROOT)}")
    full = len(scenes) == len(all_scenes) and not scales and len(looks) == len(STUDIO_LOOKS)
    orphans = studio_orphans() if full else []
    for o in orphans:
        print(f"  ✗ {os.path.relpath(o, ROOT)}: no scene makes it any more (remove it from the repository)")
    print(f"sheet: {sheet_path}")
    bad = len(failed) + len(orphans)
    if bad:
        print(f"studio goldens: {bad} differences (a deliberate change: look at the images, then `tools/visual/gallery.py studio approve <scene>`)")
    else:
        print(f"studio goldens: {len(results)} captures match their approved images")
    return 1 if bad else 0


def main():
    args = sys.argv[1:]
    if args and args[0] == "studio":
        sys.exit(studio_main(args[1:]))
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
