#!/usr/bin/env python3
"""RapidR Studio's form designer is WYSIWYG (docs/ide-plan.md I4, lane
L-DVIEW): a designed form, captured as RDESIGNSURFACE draws it with the
selection and the grid hidden, equals the running form, pixel for pixel.

For every form a program in examples/ (and tests/fixtures/designer_*.bas)
CREATEs:

1. the designer's capture — `cargo run -p rapidr-designer --example
   design_capture`: the file opened in the designer (rapidr-designer's
   Document), the form on an RDESIGNSURFACE drawn by the UI kernel and the
   CPU renderer (the headless desktop host's capture path); its inside (menu
   bar and client area) cut out;
2. the running form's capture — the program the designer's drawing stands
   for (what design_capture writes beside it: the program's directives and
   constants, the form's CREATE blocks as the designer reads them, then
   `Form.ShowModal`), built and run by ./rapidr under the headless host
   (`RAPIDR_CAPTURE`), nothing focused (`RAPIDR_TEST_NOFOCUS`, as a designer
   shows a form), its main menu in the window (`RAPIDR_MENU=window`, as on
   Windows, Linux and the web);
3. the two compared, at 1× and 2× in each theme: a pixel differs when a
   channel differs by more than TOLERANCE (as tools/visual/gallery.py).

The web host draws the same display lists as the desktop's, byte for byte
(tests/web_gui_parity.mjs): the designer fixture there
(`designer_wysiwyg`) carries the comparison to the browser.

    tools/visual/designer_wysiwyg.py [--themes classic,dark] [--scales 1] [FILE …]

Writes tests/visual/out/designer/<theme>/<case>@<scale>x-{design,run,diff}.png
and an index.html; exits 1 on any difference. Needs ./rapidr (cargo build
--release -p rapidr-cli; copied to the repository's root) and Pillow.
Programs run with RAPIDR_PRINT_TO and RAPIDR_REGISTRY in the output folder.
"""
import argparse
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
    sys.exit("tools/visual/designer_wysiwyg.py needs Pillow (python3 -m pip install pillow)")

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "tests", "visual", "out", "designer")
RAPIDR = os.environ.get("RAPIDR_BIN", os.path.join(ROOT, "rapidr"))
THEMES = ["classic", "modern", "dark", "highcontrast"]
SCALES = [1, 2]
TOLERANCE = 3


def capture_tool():
    """Builds the designer's capture tool; its path."""
    target = os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target"))
    r = subprocess.run(["cargo", "build", "-q", "-p", "rapidr-designer", "--example", "design_capture"], cwd=ROOT)
    if r.returncode != 0:
        sys.exit("cargo build -p rapidr-designer --example design_capture failed")
    return os.path.join(target, "debug", "examples", "design_capture")


def programs(paths):
    if paths:
        return [os.path.abspath(p) for p in paths]
    out = []
    for p in sorted(glob.glob(os.path.join(ROOT, "examples", "**", "*.*"), recursive=True)):
        if re.search(r"\.(bas|rr)$", p, re.I) and re.search(r"(?im)^\s*create\s+\w+\s+as\s+\w*form", open(p, encoding="latin-1").read()):
            out.append(p)
    out += sorted(glob.glob(os.path.join(ROOT, "tests", "fixtures", "designer_*.bas")))
    return out


def env_for(work, theme, scale):
    env = dict(os.environ)
    env.update({
        "RAPIDR_SCALE": str(scale),
        "RAPIDR_THEME": theme,
        "RAPIDR_PRINT_TO": os.path.join(work, "prints"),
        "RAPIDR_REGISTRY": os.path.join(work, "registry.reg"),
        "RAPIDR_MENU": "window",
        "RAPIDR_TEST_NOFOCUS": "1",
    })
    return env


def case_name(path):
    rel = os.path.relpath(path, ROOT)
    return re.sub(r"[^A-Za-z0-9]+", "_", os.path.splitext(rel)[0]).strip("_")


def designer_capture(tool, path, prefix, theme, scale, work):
    """The designer's captures of `path`'s forms: [(form, title, bmp, program)]."""
    r = subprocess.run([tool, path, prefix, "--scale", str(scale)], env=env_for(work, theme, scale), capture_output=True, text=True, timeout=120)
    forms = []
    for line in r.stdout.splitlines():
        parts = line.split("\t")
        if len(parts) == 3:
            forms.append((parts[0], parts[1], f"{prefix}-{parts[0]}.bmp", f"{prefix}-{parts[0]}.bas"))
    return forms, r.stderr.strip()


