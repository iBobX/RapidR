#!/usr/bin/env python3
"""The language registry's first data (crates/rapidr-lang/data), seeded once.

The registry's TOML files are the source of truth now; this records how
they were first made, from the code's truth and RapidQ's facts (never its
words):

- the components: rapidr_ast::COMPONENT_TYPES, the name rules of
  `canonical_type_name`, the manual's categories (the old
  tools/manual_reference.py), the runtimes' dispatch (tools/lang_dispatch.py),
  the old HTML web IDE's hand-written language data (only names the
  runtimes' sources also use; that IDE and its data are gone, so `collect`
  has none of it now);
- RapidQ's facts: its manual's tables (names, types, R/W, defaults,
  parameter lists — from the phatcode mirror, .reference/phatcode), its
  KEYWORD.LST (which builtins, statements and directives are RapidQ's), and
  RC.EXE itself (tools/rc_probe.sh: does RapidQ's compiler know this
  member?) for every member the manual doesn't list;
- the VM (./rapidr): which methods answer (`not implemented` warnings) and
  which builtins and statements compile.

    python3 tools/lang_seed.py collect           → scratch JSON (candidates)
    python3 tools/lang_seed.py vm-probe          → which methods / builtins answer
    python3 tools/lang_seed.py rc-probes <dir>   → .bas files for tools/rc_probe.sh
    python3 tools/lang_seed.py rc-read <report>  → RC.EXE's answers
    python3 tools/lang_seed.py emit [--force]    → crates/rapidr-lang/data/*.toml

Scratch files go to $LANG_SEED_WORK (default /tmp/lang_seed). `emit`
refuses to overwrite the data without --force: after the seed, the TOML is
edited by hand (docs, categories) and is what everything is generated from.
"""

import glob
import html
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import lang_dispatch  # noqa: E402

WORK = os.environ.get("LANG_SEED_WORK", "/tmp/lang_seed")
REF = os.environ.get("RAPIDQ_DOCS") or next(
    (p for p in [os.path.join(ROOT, ".reference", "phatcode"), os.path.join(ROOT, "..", "..", "..", ".reference", "phatcode")] if os.path.isdir(p)),
    "",
)
RAPIDQ = os.path.expanduser(os.environ.get("RAPIDQ_DIR", "~/Downloads/Rapidq"))
RAPIDR = os.path.join(ROOT, "rapidr")
DATA = os.path.join(ROOT, "crates", "rapidr-lang", "data")


def read(rel):
    with open(os.path.join(ROOT, rel), encoding="utf-8") as f:
        return f.read()


def const_list(src, name):
    m = re.search(r"pub const " + re.escape(name) + r"\s*:\s*&\[[^=]*=\s*&\[(.*?)\];", src, re.S)
    body = re.sub(r"//[^\n]*", "", m.group(1))
    return re.findall(r'"([^"]*)"', body)


def const_pairs(src, name):
    m = re.search(r"pub const " + re.escape(name) + r"\s*:[^=]*=\s*&\[(.*?)\];", src, re.S)
    return re.findall(r'\(\s*"([^"]*)"\s*,\s*"([^"]*)"', m.group(1))


def save(name, obj):
    os.makedirs(WORK, exist_ok=True)
    with open(os.path.join(WORK, name), "w") as f:
        json.dump(obj, f, indent=1)


def load(name):
    with open(os.path.join(WORK, name)) as f:
        return json.load(f)


# --- RapidQ's manual: the tables' facts --------------------------------------

SECTION = re.compile(r"<b>\s*(?:<font[^>]*>)?\s*([A-Za-z0-9_ ]*?)\s*(Propert(?:y|ies)|Methods?|Events?|Fields)\s*(?:</font>)?\s*</b>", re.I)
IDENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")


def _clean(c):
    c = re.sub(r"<br\s*/?>", " ", c, flags=re.I)
    c = re.sub(r"<[^>]+>", " ", c)
    c = html.unescape(c).replace("\xa0", " ")
    return re.sub(r"\s+", " ", c).strip()


def _rows(chunk):
    out = []
    for r in re.split(r"<tr\b", chunk, flags=re.I)[1:]:
        r = re.split(r"</tr\s*>", r, flags=re.I)[0]
        cells = re.split(r"<td\b", r, flags=re.I)[1:]
        cs = []
        for c in cells:
            colspan = re.match(r"[^>]*colspan", c, re.I) is not None
            body = c.split(">", 1)[1] if ">" in c else ""
            cs.append((_clean(re.split(r"</td\s*>", body, flags=re.I)[0]), colspan))
        if cs:
            out.append(cs)
    return out


CALLABLE = re.compile(r"^(SUBI|FUNCTIONI|SUB|FUNCTION|VOID|PROCEDURE)\b", re.I)


def parse_doc(path):
    """{properties: [{name, type, rw, default}], methods: [{name, type,
    params}], events: [...]} from one manual page's tables."""
    s = open(path, encoding="latin-1").read()
    marks = [(m.start(), m.group(2).lower()) for m in SECTION.finditer(s)]
    result = {"properties": [], "methods": [], "events": []}
    for i, (pos, kind) in enumerate(marks):
        end = marks[i + 1][0] if i + 1 < len(marks) else len(s)
        kind = {"fields": "properties", "property": "properties", "method": "methods", "event": "events"}.get(kind, kind)
        seen = {x["name"].lower() for x in result[kind]}
        for cs in _rows(s[pos:end]):
            if len(cs) < 3 or cs[0][1]:
                continue
            name = cs[0][0].replace(" ", "").rstrip("()")
            if not IDENT.match(name.rstrip("$")) or name.lower() in ("field", "method", "event", "property", "name", "type"):
                continue
            vals = [{"R/W": "RW"}.get(c, c) for c, _ in cs[1:]]
            if kind == "properties":
                typ = vals[0] if vals else ""
                rw = next((v for v in vals[1:3] if v in ("R", "W", "RW")), "RW")
                default = vals[2] if len(vals) > 2 and vals[1] in ("R", "W", "RW") else ""
                if not typ or typ.lower() in ("contents",):
                    continue
                item = {"name": name, "type": typ, "rw": rw, "default": default}
            else:
                typ = vals[0] if vals else ""
                if not CALLABLE.match(typ) and not (kind == "events" and name.lower().startswith("on")):
                    continue
                item = {"name": name, "type": typ, "params": vals[2] if len(vals) > 2 else ""}
            if name.lower() in seen:
                continue
            seen.add(name.lower())
            result[kind].append(item)
    return result


def doc_file(qname):
    special = {"QBEVEL": "qbevell.html", "QFORMMDI": "QFormMDI.html", "QDIGDISPLAY": "qdigdisplay.htm", "QCGI": "qcgi.htm",
               "SCREEN": "Screen.html", "APPLICATION": "qapplication.html", "CLIPBOARD": "qclipboard.html"}
    if qname in special:
        return os.path.join(REF, special[qname])
    for f in os.listdir(REF):
        base, ext = os.path.splitext(f)
        if base.lower() == qname.lower() and ext.lower() in (".html", ".htm"):
            return os.path.join(REF, f)
    return None


# Pages whose tables aren't tables (a TYPE listing, a Word export, plain
# text): their facts, transcribed.
DOC_FACTS = {
    "QRECT": {"properties": [{"name": n, "type": "INTEGER", "rw": "RW", "default": "0"} for n in ("Left", "Top", "Right", "Bottom")], "methods": [], "events": []},
    "QNOTIFYICONDATA": {"properties": [{"name": n, "type": t, "rw": rw, "default": ""} for n, t, rw in (
        ("cbSize", "INTEGER", "R"), ("hWnd", "INTEGER", "RW"), ("uID", "INTEGER", "RW"), ("uFlags", "INTEGER", "RW"),
        ("uCallBackMessage", "INTEGER", "RW"), ("hIcon", "INTEGER", "RW"), ("szTip", "STRING", "RW"))], "methods": [], "events": []},
    "QDIGDISPLAY": {
        "properties": [{"name": n, "type": t, "rw": rw, "default": d} for n, t, rw, d in (
            ("Align", "INTEGER", "RW", "alNone"), ("ClientHeight", "INTEGER", "RW", ""), ("ClientWidth", "INTEGER", "RW", ""),
            ("Color", "INTEGER", "RW", ""), ("CopyMode", "INTEGER", "RW", "cmBlackness"), ("Cursor", "INTEGER", "RW", "crDefault"),
            ("Display", "STRING", "RW", "0"), ("Enabled", "INTEGER", "RW", "True"), ("Font", "QFONT", "W", ""),
            ("Height", "INTEGER", "RW", "24"), ("Hint", "STRING", "RW", ""), ("Left", "INTEGER", "RW", "0"),
            ("Parent", "QFORM/QPANEL/QTABCONTROL", "W", ""), ("Pixel", "2D ARRAY of INTEGER", "RW", ""),
            ("PopupMenu", "QPOPUPMENU", "W", ""), ("ShowHint", "INTEGER", "RW", "False"), ("Tag", "INTEGER", "RW", ""),
            ("Top", "INTEGER", "RW", "0"), ("Visible", "INTEGER", "RW", "True"), ("Width", "INTEGER", "RW", "12"))],
        "methods": [],
        "events": [{"name": n, "type": t, "params": ""} for n, t in (
            ("OnClick", "VOID"), ("OnDblClick", "VOID"), ("OnMouseDown", "SUB (Button%, X%, Y%, Shift%)"),
            ("OnMouseMove", "SUB (X%, Y%, Shift%)"), ("OnMouseUp", "SUB (Button%, X%, Y%, Shift%)"), ("OnPaint", "VOID"))],
    },
    "QCGI": {
        "properties": [{"name": "AutoConvert", "type": "INTEGER", "rw": "RW", "default": "1"}, {"name": "MaxInput", "type": "INTEGER", "rw": "RW", "default": "32767"}]
        + [{"name": n, "type": "INTEGER" if n in ("ContentLength", "ServerPort") else "STRING", "rw": "R", "default": ""} for n in (
            "Accept", "AuthType", "ContentLength", "ContentType", "Cookie", "GatewayInterface", "PathInfo", "PathTranslated", "Referer",
            "RemoteAddr", "RemoteHost", "RemoteIdent", "RemoteUser", "RequestMethod", "ScriptName", "ServerSoftware", "ServerName",
            "ServerPort", "ServerProtocol", "UserAgent")],
        "methods": [{"name": "Parse", "type": "SUB", "params": ""}, {"name": "Get", "type": "FUNCTION (Name AS STRING, BYREF Value AS STRING) AS INTEGER", "params": "2"}],
        "events": [],
    },
    "SCREEN": {
        "properties": [{"name": n, "type": t, "rw": "R", "default": ""} for n, t in (
            ("ConsoleX", "INTEGER"), ("ConsoleY", "INTEGER"), ("Cursors", "ARRAY of INTEGER"), ("Height", "INTEGER"),
            ("MouseX", "INTEGER"), ("MouseY", "INTEGER"), ("Width", "INTEGER"))] + [{"name": "Cursor", "type": "INTEGER", "rw": "RW", "default": "crDefault"}],
        "methods": [],
        "events": [],
    },
}


