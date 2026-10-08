#!/usr/bin/env python3
"""The member names RapidR's runtimes answer, read from their dispatch code.

Every `match` on a property's or method's name in the runtimes (the shared
models in crates/rapidr-value, the desktop runtime in
crates/rapidr-runtime-core, the web runtime in crates/rapidr-runtime-web)
names what that site answers. SITES says which components each site serves
("*" = any component: the generic dispatch; "-" = not a component member:
builtins, internal keys).

    python3 tools/lang_dispatch.py            the sites and their names, as JSON
    python3 tools/lang_dispatch.py --check    the language registry's reverse check (exit 1
                                              naming what fails): every site is in SITES,
                                              and every name a site answers is a member,
                                              in crates/rapidr-lang/data, of a component
                                              it serves (of some component, for "*")

`tools/regress.sh unit` runs the check; tools/lang_seed.py used the sites
to seed the registry. Plain text matching, no Rust parser: a site is a
`match <prop|method|…> {` block, its names the string patterns of its arms
(`"caption" | "text" =>`); a tuple arm names its component itself
(`("RWEBSTORAGE", "get") =>`, `("screen", "width")`).
"""

import glob
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# The runtimes' sources.
GLOBS = [
    "crates/rapidr-value/src/**/*.rs",
    "crates/rapidr-runtime-core/src/**/*.rs",
    "crates/rapidr-runtime-web/src/**/*.rs",
]
# (tests and data tables, not dispatch)
SKIP_FILES = ("/tests.rs", "/avi/tests.rs")

MATCH_VARS = ("prop", "method", "method_lower", "prop_lower", "m", "member", "p", "lower", "name", "key", "lmethod")

