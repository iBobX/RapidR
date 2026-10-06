#!/usr/bin/env python3
"""Builds RapidR's icon set (design/icons/README.md, "Pipeline").

    python3 design/icons/tools/build.py           draw, optimize, check, generate
    python3 design/icons/tools/build.py --check   exit 1 if anything is out of date

1. Draw: every icon the geometry modules define (kit.ICONS) is written to
   design/icons/src/<category>/<name>.svg (the 24 px master) and
   <name>-16.svg, <name>-32.svg (the hinted sizes). A source holding
   `data-hand-drawn` is never overwritten (an icon drawn in an editor).
2. Optimize (our own, standard library only): each source loses its XML
   declaration, comments, <title>, <desc>, <metadata>, editor attributes and
   whitespace; its colours are checked (the tokens' values, currentColor —
   only currentColor in a monochrome icon).
3. Check the inventory (design/icons/inventory.toml): every component type in
   the code (COMPONENT_TYPES, crates/rapidr-ast/src/lib.rs), every planned
   component, command, marker, file kind, project kind, symbol and toolbox
   group names an icon that exists at every size; every component is in one
   toolbox group.
4. Generate crates/rapidr-icons/src/generated.rs (the palettes from
   palette.py, the inventory, each icon's place in the bundle) and
   crates/rapidr-icons/src/icons.deflate (every optimized SVG, one after
   another, raw-deflated: the crate inflates it on first use).
"""

import os
import re
import sys
import tomllib
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import kit  # noqa: E402
import catalog  # noqa: E402,F401
import palette  # noqa: E402

ICONS_DIR = os.path.dirname(HERE)
ROOT = os.path.dirname(os.path.dirname(ICONS_DIR))
SRC = os.path.join(ICONS_DIR, "src")
OUT_RS = os.path.join(ROOT, "crates", "rapidr-icons", "src", "generated.rs")
OUT_BIN = os.path.join(ROOT, "crates", "rapidr-icons", "src", "icons.deflate")
CATEGORIES = ("actions", "glyphs", "components", "files", "symbols", "groups")


def fail(msg):
    sys.exit(f"icons: {msg}")


# ---- 1. draw ----------------------------------------------------------------------

def src_path(cat, name, size):
    suffix = "" if size == 24 else f"-{size}"
    return os.path.join(SRC, cat, f"{name}{suffix}.svg")


def draw(check):
    """Writes the sources; returns the paths that changed (or would)."""
    changed = []
    wanted = set()
    for key, spec in sorted(kit.ICONS.items()):
        for size in kit.SIZES:
            path = src_path(spec["cat"], spec["name"], size)
            wanted.add(path)
            old = open(path, encoding="utf-8").read() if os.path.exists(path) else None
            if old is not None and "data-hand-drawn" in old:
                continue
            new = kit.draw(key, size)
            if old != new:
                changed.append(path)
                if not check:
                    os.makedirs(os.path.dirname(path), exist_ok=True)
                    with open(path, "w", encoding="utf-8") as f:
                        f.write(new)
    # sources no geometry draws any more (and not drawn by hand) go
    for cat in os.listdir(SRC) if os.path.isdir(SRC) else []:
        d = os.path.join(SRC, cat)
        for fn in os.listdir(d):
            p = os.path.join(d, fn)
            if fn.endswith(".svg") and p not in wanted and "data-hand-drawn" not in open(p, encoding="utf-8").read():
                changed.append(p)
                if not check:
                    os.remove(p)
    return changed


# ---- 2. optimize --------------------------------------------------------------------

ALLOWED = set(kit.TOKENS.values()) | {"currentColor", "none", "white", "black"}