# --- The JS data (seed only) --------------------------------------------------

def js_data():
    """The old HTML web IDE's hand-written language data was one of the
    seed's sources; the IDE was deleted (2026-10-08), the registry is the
    source of truth, so there is nothing left to read."""
    return {"components": {}, "builtins": [], "keywords": [], "types": [], "directives": []}


def runtime_literals():
    """Every lower-case string literal in the runtimes and the UI code: a
    name the code never spells isn't one it answers."""
    lits = set()
    for g in lang_dispatch.GLOBS + ["crates/rapidr-ui-app/src/**/*.rs", "crates/rapidr-ui-kernel/src/**/*.rs"]:
        for f in glob.glob(os.path.join(ROOT, g), recursive=True):
            lits.update(re.findall(r'"([a-z][a-z0-9_]*)"', open(f, encoding="utf-8").read()))
    return lits


def casing_dictionary():
    """lower → the spelling programs and docs use (CamelCase), from
    RapidQ's manual pages, KEYWORD.LST, RapidR's docs and source comments."""
    counts = {}

    def add(word):
        if not re.match(r"^[A-Za-z][A-Za-z0-9_]*\$?$", word) or word.islower():
            return
        counts.setdefault(word.lower(), {}).setdefault(word, 0)
        counts[word.lower()][word] += 1

    for f in glob.glob(os.path.join(REF, "*.htm*")):
        for w in re.findall(r"\b[A-Za-z][A-Za-z0-9_]+\b", _clean(open(f, encoding="latin-1").read())):
            add(w)
    for f in glob.glob(os.path.join(ROOT, "docs", "**", "*.md"), recursive=True):
        for w in re.findall(r"[.`]([A-Z][A-Za-z0-9_]+)", open(f, encoding="utf-8").read()):
            add(w)
    for g in lang_dispatch.GLOBS:
        for f in glob.glob(os.path.join(ROOT, g), recursive=True):
            for c in re.findall(r"//[^\n]*", open(f, encoding="utf-8").read()):
                for w in re.findall(r"[.`]([A-Z][A-Za-z0-9_]+)", c):
                    add(w)
    best = {}
    for low, forms in counts.items():
        # (a mixed-case spelling over an all-caps one)
        ranked = sorted(forms.items(), key=lambda kv: (kv[0].isupper(), -kv[1]))
        w = ranked[0][0]
        best[low] = w if not w.isupper() or len(w) <= 2 else w[0] + w[1:].lower()
    return best


# --- the component list's facts (from the manual's old generator,
# tools/manual_reference.py, which the registry replaced) ---------------------

# RapidQ's built-in components: the names RapidQ's compiler (RC.EXE, Rapid-Q
# 2006) knows (docs/rapidq-ground-truth.md). QCOMPORT is RC.EXE's too.
RAPIDQ_BUILTIN = """QBITMAP QBUTTON QCANVAS QCHECKBOX QCOMBOBOX QCOMPORT QCOOLBTN QD3DFACE
QD3DFRAME QD3DLIGHT QD3DMESH QD3DMESHBUILDER QD3DTEXTURE QD3DVECTOR QD3DVISUAL QD3DWRAP
QDIRTREE QDXIMAGELIST QDXJOYSTICK QDXSCREEN QDXSOUND QDXTIMER QEDIT QFILELISTBOX
QFILESTREAM QFONT QFONTDIALOG QFORM QGAUGE QGLASSFRAME QGROUPBOX QHEADER QIMAGE
QIMAGELIST QLABEL QLISTBOX QLISTVIEW QMAINMENU QMEMORYSTREAM QMENUITEM QMYSQL
QNOTIFYICONDATA QOLECONTAINER QOLEOBJECT QOPENDIALOG QOUTLINE QOVALBTN QPANEL
QPOPUPMENU QPRINTER QRADIOBUTTON QRECT QREGISTRY QRICHEDIT QSAVEDIALOG QSCROLLBAR QSCROLLBOX
QSOCKET QSPLITTER QSTATUSBAR QSTRINGGRID QSTRINGLIST QTABCONTROL QTIMER QTRACKBAR
QTREEVIEW""".split()

# Components RapidQ programs get from RapidQ's include libraries (RapidR
# builds them in): the name and the library.
RAPIDQ_LIBRARY = {
    "QFORMMDI": "RAPIDQ2.INC",
    "QFILEDIALOG": "RAPIDQ2.INC",
    "QCOLORDIALOG": "RAPIDQ2.INC",
    "QBEVEL": "QBevel.inc",
    "QDIGDISPLAY": "QDigDisplay.inc",
    "QCGI": "qcgi.inc",
    "QDOWNLOAD": "Qdownload.inc",
    "QMIDI": "QMidi.inc",
    "QWAVE": "QWave.inc",
    "QVIDEO": "QVideo.inc",
    "QCDAUDIO": "Qcdaudio.inc",
}

# What each component is for (the manual's grouping). Every COMPONENT_TYPES
# entry must be here.
CATEGORIES = [
    ("Forms and containers", "RFORM RFORMMDI RPANEL RGROUPBOX RTABCONTROL RSPLITTER RSCROLLBOX RBEVEL RGLASSFRAME RTOOLBAR RSTATUSBAR"),
    ("Buttons and input", "RBUTTON RCOOLBTN ROVALBTN RCHECKBOX RRADIOBUTTON REDIT RMEMO RRICHEDIT RCOMBOBOX RSCROLLBAR RTRACKBAR RUPDOWN RDATETIMEPICKER RCODEEDITOR"),
    ("Display and drawing", "RLABEL RIMAGE RCANVAS RPROGRESSBAR RPROGRESS RDIGDISPLAY RHEADER RDESIGNSURFACE"),
    ("Lists, grids and trees", "RLISTBOX RFILELISTBOX RDIRTREE RLISTVIEW RTREEVIEW RSTRINGGRID"),
    ("Menus", "RMAINMENU RMENUITEM RPOPUPMENU"),
    ("Dialogs", "ROPENDIALOG RSAVEDIALOG RFILEDIALOG RCOLORDIALOG RFONTDIALOG"),
    ("Non-visual objects", "RTIMER RFONT RBITMAP RIMAGELIST RSTRINGLIST RFILESTREAM RMEMORYSTREAM RREGISTRY RPRINTER RRECT RNOTIFYICONDATA RJSON"),
    ("Databases", "RSQLITE RMYSQL"),
    ("Network, devices and CGI", "RSOCKET RSERVERSOCKET RHTTP RDOWNLOAD RCOMPORT RCGI"),
    ("Media", "RMIDI RWAVE RVIDEO RCDAUDIO"),
    ("DirectX 2D", "RDXSCREEN RDXIMAGELIST RDXTIMER RDXSOUND RDXJOYSTICK"),
    ("Direct3D (retained mode)", "RD3DFRAME RD3DMESHBUILDER RD3DMESH RD3DFACE RD3DLIGHT RD3DTEXTURE RD3DVISUAL RD3DWRAP RD3DVECTOR"),
    ("Data science", "RNUM RDATAFRAME RPLOT"),
    ("Web only", "RWEBVIEW RDOM RJAVASCRIPT RWEBSTORAGE RWEBAUDIO RWEBVIDEO RWEBNOTIFICATION RWEBGEOLOCATION RROUTER"),
]

# Where a component works when it isn't everywhere (desktop = native and
# interpreted builds). Checked in the runtimes: RMYSQL and RSERVERSOCKET
# need raw TCP (crates/rapidr-runtime-web/src/database_web.rs warns), the
# web components are the browser's.
WHERE = {
    "RMYSQL": "desktop",
    "RSERVERSOCKET": "desktop",
    "RSOCKET": "desktop (TCP); web (WebSocket)",
    "RCOMPORT": "desktop (serial2); web (Web Serial)",
}


def q_names(r, lib_types, include_lib):
    """The RapidQ names that mean component `r` (canonical_type_name's
    rules, reversed), with where each comes from."""
    names = []
    rest = r[1:]
    q = "Q" + rest
    if q in RAPIDQ_BUILTIN:
        names.append((q, "RapidQ"))
    elif q in RAPIDQ_LIBRARY:
        names.append((q, RAPIDQ_LIBRARY[q]))
    for qq, rr in include_lib:
        if rr == r and qq not in [n for n, _ in names]:
            names.append((qq, RAPIDQ_LIBRARY.get(qq, "include library")))
    if r == "RPROGRESSBAR":
        names.append(("QGAUGE", "RapidQ"))
    if r == "RTREEVIEW":
        names.append(("QOUTLINE", "RapidQ"))
    for t, rr in lib_types:
        if rr == r and not t.startswith("Q"):
            names.append((t, "RAPIDQ2.INC"))
    return names



# --- collect -----------------------------------------------------------------

