"""Kinds a (Q&A from the registry) and d (look first) of build_corpus.py."""

from __future__ import annotations

import json
import os
import random
import re
import subprocess
from pathlib import Path

import programs as prog
from build_corpus import Builder, first_sentence
from rai_corpus import BIN, ROOT, Step, log, sha

LANGS = prog.LANGS
SNIP = ROOT / "tmp" / "snip"


# ---- running a snippet (to know what it prints) --------------------------------------------------------
def run_snippet(code: str) -> str | None:
    SNIP.mkdir(parents=True, exist_ok=True)
    f = SNIP / f"s{sha(code)}.rr"
    f.write_text(code)
    env = dict(os.environ, RAPIDR_REGISTRY=str(ROOT / "tmp" / "x.reg"), RAPIDR_PRINT_TO=str(ROOT / "tmp" / "prints"), RAPIDR_AI_KEYSTORE="memory")
    try:
        r = subprocess.run([str(BIN / "rapidr"), "run", str(f)], capture_output=True, text=True, timeout=15, env=env, cwd=str(SNIP), stdin=subprocess.DEVNULL)
    except subprocess.TimeoutExpired:
        return None
    if r.returncode != 0:
        return None
    return r.stdout.strip("\n")


def registry() -> dict:
    out = subprocess.run([str(BIN / "rapidr"), "lang", "export", "--json"], capture_output=True, text=True, check=True).stdout
    return json.loads(out)


def strict_ok(b: Builder, code: str, main: str = "main.rr") -> bool:
    ok, said = b.checker.compiles([(main, code)], main)
    return ok and said.strip() == "No problems: it compiles."


def clean_doc(text: str) -> str:
    """The first sentence of a registry doc, or nothing when it speaks of the old name."""
    s = first_sentence(text or "")
    return "" if re.search(r"RapidQ|\bQ[A-Z]{3,}\b", s) else s


# ---- a: Q&A from the registry --------------------------------------------------------------------------------
Q_BUILTIN = {
    "en": ["How do I use {n}?", "What does {n} do?", "Show me an example of {n}.", "Can you explain {n}?"],
    "es": ["¿Cómo se usa {n}?", "¿Qué hace {n}?", "Muéstrame un ejemplo de {n}.", "¿Puedes explicarme {n}?"],
    "pt": ["Como se usa {n}?", "O que faz {n}?", "Mostre um exemplo de {n}.", "Você pode explicar {n}?"],
}
A_BUILTIN = {
    "en": "`{syntax}`{returns}. {doc}\n\n```basic\n{code}\n```\n{out}",
    "es": "`{syntax}`{returns}. {doc}\n\n```basic\n{code}\n```\n{out}",
    "pt": "`{syntax}`{returns}. {doc}\n\n```basic\n{code}\n```\n{out}",
}
RETURNS = {"en": " returns {r}", "es": " devuelve {r}", "pt": " retorna {r}"}
OUT = {"en": "It prints `{o}`.", "es": "Imprime `{o}`.", "pt": "Imprime `{o}`."}
DOC_PREFIX = {"en": "", "es": "", "pt": ""}

Q_PROP = {
    "en": ["How do I change the {p} of {a} {t}?", "What does {p} do on {a} {t}?", "How can I set {p} on {a} {t}?"],
    "es": ["¿Cómo cambio la propiedad {p} de un {t}?", "¿Para qué sirve {p} en un {t}?", "¿Cómo puedo fijar {p} en un {t}?"],
    "pt": ["Como altero a propriedade {p} de um {t}?", "Para que serve {p} em um {t}?", "Como posso definir {p} em um {t}?"],
}
Q_EVENT = {
    "en": ["How do I run code when {e} happens on {a} {t}?", "How do I handle {e} of {a} {t}?"],
    "es": ["¿Cómo ejecuto código cuando ocurre {e} en un {t}?", "¿Cómo manejo {e} de un {t}?"],
    "pt": ["Como executo código quando {e} acontece em um {t}?", "Como trato {e} de um {t}?"],
}
Q_METHOD = {
    "en": ["How do I call {m} on {a} {t}?", "What does {m} do on {a} {t}?"],
    "es": ["¿Cómo llamo a {m} en un {t}?", "¿Qué hace {m} en un {t}?"],
    "pt": ["Como chamo {m} em um {t}?", "O que faz {m} em um {t}?"],
}
Q_OVERVIEW = {
    "en": ["How do I use {a} {t}?", "What is {a} {t} for?"],
    "es": ["¿Cómo uso un {t}?", "¿Para qué sirve un {t}?"],
    "pt": ["Como uso um {t}?", "Para que serve um {t}?"],
}
TYPEWORD = {
    "en": {"bool": "yes/no (1 or 0)", "int": "whole-number", "string": "text", "enum": "choice", "color": "colour", "float": "decimal"},
    "es": {"bool": "de sí/no (1 o 0)", "int": "numérica entera", "string": "de texto", "enum": "de elección", "color": "de color", "float": "decimal"},
    "pt": {"bool": "de sim/não (1 ou 0)", "int": "numérica inteira", "string": "de texto", "enum": "de escolha", "color": "de cor", "float": "decimal"},
}

