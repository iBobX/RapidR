#!/usr/bin/env python3
"""The icons for the manual, the website and the VS Code extension:

    python3 design/icons/tools/export.py           write docs/manual/icons/
    python3 design/icons/tools/export.py --check   exit 1 if it is out of date

- docs/manual/icons/light/<category>/<name>.svg and dark/…: each icon's
  24 px master in the modern and dark themes' colours, standalone (its
  currentColor set);
- docs/manual/icons/png/<category>/<name>.png: 48 × 48 (24 px at 2x), light,
  rendered by crates/rapidr-icons (with --png; needs cargo);
- docs/manual/icons/index.html: the catalog — every icon, its id and title,
  what uses it (component types, commands, file kinds …), light and dark.

The manual's reference pages (`rapidr lang export --manual`) show the
components' icons from light/.
"""

import html
import os
import re
import shutil
import subprocess
import sys
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import kit  # noqa: E402
import catalog  # noqa: E402,F401
import registry  # noqa: E402
from palette import THEMES, themed  # noqa: E402

ICONS = os.path.dirname(HERE)
ROOT = os.path.dirname(os.path.dirname(ICONS))
OUT = os.path.join(ROOT, "docs", "manual", "icons")
CAT_TITLES = {"actions": "Actions", "glyphs": "Glyphs", "components": "Components", "files": "Files",
              "symbols": "Symbols", "groups": "Toolbox groups"}


def standalone(key, theme):
    svg = themed(kit.draw(key, 24), theme)
    svg = re.sub(r"\s*<title>.*?</title>", "", svg, flags=re.S)
    fg = THEMES[theme][1]
    return svg.replace("<svg ", f'<svg color="{fg}" ', 1)


def uses():
    """What uses each icon: {id: [text]} from the inventory and the code."""
    inv = tomllib.load(open(os.path.join(ICONS, "inventory.toml"), "rb"))
    out = {}

    def add(ref, text, cat):
        ref = ref if "/" in ref else f"{cat}/{ref}"
        out.setdefault(ref, []).append(text)
    types = [t for t, _ in registry.components()]
    aliases = inv.get("component-aliases", {})
    for t in types:
        add(aliases.get(t, t[1:].lower()), t, "components")
    for t, n in inv.get("planned-components", {}).items():
        add(n, f"{t} (planned)", "components")
    for section, cat, fmt in (("commands", "actions", "command {}"), ("markers", "actions", "marker {}"),
                              ("files", "files", ".{}"), ("project-kinds", "files", "project tree: {}"),
                              ("symbols", "symbols", "symbol {}")):
        for k, v in inv.get(section, {}).items():
            add(v, fmt.format(k) if k != "*" else "any other file", cat)
    for gid, g in inv.get("toolbox-groups", {}).items():
        add(g["icon"], f"toolbox group {g['title']}", "groups")
    return out


CSS = """
:root { --bg: #F6F8FC; --card: #FFFFFF; --text: #0E1525; --sub: #5B6478; --line: #DDE3EE; --accent: #2F5BFF; }
:root[data-theme="dark"] { --bg: #16181D; --card: #202228; --text: #E8ECF4; --sub: #9AA3B5; --line: #2F333B; --accent: #7398FF; }
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { --bg: #16181D; --card: #202228; --text: #E8ECF4; --sub: #9AA3B5; --line: #2F333B; --accent: #7398FF; } }
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--text); font: 14px/1.45 Inter, -apple-system, "Segoe UI", Roboto, sans-serif; }
header { position: sticky; top: 0; z-index: 2; background: var(--bg); border-bottom: 1px solid var(--line); padding: 14px 24px; display: flex; flex-wrap: wrap; gap: 12px 20px; align-items: center; }
header h1 { font-size: 18px; margin: 0; font-weight: 650; }
header p { margin: 0; color: var(--sub); flex: 1 1 320px; }
input[type=search] { font: inherit; padding: 7px 10px; border-radius: 8px; border: 1px solid var(--line); background: var(--card); color: var(--text); width: 240px; max-width: 100%; }
button { font: inherit; padding: 7px 12px; border-radius: 8px; border: 1px solid var(--line); background: var(--card); color: var(--text); cursor: pointer; }
main { padding: 8px 24px 40px; max-width: 1400px; margin: 0 auto; }
h2 { font-size: 15px; margin: 26px 0 10px; font-weight: 650; }
h2 small { color: var(--sub); font-weight: 400; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 10px; }
.card { background: var(--card); border: 1px solid var(--line); border-radius: 10px; padding: 12px; display: flex; gap: 12px; align-items: flex-start; min-width: 0; }
.card .pics { display: flex; gap: 8px; align-items: flex-end; flex: none; }
.card .meta { min-width: 0; }
.card .id { font: 12px/1.3 "JetBrains Mono", ui-monospace, Menlo, monospace; overflow-wrap: anywhere; }
.card .title { color: var(--sub); font-size: 12px; }
.card .uses { color: var(--sub); font-size: 11px; margin-top: 4px; overflow-wrap: anywhere; }
img { display: block; }
.dark { display: none; }
:root[data-theme="dark"] .light { display: none; } :root[data-theme="dark"] .dark { display: block; }
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) .light { display: none; } :root:not([data-theme="light"]) .dark { display: block; } }
.hidden { display: none !important; }
"""