def collect():
    mr = sys.modules[__name__]
    ast = read("crates/rapidr-ast/src/lib.rs")
    types = const_list(ast, "COMPONENT_TYPES")
    not_yet = const_list(ast, "RAPIDQ_OBJECTS_NOT_YET_IMPLEMENTED")
    include_lib = const_pairs(ast, "INCLUDE_LIBRARY_COMPONENTS")
    lib_types = const_pairs(read("crates/rapidr-ast/src/library.rs"), "LIBRARY_TYPES")
    group_of = {t: title for title, ts in mr.CATEGORIES for t in ts.split()}
    js = js_data()
    lits = runtime_literals()
    sites = lang_dispatch.sites()
    comps = []
    for r in types:
        qnames = mr.q_names(r, lib_types, include_lib)
        docs = {"properties": [], "methods": [], "events": []}
        for q, _ in qnames:
            d = DOC_FACTS.get(q) or (parse_doc(doc_file(q)) if doc_file(q) else None)
            if d:
                for k in docs:
                    have = {x["name"].lower() for x in docs[k]}
                    docs[k] += [dict(x, page=q) for x in d[k] if x["name"].lower() not in have]
        site_names = {"prop": set(), "method": set()}
        for s in sites:
            serves = (s["serves"] or "").split()
            if r in serves:
                for n in s["names"]:
                    if re.match(r"^[a-z][a-z0-9_]*$", n):
                        site_names[s["kind"]].add(n)
        j = js["components"].get(r, {})
        comps.append({
            "name": r,
            "qnames": qnames,
            "group": group_of[r],
            "where": mr.WHERE.get(r),
            "docs": docs,
            "js": {k: [n for n in j.get(k, []) if n in lits] for k in ("props", "methods", "events")},
            "js_dropped": {k: [n for n in j.get(k, []) if n not in lits] for k in ("props", "methods", "events")},
            "js_sigs": j.get("methodSignatures", {}),
            "js_description": j.get("description"),
            "sites": {k: sorted(v) for k, v in site_names.items()},
        })
    # (RapidQ's objects RapidR doesn't have yet; RapidR's BASIC libraries)
    extra = []
    for q in not_yet:
        d = parse_doc(doc_file(q)) if doc_file(q) else {"properties": [], "methods": [], "events": []}
        extra.append({"name": q, "kind": "planned", "docs": d})
    pre = read("crates/rapidr-preprocessor/src/lib.rs")
    for name, file in re.findall(r'\(\s*"([A-Z]+)"\s*,\s*"([^"]+\.inc)"', pre):
        d = parse_doc(doc_file(name)) if doc_file(name) else {"properties": [], "methods": [], "events": []}
        extra.append({"name": name, "kind": "library", "file": file, "docs": d})
    globals_ = []
    for g in ("SCREEN", "APPLICATION", "CLIPBOARD"):
        globals_.append({"name": g, "docs": DOC_FACTS.get(g) or parse_doc(doc_file(g))})
    save("components.json", comps)
    save("extra.json", extra)
    save("globals.json", globals_)
    save("js.json", js)
    save("casing.json", casing_dictionary())
    save("sites.json", sites)
    print(f"collected {len(comps)} components, {len(extra)} others, into {WORK}")


# --- RC.EXE's own member table ---------------------------------------------------

def rc_table():
    """RapidQ's compiler lists its objects' members as pairs of strings
    (`QDXJOYSTICK`, `ISLEFT`, `QDXJOYSTICK`, `ISRIGHT`, …: docs/rapidq-
    ground-truth.md): {class: {MEMBER}}."""
    data = open(os.path.join(RAPIDQ, "RC.EXE"), "rb").read()
    lines = [m.decode("latin-1") for m in re.findall(rb"[\x20-\x7e]{3,}", data)]
    classes = {l for l in lines if re.match(r"^Q[A-Z0-9]{2,}$", l)}
    pairs = {}
    for i in range(len(lines) - 2):
        a, b, c = lines[i], lines[i + 1], lines[i + 2]
        if a in classes and c in classes and re.match(r"^[A-Z][A-Z0-9_]*$", b) and b not in classes:
            pairs.setdefault(a, set()).add(b)
    # (a short run of capitals that happens to start with Q isn't an object:
    # one with a single member must be in KEYWORD.LST's object list)
    objects = {n.upper() for sec, n, _ in keyword_list() if sec == "RapidQ Objects"}
    return {k: v for k, v in pairs.items() if len(v) > 1 or k in objects}


# Members RC.EXE knows but neither the manual nor RapidR lists: which kind.
RC_METHODS = {"ARRANGEICONS", "CASCADE", "NEXT", "PREVIOUS", "TILE", "GET", "PUT", "DELETECOLUMN", "LOADUDTARRAY", "READBYTE", "SAVEUDTARRAY",
              "WRITEBYTE", "LOADBLOB", "SAVEBLOB", "STARTDRAG", "CREATEANIMATION", "CREATEANIMATIONSET", "RECREATEBUF", "CLEAR", "SETSIZE",
              "FREE", "RESUME", "SUSPEND", "TERMINATE", "EXECUTE", "ADJUSTFORMSIZE", "COPY", "CREATE", "CREATEOBJECT", "INVOKE", "PASTE", "SHOW",
              "UPDATEOBJECT", "ASSIGNOBJECT", "CREATEREMOTE", "GETACTIVEOBJECT", "GETIDOFNAME", "GETOBJECT", "INVOKECOPY"}


def rc_kind(member):
    if member.startswith("ON") or member == "WNDPROC":
        return "events"
    return "methods" if member in RC_METHODS else "properties"


# --- candidates: what each component's members may be -------------------------

KINDS = ("properties", "methods", "events")


def plan():
    comps = load("components.json")
    sites = load("sites.json")
    casing = load("casing.json")
    docs_of = {c["name"]: {k: {x["name"].lower() for x in c["docs"][k]} for k in KINDS} for c in comps}
    table = rc_table()
    save("rc_table.json", {k: sorted(v) for k, v in table.items()})
    out = []
    for c in comps:
        r = c["name"]
        # (a name the manual lists twice — QIMAGE's Center as a property and
        # a method — is the first)
        members = {k: [] for k in KINDS}
        have = set()
        for k in KINDS:
            for x in c["docs"][k]:
                if x["name"].lower() not in have:
                    have.add(x["name"].lower())
                    members[k].append(dict(x, source="docs"))

        def add(kind, low, source):
            if low in have:
                return
            have.add(low)
            members[kind].append({"name": casing.get(low, low[:1].upper() + low[1:]), "source": source})

        for k, jk in (("properties", "props"), ("methods", "methods"), ("events", "events")):
            for n in c["js"][jk]:
                add(k, n.lower(), "js")
        runtimes = {}
        for s in sites:
            serves = (s["serves"] or "").split()
            if r not in serves:
                continue
            siblings = [x for x in serves if x != r]
            crate = "desktop" if "runtime-core" in s["file"] else "web" if "runtime-web" in s["file"] else "shared"
            for n in s["names"]:
                if not re.match(r"^[a-z][a-z0-9_]*$", n):
                    continue
                runtimes.setdefault(n, set()).add(crate)
                # (a shared model's member another component documents)
                if any(n in docs_of.get(x, {}).get(k, ()) for x in siblings for k in KINDS) and not any(n in docs_of[r][k] for k in KINDS):
                    continue
                add("properties" if s["kind"] == "prop" else "methods", n, "site")
        # (RapidQ's members only its compiler lists)
        for q, _ in c["qnames"]:
            for member in sorted(table.get(q, ())):
                if member.lower() not in have:
                    add(rc_kind(member), member.lower(), "rc")
        # (a name only one runtime's own dispatch answers: that runtime's)
        for k in KINDS:
            for m in members[k]:
                rt = runtimes.get(m["name"].lower(), set())
                if len(rt) == 1 and "shared" not in rt:
                    m["only"] = next(iter(rt))
        out.append({"name": r, "rapidq": bool(c["qnames"]), "members": members})
    save("plan.json", out)
    n = sum(len(m) for c in out for m in c["members"].values())
    extras = sum(1 for c in out if c["rapidq"] for k in KINDS for m in c["members"][k] if m["source"] != "docs")
    print(f"{n} members; {extras} on RapidQ components not in RapidQ's manual (RC.EXE decides)")


# --- the VM's answers ----------------------------------------------------------

def basic_type(t):
    return {"%": "INTEGER", "&": "LONG", "$": "STRING", "#": "DOUBLE", "!": "SINGLE"}.get(t, "")


def parse_params(text):
    """RapidQ's notation (`SUB (x%, y%, S AS STRING)`, `FUNCTION (Text$)
    AS WORD`, `SUBI`) → ([(name, type, byref, variadic)], returns)."""
    t = text.strip()
    t = re.sub(r"\(\*\*\*\)", "", t)
    m = re.match(r"^(SUBI|FUNCTIONI|SUB|FUNCTION|VOID|PROCEDURE)?\s*(?:[A-Za-z_]\w*[%&$#!]?)?\s*(\((.*?)\))?\s*(?:AS\s+(\w+))?", t, re.I)
    head = (m.group(1) or "").upper() if m else ""
    inner = m.group(3) if m and m.group(2) else ""
    returns = (m.group(4) or "").upper() if m else ""
    if not m.group(2) if m else True:
        # (`FUNCTION Bool%`: the result's type by its suffix)
        mm = re.match(r"^FUNCTION\s+\w+([%&$#!])", t, re.I)
        if mm:
            returns = basic_type(mm.group(1))
    params = []
    if inner:
        for p in [x.strip() for x in inner.split(",") if x.strip()]:
            byref = bool(re.match(r"(?i)^byref\s+", p))
            p = re.sub(r"(?i)^by(ref|val)\s+", "", p)
            mm = re.match(r"^([A-Za-z_][\w]*)([%&$#!])?(\(\))?\s*(?:AS\s+([\w]+))?", p, re.I)
            if not mm:
                continue
            name, suffix, arr, ty = mm.groups()
            ty = (ty or basic_type(suffix or "")).upper()
            params.append({"name": name + ("()" if arr else ""), "type": ty, "byref": byref})
    if head in ("SUBI", "FUNCTIONI"):
        params.append({"name": "Items", "type": "", "byref": False, "variadic": True})
    if head == "FUNCTION" and not returns:
        returns = "VARIANT"
    return params, returns


def dummy(ty, name):
    ty = (ty or "").upper()
    if ty == "STRING" or name.endswith("$"):
        return '""'
    if ty.startswith("Q") or name.lower() in ("rect", "r", "bmp", "image", "stream"):
        return "0"
    return "0"


def probe_program(r, member, kind, params, visual):
    lines = []
    if r == "RPRINTER":
        # (RapidQ's printer is the global Printer, called below)
        pass
    elif visual:
        lines += ["CREATE F AS QFORM", f"  CREATE O AS {r}", "  END CREATE", "END CREATE"]
    else:
        lines += [f"DIM O AS {r}"]
    if kind == "methods":
        args = ", ".join(dummy(p["type"], p["name"]) for p in params if not p.get("variadic"))
        obj = "Printer" if r == "RPRINTER" else "O"
        lines.append(f"{obj}.{member}" + (f" {args}" if args else ""))
    lines.append('PRINT "done"')
    return "\n".join(lines) + "\n"