# (file relative to the repo, enclosing `impl` type or function) → what it
# serves. The kind ("prop" / "method" / "event") is the site's; None takes
# it from the variable matched on.
D3D = "RD3DFRAME RD3DMESHBUILDER RD3DMESH RD3DFACE RD3DLIGHT RD3DTEXTURE RD3DVISUAL RD3DWRAP RD3DVECTOR"
LISTS = "RLISTBOX RCOMBOBOX RFILELISTBOX RSTRINGLIST"
TEXTS = "REDIT RMEMO RRICHEDIT RCODEEDITOR"
PICTURES = "RBITMAP RIMAGE RCANVAS RFORM RFORMMDI RDXSCREEN RHEADER RDIGDISPLAY RLISTBOX RCOMBOBOX RSTRINGGRID RLISTVIEW"
MEDIA = "RMIDI RWAVE RVIDEO RCDAUDIO"
WEB = "RWEBVIEW RDOM RJAVASCRIPT RWEBSTORAGE RWEBAUDIO RWEBVIDEO RWEBNOTIFICATION RWEBGEOLOCATION RROUTER RPLOT"
SITES = {
    # --- desktop runtime
    # --- data science: one model for every runtime
    ("crates/rapidr-value/src/datascience/num.rs", "*"): "RNUM",
    ("crates/rapidr-value/src/datascience/frame.rs", "*"): "RDATAFRAME",
    ("crates/rapidr-value/src/datascience/plot.rs", "*"): "RPLOT",
    ("crates/rapidr-runtime-core/src/layout.rs", "after_set"): "*",
    ("crates/rapidr-runtime-core/src/network.rs", "socket_method"): "RSOCKET",
    ("crates/rapidr-runtime-core/src/network.rs", "method"): "RSOCKET RSERVERSOCKET",
    ("crates/rapidr-runtime-core/src/network.rs", "server_socket_method"): "RSERVERSOCKET",
    ("crates/rapidr-runtime-core/src/network.rs", "http_method"): "RHTTP",
    ("crates/rapidr-runtime-core/src/object.rs", "json_method"): "RJSON",
    ("crates/rapidr-runtime-core/src/object.rs", "gui_generic_method"): "*",
    ("crates/rapidr-runtime-core/src/ui/kernel.rs", "canvas_method"): "RCANVAS RHEADER",
    ("crates/rapidr-runtime-core/src/ui/kernel.rs", "image_method"): "RIMAGE",
    ("crates/rapidr-runtime-core/src/ui/kernel.rs", "tree_method"): "RTREEVIEW",
    ("crates/rapidr-runtime-core/src/ui/kernel.rs", "design_surface_method"): "RDESIGNSURFACE",
    # --- web runtime
    ("crates/rapidr-runtime-web/src/io_web.rs", "method"): "RCOMPORT RDOWNLOAD " + MEDIA,
    ("crates/rapidr-runtime-web/src/kernel_web.rs", "set_prop"): "*",
    ("crates/rapidr-runtime-web/src/layout_web.rs", "after_set"): "*",
    ("crates/rapidr-runtime-web/src/network_web.rs", "http_method"): "RHTTP",
    ("crates/rapidr-runtime-web/src/network_web.rs", "websocket_method"): "RSOCKET",
    ("crates/rapidr-runtime-web/src/object_web.rs", "json_web_method"): "RJSON",
    ("crates/rapidr-runtime-web/src/object_web.rs", "filestream_web_method"): "RFILESTREAM",
    ("crates/rapidr-runtime-web/src/overlay_web.rs", "set_prop"): WEB,
    ("crates/rapidr-runtime-web/src/overlay_web.rs", "get_prop"): WEB,
    ("crates/rapidr-runtime-web/src/overlay_web.rs", "method"): WEB,
    # --- the shared models
    ("crates/rapidr-value/src/autosize.rs", "resizes"): "RLABEL",
    ("crates/rapidr-value/src/data.rs", "builtin"): "-",
    ("crates/rapidr-value/src/input.rs", "vk_of_key"): "-",
    ("crates/rapidr-value/src/lib.rs", "shared_builtin"): "-",
    ("crates/rapidr-value/src/memory.rs", "shared"): "-",
    ("crates/rapidr-value/src/font_dialog.rs", "alias"): "RFONTDIALOG",
    ("crates/rapidr-value/src/font_dialog.rs", "call"): "RFONTDIALOG",
    ("crates/rapidr-value/src/globals.rs", "file_rec"): "FILEREC",
    # (QMEMORYSTREAM's MemCopyFrom / MemCopyTo, the streams' Save / LoadUDTArray)
    ("crates/rapidr-value/src/objects/stream_ops.rs", "call"): "RFILESTREAM RMEMORYSTREAM",
    ("crates/rapidr-value/src/globals.rs", "hint_setting"): "APPLICATION",
    # (form members: HideTitleBar, ShapeForm, QFORM's MDI members, StartDrag)
    ("crates/rapidr-runtime-core/src/form_members.rs", "method"): "RFORM RFORMMDI RBUTTON RCOOLBTN ROVALBTN",
    ("crates/rapidr-runtime-core/src/form_members.rs", "get"): "RFORM RFORMMDI",
    ("crates/rapidr-runtime-web/src/form_members_web.rs", "method"): "RFORM RFORMMDI RBUTTON RCOOLBTN ROVALBTN",
    ("crates/rapidr-runtime-web/src/form_members_web.rs", "get"): "RFORM RFORMMDI",
    ("crates/rapidr-value/src/layout.rs", "with"): "*",
    # (the layout engine every runtime and the designer run; a label's AutoSize)
    ("crates/rapidr-value/src/layout.rs", "after_set"): "*",
    ("crates/rapidr-value/src/autosize.rs", "resizes"): "RLABEL",
    # (I4: the designer model replays CREATE blocks through that layout)
    ("crates/rapidr-value/src/designer/layout.rs", "stored"): "*",
    ("crates/rapidr-value/src/designer/layout.rs", "set"): "*",
    # (a panel's inside, its bevels: BorderStyle read for objects::bevel)
    ("crates/rapidr-value/src/designer/layout.rs", "client_rect"): "RPANEL",
    ("crates/rapidr-value/src/layout.rs", "default_property"): "*",
    ("crates/rapidr-value/src/autosize.rs", "resizes"): "RLABEL",
    ("crates/rapidr-value/src/mdi.rs", "call"): "RFORMMDI",
    ("crates/rapidr-value/src/mdi.rs", "set"): "RFORMMDI",
    ("crates/rapidr-value/src/statusbar.rs", "call"): "RSTATUSBAR",
    ("crates/rapidr-value/src/autosize.rs", "resizes"): "RLABEL",
    ("crates/rapidr-value/src/registry.rs", "get"): "RREGISTRY",
    ("crates/rapidr-value/src/registry.rs", "set"): "RREGISTRY",
    ("crates/rapidr-value/src/registry.rs", "answer"): "RREGISTRY",
    ("crates/rapidr-value/src/objects/bevel.rs", "default"): "RPANEL RBEVEL",
    ("crates/rapidr-value/src/objects/bevel.rs", "qbevel_set"): "RBEVEL",
    ("crates/rapidr-value/src/objects/bitmap.rs", "Bitmap"): PICTURES,
    ("crates/rapidr-value/src/objects/cgi.rs", "*"): "RCGI",
    ("crates/rapidr-value/src/objects/comport.rs", "*"): "RCOMPORT",
    ("crates/rapidr-value/src/objects/d3d/mod.rs", "screen_call"): "RDXSCREEN",
    ("crates/rapidr-value/src/objects/d3d/mod.rs", "get"): "RD3DVECTOR",
    ("crates/rapidr-value/src/objects/d3d/mod.rs", "set"): "RD3DVECTOR",
    ("crates/rapidr-value/src/objects/d3d/mod.rs", "frame_call"): "RD3DFRAME",
    ("crates/rapidr-value/src/objects/d3d/mod.rs", "mesh_call"): "RD3DMESHBUILDER RD3DMESH",
    ("crates/rapidr-value/src/objects/d3d/mod.rs", "face_call"): "RD3DFACE",
    ("crates/rapidr-value/src/objects/d3d/mod.rs", "light_call"): "RD3DLIGHT",
    ("crates/rapidr-value/src/objects/design.rs", "*"): "RDESIGNSURFACE",
    ("crates/rapidr-value/src/objects/directx.rs", "DxScreen"): "RDXSCREEN",
    ("crates/rapidr-value/src/objects/directx.rs", "DxSound"): "RDXSOUND",
    ("crates/rapidr-value/src/objects/dirtree.rs", "*"): "RDIRTREE",
    ("crates/rapidr-value/src/objects/download.rs", "*"): "RDOWNLOAD",
    ("crates/rapidr-value/src/objects/font.rs", "*"): "RFONT",
    ("crates/rapidr-value/src/objects/glass.rs", "*"): "RGLASSFRAME",
    ("crates/rapidr-value/src/objects/icons.rs", "call"): "RBITMAP RIMAGELIST",
    ("crates/rapidr-value/src/dock/runtime.rs", "*"): "RDOCKMANAGER",
    # (I1 / L-PANELS: RapidR Studio's panels)
    ("crates/rapidr-value/src/panels/inspector.rs", "*"): "RPROPERTYINSPECTOR",
    # (a designed component's laid-out Left, Top, Width, Height: any component)
    ("crates/rapidr-value/src/panels/inspector/designer_model.rs", "fallback"): "*",
    ("crates/rapidr-value/src/panels/toolbox.rs", "*"): "RTOOLBOX",
    ("crates/rapidr-value/src/panels/project_tree.rs", "*"): "RPROJECTTREE",
    ("crates/rapidr-value/src/panels/console.rs", "*"): "ROUTPUTCONSOLE",
    ("crates/rapidr-value/src/panels/toolbar.rs", "*"): "RTOOLBAR",
    ("crates/rapidr-value/src/panels/palette.rs", "*"): "RCOMMANDPALETTE",
    ("crates/rapidr-value/src/objects/grid.rs", "*"): "RSTRINGGRID",
    ("crates/rapidr-value/src/objects/header.rs", "call", "member"): "HEADERSECTION",
    ("crates/rapidr-value/src/objects/header.rs", "*"): "RHEADER",
    ("crates/rapidr-value/src/objects/imagelist.rs", "*"): "RIMAGELIST",
    ("crates/rapidr-value/src/objects/joystick.rs", "*"): "RDXJOYSTICK",
    ("crates/rapidr-value/src/objects/list.rs", "file_member"): "RFILELISTBOX",
    ("crates/rapidr-value/src/objects/list.rs", "*"): LISTS,
    ("crates/rapidr-value/src/objects/listview.rs", "*"): "RLISTVIEW",
    ("crates/rapidr-value/src/objects/media.rs", "*"): MEDIA,
    ("crates/rapidr-value/src/objects/memstream.rs", "*"): "RMEMORYSTREAM RFILESTREAM",
    ("crates/rapidr-value/src/objects/menu.rs", "*"): "RMAINMENU RMENUITEM RPOPUPMENU",
    ("crates/rapidr-value/src/objects/mod.rs", "call"): "*",
    ("crates/rapidr-value/src/objects/printer.rs", "*"): "RPRINTER",
    ("crates/rapidr-value/src/objects/tabcontrol.rs", "*"): "RTABCONTROL",
    ("crates/rapidr-value/src/objects/textedit.rs", "*"): TEXTS,
    ("crates/rapidr-value/src/objects/trackbar.rs", "*"): "RTRACKBAR",
    ("crates/rapidr-value/src/objects/tree.rs", "item"): "TREENODE",
    ("crates/rapidr-value/src/objects/tree.rs", "*"): "RTREEVIEW",
}