def run_capture(program, original, theme, scale, work):
    """The running form's capture (its first window), or (None, why)."""
    folder = os.path.dirname(original)
    # (beside the original: its $INCLUDEs are found as the original's)
    fd, local = tempfile.mkstemp(prefix=".wysiwyg-", suffix=".bas", dir=folder)
    os.close(fd)
    shutil.copy(program, local)
    try:
        rrbc = os.path.join(work, os.path.basename(local) + ".rrbc")
        b = subprocess.run([RAPIDR, "build-bc", local, "-o", rrbc], cwd=folder, capture_output=True, text=True, timeout=120)
        if b.returncode != 0:
            return None, "build failed: " + (b.stderr or b.stdout).strip().splitlines()[-1][:200] if (b.stderr or b.stdout).strip() else "build failed"
    finally:
        os.remove(local)
    tmp = tempfile.mkdtemp(prefix="run-", dir=work)
    env = env_for(work, theme, scale)
    env["RAPIDR_CAPTURE"] = os.path.join(tmp, "cap")
    env["RAPIDR_CAPTURE_DELAY"] = "0.3"
    try:
        subprocess.run([RAPIDR, "run-bc", rrbc], cwd=folder, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=60)
    except subprocess.TimeoutExpired:
        return None, "timed out"
    shots = sorted(glob.glob(os.path.join(tmp, "cap-*.bmp")))
    if not shots:
        return None, "no window captured"
    return shots[0], None


def compare(design_bmp, run_bmp, base):
    """None when equal (within TOLERANCE), else why; PNGs of both and the diff."""
    a, b = Image.open(design_bmp).convert("RGB"), Image.open(run_bmp).convert("RGB")
    a.save(base + "-design.png")
    b.save(base + "-run.png")
    if a.size != b.size:
        return f"size {a.size[0]}x{a.size[1]} (designer) vs {b.size[0]}x{b.size[1]} (running)"
    d = ImageChops.difference(a, b)
    mask = d.convert("L").point(lambda v: 255 if v > TOLERANCE else 0)
    count = mask.histogram()[255]
    if not count:
        return None
    shown = Image.blend(a, Image.new("RGB", a.size, (255, 255, 255)), 0.6)
    shown.paste((255, 0, 0), mask=mask)
    shown.save(base + "-diff.png")
    return f"{count} pixels differ"


def one(job):
    tool, path, theme, scale = job
    name = case_name(path)
    work = tempfile.mkdtemp(prefix=f"{name}-{theme}-{scale}-", dir=os.path.join(OUT, ".work"))
    results = []
    forms, err = designer_capture(tool, path, os.path.join(work, "design"), theme, scale, work)
    if not forms:
        return [(name, "-", theme, scale, "skip", err or "no form")]
    for form, title, bmp, program in forms:
        case = f"{name}-{form}"
        base = os.path.join(OUT, theme, f"{case}@{scale}x")
        shot, why = run_capture(program, path, theme, scale, work)
        if shot is None:
            results.append((case, title, theme, scale, "skip", why))
            continue
        try:
            why = compare(bmp, shot, base)
        except OSError as e:
            results.append((case, title, theme, scale, "FAIL", f"unreadable capture: {e}"))
            continue
        results.append((case, title, theme, scale, "ok" if why is None else "FAIL", why or ""))
    shutil.rmtree(work, ignore_errors=True)
    return results


def sheet(results):
    rows = []
    for case, title, theme, scale, status, why in sorted(results):
        base = f"{theme}/{case}@{scale}x"
        imgs = "".join(f"<td><img src='{base}-{k}.png'></td>" if os.path.exists(os.path.join(OUT, f"{base}-{k}.png")) else "<td></td>" for k in ("design", "run", "diff"))
        rows.append(f"<tr class={status.lower()}><th>{html.escape(case)}<br><small>{theme} {scale}× — {status} {html.escape(why)}</small></th>{imgs}</tr>")
    page = f"""<!doctype html><meta charset=utf-8><title>Designer WYSIWYG</title>
<style>body{{font:13px system-ui;margin:16px}} td,th{{border:1px solid #ddd;padding:4px;vertical-align:top}} th{{text-align:left}}
tr.fail th{{background:#fdd}} tr.skip th{{background:#eee}} img{{max-width:420px}}</style>
<h1>RapidR Studio's designer vs the running form</h1><table><tr><th></th><th>designer</th><th>running</th><th>difference</th></tr>{''.join(rows)}</table>"""
    open(os.path.join(OUT, "index.html"), "w").write(page)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--themes", default=",".join(THEMES))
    ap.add_argument("--scales", default=",".join(map(str, SCALES)))
    ap.add_argument("files", nargs="*")
    args = ap.parse_args()
    if not os.path.exists(RAPIDR):
        sys.exit(f"{RAPIDR} not found: cargo build --release -p rapidr-cli && cp target/release/rapidr .")
    tool = capture_tool()
    themes = args.themes.split(",")
    scales = [int(s) for s in args.scales.split(",")]
    for t in themes:
        os.makedirs(os.path.join(OUT, t), exist_ok=True)
    os.makedirs(os.path.join(OUT, ".work"), exist_ok=True)
    jobs = [(tool, p, t, s) for p in programs(args.files) for t in themes for s in scales]
    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=os.cpu_count() or 4) as pool:
        for r in pool.map(one, jobs):
            results.extend(r)
    shutil.rmtree(os.path.join(OUT, ".work"), ignore_errors=True)
    sheet(results)
    fails = [r for r in results if r[4] == "FAIL"]
    skips = [r for r in results if r[4] == "skip"]
    oks = [r for r in results if r[4] == "ok"]
    for case, title, theme, scale, status, why in sorted(fails + skips):
        print(f"  {status:4} {case} [{theme} {scale}x] {why}")
    print(f"{len(oks)} equal, {len(fails)} different, {len(skips)} skipped — {os.path.join(OUT, 'index.html')}")
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