def optimize(svg, where, mono):
    s = re.sub(r"<\?xml.*?\?>", "", svg, flags=re.S)
    s = re.sub(r"<!--.*?-->", "", s, flags=re.S)
    s = re.sub(r"<(title|desc|metadata)\b.*?</\1>", "", s, flags=re.S)
    s = re.sub(r"<(sodipodi|inkscape):[^>]*/>", "", s)
    s = re.sub(r'\s(?:inkscape|sodipodi|xmlns:inkscape|xmlns:sodipodi|xmlns:xlink|data-[\w-]+|xml:space|version)="[^"]*"', "", s)
    s = re.sub(r">\s+<", "><", s).strip()
    s = re.sub(r"\s+", " ", s)
    if not re.match(r'<svg xmlns="http://www.w3.org/2000/svg" width="(\d+)" height="\1" viewBox="0 0 \1 \1"', s):
        fail(f"{where}: the root must be <svg xmlns=… width=S height=S viewBox=\"0 0 S S\">")
    for attr, val in re.findall(r'\s(fill|stroke|stop-color|color)="([^"]*)"', s):
        if val.startswith("url(#"):
            continue
        if val not in ALLOWED:
            fail(f"{where}: colour {val} isn't a token (kit.TOKENS) or currentColor")
        if mono and val not in ("currentColor", "none", "white", "black"):
            fail(f"{where}: a monochrome icon may only use currentColor (found {val})")
    if re.search(r"<(text|image|script|foreignObject|style)\b", s):
        fail(f"{where}: no text, images, scripts or styles in an icon")
    return s


def load_sources():
    """{id: {size: optimized svg}} from design/icons/src."""
    out = {}
    if not os.path.isdir(SRC):
        return out
    for cat in sorted(os.listdir(SRC)):
        if cat not in CATEGORIES:
            fail(f"src/{cat}: unknown category (one of {', '.join(CATEGORIES)})")
        for fn in sorted(os.listdir(os.path.join(SRC, cat))):
            if not fn.endswith(".svg"):
                continue
            m = re.match(r"^([a-z0-9][a-z0-9-]*?)(?:-(16|32))?\.svg$", fn)
            if not m:
                fail(f"src/{cat}/{fn}: names are lower-case kebab-case, -16 / -32 for the hinted sizes")
            name, size = m.group(1), int(m.group(2) or 24)
            key = f"{cat}/{name}"
            text = open(os.path.join(SRC, cat, fn), encoding="utf-8").read()
            spec = kit.ICONS.get(key)
            mono = spec["mono"] if spec else ('stroke="#' not in text and 'fill="#' not in text)
            out.setdefault(key, {})[size] = optimize(text, f"src/{cat}/{fn}", mono)
    for key, sizes in out.items():
        missing = [s for s in kit.SIZES if s not in sizes]
        if missing:
            fail(f"{key}: no source at {', '.join(map(str, missing))} px")
    return out


# ---- 3. inventory --------------------------------------------------------------------

def const_list(src, name):
    m = re.search(r"pub const " + re.escape(name) + r"\s*:\s*&\[[^=]*=\s*&\[(.*?)\];", src, re.S)
    if not m:
        fail(f"{name} not found")
    return re.findall(r'"([^"]*)"', re.sub(r"//[^\n]*", "", m.group(1)))


def icon_ref(ref, default_cat):
    return ref if "/" in ref else f"{default_cat}/{ref}"


