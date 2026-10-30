#!/usr/bin/env python3
"""Checks a built corpus before anything is trained on it. Never part of the quick checks or of
tools/regress.sh by default (it needs the corpus on the SSD and the rai_check binary); run it by
hand, or as an extra step of a local regress:

    python3 tools/rai_train/validate_corpus.py [--data DIR] [--holdout-ref DIR] [--no-replay] [--report]

What it checks (exit status 1 when any fails):
  schema      every row: roles in order, tool calls named in the row's tool list with their
              required arguments and nothing else, the last message an assistant turn
  compile     every program a row teaches (write_file, create_project, an edit applied to the
              file the model read, check_code text, code blocks in the answer) compiles with
              RapidR's own compiler (rai_check), and every slip shown for context fails
  replay      every tool answer in every conversation equals what RapidR's tools answer now
              (run again with no cache): the corpus has no invented tool output
  hold-out    no prompt and no program is (near) the eval's: tests/ai-eval/{apps,held-out,school}
              prompts and click tests, their reference solutions, and the recipes that are their
              solutions; fuzzy: word-set and sequence similarity, 6-word shingles, exact hashes
  sources     every item's provenance names allowed material only (examples/, docs/manual/, the
              registry, templates), none of the forbidden (RapidQ's corpus, .reference, eval
              transcripts, other projects); a provenance line for every item; paraphrases marked
  hygiene     no key-shaped strings, no absolute paths of the machine, no foreign-language
              boilerplate about other products
"""

from __future__ import annotations

import argparse
import collections
import difflib
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from rai_corpus import (  # noqa: E402
    DATA, EVAL_RECIPES, HOLDOUT_ADJACENT_RECIPES, REPO, Checker, mcp_applied, norm, shingles, sha, source_allowed, words,
)

STOP = set("a an the me my i it is of to for in on and or with that this please can you could would build make create write need want like un una el la los las de del con para por que y o en mi hazme crea quiero necesito um uma o os as do da com e faz faca cria crie quero preciso".split())
DEFAULT_REF = Path("/Users/roanbema/Programming/rust/RapidR/dist/ai-eval-2026-10-29/reference")
PROMPT_RATIO, PROMPT_JACCARD, CODE_CONTAIN, STRUCT_CONTAIN = 0.80, 0.60, 0.50, 0.50


# ---- hold-out ------------------------------------------------------------------------------------
def load_holdout(ref: Path) -> dict:
    prompts: list[tuple[str, str]] = []
    scripts: list[tuple[str, str]] = []
    for d in ("apps", "held-out", "school"):
        for f in sorted((REPO / "tests/ai-eval" / d).glob("*.txt")):
            text = f.read_text(encoding="utf-8")
            scripts.append((f"tests/ai-eval/{d}/{f.name}", text))
            for line in text.splitlines():
                if line.strip() and not line.startswith("#"):
                    prompts.append((f"tests/ai-eval/{d}/{f.name}", line.strip()))
    programs: list[tuple[str, str]] = []
    for f in sorted(ref.rglob("*.rr")) if ref.exists() else []:
        programs.append((f"reference/{f.relative_to(ref)}", f.read_text(encoding="utf-8", errors="replace")))
    for name in sorted(EVAL_RECIPES | HOLDOUT_ADJACENT_RECIPES):
        programs.append((f"examples/recipes/{name}", (REPO / "examples/recipes" / name).read_text(encoding="utf-8")))
    return {"prompts": prompts, "scripts": scripts, "programs": programs, "ref_found": ref.exists()}


def content(text: str) -> set[str]:
    return {w for w in words(text) if w not in STOP}


def prompt_similarity(a: str, b: str) -> float:
    na, nb = norm(a), norm(b)
    if na == nb or na in nb or nb in na:
        return 1.0 if min(len(na), len(nb)) > 12 else 0.0
    ratio = difflib.SequenceMatcher(None, na, nb).ratio()
    ca, cb = content(a), content(b)
    jac = len(ca & cb) / len(ca | cb) if ca | cb else 0.0
    return max(ratio if ratio >= PROMPT_RATIO else 0.0, jac if jac >= PROMPT_JACCARD and len(ca & cb) >= 3 else 0.0, ratio * 0.5)