# Names a site matches on that aren't members: sub-object keys, internal
# properties the runtimes keep for themselves, aliases spelt with a dot.
INTERNAL = re.compile(r"^__|[().=]")


def sources():
    files = []
    for g in GLOBS:
        files += glob.glob(os.path.join(ROOT, g), recursive=True)
    return sorted(f for f in files if not f.endswith(SKIP_FILES))


def enclosing(src, pos):
    """(the nearest `impl X` before pos, the nearest `fn name` before pos)."""
    impls = list(re.finditer(r"^impl(?:<[^>]*>)?\s+(?:[A-Za-z_:]+\s+for\s+)?([A-Za-z_]+)", src[:pos], re.M))
    fns = list(re.finditer(r"\bfn\s+([a-z_0-9]+)", src[:pos]))
    impl = impls[-1] if impls else None
    # (an impl block ends at a line that is just `}` at column 0)
    if impl and re.search(r"^\}", src[impl.end():pos], re.M):
        impl = None
    return (impl.group(1) if impl else None), (fns[-1].group(1) if fns else None)


def sites():
    out = []
    for path in sources():
        rel = os.path.relpath(path, ROOT)
        src = open(path, encoding="utf-8").read()
        # (unit tests at the end of a file aren't dispatch)
        cut = src.find("#[cfg(test)]")
        body_src = src if cut < 0 else src[:cut]
        for m in re.finditer(r"match\s+(?:&?\*?)([a-z_]+)(?:\.as_str\(\))?\s*\{", body_src):
            var = m.group(1)
            if var not in MATCH_VARS:
                continue
            i = m.end()
            depth, j = 1, i
            while depth and j < len(body_src):
                c = body_src[j]
                if c == "{":
                    depth += 1
                elif c == "}":
                    depth -= 1
                j += 1
            block = body_src[i:j]
            # (this match's own arms only: not those of a match inside an arm)
            names = []
            depth = 0
            for text in block.split("\n"):
                if depth == 0:
                    a = re.match(r'^\s*((?:"[^"\n]+"\s*\|\s*)*"[^"\n]+")\s*(?:if [^\n]*)?=>', text)
                    if a:
                        names += re.findall(r'"([^"]+)"', a.group(1))
                code = re.sub(r'"(?:[^"\\]|\\.)*"', '""', text)
                code = re.sub(r"'(?:[^'\\]|\\.)'", "''", code)
                depth += code.count("{") + code.count("(") - code.count("}") - code.count(")")
            if not names:
                continue
            impl, fn = enclosing(body_src, m.start())
            line = body_src[: m.start()].count("\n") + 1
            serves = SITES.get((rel, fn, var)) or SITES.get((rel, fn)) or SITES.get((rel, impl)) or SITES.get((rel, "*"))
            kind = "method" if var in ("method", "method_lower", "m", "lmethod") or (fn or "").endswith(("call", "method")) else "prop"
            out.append({"file": rel, "line": line, "fn": fn, "impl": impl, "var": var, "kind": kind, "serves": serves, "names": names})
    return out