def vm_probe():
    """Each method called once on the VM: whether it answers (no `not
    implemented` / `no such method`), blocks (a modal wait), or fails."""
    pl = load("plan.json")
    comps = {c["name"]: c for c in load("components.json")}
    work = os.path.join(WORK, "vm")
    os.makedirs(work, exist_ok=True)
    env = dict(os.environ, RAPIDR_PRINT_TO=os.path.join(work, "prints"), RAPIDR_REGISTRY=os.path.join(work, "registry.reg"),
               RAPIDR_TEST_CLIPBOARD="1", RAPIDR_TEST_JOYSTICK="", RAPIDR_TEST_COMPORT="", RAPIDR_TEST_SOUND="", RAPIDR_TEST_MIDI="",
               RAPIDR_TEST_WAVE_IN="", RAPIDR_TEST_FILE_DIALOG="", RAPIDR_TEST_COLOR_DIALOG="", RAPIDR_TEST_FONT_DIALOG="",
               RAPIDR_TEST_MESSAGE_DIALOG="", RAPIDR_TEST_EVENTS="__none.onclick", RAPIDR_TEST_HTTP="")
    os.makedirs(env["RAPIDR_PRINT_TO"], exist_ok=True)
    only = sys.argv[2:] 
    results = {} if not only else load("vm.json")
    for c in pl:
        r = c["name"]
        if only and r not in only:
            continue
        visual = any(p["name"].lower() == "left" for p in c["members"]["properties"])
        for m in c["members"]["methods"]:
            params, _ = parse_params(m.get("type", "SUB"))
            src = os.path.join(work, "p.bas")
            open(src, "w").write(probe_program(r, m["name"], "methods", params, visual))
            bc = os.path.join(work, "p.rrbc")
            b = subprocess.run([RAPIDR, "build-bc", src, "-o", bc], capture_output=True, text=True, env=env, cwd=work)
            key = f"{r}.{m['name'].lower()}"
            if b.returncode:
                results[key] = {"status": "compile", "detail": (b.stdout + b.stderr).strip()[-300:]}
                continue
            try:
                p = subprocess.run([RAPIDR, "run-bc", bc], capture_output=True, text=True, env=env, cwd=work, timeout=6, stdin=subprocess.DEVNULL)
                err = p.stderr
                if "not implemented" in err or "no such method" in err:
                    status = "missing"
                elif "done" in p.stdout:
                    status = "ok"
                else:
                    status = "error"
                results[key] = {"status": status, "detail": err.strip()[-300:]}
            except subprocess.TimeoutExpired:
                results[key] = {"status": "blocks", "detail": ""}
        print(r, sum(1 for k, v in results.items() if k.startswith(r + ".") and v["status"] == "ok"), "/", len(c["members"]["methods"]), flush=True)
    save("vm.json", results)


def vm_props():
    """Every readable property read once on a new component (the VM): what
    a program reads before it sets anything."""
    pl = load("plan.json")
    work = os.path.join(WORK, "vm")
    os.makedirs(work, exist_ok=True)
    env = dict(os.environ, RAPIDR_PRINT_TO=os.path.join(work, "prints"), RAPIDR_REGISTRY=os.path.join(work, "registry.reg"),
               RAPIDR_TEST_CLIPBOARD="1", RAPIDR_TEST_JOYSTICK="", RAPIDR_TEST_COMPORT="", RAPIDR_TEST_SOUND="", RAPIDR_TEST_MIDI="", RAPIDR_TEST_WAVE_IN="")
    results = {}
    for c in pl:
        r = c["name"]
        visual = any(p["name"].lower() == "left" for p in c["members"]["properties"])
        props = [p for p in c["members"]["properties"] if p.get("rw", "RW") != "W" and IDENT.match(p["name"])]
        head = ["CREATE F AS QFORM", f"  CREATE O AS {r}", "  END CREATE", "END CREATE"] if visual else [f"DIM O AS {r}"]
        obj = "Printer" if r == "RPRINTER" else "O"
        if r == "RPRINTER":
            head = []
        body = [f'PRINT "@{p["name"]}="; {obj}.{p["name"]}' for p in props]
        src = os.path.join(work, "props.bas")
        open(src, "w").write("\n".join(head + body) + "\n")
        bc = os.path.join(work, "props.rrbc")
        b = subprocess.run([RAPIDR, "build-bc", src, "-o", bc], capture_output=True, text=True, env=env, cwd=work)
        if b.returncode:
            print(r, "compile:", (b.stdout + b.stderr)[-300:])
            continue
        try:
            p = subprocess.run([RAPIDR, "run-bc", bc], capture_output=True, text=True, env=env, cwd=work, timeout=10, stdin=subprocess.DEVNULL)
        except subprocess.TimeoutExpired:
            print(r, "timeout")
            continue
        vals = {}
        for line in p.stdout.split("\n"):
            m = re.match(r"^@([A-Za-z_0-9]+)=(.*)$", line)
            if m:
                vals[m.group(1).lower()] = m.group(2)
        results[r] = vals
        if p.returncode:
            print(r, "exit", p.returncode, p.stderr[-200:])
    save("vm_props.json", results)
    print(f"{len(results)} components read")


# --- RC.EXE's answers ------------------------------------------------------------

def rc_probes(outdir):
    """One .bas per member RapidQ's manual doesn't list (on a component
    RapidQ has): RC.EXE compiles it or says the member isn't QX's.
    Compile-only (g_ names: tools/windows/rc_probe.ps1 doesn't run them)."""
    pl = load("plan.json")
    comps = {c["name"]: c for c in load("components.json")}
    os.makedirs(outdir, exist_ok=True)
    index = {}
    n = 0
    for c in pl:
        if not c["rapidq"]:
            continue
        q = comps[c["name"]]["qnames"][0][0]
        if q == "COMPORT":
            q = "QCOMPORT"
        lib = {"QFORMMDI": "RAPIDQ2.INC", "QFILEDIALOG": "RAPIDQ2.INC", "QCOLORDIALOG": "RAPIDQ2.INC", "QBEVEL": "QBevel.inc",
               "QDIGDISPLAY": "QDigDisplay.inc", "QCGI": "qcgi.inc", "QDOWNLOAD": "Qdownload.inc", "QMIDI": "QMidi.inc", "QWAVE": "QWave.inc",
               "QVIDEO": "QVideo.inc", "QCDAUDIO": "Qcdaudio.inc", "QCOMPORT": "QComPort.inc"}.get(q)
        for k in KINDS:
            for m in c["members"][k]:
                if m["source"] == "docs":
                    continue
                n += 1
                name = f"g_{n:04d}.bas"
                head = ['$INCLUDE "RAPIDQ.INC"'] + ([f'$INCLUDE "{lib}"'] if lib else [])
                if k == "events":
                    body = ["SUB Handler", "END SUB", f"DIM O AS {q}", f"O.{m['name']} = Handler"]
                elif k == "methods":
                    body = [f"DIM O AS {q}", f"O.{m['name']}"]
                else:
                    body = [f"DIM O AS {q}", f"DEFINT x = 0", f"x = O.{m['name']}"]
                open(os.path.join(outdir, name), "w", newline="\r\n").write("\n".join(head + body) + "\n")
                index[name] = [c["name"], k, m["name"]]
    save("rc_index.json", index)
    print(f"{n} programs in {outdir}: tools/rc_probe.sh {outdir} > report.txt; then rc-read")


def rc_read(report):
    """RapidQ's compiler's verdicts: a member it says is not part of the
    class isn't RapidQ's."""
    index = load("rc_index.json")
    text = open(report, encoding="utf-8", errors="replace").read()
    verdict = {}
    for block in re.split(r"^== ", text, flags=re.M)[1:]:
        name = block.split("\n", 1)[0].strip()
        if name not in index:
            continue
        unknown = re.search(r"not part of (class|type)|has no property named|Can't find property|Undefined|Unknown", block, re.I)
        verdict[name] = {"member": index[name], "rapidq": not unknown, "said": " ".join(l.strip() for l in block.split("\n")[1:] if "ERROR" in l)[:200]}
    save("rc.json", verdict)
    print(f"{len(verdict)} verdicts; {sum(1 for v in verdict.values() if v['rapidq'])} RapidQ members")


# --- builtins, statements, directives ----------------------------------------

# Names in BUILTINS that are the compilers' own lowerings of statements or
# of other spellings, not names a program writes.
INTERNAL_BUILTINS = {"print", "println", "print_hash", "write_hash", "input_func", "input_field", "line_input",
                     "date_func", "time_func", "rapidr__waitkey", "end", "e", "math.e", "math.pi", "get"}


def builtin_key(name):
    k = name.lower()
    if len(k) > 1 and k[-1] in "$%#&!":
        k = k[:-1]
    return k


def keyword_list():
    """RapidQ's KEYWORD.LST: [(section, NAME, syntax line or "")]."""
    src = open(os.path.join(RAPIDQ, "KEYWORD.LST"), encoding="latin-1").read().replace("\r", "")
    sec, out, last = None, [], None
    for line in src.split("\n"):
        if line.startswith(";"):
            t = line[1:].strip()
            if t and set(t) != {"="} and "keyword command list" not in t:
                sec = t
            continue
        if line.startswith("'"):
            if last is not None and not out[last][2]:
                body = line[1:].strip()
                name = out[last][1]
                if body.upper().startswith(name.upper().rstrip("$")) or body.upper().startswith(name.upper()):
                    out[last] = (out[last][0], name, body)
            continue
        if line.strip():
            out.append((sec, line.strip(), ""))
            last = len(out) - 1
    return out


def builtins_collect():
    bsrc = read("interpreter/rapidr-bytecode/src/builtins.rs")
    builtins = const_list(bsrc, "BUILTINS")
    bare = set(const_list(bsrc, "BARE_BUILTINS"))
    m = re.search(r"const RAPIDQ_BUILTINS: &\[&str\] = &\[(.*?)\];", read("interpreter/rapidr-bcgen/src/lib.rs"), re.S)
    rq_builtins = re.findall(r'"([^"]+)"', m.group(1))
    kw = keyword_list()
    kw_keys = {}
    for sec, name, syntax in kw:
        if sec in ("Standard BASIC commands", "Console commands", "Internal commands/functions") and re.match(r"^[A-Za-z_][A-Za-z0-9_$]*$", name):
            kw_keys.setdefault(builtin_key(name), (name, syntax, sec))
    keys = sorted({k for k in builtins if not k.startswith("__") and k not in INTERNAL_BUILTINS} | set(rq_builtins))
    work = os.path.join(WORK, "vm")
    os.makedirs(work, exist_ok=True)
    out = []
    for k in keys:
        name, syntax, sec = kw_keys.get(k, (k.upper(), "", None))
        status = "ok" if k in builtins else None
        if status is None:
            # (a RapidQ builtin the compilers handle themselves, or one they
            # don't support yet: bcgen says so)
            status = "missing"
            # (as a function of a variable, and as a statement; PARAMSTR$ …
            # only inside a SUBI, where the parser lowers them)
            if k in ("paramstr", "paramstrcount", "paramval", "paramvalcount"):
                status = "compiler"
            for text in (f"DIM v AS INTEGER\nx = {name}(v)\n", f"{name} 1\n"):
                src = os.path.join(work, "b.bas")
                open(src, "w").write(text)
                b = subprocess.run([RAPIDR, "build-bc", src, "-o", os.path.join(work, "b.rrbc")], capture_output=True, text=True, cwd=work)
                if b.returncode == 0:
                    status = "compiler"
        out.append({"key": k, "name": name, "syntax": syntax, "section": sec, "bare": k in bare, "status": status,
                    "rapidq": k in rq_builtins or k in kw_keys})
    # (the rest of KEYWORD.LST: statements, operators, directives, types)
    save("builtins.json", out)
    save("keywords.json", kw)
    print(f"{len(out)} builtins: {sum(1 for b in out if b['status'] == 'missing')} missing, {sum(1 for b in out if not b['rapidq'])} to ask RC.EXE about")