KEYWORDS = set("""DIM AS INTEGER STRING DOUBLE LONG SINGLE BYTE WORD DWORD SHORT SUB FUNCTION END IF THEN ELSE ELSEIF FOR TO STEP NEXT DO WHILE LOOP UNTIL
SELECT CASE CONST TYPE CREATE DECLARE EXIT PRINT INPUT OPEN CLOSE GOTO GOSUB RETURN WITH AND OR NOT MOD XOR BYREF BYVAL REDIM""".split())
TOKEN = re.compile(r'"[^"]*"|[A-Za-z_][A-Za-z0-9_$]*|\d+(?:\.\d+)?|\S')


def structure_shingles(text: str, n: int = 12) -> set[str]:
    """The program's shape with names, strings and numbers blurred: a copy with every identifier
    renamed still has the same shape."""
    toks = []
    for line in text.splitlines():
        line = line.split("'", 1)[0] if "'" in line and line.count('"') % 2 == 0 else line
        for t in TOKEN.findall(line):
            if t.startswith('"'):
                toks.append("S")
            elif t[0].isdigit():
                toks.append("N")
            elif t.upper().rstrip("$") in KEYWORDS:
                toks.append(t.upper())
            elif t[0].isalpha() or t[0] == "_":
                toks.append("I")
            else:
                toks.append(t)
    return {" ".join(toks[i : i + n]) for i in range(max(0, len(toks) - n + 1))}


def code_shingles(text: str) -> set[str]:
    code = "\n".join(l for l in text.splitlines() if not l.strip().startswith("'"))
    return shingles(code, 6)


# ---- reading the corpus ---------------------------------------------------------------------------
def code_blocks(text: str) -> list[str]:
    return re.findall(r"```(?:basic)?\n(.*?)```", text or "", re.S)


def tool_args(call: dict) -> tuple[str, dict]:
    f = call["function"]
    return f["name"], f["arguments"]


def base_text(messages: list[dict], path: str) -> str | None:
    """The text of `path` as the model last saw it: a read_file answer, or what it last wrote."""
    for m in reversed(messages):
        if m["role"] == "tool":
            try:
                d = json.loads(m["content"])
                if isinstance(d, dict) and d.get("source") == f"file:{path}":
                    return d["content"]
            except Exception:
                pass
        if m["role"] == "assistant":
            for c in m.get("tool_calls", []):
                n, a = tool_args(c)
                if n == "write_file" and a.get("path") == path:
                    return a["text"]
    return None


def apply_edits(text: str, edits: list[dict]) -> str | None:
    for e in edits:
        if text.count(e["old_text"]) != 1:
            return None
        text = text.replace(e["old_text"], e["new_text"])
    return text


def learned_code(row: dict) -> list[tuple[str, list[tuple[str, str]], str]]:
    last = row["messages"][-1]
    out = []
    for c in last.get("tool_calls", []):
        n, a = tool_args(c)
        if n == "create_project":
            out.append((n, [(f["path"], f["text"]) for f in a["files"]], a["main"]))
        elif n == "write_file":
            out.append((n, [(a["path"], a["text"])], a["path"]))
        elif n == "check_code" and "text" in a:
            out.append((n, [(a.get("path", "main.rr"), a["text"])], a.get("path", "main.rr")))
        elif n == "propose_edit":
            base = base_text(row["messages"][:-1], a["path"])
            new = apply_edits(base, a["edits"]) if base is not None else None
            out.append((n, [(a["path"], new)] if new is not None else [], a["path"]))
    for blk in code_blocks(last.get("content") or ""):
        out.append(("answer", [("main.rr", blk)], "main.rr"))
    return out