def inventory(icons):
    inv = tomllib.load(open(os.path.join(ICONS_DIR, "inventory.toml"), "rb"))
    ast = open(os.path.join(ROOT, "crates", "rapidr-ast", "src", "lib.rs"), encoding="utf-8").read()
    types = const_list(ast, "COMPONENT_TYPES")
    aliases = inv.get("component-aliases", {})
    planned = inv.get("planned-components", {})
    errors = []

    def need(ref, what):
        if ref not in icons:
            errors.append(f"{what}: no icon {ref}")

    components = []
    for t in types:
        ref = f"components/{aliases.get(t, t[1:].lower())}"
        need(ref, f"component {t}")
        components.append((t, ref, False))
    for t, name in planned.items():
        if t in types:
            errors.append(f"planned component {t} exists now: remove it from [planned-components]")
        ref = icon_ref(name, "components")
        need(ref, f"planned component {t}")
        components.append((t, ref, True))
    tables = {}
    for section, cat in (("commands", "actions"), ("markers", "actions"), ("files", "files"),
                         ("project-kinds", "files"), ("symbols", "symbols")):
        rows = []
        for k, v in inv.get(section, {}).items():
            ref = icon_ref(v, cat)
            need(ref, f"[{section}] {k}")
            rows.append((k, ref))
        tables[section] = rows
    groups = []
    seen = {}
    all_types = {t for t, _, _ in components}
    for gid, g in inv.get("toolbox-groups", {}).items():
        ref = icon_ref(g["icon"], "groups")
        need(ref, f"toolbox group {gid}")
        members = g.get("members", "").split()
        for mbr in members:
            if mbr not in all_types:
                errors.append(f"toolbox group {gid}: {mbr} is no component")
            if mbr in seen:
                errors.append(f"{mbr} is in toolbox groups {seen[mbr]} and {gid}")
            seen[mbr] = gid
        groups.append((gid, g["title"], ref, g.get("parent", ""), members))
    for t in sorted(all_types - set(seen)):
        errors.append(f"component {t} is in no toolbox group")
    # (RapidQ's groups hold RapidQ's components, RapidR's the rest)
    sys.path.insert(0, os.path.join(ROOT, "tools"))
    import manual_reference
    rapidq = set(manual_reference.RAPIDQ_BUILTIN) | set(manual_reference.RAPIDQ_LIBRARY)
    def has_q_name(t):
        return t in ("RPROGRESSBAR", "RTREEVIEW") or ("Q" + t[1:]) in rapidq
    for gid, title, ref, parent, members in groups:
        for mbr in members:
            if parent == "rapidq" and not has_q_name(mbr):
                errors.append(f"toolbox group {gid} is RapidQ's but {mbr} has no RapidQ name: put it under rapidr")
            if parent == "rapidr" and has_q_name(mbr) and mbr in types:
                errors.append(f"toolbox group {gid} is RapidR's but {mbr} is RapidQ's (Q{mbr[1:]}): put it under rapidq")
    if errors:
        fail("inventory:\n  " + "\n  ".join(errors))
    return components, tables, groups


# ---- 4. generate ----------------------------------------------------------------------

def rs_str(s):
    hashes = "#"
    while f'"{hashes}' in s:
        hashes += "#"
    return f'r{hashes}"{s}"{hashes}'