def rc_builtin_probes(outdir):
    """RC.EXE with $TYPECHECK ON says `Undefined symbol` for a name it doesn't know."""
    bs = load("builtins.json")
    os.makedirs(outdir, exist_ok=True)
    index = {}
    for i, b in enumerate(x for x in bs if not x["rapidq"]):
        name = f"g_b{i:03d}.bas"
        open(os.path.join(outdir, name), "w", newline="\r\n").write(f'$TYPECHECK ON\n$INCLUDE "RAPIDQ.INC"\nDIM x AS VARIANT\nx = {b["name"]}(1)\n')
        index[name] = b["key"]
    save("rc_builtin_index.json", index)
    print(f"{len(index)} programs in {outdir}")


def rc_builtin_read(report):
    index = load("rc_builtin_index.json")
    bs = load("builtins.json")
    text = open(report, encoding="utf-8", errors="replace").read()
    rq = set()
    for block in re.split(r"^== ", text, flags=re.M)[1:]:
        name = block.split("\n", 1)[0].strip()
        if name in index and "Undefined symbol" not in block:
            rq.add(index[name])
    for b in bs:
        if not b["rapidq"] and b["key"] in rq:
            b["rapidq"] = True
            b["rc"] = True
    save("builtins.json", bs)
    print(f"RC.EXE knows {len(rq)} more: {sorted(rq)}")


# --- emit ----------------------------------------------------------------------

def toml_str(s):
    return json.dumps(s, ensure_ascii=False)


def toml_val(v):
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        return str(v)
    if isinstance(v, (list, tuple)):
        return "[" + ", ".join(toml_val(x) for x in v) + "]"
    return toml_str(v)


def inline(d):
    return "{ " + ", ".join(f"{k} = {toml_val(v)}" for k, v in d.items() if v is not None) + " }"


def constants():
    """RAPIDQ.INC's constants in runs of one prefix (the order the
    preprocessor lists them), the library includes', RapidR's."""
    pre = read("crates/rapidr-preprocessor/src/lib.rs")
    body = re.search(r"pub const RAPIDQ_INC_CONSTANTS: &\[\(&str, i64\)\] = &\[(.*?)\n\];", pre, re.S).group(1)
    pairs = []
    for name, val in re.findall(r'\("([A-Za-z_0-9]+)",\s*(-?(?:0x[0-9A-Fa-f_]+|\d+))\)', body):
        pairs.append((name, int(val.replace("_", ""), 0)))
    libs = []
    for file, inner in re.findall(r'\("([a-z]+\.inc)",\s*&\[(.*?)\]\)', pre):
        libs.append((file, [(n, int(v)) for n, v in re.findall(r'\("([A-Z_0-9]+)",\s*(-?\d+)\)', inner)]))
    ast = read("crates/rapidr-ast/src/lib.rs")
    rapidr = [(n, int(v)) for n, v in re.findall(r'\("(ak[a-z]+)",\s*(\d+)\)', re.search(r"RAPIDR_CONSTANTS.*?\];", ast, re.S).group(0))]
    return pairs, libs, rapidr


def prefix_of(name):
    m = re.match(r"^([A-Z]+_)", name) or re.match(r"^([a-z]+)(?=[A-Z0-9])", name) or re.match(r"^([A-Z][a-z]+)(?=[A-Z_])", name)
    return m.group(1) if m else name


def const_runs(pairs):
    runs = []
    for n, v in pairs:
        p = prefix_of(n)
        if runs and runs[-1][0] == p:
            runs[-1][1].append((n, v))
        else:
            runs.append((p, [(n, v)]))
    return runs


ENUM_BY_NAME = {"align": "alNone", "cursor": "crDefault", "borderstyle": "bsSingle", "windowstate": "wsNormal", "formstyle": "fsNormal",
                "alignment": "taLeftJustify", "layout": "blBMPLeft", "kind": "bkCustom", "modalresult": "mrNone", "copymode": "cmBlackness",
                "charcase": "ecNormal", "bevelinner": "bvNone", "bevelouter": "bvNone", "scrollbars": "ssNone", "viewstyle": "vsIcon",
                "tickmarks": "tmBottomRight", "tickstyle": "tsAuto", "orientation": "tbHorizontal", "pitch": "fpDefault", "charset": "DEFAULT_CHARSET",
                "pixelformat": "pfDevice", "labelstyle": "tsNone"}
COMPONENT_TYPE = re.compile(r"^Q[A-Z0-9]+(/Q?[A-Za-z0-9]+)*$")


def prop_type(name, typ, default, runs_by_const):
    """The registry's type for a manual's `INTEGER` / `QFONT` / `ARRAY of
    STRING` …, with its enum values and component kinds."""
    t = typ.strip()
    m = re.match(r"^(2D ARRAY of \w+|ARRAY of \w+|Array of \w+|[A-Za-z][A-Za-z0-9]*(?:\s*/\s*[A-Za-z0-9]+)*)", t)
    t = re.sub(r"\s+", "", m.group(1)) if m else t
    out = {}
    low = name.lower()
    up = t.upper()
    if up.startswith("2DARRAYOF"):
        out["indexed"] = 2
        up = up[len("2DARRAYOF"):]
    elif up.startswith("ARRAYOF"):
        out["indexed"] = 1
        up = up[len("ARRAYOF"):]
    if up in ("STRING",):
        ty = "picture" if low in ("bmp", "icon", "picture", "ico") else "string"
    elif up in ("SINGLE", "DOUBLE"):
        ty = "float"
    elif up in ("BOOLEAN",) or default.lower() in ("true", "false"):
        ty = "bool"
    elif up == "QFONT":
        ty = "font"
    elif up == "RESOURCE":
        ty = "resource"
    elif COMPONENT_TYPE.match(up) or up in ("QGAUGE", "QTIMER"):
        ty = "component"
        out["kinds"] = [k.strip() for k in up.split("/")]
    elif up in ("INTEGER", "LONG", "SHORT", "BYTE", "WORD", "DWORD", ""):
        ty = "int"
        rep = default if default in runs_by_const else ENUM_BY_NAME.get(low)
        if low.endswith("color") or low.endswith("colour"):
            ty = "color"
        elif rep in runs_by_const and not re.match(r"^-?\d+$", default or "x"):
            ty = "enum"
            out["values"] = runs_by_const[rep]
    else:
        ty = "any"
    out = {"type": ty, **out}
    return out


def norm_default(default, ty, consts):
    d = default.strip().strip('"')
    if not d or d.lower() in ("null", "none"):
        return None
    if ty == "bool":
        return {"true": True, "false": False}.get(d.lower())
    if ty in ("string", "picture"):
        return d
    if re.match(r"^-?\d+$", d):
        return int(d)
    if re.match(r"^-?\d+\.\d+$", d):
        return float(d)
    if d.lower() in consts:
        return consts[d.lower()]
    return None


def params_text(params):
    parts = []
    for p in params:
        s = ("BYREF " if p.get("byref") else "") + p["name"] + (f" AS {p['type']}" if p.get("type") else "")
        if p.get("variadic"):
            s += " ..."
        parts.append(s)
    return ", ".join(parts)


def js_params(sig):
    """`DrawText(x, y, text [, color])` → x, y, text, [color]."""
    m = re.match(r"^\s*\w+\s*\((.*)\)", sig or "")
    if not m:
        return None
    inner = m.group(1)
    opt = False
    out = []
    for tok in re.split(r",", inner.replace("[", ",[").replace("]", "")):
        tok = tok.strip()
        if not tok:
            continue
        if tok.startswith("["):
            opt = True
            tok = tok[1:].strip()
        if tok.startswith("...") or tok.startswith("…"):
            if out and not out[-1].endswith("..."):
                out[-1] += " ..."
            continue
        name = re.sub(r"[^A-Za-z0-9_$]", "", tok.split()[0]) if tok.split() else ""
        if not name:
            continue
        out.append(("[" + name + "]") if opt else name)
    return ", ".join(out)


GROUP_FILES = [
    ("Forms and containers", "forms"), ("Buttons and input", "input"), ("Display and drawing", "display"),
    ("Lists, grids and trees", "lists"), ("Menus", "menus"), ("Dialogs", "dialogs"), ("Non-visual objects", "objects"),
    ("Databases", "databases"), ("Network, devices and CGI", "network"), ("Media", "media"), ("DirectX 2D", "directx"),
    ("Direct3D (retained mode)", "d3d"), ("Data science", "datascience"), ("Web only", "web"),
]
DESIGN_NOT = {"handle", "parent", "itemcount", "selcount", "linecount", "clientwidth", "clientheight", "pixel", "item", "selected"}
EXTENSION_SETS = {"layout": {"anchors", "minwidth", "minheight", "maxwidth", "maxheight"}, "a11y": {"accessiblename", "accessibledescription"}}
CONTAINERS = {"RFORM", "RFORMMDI", "RPANEL", "RGROUPBOX", "RTABCONTROL", "RSCROLLBOX", "RBEVEL", "RGLASSFRAME", "RTOOLBAR", "RSTATUSBAR"}


def default_event(r, events):
    names = [e["name"] for e in events]
    low = {n.lower(): n for n in names}
    for pref in (["ontimer"] + (["onchange"] if r in ("REDIT", "RMEMO", "RRICHEDIT", "RCODEEDITOR", "RTRACKBAR", "RSCROLLBAR", "RUPDOWN", "RDATETIMEPICKER") else [])
                 + (["onshow"] if r in ("RFORM", "RFORMMDI") else []) + ["onclick", "onchange"]):
        if pref in low:
            return low[pref]
    return names[0] if names else None


def value_methods():
    """rapidr_value::members' rule (`Obj.Member` read without parentheses
    calls the method): (ANY, {type: names})."""
    src = read("crates/rapidr-value/src/members.rs")
    lists = {n: re.findall(r'"([a-z_]+)"', body) for n, body in re.findall(r"const ([A-Z_]+): &\[&str\] = &\[(.*?)\];", src, re.S)}
    by_type = {}
    m = re.search(r"fn methods_of.*?match t \{(.*?)\n    \}", src, re.S)
    for types, names in re.findall(r'((?:"R[A-Z0-9]+"\s*\|?\s*)+)=> &\[([A-Z_, ]+)\]', m.group(1)):
        for t in re.findall(r'"(R[A-Z0-9]+)"', types):
            by_type[t] = {x for n in names.split(",") for x in lists[n.strip()]}
    return set(lists["ANY"]), by_type