def check_schema(row: dict, schemas: dict[str, dict]) -> str | None:
    ms = row["messages"]
    if not ms or ms[0]["role"] != "system" or len(ms) < 3 or ms[1]["role"] != "user":
        return "must start with system, user"
    if ms[-1]["role"] != "assistant":
        return "last message must be the assistant's"
    names = {t["function"]["name"] for t in row["tools"]}
    pending = 0
    for mi, m in enumerate(ms[2:], start=2):
        r = m["role"]
        if r == "assistant":
            if pending:
                return "tool call without its answer"
            for c in m.get("tool_calls", []):
                n, a = tool_args(c)
                if n not in names:
                    return f"tool {n} is not in the row's tool list"
                if not isinstance(a, dict):
                    return f"{n}: arguments are not an object"
                sch = schemas[n]
                for req in sch.get("required", []):
                    if req not in a:
                        return f"{n}: lacks {req}"
                extra = set(a) - set(sch.get("properties", {}))
                if extra:
                    return f"{n}: unknown arguments {sorted(extra)}"
                for k, v in a.items():
                    t = sch["properties"][k].get("type")
                    if t == "string" and not isinstance(v, str) or t == "array" and not isinstance(v, list) or t == "integer" and not isinstance(v, int) or t == "boolean" and not isinstance(v, bool):
                        return f"{n}.{k}: wrong type"
                pending += 0 if mi == len(ms) - 1 else 1
            if not m.get("tool_calls") and not (m.get("content") or "").strip():
                return "empty assistant message"
        elif r == "tool":
            if not pending:
                return "tool answer without a call"
            pending -= 1
        elif r == "user":
            if pending:
                return "user message inside a tool exchange"
        else:
            return f"unexpected role {r}"
    if pending:
        return "unanswered tool call"
    return None


# ---- the checks ---------------------------------------------------------------------------------------
class Report:
    def __init__(self) -> None:
        self.lines: list[str] = []
        self.failed = False

    def ok(self, name: str, text: str) -> None:
        self.lines.append(f"- PASS {name}: {text}")
        print(f"PASS {name}: {text}")

    def fail(self, name: str, text: str, examples: list[str] | None = None) -> None:
        self.failed = True
        self.lines.append(f"- FAIL {name}: {text}")
        print(f"FAIL {name}: {text}")
        for e in (examples or [])[:6]:
            self.lines.append(f"  - {e}")
            print("   ", e)

    def info(self, text: str) -> None:
        self.lines.append(f"- {text}")
        print(text)