JS = """
const q = document.getElementById('q');
q.addEventListener('input', () => {
  const v = q.value.trim().toLowerCase();
  document.querySelectorAll('.card').forEach(c => c.classList.toggle('hidden', v && !c.dataset.k.includes(v)));
  document.querySelectorAll('section').forEach(s => s.classList.toggle('hidden', !s.querySelector('.card:not(.hidden)')));
});
const root = document.documentElement;
document.getElementById('theme').addEventListener('click', () => {
  const dark = root.dataset.theme ? root.dataset.theme === 'dark' : matchMedia('(prefers-color-scheme: dark)').matches;
  root.dataset.theme = dark ? 'light' : 'dark';
});
"""


def catalog_html():
    by = uses()
    parts = ["<!doctype html>", '<html lang="en">', "<head>", '<meta charset="utf-8">',
             '<meta name="viewport" content="width=device-width, initial-scale=1">',
             "<title>RapidR Icons</title>", f"<style>{CSS}</style>", "</head>", "<body>",
             "<header><h1>RapidR's icons</h1>",
             f"<p>{len(kit.ICONS)} icons, our own drawings (MIT), each hinted at 16, 24 and 32 px; "
             "shown here at 24 and 48 px. Generated by design/icons/tools/export.py; "
             "the design system is design/icons/README.md.</p>",
             '<input id="q" type="search" placeholder="Filter: name, type, command…" aria-label="Filter the icons">',
             '<button id="theme" type="button">Light / dark</button></header>', "<main>"]
    for cat, title in CAT_TITLES.items():
        keys = sorted(k for k, s in kit.ICONS.items() if s["cat"] == cat)
        parts.append(f'<section><h2>{title} <small>{len(keys)}</small></h2><div class="grid">')
        for key in keys:
            spec = kit.ICONS[key]
            name = spec["name"]
            u = by.get(key, [])
            words = " ".join([key, spec["title"]] + u).lower()
            pics = "".join(
                f'<img class="{t}" src="{t}/{key}.svg" width="{px}" height="{px}" alt="">'
                for px in (24, 48) for t in ("light", "dark"))
            parts.append(
                f'<div class="card" data-k="{html.escape(words)}"><div class="pics">{pics}</div>'
                f'<div class="meta"><div class="id">{html.escape(key)}</div><div class="title">{html.escape(spec["title"])}</div>'
                + (f'<div class="uses">{html.escape(", ".join(u[:8]) + (" …" if len(u) > 8 else ""))}</div>' if u else "")
                + "</div></div>")
        parts.append("</div></section>")
    parts += ["</main>", f"<script>{JS}</script>", "</body>", "</html>", ""]
    return "\n".join(parts)


def files():
    """{relative path: text} of everything but the PNGs."""
    out = {}
    for key in kit.ICONS:
        out[f"light/{key}.svg"] = standalone(key, "modern")
        out[f"dark/{key}.svg"] = standalone(key, "dark")
    out["index.html"] = catalog_html()
    return out


def pngs():
    tmp = os.path.join(ROOT, "target", "icons-export")
    shutil.rmtree(tmp, ignore_errors=True)
    os.makedirs(tmp)
    req = os.path.join(tmp, "request.txt")
    open(req, "w").write(f"2\n24\nmodern\n{tmp}\n*\n")
    subprocess.run(["cargo", "run", "-q", "--release", "-p", "rapidr-icons", "--example", "render", "--", req], cwd=ROOT, check=True)
    for key in kit.ICONS:
        dst = os.path.join(OUT, "png", key + ".png")
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        shutil.copyfile(os.path.join(tmp, f"{key.replace('/', '__')}_24_modern.png"), dst)


def main():
    check = "--check" in sys.argv[1:]
    stale = []
    want = files()
    for rel, text in want.items():
        path = os.path.join(OUT, rel)
        old = open(path, encoding="utf-8").read() if os.path.exists(path) else None
        if old == text:
            continue
        stale.append(rel)
        if not check:
            os.makedirs(os.path.dirname(path), exist_ok=True)
            open(path, "w", encoding="utf-8").write(text)
    # (icons no longer drawn)
    for theme in ("light", "dark"):
        d = os.path.join(OUT, theme)
        for dirpath, _, fns in os.walk(d) if os.path.isdir(d) else []:
            for fn in fns:
                rel = os.path.relpath(os.path.join(dirpath, fn), OUT)
                if rel not in want:
                    stale.append(rel)
                    if not check:
                        os.remove(os.path.join(dirpath, fn))
    if check and stale:
        sys.exit(f"docs/manual/icons out of date ({len(stale)} files; python3 design/icons/tools/export.py)")
    if "--png" in sys.argv[1:]:
        pngs()
    if not check:
        print(f"docs/manual/icons: {len(want)} files ({len(stale)} changed)")


if __name__ == "__main__":
    main()