def emit(force):
    if os.path.isdir(DATA) and os.listdir(DATA) and not force:
        sys.exit(f"{DATA} has data already (it's hand-edited now): --force to overwrite")
    os.makedirs(os.path.join(DATA, "components"), exist_ok=True)
    comps = {c["name"]: c for c in load("components.json")}
    pl = load("plan.json")
    vm = load("vm.json")
    rc = {tuple(v["member"]): v["rapidq"] for v in load("rc.json").values()}
    vprops = load("vm_props.json")
    table = {k: set(v) for k, v in load("rc_table.json").items()}
    any_values, type_values = value_methods()
    pairs, libs, rapidr_consts = constants()
    consts = {n.lower(): v for n, v in pairs}
    for _, cs in libs:
        consts.update({n.lower(): v for n, v in cs})
    runs = const_runs(pairs)
    runs_by_const = {}
    for _, cs in runs:
        for n, _ in cs:
            runs_by_const[n] = [x for x, _ in cs]
    # (a property RapidQ's manual types somewhere: that type, wherever it is)
    doc_types = {}
    for c in pl:
        for p in c["members"]["properties"]:
            if p["source"] == "docs" and p["name"].lower() not in doc_types:
                t = prop_type(p["name"], p.get("type", ""), "", runs_by_const)
                t.pop("values", None) if t["type"] != "enum" else None
                doc_types[p["name"].lower()] = t
    doc_types.update({"fontsize": {"type": "int"}, "fontname": {"type": "string"}, "fontcolor": {"type": "color"}, "text": {"type": "string"}})
    by_group = {}
    for c in pl:
        r = c["name"]
        meta = comps[r]
        is_web = (meta.get("where") or "") == "web" or meta["group"] == "Web only"
        qn = meta["qnames"]
        entry = {"name": r}
        if qn:
            # (the first is RapidQ's own name; the others mean it too)
            q0, src0 = qn[0]
            entry["rapidq"] = q0
            if src0 not in ("RapidQ",):
                entry["from"] = src0
            aliases = [q for q, _ in qn[1:]]
            if aliases:
                entry["aliases"] = aliases
        props_low = {p["name"].lower() for p in c["members"]["properties"]}
        visual = "left" in props_low
        entry["visual"] = visual
        if r in CONTAINERS:
            entry["container"] = True
        if visual and r in vprops and re.match(r"^\d+$", vprops[r].get("width", "")) and re.match(r"^\d+$", vprops[r].get("height", "")):
            entry["size"] = [int(vprops[r]["width"]), int(vprops[r]["height"])]
        if meta.get("where"):
            entry["only"] = {"desktop": "desktop", "web": "web"}.get(meta["where"], None)
            if entry["only"] is None:
                entry["where"] = meta["where"]
                del entry["only"]
        elif is_web:
            entry["only"] = "web"
        if visual and r not in ("RDXSCREEN",):
            entry["sets"] = ["layout", "a11y"]
        entry["doc"] = meta.get("js_description") or ""

        def origin_of(kind, m):
            if not qn:
                return None
            if m["source"] in ("docs", "rc"):
                return None
            if any(m["name"].upper() in table.get(q, ()) for q, _ in qn):
                return None
            return None if rc.get((r, kind, m["name"]), False) else "rapidr"

        props = []
        for p in c["members"]["properties"]:
            if visual and any(p["name"].lower() in s for s in EXTENSION_SETS.values()):
                continue
            typ = prop_type(p["name"], p.get("type", ""), p.get("default", ""), runs_by_const) if p["source"] == "docs" else {"type": "any"}
            if p["source"] != "docs":
                low = p["name"].lower()
                if low in doc_types:
                    typ = doc_types[low]
                elif low.endswith("color"):
                    typ = {"type": "color"}
                elif low in ENUM_BY_NAME and ENUM_BY_NAME[low] in runs_by_const:
                    typ = {"type": "enum", "values": runs_by_const[ENUM_BY_NAME[low]]}
                else:
                    got = vprops.get(r, {}).get(low)
                    typ = {"type": "int" if got is not None and re.match(r"^-?\d+$", got) else "string" if got == "" else "any"}
            d = {"name": p["name"], **typ}
            dv = norm_default(p.get("default", ""), typ["type"], consts) if p["source"] == "docs" else None
            if dv is not None and not (typ["type"] == "enum" and isinstance(dv, int)):
                d["default"] = dv
            elif dv is not None:
                # (an enum's default is its constant's name)
                d["default"] = p["default"].strip()
            rw = p.get("rw", "RW")
            if rw == "R":
                d["access"] = "read"
            elif rw == "W":
                d["access"] = "write"
            if rw != "RW" or p["name"].lower() in DESIGN_NOT or d.get("indexed"):
                d["design"] = False
            o = origin_of("properties", p)
            if o:
                d["origin"] = o
            if p.get("only"):
                d["only"] = p["only"]
            props.append(d)
        methods = []
        for m in c["members"]["methods"]:
            st = vm.get(f"{r}.{m['name'].lower()}", {}).get("status", "ok")
            web_only = is_web or m.get("only") == "web"
            if st == "missing" and m["source"] not in ("docs", "rc") and not web_only:
                continue  # (a name the dispatch matches that isn't this component's method)
            params, returns = parse_params(m.get("type", "SUB")) if m["source"] == "docs" else ([], "")
            d = {"name": m["name"]}
            js_sig = meta["js_sigs"].get(m["name"].lower(), {})
            if params:
                d["params"] = params_text(params)
            elif m["source"] not in ("docs", "rc"):
                jp = js_params(js_sig.get("sig"))
                if jp:
                    d["params"] = jp
            if js_sig.get("desc"):
                # (RapidR's own words, from the web IDE's data)
                d["doc"] = js_sig["desc"]
            if returns:
                d["returns"] = returns
            o = origin_of("methods", m)
            if o:
                d["origin"] = o
            if m.get("only"):
                d["only"] = m["only"]
            if st == "missing" and not web_only:
                d["missing"] = True
            if st == "blocks":
                d["test"] = "skip: waits for the user"
            low = m["name"].lower()
            if low in any_values or low in type_values.get(r, ()):
                d["value"] = True
            methods.append(d)
        events = []
        for e in c["members"]["events"]:
            params, _ = parse_params(e.get("type", "")) if e["source"] == "docs" else ([], "")
            d = {"name": e["name"]}
            if params:
                d["params"] = params_text(params)
            o = origin_of("events", e)
            if o:
                d["origin"] = o
            events.append(d)
        de = default_event(r, events)
        if de:
            entry["default_event"] = de
        entry["properties"] = props
        entry["methods"] = methods
        entry["events"] = events
        by_group.setdefault(meta["group"], []).append(entry)
    for title, fname in GROUP_FILES:
        write_components(os.path.join(DATA, "components", fname + ".toml"), title, by_group.get(title, []))
    write_constants(runs, libs, rapidr_consts)
    emit_rest(runs_by_const, doc_types)
    print("emitted", sum(len(v) for v in by_group.values()), "components")