def tuple_sites():
    """`("RTYPE", "member" | …) =>` arms: their component and names."""
    out = []
    pat = re.compile(r'\(\s*"(R[A-Z0-9]+|screen|application|clipboard|mouse|filerec)"\s*,\s*((?:"[a-z0-9_.]+"\s*\|\s*)*"[a-z0-9_.]+")\s*\)\s*(?:if [^\n]*)?=>')
    for path in sources():
        rel = os.path.relpath(path, ROOT)
        src = open(path, encoding="utf-8").read()
        cut = src.find("#[cfg(test)]")
        src = src if cut < 0 else src[:cut]
        for m in pat.finditer(src):
            who = m.group(1)
            out.append({"file": rel, "line": src[: m.start()].count("\n") + 1, "serves": who if who.startswith("R") else who.capitalize(),
                        "names": re.findall(r'"([^"]+)"', m.group(2))})
    return out


def registry():
    """{component or global object name: {lower-case member names}}, from crates/rapidr-lang/data."""
    import tomllib
    data = os.path.join(ROOT, "crates", "rapidr-lang", "data")
    sets = {s["name"]: s for s in tomllib.load(open(os.path.join(data, "sets.toml"), "rb"))["set"]}
    tables = []
    for f in glob.glob(os.path.join(data, "components", "*.toml")):
        tables += tomllib.load(open(f, "rb"))["component"]
    tables += tomllib.load(open(os.path.join(data, "globals.toml"), "rb"))["object"]
    tables += tomllib.load(open(os.path.join(data, "items.toml"), "rb"))["object"]
    out = {}
    for c in tables:
        names = {m["name"].lower() for k in ("properties", "methods", "events") for m in c.get(k, [])}
        for st in c.get("sets", []):
            names |= {m["name"].lower() for k in ("properties", "methods", "events") for m in sets[st].get(k, [])}
        out[c["name"].upper()] = names
    return out