def selftest(ref: Path) -> int:
    """The guards must fire on what they guard against (run: validate_corpus.py --selftest)."""
    h = load_holdout(ref)
    fails = 0

    def expect(name: str, cond: bool) -> None:
        nonlocal fails
        print(("ok   " if cond else "FAIL ") + name)
        fails += 0 if cond else 1

    tip = next(p for _, p in h["prompts"] if "tip calculator" in p.lower())
    expect("a hold-out prompt, verbatim", prompt_similarity(tip, tip) >= PROMPT_JACCARD)
    expect("a hold-out prompt, reworded", prompt_similarity("Could you build me a tip calculator, please", tip) >= PROMPT_JACCARD)
    expect("an unrelated prompt is not flagged", prompt_similarity("Build me a window with a clock", tip) < PROMPT_JACCARD)
    recipe = (REPO / "examples/recipes/tip_calculator.rr").read_text()
    hs = code_shingles(recipe)
    renamed = recipe.replace("Bill", "Amount").replace("Tip", "Gratuity")
    expect("an eval recipe, verbatim", len(hs & code_shingles(recipe)) / len(hs) >= CODE_CONTAIN)
    hst = structure_shingles(recipe)
    expect("an eval recipe with every name renamed, by shape", len(hst & structure_shingles(renamed)) / len(hst) >= STRUCT_CONTAIN)
    expect("a program of another shape is not flagged", len(hst & structure_shingles("PRINT 1\nFOR i = 1 TO 3\n  PRINT i\nNEXT i\n" * 8)) / len(hst) < STRUCT_CONTAIN)
    expect("a source in .reference is refused", not source_allowed(".reference/corpus26.json"))
    expect("a RapidQ example is refused", not source_allowed("examples/rapidq/calc.bas"))
    expect("a transcript is refused", not source_allowed("dist/ai-eval-2026-10-29/runs/x/transcript.jsonl"))
    expect("the manual is allowed", source_allowed("docs/manual/language.md"))
    chk = Checker(cache=False)
    expect("foreign syntax does not compile", not chk.compiles([("main.rr", "SET x = 1\n")], "main.rr")[0])
    expect("a plain program compiles", chk.compiles([("main.rr", "PRINT 1\n")], "main.rr")[0])
    chk.close()
    bad_row = {"messages": [{"role": "system", "content": "s"}, {"role": "user", "content": "u"}, {"role": "assistant", "content": "", "tool_calls": [{"type": "function", "function": {"name": "nope", "arguments": {}}}]}], "tools": []}
    expect("a call to a tool that is not offered fails the schema check", check_schema(bad_row, {}) is not None)
    print("selftest:", "PASSED" if not fails else f"{fails} FAILED")
    return 1 if fails else 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--data", default=str(DATA))
    ap.add_argument("--holdout-ref", default=str(DEFAULT_REF), help="the eval's reference solutions (not in the repository)")
    ap.add_argument("--no-replay", action="store_true", help="skip running every tool answer again (the slow check)")
    ap.add_argument("--report", action="store_true", help="write the result into tools/rai_train/CORPUS.md")
    ap.add_argument("--selftest", action="store_true", help="check that the guards fire on what they guard against, then stop")
    args = ap.parse_args()
    if args.selftest:
        return selftest(Path(args.holdout_ref))
    data = Path(args.data)
    rep = Report()
    rows = {s: [json.loads(l) for l in (data / f"{s}.jsonl").open()] for s in ("train", "valid")}
    items = [json.loads(l) for l in (data / "items.jsonl").open()]
    prov = {json.loads(l)["id"]: json.loads(l) for l in (data / "provenance.jsonl").open()}
    n_rows = sum(len(v) for v in rows.values())
    rep.info(f"{len(items)} items, {n_rows} rows ({len(rows['train'])} train, {len(rows['valid'])} valid) in {data}")

    # -- schema
    bad = []
    for split, rs in rows.items():
        for k, row in enumerate(rs):
            schemas = {t["function"]["name"]: t["function"]["parameters"] for t in row["tools"]}
            e = check_schema(row, schemas)
            if e:
                bad.append(f"{split}[{k}]: {e}")
    rep.ok("schema", f"all {n_rows} rows well formed") if not bad else rep.fail("schema", f"{len(bad)} rows malformed", bad)

    # -- compile (learned code)
    chk = Checker()
    total = failed = 0
    kinds = collections.Counter()
    unverifiable = []
    fails = []
    seen: dict[str, bool] = {}
    for split, rs in rows.items():
        for k, row in enumerate(rs):
            for label, files, main_f in learned_code(row):
                kinds[label] += 1
                total += 1
                if not files or any(t is None for _, t in files):
                    unverifiable.append(f"{split}[{k}] {label}: edit not applicable to what the model read")
                    continue
                key = sha(json.dumps(files) + main_f)
                if key not in seen:
                    ok, said = chk.compiles(files, main_f)
                    if label == "answer" and ok and not said.startswith("No problems"):
                        ok = False
                    seen[key] = ok
                    if not ok:
                        fails.append(f"{split}[{k}] {label}: {said.splitlines()[0][:110]}")
                if not seen[key]:
                    failed += 1
    if total and not failed and not unverifiable:
        rep.ok("compile", f"100% of the {total} code samples learned compile ({', '.join(f'{k} {v}' for k, v in kinds.most_common())}; {len(seen)} distinct)")
    else:
        rep.fail("compile", f"{failed} of {total} code samples do not compile, {len(unverifiable)} unverifiable", fails + unverifiable)

    # -- slips shown for context really fail; replay of tool answers
    if args.no_replay:
        rep.info("replay: skipped (--no-replay)")
    else:
        fresh = Checker(cache=False)
        mism = []
        slips = slip_ok = 0
        for it in items:
            msgs = it["messages"]
            proj = it.get("project")
            files = [tuple(f) for f in proj[2]] if proj else []
            main_f = proj[1] if proj else ""
            calls = [(c["function"]["name"], c["function"]["arguments"]) for m in msgs if m["role"] == "assistant" for c in m.get("tool_calls", [])]
            real = [c for c in calls if c[0] != "studio_status"]
            res = fresh.run(files, main_f, real, apply=True) if real else []
            tool_msgs = [m for m in msgs if m["role"] == "tool"]
            k = 0
            for (name, inp), tm in zip(calls, tool_msgs):
                if name == "studio_status":
                    continue
                r = res[k]
                k += 1
                expect = mcp_applied(name, inp, r["content"], r["is_error"], proj is not None) if it["profile"] == "mcp" else r["content"]
                if expect != tm["content"]:
                    mism.append(f"{it['id']}: {name} answer differs now")
            # a slip (a try shown for context, never learned): its answer must be a refusal or errors
            for mi in it.get("slips", []):
                slips += 1
                if re.search(r"error |Not proposed|doesn't compile|Refused", msgs[mi + 1]["content"]):
                    slip_ok += 1
        fresh.close()
        rep.ok("replay", f"all tool answers of {len(items)} conversations equal RapidR's answers now (run again, no cache)") if not mism else rep.fail("replay", f"{len(mism)} tool answers differ", mism)
        rep.ok("slips", f"all {slips} slips shown for context really fail") if slips == slip_ok else rep.fail("slips", f"{slips - slip_ok} of {slips} context slips did not fail")
    chk.close()

    # -- hold-out
    h = load_holdout(Path(args.holdout_ref))
    if not h["ref_found"]:
        rep.info(f"hold-out: reference solutions not found at {args.holdout_ref}: compared with the eval recipes only")
    user_prompts = {}
    for it in items:
        u = next(m for m in it["messages"] if m["role"] == "user")["content"]
        u = re.split(r"\n\n\(An example RapidR Studio found", u)[0]
        user_prompts[u] = it["id"]
    worst = (0.0, "", "")
    over = []
    for u, iid in user_prompts.items():
        for src, hp in h["prompts"]:
            s = prompt_similarity(u, hp)
            if s > worst[0]:
                worst = (s, u, hp)
            if s >= min(PROMPT_RATIO, PROMPT_JACCARD):
                over.append(f"prompt of {iid}: {u!r} ~ {hp!r} ({src}) {s:.2f}")
    rep.ok("holdout-prompts", f"{len(user_prompts)} distinct prompts vs {len(h['prompts'])} eval prompts: none near (closest {worst[0]:.2f}: {worst[1][:50]!r} ~ {worst[2][:50]!r})") if not over else rep.fail("holdout-prompts", f"{len(over)} prompts too close to an eval prompt", over)
    # programs and anything that looks like one, in learned turns and in tool answers
    hp_sh = [(src, code_shingles(t), norm(t)) for src, t in h["programs"]]
    hp_struct = [(src, structure_shingles(t)) for src, t in h["programs"]]
    script_sh = [(src, norm(t)) for src, t in h["scripts"]]
    texts: dict[str, str] = {}
    for it in items:
        for m in it["messages"]:
            if m["role"] == "system":
                continue
            blobs = [m.get("content") or ""]
            for c in m.get("tool_calls", []):
                n, a = tool_args(c)
                blobs += [v for v in a.values() if isinstance(v, str)] + [f["text"] for f in a.get("files", [])] + [e["new_text"] for e in a.get("edits", [])]
            for b in blobs:
                if len(b) > 200:
                    texts.setdefault(sha(b), b)
    worst_c = (0.0, "")
    worst_s = (0.0, "")
    over_c = []
    for k, t in texts.items():
        sh = code_shingles(t)
        nt = norm(t)
        for src, hs, hn in hp_sh:
            if not hs:
                continue
            contain = len(hs & sh) / len(hs)  # how much of the eval program is in this text
            back = len(hs & sh) / len(sh) if sh else 0.0
            sc = max(contain, back if len(sh) > 40 else 0.0)
            if sc > worst_c[0]:
                worst_c = (sc, f"{src}")
            if sc >= CODE_CONTAIN or (len(hn) > 200 and hn in nt):
                over_c.append(f"text {k} ~ {src} ({sc:.2f})")
        st = structure_shingles(t)
        for src, hs2 in hp_struct:
            if len(hs2) > 30:
                sc2 = len(hs2 & st) / len(hs2)
                if sc2 > worst_s[0]:
                    worst_s = (sc2, src)
                if sc2 >= STRUCT_CONTAIN:
                    over_c.append(f"text {k} has the shape of {src} ({sc2:.2f})")
        for src, sn in script_sh:
            if len(sn) > 200 and sn in nt:
                over_c.append(f"text {k} contains {src}")
    rep.ok("holdout-code", f"{len(texts)} distinct code/answer texts vs {len(hp_sh)} eval programs ({'reference solutions + ' if h['ref_found'] else ''}solution recipes): none near (closest by words {worst_c[0]:.2f} to {worst_c[1]}; by shape, names blurred, {worst_s[0]:.2f} to {worst_s[1]})") if not over_c else rep.fail("holdout-code", f"{len(over_c)} texts too close to an eval program", over_c)

    # -- sources and provenance
    problems = []
    for it in items:
        p = prov.get(it["id"])
        if not p:
            problems.append(f"{it['id']}: no provenance")
            continue
        if not p.get("sources") or not p.get("generator"):
            problems.append(f"{it['id']}: provenance incomplete")
        for s in p.get("sources", []):
            if not source_allowed(s):
                problems.append(f"{it['id']}: source {s} not allowed")
        pb = p.get("paraphrased_by")
        if pb is not None and not str(pb).startswith("paraphrased-by:"):
            problems.append(f"{it['id']}: paraphrase not marked")
    if sum(p["rows"] for p in prov.values()) != n_rows:
        problems.append(f"rows in provenance {sum(p['rows'] for p in prov.values())} != rows in files {n_rows}")
    for recipe in sorted(EVAL_RECIPES | HOLDOUT_ADJACENT_RECIPES):
        for p in prov.values():
            if any(recipe in s for s in p.get("sources", [])):
                problems.append(f"{p['id']}: uses withheld recipe {recipe}")
    rep.ok("sources", f"{len(prov)} provenance lines, every source allowed, none withheld, no cloud-model material") if not problems else rep.fail("sources", f"{len(problems)} provenance problems", problems)

    # -- hygiene
    bad = []
    pat = re.compile(r"sk-ant-|sk-[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{12,}|/Users/roanbema|/Volumes/RapidRBuild|Downloads/Rapidq|\.reference/|ghp_[A-Za-z0-9]{20,}")
    for it in items:
        for m in it["messages"]:
            if m["role"] == "system":
                continue
            blob = json.dumps(m, ensure_ascii=False)
            mm = pat.search(blob)
            if mm:
                bad.append(f"{it['id']}: {mm.group(0)}")
    rep.ok("hygiene", "no keys, machine paths or RapidQ-corpus paths in any message") if not bad else rep.fail("hygiene", f"{len(bad)} messages with sensitive-looking text", bad)

    status = "FAILED" if rep.failed else "PASSED"
    print(f"\nvalidate_corpus: {status}")
    if args.report:
        path = REPO / "tools/rai_train/CORPUS.md"
        text = path.read_text()
        import datetime

        block = "<!-- validate:begin -->\n## Validation (`tools/rai_train/validate_corpus.py`, " + str(datetime.date.today()) + f"): {status}\n\n" + "\n".join(rep.lines) + "\n<!-- validate:end -->"
        text = re.sub(r"<!-- validate:begin -->.*?<!-- validate:end -->", lambda _: block, text, flags=re.S)
        path.write_text(text)
        print(f"wrote {path}")
    return 1 if rep.failed else 0


if __name__ == "__main__":
    sys.exit(main())