def emit_rest(runs_by_const, doc_types):
    """Global objects, BASIC libraries, RapidQ objects not yet built,
    builtins, statements, directives, keywords and the extension sets."""
    # --- globals
    g = {x["name"]: x["docs"] for x in load("globals.json")}
    src = read("crates/rapidr-value/src/globals.rs")
    code = set()
    for o, alt in re.findall(r'\("(screen|application|clipboard|mouse)", ((?:"[a-z]+"\s*\|\s*)*"[a-z]+")\)', src):
        for m in re.findall(r'"([a-z]+)"', alt):
            code.add((o, m))
    casing = load("casing.json")
    methods_in_code = {("application", "terminate"), ("application", "minimize"), ("clipboard", "setastext"), ("clipboard", "getastext"),
                       ("clipboard", "clear"), ("clipboard", "hasformat"), ("clipboard", "format"), ("clipboard", "open"), ("clipboard", "close"),
                       ("screen", "getpixeldepth")}
    rapidq2 = {"clientheight", "clientwidth", "getpixeldepth", "monitors", "mousebuttons", "mousepresent", "mouseswap"}
    out = ["# The global objects: there in every program, never created (RapidQ's Screen, Application,\n",
           "# Clipboard; FileRec, which DIR$ fills; Printer, a QPRINTER; RapidR's Mouse).\n"]
    globals_def = [("SCREEN", "Screen"), ("APPLICATION", "Application"), ("CLIPBOARD", "Clipboard")]
    for key, disp in globals_def:
        d = g[key]
        have = {p["name"].lower() for k in KINDS for p in d[k]}
        props, methods = [], []
        for p in d["properties"]:
            t = prop_type(p["name"], p.get("type", ""), p.get("default", ""), runs_by_const)
            e = {"name": p["name"], **t}
            if p.get("rw") == "R":
                e["access"] = "read"
            if p.get("rw") == "W":
                e["access"] = "write"
            props.append(e)
        for m in d["methods"]:
            params, returns = parse_params(m.get("type", "SUB"))
            e = {"name": m["name"]}
            if params:
                e["params"] = params_text(params)
            if returns:
                e["returns"] = returns
            if (key.lower(), m["name"].lower()) not in code and (key.lower(), m["name"].lower()) not in methods_in_code:
                e["missing"] = True
            methods.append(e)
        for o, m in sorted(code):
            if o != key.lower() or m in have:
                continue
            e = {"name": casing.get(m, m.capitalize())}
            if m in rapidq2:
                e["from"] = "RAPIDQ2.INC"
            elif not (key == "APPLICATION" and m == "path"):
                e["origin"] = "rapidr"
            if (o, m) in methods_in_code:
                methods.append(e)
            else:
                e = {"name": e["name"], **doc_types.get(m, {"type": "int"}), **{k: v for k, v in e.items() if k != "name"}}
                e["access"] = "read" if m not in ("theme",) else None
                props.append({k: v for k, v in e.items() if v is not None})
        out.append(f"\n[[object]]\nname = {toml_str(disp)}\nkind = \"global\"\ndoc = \"\"\n")
        for k, items in (("properties", props), ("methods", methods)):
            out.append(f"{k} = [\n" + "".join(f"  {inline(x)},\n" for x in items) + "]\n")
        out.append("events = []\n")
    out.append('\n[[object]]\nname = "Mouse"\nkind = "global"\norigin = "rapidr"\ndoc = ""\nproperties = [\n'
               '  { name = "X", type = "int", access = "read" },\n  { name = "Y", type = "int", access = "read" },\n]\nmethods = []\nevents = []\n')
    out.append('\n[[object]]\nname = "FileRec"\nkind = "global"\ndoc = ""\nproperties = [\n' + "".join(
        f"  {inline({'name': n, 'type': t, 'access': 'read'})},\n" for n, t in (("FileName", "string"), ("ShortName", "string"), ("Date", "string"),
                                                                               ("Time", "string"), ("Size", "int"), ("FileTime", "int"))) + "]\nmethods = []\nevents = []\n")
    out.append('\n[[object]]\nname = "Printer"\nkind = "global"\ninstance_of = "RPRINTER"\ndoc = ""\nproperties = []\nmethods = []\nevents = []\n')
    open(os.path.join(DATA, "globals.toml"), "w").write("".join(out))

    # --- RapidR's BASIC libraries, RapidQ's objects not built yet
    extra = load("extra.json")
    out = ["# Components that aren't built into the runtimes: RapidQ's community libraries RapidR\n",
           "# supplies in BASIC (rapidr_preprocessor::RAPIDR_LIBRARIES), and RapidQ's objects RapidR\n",
           "# doesn't have yet (rapidr_ast::RAPIDQ_OBJECTS_NOT_YET_IMPLEMENTED).\n",
           "group = \"Libraries\"\n"]
    for x in extra:
        out.append(f"\n[[component]]\nname = {toml_str(x['name'])}\nrapidq = {toml_str(x['name'])}\nkind = {toml_str(x['kind'])}\n")
        if x.get("file"):
            out.append(f"from = {toml_str(x['file'])}\n")
        visual = any(p["name"].lower() == "left" for p in x["docs"]["properties"])
        out.append(f"visual = {toml_val(visual)}\ndoc = \"\"\n")
        for k in KINDS:
            items = []
            for m in x["docs"][k]:
                if k == "properties":
                    t = prop_type(m["name"], m.get("type", ""), m.get("default", ""), runs_by_const)
                    e = {"name": m["name"], **t}
                    if m.get("rw") == "R":
                        e["access"] = "read"
                    if m.get("rw") == "W":
                        e["access"] = "write"
                else:
                    params, returns = parse_params(m.get("type", "SUB"))
                    e = {"name": m["name"]}
                    if params:
                        e["params"] = params_text(params)
                    if returns and k == "methods":
                        e["returns"] = returns
                items.append(e)
            out.append(f"{k} = [\n" + "".join(f"  {inline(x)},\n" for x in items) + "]\n" if items else f"{k} = []\n")
    # (RapidQ's compiler has these objects too; RapidR doesn't yet)
    table = {k: set(v) for k, v in load("rc_table.json").items()}
    known = {q for c in load("components.json") for q, _ in c["qnames"]} | {x["name"] for x in extra}
    for q in sorted(set(table) - known):
        out.append(f"\n[[component]]\nname = {toml_str(q)}\nrapidq = {toml_str(q)}\nkind = \"planned\"\nvisual = {toml_val('LEFT' in table[q])}\ndoc = \"\"\n")
        casing = load("casing.json")
        for k in KINDS:
            items = [{"name": casing.get(m.lower(), m.capitalize())} for m in sorted(table[q]) if rc_kind(m) == k]
            if k == "properties":
                items = [dict(x, type="any") for x in items]
            out.append(f"{k} = [\n" + "".join(f"  {inline(x)},\n" for x in items) + "]\n" if items else f"{k} = []\n")
    open(os.path.join(DATA, "components", "libraries.toml"), "w").write("".join(out))

    # --- builtins
    bs = load("builtins.json")
    groups = builtin_groups()
    internal = sorted(k for k in const_list(read("interpreter/rapidr-bytecode/src/builtins.rs"), "BUILTINS") if k.startswith("__") or k in INTERNAL_BUILTINS)
    out = ["# The builtin functions and procedures. `name` is how programs write it (a `$` for those\n",
           "# that return text); the runtimes' dispatch key drops the suffix (MID$ is `mid`).\n",
           "\n# BUILTINS' names programs never write: the compilers' lowerings of statements (PRINT,\n"
           "# INPUT, DATA, …) and of objects, memory and numeric stores.\ninternal = [\n" + "".join(f"  {toml_str(k)},\n" for k in internal) + "]\n"]
    for b in sorted(bs, key=lambda x: x["key"]):
        e = {"name": b["name"]}
        syn = b.get("syntax") or ""
        if syn:
            e["syntax"] = syn
        e["group"] = groups.get(b["key"], "Other")
        if b["bare"]:
            e["bare"] = True
        if not b["rapidq"]:
            e["origin"] = "rapidr"
        if b["status"] == "missing":
            e["missing"] = True
        if b["key"] in ("print_hash", "write_hash"):
            e["only"] = "desktop"
        e["doc"] = ""
        out.append("\n[[builtin]]\n" + "".join(f"{k} = {toml_val(v)}\n" for k, v in e.items()))
    open(os.path.join(DATA, "builtins.toml"), "w").write("".join(out))
    write_statements()


def builtin_groups():
    g = {}
    def put(group, names):
        for n in names.split():
            g[n] = group
    put("Strings", "asc bin chr convbase delete field format hex insert instr lcase left len ltrim mid oct replace replacesubstr reverse right rinstr rtrim space str strf string tally trim ucase val hextodec isnumeric convbasex wstring wstringtoascii")
    put("Numbers", "abs acos asin atan atn ceil cint clng cos exp fix floor frac int log rnd round sgn sin sqr tan randomize pi cdbl csng cbool inv shl shr iif vartype")
    put("Files and folders", "chdir chdrive close curdir dir direxists eof extractresource fileexists filelen freefile kill lof mkdir open rename rmdir seek resource resourcecount")
    put("Console", "cls color csrlin inkey input locate pos print tab setconsoletitle pcopy isconsole")
    put("Dialogs and sound", "beep messagebox messagedlg msgbox playsound playwav showmessage sound")
    put("System", "command commandcount date doevents environ run shell shellwait sleep time timer mousex mousey microtimer setcapture releasecapture getcapture getfocus setfocus sendmessage postmessage killmessage nviewlibpresent libraryinst unloadlibrary execute inp inpw out outw lflush lprint")
    put("Arrays and memory", "lbound ubound initarray memcmp memcpy memset peek poke rtlmovememory sizeof varptr udtptr")
    put("Routines", "callback callfunc codeptr paramstr paramstrcount paramval paramvalcount")
    put("Colours", "rgb")
    return g


STATEMENTS = [
    # (name, syntax, group, rapidq?)
    ("DIM", "DIM name[(bounds)] [AS type] [= value] [, …]", "Declarations", True),
    ("REDIM", "REDIM [PRESERVE] name(bounds) [AS type]", "Declarations", True),
    ("STATIC", "STATIC name[(bounds)] AS type", "Declarations", True),
    ("CONST", "CONST name = expression [, …]", "Declarations", True),
    ("DEFINT", "DEFINT name [= value] [, …]", "Declarations", True),
    ("DEFSTR", "DEFSTR name [= value] [, …]", "Declarations", True),
    ("DEFBYTE", "DEFBYTE name [= value] [, …]", "Declarations", True),
    ("DEFWORD", "DEFWORD name [= value] [, …]", "Declarations", True),
    ("DEFDWORD", "DEFDWORD name [= value] [, …]", "Declarations", True),
    ("DEFLNG", "DEFLNG name [= value] [, …] (also DEFLONG)", "Declarations", True),
    ("DEFSHORT", "DEFSHORT name [= value] [, …]", "Declarations", True),
    ("DEFSNG", "DEFSNG name [= value] [, …]", "Declarations", True),
    ("DEFDBL", "DEFDBL name [= value] [, …]", "Declarations", True),
    ("DEFCUR", "DEFCUR name [= value] [, …]", "Declarations", False),
    ("GLOBAL", "GLOBAL name AS type (also PUBLIC / PRIVATE before a declaration)", "Declarations", False),
    ("TYPE", "TYPE name [EXTENDS type] … END TYPE (also STRUCT … END STRUCT)", "Types and objects", True),
    ("EXTENDS", "TYPE name EXTENDS QOBJECT | component", "Types and objects", True),
    ("PROPERTY SET", "PROPERTY SET name(value AS type)", "Types and objects", True),
    ("EVENT", "EVENT(member) … END EVENT", "Types and objects", True),
    ("CONSTRUCTOR", "CONSTRUCTOR … END CONSTRUCTOR", "Types and objects", True),
    ("PUBLIC", "PUBLIC: / PRIVATE: in a TYPE", "Types and objects", True),
    ("CREATE", "CREATE name AS type … END CREATE", "Types and objects", True),
    ("WITH", "WITH object … END WITH", "Types and objects", True),
    ("SUB", "SUB name[(params)] … END SUB", "Routines", True),
    ("FUNCTION", "FUNCTION name[(params)] AS type … END FUNCTION", "Routines", True),
    ("SUBI", "SUBI name(…) … END SUBI", "Routines", True),
    ("FUNCTIONI", "FUNCTIONI name(…) AS type … END FUNCTIONI", "Routines", True),
    ("DECLARE", "DECLARE SUB|FUNCTION name [LIB \"file\" [ALIAS \"name\"]] [(params)] [AS type]", "Routines", True),
    ("CALL", "CALL name[(args)]", "Routines", True),
    ("BIND", "BIND pointer TO routine", "Routines", True),
    ("EXIT", "EXIT FOR | DO | WHILE | SUB | FUNCTION", "Flow", True),
    ("IF", "IF condition THEN … [ELSEIF condition THEN …] [ELSE …] END IF", "Flow", True),
    ("SELECT CASE", "SELECT CASE expression … CASE values … CASE ELSE … END SELECT", "Flow", True),
    ("FOR", "FOR variable = start TO end [STEP step] … NEXT [variable]", "Flow", True),
    ("WHILE", "WHILE condition … WEND", "Flow", True),
    ("DO", "DO [WHILE|UNTIL condition] … LOOP [WHILE|UNTIL condition]", "Flow", True),
    ("GOTO", "GOTO label", "Flow", True),
    ("GOSUB", "GOSUB label … RETURN", "Flow", True),
    ("RETURN", "RETURN", "Flow", True),
    ("END", "END", "Flow", True),
    ("ON ERROR", "ON ERROR RESUME NEXT | GOTO label", "Flow", False),
    ("PRINT", "PRINT [expression][; | ,] …", "Console", True),
    ("LPRINT", "LPRINT [expression][; | ,] …", "Console", True),
    ("INPUT", "INPUT [\"prompt\";] variable", "Console", True),
    ("LINE INPUT", "LINE INPUT #n, variable", "Files", False),
    ("OPEN", "OPEN file FOR INPUT|OUTPUT|APPEND|BINARY|RANDOM AS #n", "Files", False),
    ("CLOSE", "CLOSE [#n]", "Files", False),
    ("PRINT #", "PRINT #n, expression [; …]", "Files", False),
    ("WRITE #", "WRITE #n, expression [, …]", "Files", False),
    ("INPUT #", "INPUT #n, variable [, …]", "Files", False),
    ("SEEK", "SEEK #n, position", "Files", False),
    ("DATA", "DATA value [, …]", "Data", True),
    ("READ", "READ variable [, …]", "Data", True),
    ("RESTORE", "RESTORE [label]", "Data", True),
    ("SWAP", "SWAP a, b", "Data", True),
    ("INC", "INC variable [, amount]", "Data", True),
    ("DEC", "DEC variable [, amount]", "Data", True),
    ("QUICKSORT", "QUICKSORT(array(first), array(last), ascending [, …])", "Data", True),
    ("REM", "REM comment (also ' comment)", "Other", True),
    ("IMPORT", "IMPORT \"file\"", "Other", False),
    ("RUSTSTART", "RUSTSTART … RUSTEND (Rust code, native builds)", "Other", False),
]