# Names the runtimes match on that are deliberately no component's member:
# what they keep for themselves, other spellings of a member, and the
# desktop's catch-all stubs (gui_generic_method: no-ops any component
# accepts, so a RapidQ program calling them on the wrong one runs on).
NOT_MEMBERS = {
    "items": "the generic list store's key (Item(i) is the member)",
    "focus": "SetFocus's other spelling",
    "copy": "a no-op stub (a text control's is CopyToClipboard)",
    "paste": "a no-op stub (a text control's is PasteFromClipboard)",
    "cut": "a no-op stub (a text control's is CutToClipboard)",
    "getpixel": "a no-op drawing stub (a picture's is Pixel)",
    "loadimage": "a no-op drawing stub (a picture's is LoadFromFile)",
    "saveimage": "a no-op drawing stub (a picture's is SaveToFile)",
}


def check():
    problems = []
    reg = registry()
    every = set().union(*reg.values())
    for s in sites():
        where = f"{s['file']}:{s['line']} (fn {s['fn']})"
        if s["serves"] is None:
            problems.append(f"{where}: a dispatch site missing from SITES (tools/lang_dispatch.py)")
            continue
        if s["serves"] == "-":
            continue
        serves = s["serves"].split()
        for n in s["names"]:
            if not re.match(r"^[a-z][a-z0-9_]*$", n) or n in NOT_MEMBERS:
                continue
            if serves == ["*"]:
                if n not in every:
                    problems.append(f"{where}: `{n}` is no component's member in the registry")
            elif not any(n in reg.get(c.upper(), ()) for c in serves):
                problems.append(f"{where}: `{n}` isn't a member of {' / '.join(serves)} in the registry")
    for t in tuple_sites():
        for n in t["names"]:
            if "." in n or n in NOT_MEMBERS:
                continue
            if n not in reg.get(t["serves"].upper(), ()):
                problems.append(f"{t['file']}:{t['line']}: `{n}` isn't a member of {t['serves']} in the registry")
    return problems


def main():
    if "--check" in sys.argv[1:]:
        problems = check()
        if problems:
            sys.exit(f"lang_dispatch: {len(problems)} names the runtimes answer aren't in the language registry:\n  " + "\n  ".join(problems))
        print("lang_dispatch: every name the runtimes answer is in the language registry")
        return
    json.dump(sites(), sys.stdout, indent=1)


if __name__ == "__main__":
    main()
