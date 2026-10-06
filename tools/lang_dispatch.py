#!/usr/bin/env python3
"""The member names RapidR's runtimes answer, read from their dispatch code.

Every `match` on a property's or method's name in the runtimes (the shared
models in crates/rapidr-value, the desktop runtime in
crates/rapidr-runtime-core, the web runtime in crates/rapidr-runtime-web)
names what that site answers. SITES says which components each site serves
("*" = any component: the generic dispatch; "-" = not a component member:
builtins, internal keys).

    python3 tools/lang_dispatch.py            the sites and their names, as JSON
    python3 tools/lang_dispatch.py --check    every site is in SITES (exit 1 if not)

Used by crates/rapidr-lang's reverse check (tests/reverse.rs runs it) and
by tools/lang_seed.py. Plain text matching, no Rust parser: a site is a
`match <prop|method|…> {` block; its names are the string patterns of its
arms (`"caption" | "text" =>`).
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
    ("crates/rapidr-runtime-core/src/datascience.rs", "num_method"): "RNUM",
    ("crates/rapidr-runtime-core/src/datascience.rs", "num_get_prop"): "RNUM",
    ("crates/rapidr-runtime-core/src/datascience.rs", "num_set_prop"): "RNUM",
    ("crates/rapidr-runtime-core/src/datascience.rs", "dataframe_method"): "RDATAFRAME",
    ("crates/rapidr-runtime-core/src/datascience.rs", "dataframe_get_prop"): "RDATAFRAME",
    ("crates/rapidr-runtime-core/src/datascience.rs", "plot_method"): "RPLOT",
    ("crates/rapidr-runtime-core/src/datascience.rs", "plot_get_prop"): "RPLOT",
    ("crates/rapidr-runtime-core/src/datascience.rs", "plot_set_prop"): "RPLOT",
    ("crates/rapidr-runtime-core/src/layout.rs", "after_set"): "*",
    ("crates/rapidr-runtime-core/src/network.rs", "socket_method"): "RSOCKET",
    ("crates/rapidr-runtime-core/src/network.rs", "method"): "RSOCKET RSERVERSOCKET",
    ("crates/rapidr-runtime-core/src/network.rs", "server_socket_method"): "RSERVERSOCKET",
    ("crates/rapidr-runtime-core/src/network.rs", "http_method"): "RHTTP",
    ("crates/rapidr-runtime-core/src/object.rs", "json_method"): "RJSON",
    ("crates/rapidr-runtime-core/src/object.rs", "gui_generic_method"): "*",
    ("crates/rapidr-runtime-core/src/object.rs", "statusbar_method"): "RSTATUSBAR",
    ("crates/rapidr-runtime-core/src/ui/kernel.rs", "canvas_method"): "RCANVAS RHEADER",
    ("crates/rapidr-runtime-core/src/ui/kernel.rs", "image_method"): "RIMAGE",
    ("crates/rapidr-runtime-core/src/ui/kernel.rs", "tree_method"): "RTREEVIEW",
    ("crates/rapidr-runtime-core/src/ui/kernel.rs", "design_surface_method"): "RDESIGNSURFACE",
    # --- web runtime
    ("crates/rapidr-runtime-web/src/datascience_web.rs", "num_method"): "RNUM",
    ("crates/rapidr-runtime-web/src/datascience_web.rs", "num_get_prop"): "RNUM",
    ("crates/rapidr-runtime-web/src/datascience_web.rs", "dataframe_method"): "RDATAFRAME",
    ("crates/rapidr-runtime-web/src/datascience_web.rs", "dataframe_get_prop"): "RDATAFRAME",
    ("crates/rapidr-runtime-web/src/datascience_web.rs", "plot_method"): "RPLOT",
    ("crates/rapidr-runtime-web/src/datascience_web.rs", "plot_get_prop"): "RPLOT",
    ("crates/rapidr-runtime-web/src/datascience_web.rs", "plot_set_prop"): "RPLOT",
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
    ("crates/rapidr-value/src/data.rs", "builtin"): "-",
    ("crates/rapidr-value/src/input.rs", "vk_of_key"): "-",
    ("crates/rapidr-value/src/lib.rs", "shared_builtin"): "-",
    ("crates/rapidr-value/src/memory.rs", "shared"): "-",
    ("crates/rapidr-value/src/font_dialog.rs", "alias"): "RFONTDIALOG",
    ("crates/rapidr-value/src/font_dialog.rs", "call"): "RFONTDIALOG",
    ("crates/rapidr-value/src/globals.rs", "file_rec"): "FILEREC",
    ("crates/rapidr-value/src/layout.rs", "with"): "*",
    ("crates/rapidr-value/src/layout.rs", "default_property"): "*",
    ("crates/rapidr-value/src/mdi.rs", "call"): "RFORMMDI",
    ("crates/rapidr-value/src/mdi.rs", "set"): "RFORMMDI",
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
    ("crates/rapidr-value/src/objects/grid.rs", "*"): "RSTRINGGRID",
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
            arms = re.findall(r'^\s*((?:"[^"\n]+"\s*\|\s*)*"[^"\n]+")\s*(?:if [^\n]*)?=>', block, re.M)
            names = [n for a in arms for n in re.findall(r'"([^"]+)"', a)]
            if not names:
                continue
            impl, fn = enclosing(body_src, m.start())
            line = body_src[: m.start()].count("\n") + 1
            serves = SITES.get((rel, impl)) or SITES.get((rel, fn)) or SITES.get((rel, "*"))
            kind = "method" if var in ("method", "method_lower", "m", "lmethod") or (fn or "").endswith(("call", "method")) else "prop"
            out.append({"file": rel, "line": line, "fn": fn, "impl": impl, "var": var, "kind": kind, "serves": serves, "names": names})
    return out


def main():
    found = sites()
    if "--check" in sys.argv[1:]:
        unmapped = [f"{s['file']}:{s['line']} (fn {s['fn']}, impl {s['impl']})" for s in found if s["serves"] is None]
        if unmapped:
            sys.exit("lang_dispatch: dispatch sites missing from SITES:\n  " + "\n  ".join(unmapped))
        return
    json.dump(found, sys.stdout, indent=1)


if __name__ == "__main__":
    main()