def generate(icons, components, tables, groups):
    L = ["// Generated by design/icons/tools/build.py from design/icons/ (src/, inventory.toml,",
         "// tools/palette.py). Do not edit: change the sources and run the script again.",
         "",
         "use crate::{IconData, ToolboxGroup};",
         ""]
    tokens = list(kit.TOKENS)
    L.append(f"/// The colour tokens, in the order of every palette's values.")
    L.append(f"pub(crate) static TOKENS: [&str; {len(tokens)}] = [{', '.join(rs_str(t) for t in tokens)}];")
    L.append(f"/// Each token's value in the sources (the light themes'), as the SVGs spell it.")
    L.append(f"pub(crate) static KEYS: [&str; {len(tokens)}] = [{', '.join(rs_str(kit.TOKENS[t]) for t in tokens)}];")
    L.append("/// The themes' palettes: (theme, currentColor's default, a disabled icon's ink,")
    L.append("/// the backgrounds icons are checked against, token values 0xRRGGBB).")
    L.append(f"pub(crate) type PaletteRow = (&'static str, u32, u32, &'static [u32], [u32; {len(tokens)}]);")
    L.append(f"pub(crate) static PALETTES: [PaletteRow; {len(palette.THEMES)}] = [")
    for name, (tok, fg, bgs) in palette.THEMES.items():
        vals = ", ".join(f"0x{tok[t][1:].upper()}" for t in tokens)
        bg = ", ".join(f"0x{c[1:].upper()}" for c in bgs)
        L.append(f"    ({rs_str(name)}, 0x{fg[1:].upper()}, 0x{palette.DISABLED[name][1:].upper()}, &[{bg}], [{vals}]),")
    L.append("];")
    L.append("")
    L.append("/// Every icon, by id: where its 16, 24 and 32 px drawings are in the")
    L.append("/// inflated bundle (offset, length).")
    L.append("pub(crate) static ICONS: &[IconData] = &[")
    blob = []
    at = 0
    for key in sorted(icons):
        spec = kit.ICONS.get(key)
        cat, name = key.split("/", 1)
        title = spec["title"] if spec else name
        mono = "true" if (spec["mono"] if spec else False) else "false"
        places = []
        for s_ in kit.SIZES:
            b = icons[key][s_].encode("utf-8")
            places.append(f"({at}, {len(b)})")
            blob.append(b)
            at += len(b)
        L.append(f"    IconData {{ id: {rs_str(key)}, category: {rs_str(cat)}, name: {rs_str(name)}, title: {rs_str(title)}, mono: {mono}, svg: [{', '.join(places)}] }},")
    L.append("];")
    L.append("")
    L.append(f"/// The bundle's inflated size.")
    L.append(f"pub(crate) const BUNDLE_LEN: usize = {at};")
    L.append("")
    L.append("/// Every component type (the code's, then the planned ones) and its icon; true when planned.")
    L.append("pub static COMPONENTS: &[(&str, &str, bool)] = &[")
    for t, ref, planned in components:
        L.append(f"    ({rs_str(t)}, {rs_str(ref)}, {'true' if planned else 'false'}),")
    L.append("];")
    for section, const in (("commands", "COMMANDS"), ("markers", "MARKERS"), ("files", "FILES"),
                           ("project-kinds", "PROJECT_KINDS"), ("symbols", "SYMBOLS")):
        L.append("")
        L.append(f"/// inventory.toml's [{section}]: (key, icon).")
        L.append(f"pub static {const}: &[(&str, &str)] = &[")
        for k, ref in tables[section]:
            L.append(f"    ({rs_str(k)}, {rs_str(ref)}),")
        L.append("];")
    L.append("")
    L.append("/// The toolbox's groups (RToolbox), in order.")
    L.append("pub static TOOLBOX_GROUPS: &[ToolboxGroup] = &[")
    for gid, title, ref, parent, members in groups:
        mem = ", ".join(rs_str(m) for m in members)
        L.append(f"    ToolboxGroup {{ id: {rs_str(gid)}, title: {rs_str(title)}, icon: {rs_str(ref)}, parent: {rs_str(parent)}, members: &[{mem}] }},")
    L.append("];")
    raw = b"".join(blob)
    c = zlib.compressobj(9, zlib.DEFLATED, -15, 9)
    packed = c.compress(raw) + c.flush()
    return "\n".join(L) + "\n", packed


def main():
    check = "--check" in sys.argv[1:]
    changed = draw(check)
    if check and changed:
        fail("sources out of date (python3 design/icons/tools/build.py):\n  " + "\n  ".join(os.path.relpath(p, ROOT) for p in changed[:20]))
    icons = load_sources()
    components, tables, groups = inventory(icons)
    text, packed = generate(icons, components, tables, groups)
    old = open(OUT_RS, encoding="utf-8").read() if os.path.exists(OUT_RS) else None
    old_bin = open(OUT_BIN, "rb").read() if os.path.exists(OUT_BIN) else None
    # (the deflate stream is compared inflated: another zlib may pack it differently)
    same_bin = old_bin is not None and zlib.decompress(old_bin, -15) == zlib.decompress(packed, -15)
    if old != text or not same_bin:
        if check:
            fail(f"{os.path.relpath(OUT_RS, ROOT)} out of date (python3 design/icons/tools/build.py)")
        os.makedirs(os.path.dirname(OUT_RS), exist_ok=True)
        with open(OUT_RS, "w", encoding="utf-8") as f:
            f.write(text)
        with open(OUT_BIN, "wb") as f:
            f.write(packed)
    if not check:
        size = sum(len(v) for sizes in icons.values() for v in sizes.values())
        print(f"icons: {len(icons)} icons ({len(changed)} source files changed), {len(components)} components, "
              f"{len(tables['commands'])} commands; {size // 1024} KB of SVG, {len(packed) // 1024} KB deflated")


if __name__ == "__main__":
    main()