DIRECTIVES = [
    ("$INCLUDE", "$INCLUDE \"file\"", True), ("$RESOURCE", "$RESOURCE name AS \"file\"", True), ("$DEFINE", "$DEFINE name [text]", True),
    ("$UNDEF", "$UNDEF name", True), ("$IFDEF", "$IFDEF name", True), ("$IFNDEF", "$IFNDEF name", True), ("$ELSE", "$ELSE", True),
    ("$ENDIF", "$ENDIF", True), ("$MACRO", "$MACRO name(params) = text", True), ("$APPTYPE", "$APPTYPE GUI | CONSOLE | CGI", True),
    ("$TYPECHECK", "$TYPECHECK ON | OFF", True), ("$OPTIMIZE", "$OPTIMIZE ON | OFF", True), ("$ESCAPECHARS", "$ESCAPECHARS ON | OFF", True),
    ("$OPTION", "$OPTION ICON \"file\" | DECIMAL | BYREF | INKEY$ TRAPALL | EXPLICIT …", True), ("$THEME", "$THEME name", False),
]


def write_statements():
    out = ["# The statements the parser accepts (keywords that start a statement).\n"]
    for name, syntax, group, rq in STATEMENTS:
        e = {"name": name, "syntax": syntax, "group": group}
        if not rq:
            e["origin"] = "rapidr"
        e["doc"] = ""
        out.append("\n[[statement]]\n" + "".join(f"{k} = {toml_val(v)}\n" for k, v in e.items()))
    for name, syntax, rq in DIRECTIVES:
        e = {"name": name, "syntax": syntax}
        if not rq:
            e["origin"] = "rapidr"
        e["doc"] = ""
        out.append("\n[[directive]]\n" + "".join(f"{k} = {toml_val(v)}\n" for k, v in e.items()))
    kws = [("AND", "operator"), ("OR", "operator"), ("XOR", "operator"), ("NOT", "operator"), ("MOD", "operator"), ("SHL", "operator"),
           ("SHR", "operator"), ("INV", "operator"), ("IS", "operator"), ("AS", "keyword"), ("TO", "keyword"), ("STEP", "keyword"), ("THEN", "keyword"),
           ("ELSE", "keyword"), ("ELSEIF", "keyword"), ("NEXT", "keyword"), ("WEND", "keyword"), ("LOOP", "keyword"), ("UNTIL", "keyword"),
           ("CASE", "keyword"), ("LIB", "keyword"), ("ALIAS", "keyword"), ("BYVAL", "keyword"), ("BYREF", "keyword"), ("PRESERVE", "keyword"),
           ("SET", "keyword"), ("STRUCT", "keyword"), ("PRIVATE", "keyword"), ("THIS", "keyword"), ("SUPER", "keyword"), ("RUSTEND", "keyword")]
    types = [("BYTE", True), ("SHORT", True), ("WORD", True), ("INTEGER", True), ("DWORD", True), ("LONG", True), ("SINGLE", True), ("DOUBLE", True),
             ("STRING", True), ("VARIANT", True), ("QOBJECT", True), ("INT64", False), ("CURRENCY", False), ("ROBJECT", False)]
    for n, kind in kws:
        out.append("\n[[keyword]]\n" + f"name = {toml_str(n)}\nkind = {toml_str(kind)}\n" + ("origin = \"rapidr\"\n" if n == "RUSTEND" else "") + "doc = \"\"\n")
    for n, rq in types:
        out.append("\n[[type]]\n" + f"name = {toml_str(n)}\n" + ("" if rq else "origin = \"rapidr\"\n") + "doc = \"\"\n")
    open(os.path.join(DATA, "language.toml"), "w").write("".join(out))
    open(os.path.join(DATA, "sets.toml"), "w").write(
        "# Members RapidR adds to every visual component (`sets = [\"layout\", \"a11y\"]` in a component).\n\n"
        "[[set]]\nname = \"layout\"\norigin = \"rapidr\"\nproperties = [\n"
        "  { name = \"Anchors\", type = \"set\", values = [\"akLeft\", \"akTop\", \"akRight\", \"akBottom\"], default = \"akLeft + akTop\" },\n"
        "  { name = \"MinWidth\", type = \"int\", default = 0 },\n  { name = \"MinHeight\", type = \"int\", default = 0 },\n"
        "  { name = \"MaxWidth\", type = \"int\", default = 0 },\n  { name = \"MaxHeight\", type = \"int\", default = 0 },\n]\nmethods = []\nevents = []\n\n"
        "[[set]]\nname = \"a11y\"\norigin = \"rapidr\"\nproperties = [\n"
        "  { name = \"AccessibleName\", type = \"string\", default = \"\" },\n  { name = \"AccessibleDescription\", type = \"string\", default = \"\" },\n]\nmethods = []\nevents = []\n")


def write_components(path, title, entries):
    out = [f"# {title}: the registry's components (crates/rapidr-lang; src/lib.rs says how to edit).\n",
           f"group = {toml_str(title)}\n"]
    for e in entries:
        out.append("\n[[component]]\n")
        for k in ("name", "rapidq", "from", "aliases", "visual", "container", "size", "only", "where", "default_event", "sets", "doc"):
            if k in e and e[k] is not None and e[k] != "":
                out.append(f"{k} = {toml_val(e[k])}\n")
        if "doc" not in e or not e["doc"]:
            out.append('doc = ""\n')
        for k in ("properties", "methods", "events"):
            if e[k]:
                out.append(f"{k} = [\n")
                for m in e[k]:
                    out.append(f"  {inline(m)},\n")
                out.append("]\n")
            else:
                out.append(f"{k} = []\n")
    open(path, "w").write("".join(out))


CONST_GROUPS = {
    "False": "Truth values", "SND_": "PLAYWAV options", "cl": "Colours", "mr": "Modal results", "MB_": "Message box buttons and icons",
    "ID": "Message box answers", "bs": "Border styles", "ws": "Window states", "al": "Alignment in the parent", "mb": "Mouse buttons and message dialog buttons",
    "mt": "Message dialog types", "fm": "File stream modes", "so": "Seek origins", "Num_": "Number types (ReadNum / WriteNum)",
    "fs": "Font styles and form styles", "pf": "Pixel formats", "VK_": "Virtual key codes", "ta": "Text alignment", "ss": "Shift states and scroll bars",
    "fp": "Font pitch", "biSystemMenu": "Border icons", "ca": "Close actions", "tl": "Text layout", "ls": "Label styles", "bv": "Bevels", "bp": "Bevel panels",
    "ec": "Edit character case", "cs": "Combo box styles", "bl": "Button glyph layout", "bk": "Button kinds", "cr": "Cursors", "ft": "File types",
    "sb": "Scroll bar kinds and stop bits", "sc": "Scroll codes", "ds": "Draw states", "it": "Image types", "st": "Status bar panel styles",
    "vs": "List view styles", "tb": "Track bar orientation", "tm": "Tick marks", "ts": "Tick styles", "go": "Grid options", "gcs": "Grid column styles",
    "os": "Outline styles and OLE states", "oo": "Outline options", "gk": "Gauge kinds", "cm": "Copy modes", "lb": "List box styles",
    "br": "Baud rates", "pr": "Parity", "fd": "Font dialog options", "dt": "Directory types", "drt": "Drive types", "IPPROTO_": "IP protocols",
    "SOCK_": "Socket types", "AF_": "Address families", "hs": "Header section styles", "dup": "Duplicates", "sm": "Stretch modes", "ff": "Float formats",
    "fa": "File attributes", "po": "Printer orientation", "CtrlDown": "Key states", "CHARSET": "Character sets",
}


def write_constants(runs, libs, rapidr_consts):
    out = ["# The constants: RapidQ's RAPIDQ.INC (what `$INCLUDE \"RAPIDQ.INC\"` gives, built in), its\n",
           "# library includes' and RapidR's own. Values as RapidQ's (colours are BGR).\n"]
    for p, cs in runs:
        title = CONST_GROUPS.get(p) or CONST_GROUPS.get(cs[0][0]) or ("Character sets" if p.endswith("_CHARSET") or "CHARSET" in cs[0][0] else p)
        out.append(f"\n[[group]]\nname = {toml_str(title)}\nsource = \"RAPIDQ.INC\"\ndoc = \"\"\nconstants = [\n")
        for n, v in cs:
            out.append(f"  [{toml_str(n)}, {v}],\n")
        out.append("]\n")
    for file, cs in libs:
        if not cs:
            continue
        out.append(f"\n[[group]]\nname = {toml_str(file + ' constants')}\nsource = {toml_str(file)}\ndoc = \"\"\nconstants = [\n")
        for n, v in cs:
            out.append(f"  [{toml_str(n)}, {v}],\n")
        out.append("]\n")
    out.append("\n[[group]]\nname = \"Anchors\"\nsource = \"RapidR\"\norigin = \"rapidr\"\ndoc = \"\"\nconstants = [\n")
    for n, v in rapidr_consts:
        out.append(f"  [{toml_str(n)}, {v}],\n")
    out.append("]\n")
    open(os.path.join(DATA, "constants.toml"), "w").write("".join(out))


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "emit":
        emit("--force" in sys.argv)
    elif cmd == "collect":
        collect()
    elif cmd == "builtins":
        builtins_collect()
    elif cmd == "rc-builtin-probes":
        rc_builtin_probes(sys.argv[2])
    elif cmd == "rc-builtin-read":
        rc_builtin_read(sys.argv[2])
    elif cmd == "plan":
        plan()
    elif cmd == "vm-probe":
        vm_probe()
    elif cmd == "vm-props":
        vm_props()
    elif cmd == "rc-probes":
        rc_probes(sys.argv[2])
    elif cmd == "rc-read":
        rc_read(sys.argv[2])
    else:
        sys.exit(__doc__)