SKIP_GROUPS = {"IDE", "Libraries", "Web only", "Direct3D (retained mode)", "DirectX 2D", "AI"}
SKIP_PROPS = {"Handle", "Parent", "PopupMenu", "Font", "Tag", "BMPHandle", "Cursor", "Hint", "ShowHint"}


def article(t: str) -> str:
    return "an" if t[:2] in ("RE", "RI", "RO", "RU", "RA") and t[1] in "EIOUA" else "a"


def prop_value(p: dict) -> str | None:
    t = p.get("type")
    d = p.get("default")
    if t == "bool":
        return "0" if d is True else "1"
    if t == "int":
        rng = (p.get("range") or "").replace(" ", "")
        m = re.match(r"^(-?\d+)\.\.(-?\d+)?$", rng)
        if m:
            lo = int(m.group(1))
            hi = int(m.group(2)) if m.group(2) else lo + 100
            return str(lo + (hi - lo) // 3 if hi - lo >= 3 else lo)
        return str((d if isinstance(d, int) else 0) + 10)
    if t == "string":
        return '"Text"'
    if t == "enum":
        vals = [v for v in p.get("values", []) if v != d]
        return vals[0] if vals else None
    if t == "color":
        return "clBlue"
    return None


def demo_form(t: str, body: str, size: list | None, handler: str = "", declare: str = "", extra: str = "") -> str:
    w, h = (size or [100, 24])
    w, h = min(max(w, 60), 240), min(max(h, 20), 120)
    code = declare
    code += f'CREATE Form AS RForm\n    Caption = "{t} demo"\n    ClientWidth = 320: ClientHeight = 200\n    Center\n    CREATE Demo AS {t}\n        Left = 16: Top = 16: Width = {w}: Height = {h}\n{body}    END CREATE\n{extra}END CREATE\n\n'
    code += handler
    code += "Form.ShowModal\n"
    return code


def component_demos(b: Builder, reg: dict) -> list[dict]:
    """Candidate (component, member) demos; each program compiled clean by RapidR."""
    out = []
    rng = b.rng("a-components")
    spell = {c["name"]: c["spelling"] for c in reg["components"]}
    comps = [c for c in reg["components"] if c["visual"] and c["group"] not in SKIP_GROUPS and c["origin"] in ("rapidq", "rapidr")]
    rng.shuffle(comps)
    for c in comps:
        t = c["spelling"]
        if t.startswith("Q") or t in ("RForm", "RFormMDI", "RDesignSurface", "RCodeEditor", "RDiffView", "RGlassFrame", "RDockManager"):
            continue
        # properties
        props = [p for p in c["properties"] if p["name"] not in SKIP_PROPS and p.get("access") != "read" and p.get("design", True) and p["type"] in ("bool", "int", "string", "enum", "color") and p["name"] not in ("Left", "Top", "Width", "Height")]
        rng.shuffle(props)
        for p in props[:6]:
            v = prop_value(p)
            if v is None:
                continue
            code = demo_form(t, f"        {p['name']} = {v}\n", c.get("size"))
            if strict_ok(b, code):
                out.append({"kind": "property", "c": c, "p": p, "value": v, "code": code})
        # events
        evs = [e for e in c["events"] if e["name"] in ("OnClick", "OnChange", "OnDblClick", "OnEnter", "OnExit", "OnKeyPress", "OnMouseDown", "OnTimer", "OnClose", "OnResize")]
        for e in evs[:3]:
            params = ", ".join(f"{q['name']} AS {spell.get(q['type'], q['type'])}" for q in e.get("params", []))
            sig = f" ({params})" if params else ""
            extra = '    CREATE Info AS RLabel\n        Left = 16: Top = 160: Width = 280\n    END CREATE\n'
            handler = f'SUB Handler{sig}\n    Info.Caption = "{e["name"]}"\nEND SUB\n\n'
            code = demo_form(t, f"        {e['name']} = Handler\n", c.get("size"), handler=handler, declare=f"DECLARE SUB Handler{sig}\n\n", extra=extra)
            if strict_ok(b, code):
                out.append({"kind": "event", "c": c, "e": e, "code": code, "sig": sig})
        # methods without parameters
        ms = [m for m in c["methods"] if not m.get("params") and not m.get("value") and m["name"] in ("SetFocus", "Clear", "Repaint", "Refresh", "SelectAll", "Hide", "Show", "Center", "Update", "Close", "Undo", "Cut", "Copy", "Paste", "SelectAll", "ClearSelection")]
        for m in ms[:2]:
            code = demo_form(t, "", c.get("size"), handler="").replace("Form.ShowModal", f"Demo.{m['name']}\nForm.ShowModal")
            if strict_ok(b, code):
                out.append({"kind": "method", "c": c, "m": m, "code": code})
    return out


def sample_args(syntax: str) -> list[str] | None:
    """Arguments for a call of a builtin, from its syntax line ("MID$(Text$, Position, Num)")."""
    m = re.match(r"^([A-Z][A-Z0-9$]*)\((.*)\)$", syntax.strip())
    if not m:
        return None
    inner = re.sub(r"\[[^\]]*\]", "", m.group(2))
    parts = [x.strip() for x in inner.split(",") if x.strip()]
    if not parts or any("..." in x or "|" in x or " " in x for x in parts):
        return None
    out = []
    for x in parts:
        base = x.rstrip("$%").lower()
        if x.endswith("$") or base in ("text", "string", "source", "search", "find", "str", "s", "char", "bytes"):
            out.append('"R"' if base in ("find", "char", "sub") else '"Hello, RapidR"')
        elif base in ("position", "index", "start"):
            out.append("2")
        elif "base" in base:
            out.append("16" if base.startswith("to") else "10")
        elif base in ("num", "count", "length", "n", "integer", "byte", "number", "numchars"):
            out.append("3" if base != "byte" else "65")
        else:
            out.append("2.5" if base in ("x", "y", "angle", "value", "expression") else "3")
    return out


def builtin_demos(b: Builder, reg: dict) -> list[dict]:
    out = []
    for bi in reg["builtins"]:
        n = bi["name"]
        if bi.get("group") not in ("Numbers", "Strings") or n.startswith("__"):
            continue
        code = bi.get("example")
        if not code:
            args = sample_args(bi["syntax"])
            if args is None:
                continue
            code = f"PRINT {n}({', '.join(args)})\n"
        code = code.rstrip("\n") + "\n"
        if not strict_ok(b, code):
            continue
        o1, o2 = run_snippet(code), run_snippet(code + "' again\n")
        if o1 is None or o1 != o2 or not o1.strip() or len(o1) > 200 or re.search(r"\bnan\b|\binf\b", o1, re.I):
            continue
        out.append({"b": bi, "code": code, "out": o1})
    return out


def facts_sentence(lang: str, p: dict) -> str:
    L = LANGS.index(lang)
    t = p["type"]
    s = TYPEWORD[lang].get(t, t)
    parts = []
    if "default" in p and p["default"] is not None:
        d = p["default"]
        d = ("1" if d else "0") if isinstance(d, bool) else d
        parts.append({"en": f"default {d}", "es": f"por defecto {d}", "pt": f"padrão {d}"}[lang])
    if p.get("range") and p["range"] != "any":
        parts.append({"en": f"range {p['range']}", "es": f"rango {p['range']}", "pt": f"faixa {p['range']}"}[lang])
    if p.get("unit"):
        parts.append({"en": f"in {p['unit']}", "es": f"en {p['unit']}", "pt": f"em {p['unit']}"}[lang])
    if t == "enum" and p.get("values"):
        parts.append({"en": "values", "es": "valores", "pt": "valores"}[lang] + ": " + ", ".join(p["values"][:6]))
    return (", ".join(parts)) if parts else ""


def gen_a(b: Builder, quota: int) -> None:
    reg = registry()
    made = 0
    # --- builtins
    demos = builtin_demos(b, reg)
    log(f"a: {len(demos)} builtin demos that compile and run")
    rng = b.rng("a-builtins")
    quota_b = int(quota * 0.4)
    rounds = 0
    while made < quota_b and demos and rounds < 6:
        for d in demos:
            if made >= quota_b:
                break
            r = b.rng("a-b", d["b"]["name"], rounds)
            lang = r.choice(LANGS)
            L = LANGS.index(lang)
            profile = ("lite", "full", "mcp")[r.choices([0, 1, 2], weights=[50, 10, 40])[0]]
            n = d["b"]["name"]
            user = r.choice(Q_BUILTIN[lang]).format(n=n)
            doc = clean_doc(d["b"].get("doc", "")) if lang == "en" else ""
            returns = ""
            answer = A_BUILTIN[lang].format(syntax=d["b"]["syntax"], returns=returns, doc=doc, code=d["code"].rstrip(), out=OUT[lang].format(o=d["out"].replace("\n", " / "))).replace(" . ", ". ").replace("  ", " ")
            steps, project = [], None
            if profile == "mcp":
                project = b.placeholder()
                steps.append(Step(calls=[("studio_status", {})]))
            steps.append(Step(calls=[("rapidr_describe", {"builtin": n})]))
            if r.random() < 0.6:
                steps.append(Step(calls=[("check_code", {"path": "main.rr", "text": d["code"]})]))
            steps.append(Step(text=answer))
            key = f"a-b/{n}/{lang}/{profile}/{sha(user)}"
            prov = {"sources": ["registry:builtins"], "generator": "build_corpus_more.gen_a+builtin", "builtin": n, "snippet_output_verified": True}
            if b.add(key, "a", profile, lang, project, user, steps, prov, f"builtin:{n}"):
                made += 1
        rounds += 1
    log(f"a: {made} builtin conversations")
    # --- components
    cdemos = component_demos(b, reg)
    log(f"a: {len(cdemos)} component demos that compile clean")
    r0 = b.rng("a-comp-order")
    r0.shuffle(cdemos)
    rounds = 0
    start = made
    while made < quota and cdemos and rounds < 4:
        for d in cdemos:
            if made >= quota:
                break
            c = d["c"]
            t = c["spelling"]
            r = b.rng("a-c", t, d["kind"], str(d.get("p", d.get("e", d.get("m", {}))).get("name", "")), rounds)
            lang = r.choice(LANGS)
            L = LANGS.index(lang)
            profile = ("lite", "full", "mcp")[r.choices([0, 1, 2], weights=[50, 10, 40])[0]]
            a = article(t)
            code = d["code"]
            if d["kind"] == "property":
                p = d["p"]
                user = r.choice(Q_PROP[lang]).format(p=p["name"], t=t, a=a)
                facts = facts_sentence(lang, p)
                doc = clean_doc(p.get("doc", "")) if lang == "en" else ""
                head = {
                    "en": f"`{p['name']}` is a {TYPEWORD['en'].get(p['type'], p['type'])} property of {t}{' (' + facts + ')' if facts else ''}. {doc}",
                    "es": f"`{p['name']}` es una propiedad {TYPEWORD['es'].get(p['type'], p['type'])} de {t}{' (' + facts + ')' if facts else ''}. Se fija en su bloque CREATE o en el programa:",
                    "pt": f"`{p['name']}` é uma propriedade {TYPEWORD['pt'].get(p['type'], p['type'])} de {t}{' (' + facts + ')' if facts else ''}. Ela se define no bloco CREATE ou no programa:",
                }[lang]
                member = p["name"]
            elif d["kind"] == "event":
                e = d["e"]
                user = r.choice(Q_EVENT[lang]).format(e=e["name"], t=t, a=a)
                doc = clean_doc(e.get("doc", "")) if lang == "en" else ""
                head = {
                    "en": f"`{e['name']}`{' fires: ' + doc[0].lower() + doc[1:] if doc else ''} Write a SUB with the event's parameters and bind it in the CREATE block with `{e['name']} = Handler`:".replace("fires: Fires", "fires:"),
                    "es": f"Escribe un SUB con los parámetros del evento `{e['name']}` y enlázalo en el bloque CREATE con `{e['name']} = Handler`:",
                    "pt": f"Escreva um SUB com os parâmetros do evento `{e['name']}` e ligue-o no bloco CREATE com `{e['name']} = Handler`:",
                }[lang]
                member = e["name"]
            else:
                m = d["m"]
                user = r.choice(Q_METHOD[lang]).format(m=m["name"], t=t, a=a)
                doc = clean_doc(m.get("doc", "")) if lang == "en" else ""
                head = {
                    "en": f"Call it on the component's name, `Demo.{m['name']}`. {doc}",
                    "es": f"Se llama con el nombre del componente: `Demo.{m['name']}`.",
                    "pt": f"Chama-se com o nome do componente: `Demo.{m['name']}`.",
                }[lang]
                member = m["name"]
            answer = f"{head.strip()}\n\n```basic\n{code.rstrip()}\n```"
            steps, project = [], None
            if profile == "mcp":
                project = b.placeholder()
                steps.append(Step(calls=[("studio_status", {})]))
            steps.append(Step(calls=[("rapidr_describe", {"type": t})]))
            steps.append(Step(calls=[("check_code", {"path": "main.rr", "text": code})]))
            steps.append(Step(text=answer))
            key = f"a-c/{t}/{d['kind']}/{member}/{lang}/{profile}/{sha(user)}"
            prov = {"sources": ["registry:components"], "generator": "build_corpus_more.gen_a+component", "component": t, "member": member, "member_kind": d["kind"]}
            if b.add(key, "a", profile, lang, project, user, steps, prov, f"component:{t}"):
                made += 1
        rounds += 1
    log(f"a: {made} conversations in all ({made - start} about components)")


# ---- d: look first ----------------------------------------------------------------------------------------------------
# How-to questions: (id, question per language, what to search for, a program that answers it, what it does per language)
HOWTO = [
    ("read-file", ("How do I read a text file line by line?", "¿Cómo leo un archivo de texto línea por línea?", "Como leio um arquivo de texto linha por linha?"), "read a text file line by line",
     'DIM f AS INTEGER, line AS STRING\nf = FREEFILE\nOPEN "notes.txt" FOR OUTPUT AS #f\nPRINT #f, "first"\nPRINT #f, "second"\nCLOSE #f\nf = FREEFILE\nOPEN "notes.txt" FOR INPUT AS #f\nDO WHILE NOT EOF(f)\n    LINE INPUT #f, line\n    PRINT line\nLOOP\nCLOSE #f\n',
     ("`OPEN … FOR INPUT` opens the file and `LINE INPUT #f, line` reads one line until `EOF(f)`. The example writes a small file first so it runs as it is.", "`OPEN … FOR INPUT` abre el archivo y `LINE INPUT #f, line` lee una línea hasta `EOF(f)`. El ejemplo escribe antes un archivo pequeño para que corra tal cual.", "`OPEN … FOR INPUT` abre o arquivo e `LINE INPUT #f, line` lê uma linha até `EOF(f)`. O exemplo grava antes um arquivo pequeno para rodar como está.")),
    ("write-file", ("How do I write text to a file?", "¿Cómo escribo texto en un archivo?", "Como escrevo texto em um arquivo?"), "write text to a file",
     'DIM f AS INTEGER\nf = FREEFILE\nOPEN "hello.txt" FOR OUTPUT AS #f\nPRINT #f, "Hello, file"\nCLOSE #f\nPRINT "written"\n',
     ("`OPEN … FOR OUTPUT` creates the file, `PRINT #f, text` writes a line and `CLOSE #f` finishes.", "`OPEN … FOR OUTPUT` crea el archivo, `PRINT #f, texto` escribe una línea y `CLOSE #f` termina.", "`OPEN … FOR OUTPUT` cria o arquivo, `PRINT #f, texto` escreve uma linha e `CLOSE #f` termina.")),
    ("file-exists", ("How do I check whether a file exists?", "¿Cómo compruebo si un archivo existe?", "Como verifico se um arquivo existe?"), "check if a file exists",
     'IF FILEEXISTS("no-such-file.txt") = 0 THEN\n    PRINT "missing"\nELSE\n    PRINT "found"\nEND IF\n',
     ("`FILEEXISTS(name)` gives 1 for yes and 0 for no: compare it with `= 0`, never with `NOT`.", "`FILEEXISTS(nombre)` da 1 si existe y 0 si no: compáralo con `= 0`, nunca con `NOT`.", "`FILEEXISTS(nome)` dá 1 se existe e 0 se não: compare com `= 0`, nunca com `NOT`.")),
    ("split", ("How do I split a text at the commas?", "¿Cómo separo un texto en las comas?", "Como separo um texto nas vírgulas?"), "split a string at a separator FIELD$",
     'DIM s AS STRING, i AS INTEGER\ns = "red,green,blue"\nFOR i = 1 TO TALLY(s, ",") + 1\n    PRINT FIELD$(s, ",", i)\nNEXT i\n',
     ("`FIELD$(text, separator, n)` is the n-th piece; `TALLY(text, separator)` counts the separators.", "`FIELD$(texto, separador, n)` es la pieza n; `TALLY(texto, separador)` cuenta los separadores.", "`FIELD$(texto, separador, n)` é a parte n; `TALLY(texto, separador)` conta os separadores.")),
    ("upper", ("How do I turn a text into capital letters?", "¿Cómo paso un texto a mayúsculas?", "Como passo um texto para maiúsculas?"), "uppercase lowercase text UCASE$",
     'DIM s AS STRING\ns = "RapidR"\nPRINT UCASE$(s)\nPRINT LCASE$(s)\n',
     ("`UCASE$` gives capitals and `LCASE$` small letters.", "`UCASE$` da mayúsculas y `LCASE$` minúsculas.", "`UCASE$` dá maiúsculas e `LCASE$` minúsculas.")),
    ("replace", ("How do I replace a word inside a text?", "¿Cómo reemplazo una palabra dentro de un texto?", "Como substituo uma palavra dentro de um texto?"), "replace text inside a string REPLACESUBSTR$",
     'DIM s AS STRING\ns = "I like cats"\nPRINT REPLACESUBSTR$(s, "cats", "dogs")\n',
     ("`REPLACESUBSTR$(text, find, replacement)` replaces every occurrence.", "`REPLACESUBSTR$(texto, buscar, reemplazo)` reemplaza cada aparición.", "`REPLACESUBSTR$(texto, procurar, substituto)` substitui cada ocorrência.")),
    ("find", ("How do I find where a word is in a text?", "¿Cómo encuentro dónde está una palabra en un texto?", "Como encontro onde está uma palavra em um texto?"), "find a word in a text INSTR",
     'DIM s AS STRING\ns = "the quick brown fox"\nPRINT INSTR(s, "quick")\nPRINT INSTR(s, "slow")\n',
     ("`INSTR(text, find)` is the position of the first match (1 is the first letter) and 0 when there is none.", "`INSTR(texto, buscar)` es la posición de la primera coincidencia (1 es la primera letra) y 0 si no hay.", "`INSTR(texto, procurar)` é a posição da primeira ocorrência (1 é a primeira letra) e 0 se não houver.")),
    ("decimals", ("How do I show a number with two decimals?", "¿Cómo muestro un número con dos decimales?", "Como mostro um número com duas casas decimais?"), "format a number with two decimals FORMAT$",
     'DIM x AS DOUBLE\nx = 3.14159\nPRINT FORMAT$("%.2f", x)\n',
     ("`FORMAT$(\"%.2f\", x)` writes `x` with two decimals.", "`FORMAT$(\"%.2f\", x)` escribe `x` con dos decimales.", "`FORMAT$(\"%.2f\", x)` escreve `x` com duas casas decimais.")),
    ("random", ("How do I get a random number from 1 to 6?", "¿Cómo obtengo un número al azar del 1 al 6?", "Como obtenho um número aleatório de 1 a 6?"), "random number RND RANDOMIZE",
     'DIM n AS INTEGER\nRANDOMIZE\nn = INT(RND * 6) + 1\nPRINT n >= 1 AND n <= 6\n',
     ("`RANDOMIZE` once starts the sequence, then `INT(RND * 6) + 1` is a whole number from 1 to 6.", "`RANDOMIZE` una vez inicia la secuencia y `INT(RND * 6) + 1` es un entero del 1 al 6.", "`RANDOMIZE` uma vez inicia a sequência e `INT(RND * 6) + 1` é um inteiro de 1 a 6.")),
    ("time", ("How do I get the current time?", "¿Cómo obtengo la hora actual?", "Como obtenho a hora atual?"), "current date and time DATE$ TIME$",
     'DIM t AS STRING\nt = TIME$\nPRINT LEN(t) > 0\nPRINT LEN(DATE$) > 0\n',
     ("`TIME$` is the time of day as text and `DATE$` today's date.", "`TIME$` es la hora del día como texto y `DATE$` la fecha de hoy.", "`TIME$` é a hora do dia como texto e `DATE$` a data de hoje.")),
    ("message", ("How do I show a message to the user?", "¿Cómo muestro un mensaje al usuario?", "Como mostro uma mensagem ao usuário?"), "show a message box ShowMessage",
     'CREATE Form AS RForm\n    Caption = "Message"\n    ClientWidth = 240: ClientHeight = 80\n    Center\n    CREATE Go AS RButton\n        Caption = "Hello"\n        Left = 16: Top = 20: Width = 100\n        OnClick = SayHello\n    END CREATE\nEND CREATE\n\nSUB SayHello\n    ShowMessage "Hello!"\nEND SUB\n\nForm.ShowModal\n',
     ("`ShowMessage text` shows a message box; here a button's `OnClick` SUB calls it.", "`ShowMessage texto` muestra un cuadro de mensaje; aquí lo llama el SUB `OnClick` de un botón.", "`ShowMessage texto` mostra uma caixa de mensagem; aqui o SUB `OnClick` de um botão a chama.")),
    ("val", ("How do I turn text into a number?", "¿Cómo convierto un texto en número?", "Como converto um texto em número?"), "convert text to number VAL STR$",
     'DIM s AS STRING, n AS DOUBLE\ns = "42"\nn = VAL(s) + 1\nPRINT n\nPRINT "Total: " + STR$(n)\n',
     ("`VAL(text)` reads a number from text and `STR$(n)` makes text from a number.", "`VAL(texto)` lee un número de un texto y `STR$(n)` hace un texto de un número.", "`VAL(texto)` lê um número de um texto e `STR$(n)` faz um texto de um número.")),
    ("array", ("How do I keep a list of names in an array?", "¿Cómo guardo una lista de nombres en un arreglo?", "Como guardo uma lista de nomes em um array?"), "array DIM items loop",
     'DIM names(2) AS STRING, i AS INTEGER\nnames(0) = "Ana"\nnames(1) = "Luis"\nnames(2) = "Marta"\nFOR i = 0 TO UBOUND(names)\n    PRINT names(i)\nNEXT i\n',
     ("`DIM names(2) AS STRING` makes three items, 0 to 2; fill them one at a time and loop with `UBOUND`.", "`DIM names(2) AS STRING` crea tres elementos, 0 a 2; se llenan uno a uno y se recorren con `UBOUND`.", "`DIM names(2) AS STRING` cria três itens, 0 a 2; preencha um a um e percorra com `UBOUND`.")),
    ("function", ("How do I write my own function that returns a value?", "¿Cómo escribo mi propia función que devuelva un valor?", "Como escrevo minha própria função que retorna um valor?"), "FUNCTION return a value",
     'FUNCTION Double(n AS INTEGER) AS INTEGER\n    Double = n * 2\nEND FUNCTION\n\nPRINT Double(21)\n',
     ("A FUNCTION returns by assigning to its own name: `Double = n * 2`.", "Una FUNCTION devuelve asignando a su propio nombre: `Double = n * 2`.", "Uma FUNCTION retorna atribuindo ao próprio nome: `Double = n * 2`.")),
    ("select", ("How do I do something different for each value of a number?", "¿Cómo hago algo distinto para cada valor de un número?", "Como faço algo diferente para cada valor de um número?"), "SELECT CASE different value",
     'DIM n AS INTEGER\nn = 2\nSELECT CASE n\n    CASE 1: PRINT "one"\n    CASE 2, 3: PRINT "two or three"\n    CASE ELSE: PRINT "other"\nEND SELECT\n',
     ("`SELECT CASE` compares one value with each `CASE`; `CASE ELSE` takes the rest.", "`SELECT CASE` compara un valor con cada `CASE`; `CASE ELSE` toma el resto.", "`SELECT CASE` compara um valor com cada `CASE`; `CASE ELSE` pega o resto.")),
    ("type", ("How do I group a name and an age in one record?", "¿Cómo agrupo un nombre y una edad en un solo registro?", "Como agrupo um nome e uma idade em um único registro?"), "TYPE record fields",
     'TYPE Person\n    Name AS STRING\n    Age AS INTEGER\nEND TYPE\n\nDIM p AS Person\np.Name = "Ana"\np.Age = 30\nPRINT p.Name; " is "; p.Age\n',
     ("A `TYPE … END TYPE` is a record; its fields are read and set as `p.Name`.", "Un `TYPE … END TYPE` es un registro; sus campos se leen y fijan como `p.Name`.", "Um `TYPE … END TYPE` é um registro; seus campos se leem e definem como `p.Name`.")),
    ("mid", ("How do I take part of a text?", "¿Cómo tomo una parte de un texto?", "Como pego uma parte de um texto?"), "part of a text MID$ LEFT$ RIGHT$",
     'DIM s AS STRING\ns = "RapidR Studio"\nPRINT LEFT$(s, 6)\nPRINT MID$(s, 8, 3)\nPRINT RIGHT$(s, 2)\n',
     ("`LEFT$` and `RIGHT$` take letters from the ends, `MID$(text, start, count)` from the middle.", "`LEFT$` y `RIGHT$` toman letras de los extremos, `MID$(texto, inicio, cantidad)` del medio.", "`LEFT$` e `RIGHT$` pegam letras das pontas, `MID$(texto, início, quantidade)` do meio.")),
    ("while", ("How do I repeat something until a condition is met?", "¿Cómo repito algo hasta que se cumpla una condición?", "Como repito algo até que uma condição seja atendida?"), "DO WHILE LOOP repeat until",
     'DIM n AS INTEGER\nn = 1\nDO WHILE n < 100\n    n = n * 2\nLOOP\nPRINT n\n',
     ("`DO WHILE condition … LOOP` repeats while the condition is true.", "`DO WHILE condición … LOOP` repite mientras la condición sea verdadera.", "`DO WHILE condição … LOOP` repete enquanto a condição for verdadeira.")),
    ("timer", ("How do I do something every second?", "¿Cómo hago algo cada segundo?", "Como faço algo a cada segundo?"), "timer every second RTimer OnTimer",
     'DECLARE SUB Tick\nDIM n AS INTEGER\n\nCREATE Form AS RForm\n    Caption = "Ticks"\n    ClientWidth = 200: ClientHeight = 60\n    Center\n    CREATE Shown AS RLabel\n        Left = 16: Top = 16: Width = 160\n        Caption = "0"\n    END CREATE\n    CREATE Clock AS RTimer\n        Interval = 1000\n        Enabled = 1\n        OnTimer = Tick\n    END CREATE\nEND CREATE\n\nSUB Tick\n    n = n + 1\n    Shown.Caption = STR$(n)\nEND SUB\n\nForm.ShowModal\n',
     ("An `RTimer` with `Interval = 1000` calls its `OnTimer` SUB every second.", "Un `RTimer` con `Interval = 1000` llama a su SUB `OnTimer` cada segundo.", "Um `RTimer` com `Interval = 1000` chama seu SUB `OnTimer` a cada segundo.")),
]
HOWTO_Q = {"en": ["{q}", "Quick question: {q}", "{q} I'm new to RapidR."], "es": ["{q}", "Una pregunta: {q}", "{q} Soy nuevo en RapidR."], "pt": ["{q}", "Uma pergunta: {q}", "{q} Sou novo no RapidR."]}


def gen_d(b: Builder, pool: list[prog.Program], quota: int) -> None:
    made = 0
    # --- d1: how-to questions, answered after the manual
    tasks = []
    for tid, qs, query, code, expl in HOWTO:
        if not strict_ok(b, code):
            log(f"d: how-to {tid} does not compile clean: {b.checker.compiles([('main.rr', code)], 'main.rr')[1][:100]}")
            continue
        out = "" if "ShowModal" in code else run_snippet(code)
        if out is None:
            log(f"d: how-to {tid} does not run")
            continue
        tasks.append((tid, qs, query, code, expl, out))
    log(f"d: {len(tasks)} how-to tasks compile and run")
    quota1 = int(quota * 0.45)
    rounds = 0
    while made < quota1 and tasks and rounds < 12:
        for tid, qs, query, code, expl, out in tasks:
            if made >= quota1:
                break
            r = b.rng("d1", tid, rounds)
            lang = r.choice(LANGS)
            L = LANGS.index(lang)
            profile = ("lite", "full", "mcp")[r.choices([0, 1, 2], weights=[50, 10, 40])[0]]
            user = r.choice(HOWTO_Q[lang]).format(q=qs[L])
            steps, project = [], None
            if profile == "mcp":
                project = b.placeholder()
                steps.append(Step(calls=[("studio_status", {})]))
            steps.append(Step(calls=[("search_manual", {"query": query})]))
            if r.random() < 0.5:
                steps.append(Step(calls=[("check_code", {"path": "main.rr", "text": code})]))
            shown = {"en": f"Output: `{out.splitlines()[0]}`" if out.strip() else "", "es": f"Salida: `{out.splitlines()[0]}`" if out.strip() else "", "pt": f"Saída: `{out.splitlines()[0]}`" if out.strip() else ""}[lang]
            answer = f"{expl[L]}\n\n```basic\n{code.rstrip()}\n```\n{shown}".strip()
            key = f"d1/{tid}/{lang}/{profile}/{sha(user)}"
            prov = {"sources": ["templates:howto", "docs/manual/"], "generator": "build_corpus_more.gen_d+howto", "task": tid, "snippet_output_verified": True}
            if b.add(key, "d", profile, lang, project, user, steps, prov, f"howto:{tid}"):
                made += 1
        rounds += 1
    log(f"d: {made} how-to conversations")
    # --- d3: a change request on a program that is open: read it, change only what is asked
    changes = {
        "caption": (r'^(\s*)Caption = "([^"\n]+)"$', ("Change the title of the window to \"{v}\"", "Cambia el título de la ventana a \"{v}\"", "Mude o título da janela para \"{v}\""), ["My window", "Demo", "Hello", "Notes", "Workshop"], ("the window's title is now \"{v}\"", "el título de la ventana es ahora \"{v}\"", "o título da janela agora é \"{v}\"")),
        "interval": (r"^(\s*)Interval = (\d+)$", ("Make the timer tick every {v} ms", "Haz que el temporizador lata cada {v} ms", "Faça o temporizador bater a cada {v} ms"), ["500", "250", "2000", "100"], ("the timer ticks every {v} ms", "el temporizador late cada {v} ms", "o temporizador bate a cada {v} ms")),
        "width": (r"^(\s*)ClientWidth = (\d+): ClientHeight = (\d+)$", ("Make the window wider: {v} pixels", "Haz la ventana más ancha: {v} píxeles", "Deixe a janela mais larga: {v} pixels"), ["400", "480", "560"], ("the window is {v} pixels wide", "la ventana mide {v} píxeles de ancho", "a janela tem {v} pixels de largura")),
    }
    targets = [p for p in pool if p.generator.startswith("template") and p.kind == "gui"]
    rounds = 0
    quota3 = int(quota * 0.9)
    while made < quota3 and targets and rounds < 10:
        for p in targets:
            if made >= quota3:
                break
            r = b.rng("d3", p.id, rounds)
            name = r.choice(list(changes))
            pat, qs, vals, done = changes[name]
            lines = p.text.split("\n")
            hits = [i for i, l in enumerate(lines) if re.match(pat, l)]
            if not hits:
                continue
            k = hits[0] if name == "caption" else hits[0]
            old = lines[k]
            val = r.choice(vals)
            m = re.match(pat, old)
            if name == "caption":
                new = f'{m.group(1)}Caption = "{val}"'
            elif name == "interval":
                new = f"{m.group(1)}Interval = {val}"
            else:
                new = f"{m.group(1)}ClientWidth = {val}: ClientHeight = {m.group(3)}"
            if old == new or p.text.count(old) != 1:
                continue
            lang = r.choice(LANGS)
            L = LANGS.index(lang)
            profile = ("lite", "full", "mcp")[r.choices([0, 1, 2], weights=[50, 10, 40])[0]]
            user = qs[L].format(v=val)
            steps = []
            if profile == "mcp":
                steps.append(Step(calls=[("studio_status", {})]))
            steps.append(Step(calls=[("read_file", {"path": "main.rr"})]))
            steps.append(Step(calls=[("propose_edit", {"path": "main.rr", "edits": [{"old_text": old, "new_text": new}], "summary": done[0].format(v=val).capitalize() + "."})]))
            steps.append(Step(text=prog_changed(lang, done[L].format(v=val))))
            key = f"d3/{p.id}/{name}/{lang}/{profile}/{sha(user + val)}"
            prov = {"sources": p.sources, "generator": f"build_corpus_more.gen_d+change+{p.generator}", "program": p.id, "change": name}
            if b.add(key, "d", profile, lang, b.open_with(p.text), user, steps, prov, p.id):
                made += 1
        rounds += 1
    log(f"d: {made} conversations in all")


def prog_changed(lang: str, what: str) -> str:
    return {"en": f"Done: {what}.", "es": f"Hecho: {what}.", "pt": f"Pronto: {what}."}[lang]
