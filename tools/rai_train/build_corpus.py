#!/usr/bin/env python3
"""Builds the RAI training corpus: conversations with RapidR Studio's assistant (and with
Studio's MCP server), in the tools format of mlx-lm, from this repository's own material.

    $RAI_ROOT/venv/bin/python tools/rai_train/build_corpus.py [--seed N] [--scale 1.0] [--quick]

Writes (never into the repository): $RAI_ROOT/data/{train,valid}.jsonl (rows), items.jsonl
(whole conversations), provenance.jsonl (one line per item: its sources and generator), and
the stats report tools/rai_train/CORPUS.md (small, committed).

The five kinds (docs: tools/rai_train/README.md):
  a  Q&A from the registry: how do I ... with RTimer / MID$ ... -> rapidr_describe -> answer + code
  b  whole programs: a request (EN/ES/PT) -> find_example (or the example Studio sends) -> write -> check -> answer
  c  error -> fix: a foreign-syntax slip, the compiler's real message, the corrected code
  d  look first: how-to and change requests answered only after search_manual / read_file
  e  do not repeat: the same error twice, then look at an example, then fix

Every tool answer in a conversation is the REAL one (rai_check replays RapidR's own tools); every
program the model is trained to write compiles; a model's slip shown for context is marked and
never learned. No cloud model, no RapidQ corpus, no outside code.
"""

from __future__ import annotations

import argparse
import collections
import json
import random
import re
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import mutations as mut  # noqa: E402
import programs as prog  # noqa: E402
from rai_corpus import (  # noqa: E402
    DATA, EVAL_RECIPES, HOLDOUT_ADJACENT_RECIPES, REPO, Checker, Setups, Step, conversation, log, rows_of, sha, source_allowed,
)

LANGS = prog.LANGS
WEIGHTS = {"lite": 0.5, "full": 0.1, "mcp": 0.4}

# ---- phrases (all written for this corpus) ----------------------------------------------------------
BUILD = {
    "en": ["Build me {x}", "Make {x}", "Write {x}", "I need {x}", "Can you create {x}?", "Create {x}, please", "I'd like {x}", "Please write {x}"],
    "es": ["Hazme {x}", "Crea {x}", "Escribe {x}", "Necesito {x}", "¿Puedes crear {x}?", "Quiero {x}", "Crea {x}, por favor", "Programa {x}"],
    "pt": ["Faça {x}", "Crie {x}", "Escreva {x}", "Preciso de {x}", "Você pode criar {x}?", "Quero {x}", "Crie {x}, por favor", "Desenvolva {x}"],
}
BROKEN = {
    "en": ["My program doesn't compile. Can you fix it?", "Fix the errors in main.rr, please.", "It says there is an error. Please fix it.", "This won't run, what is wrong?", "Can you make it compile?"],
    "es": ["Mi programa no compila. ¿Puedes arreglarlo?", "Arregla los errores de main.rr, por favor.", "Dice que hay un error. Arréglalo, por favor.", "Esto no corre, ¿qué pasa?", "¿Puedes hacer que compile?"],
    "pt": ["Meu programa não compila. Você pode consertar?", "Conserte os erros de main.rr, por favor.", "Diz que há um erro. Conserte, por favor.", "Isto não roda, o que está errado?", "Você pode fazer compilar?"],
}
RUN = {"en": "Press Run (F5) to try it.", "es": "Pulsa Ejecutar (F5) para probarlo.", "pt": "Pressione Executar (F5) para testar."}
RUN_CONSOLE = {"en": "Press Run (F5) and read the output.", "es": "Pulsa Ejecutar (F5) y lee la salida.", "pt": "Pressione Executar (F5) e leia a saída."}
FIXED = {"en": "Fixed: {why} It compiles now.", "es": "Arreglado: {why} Ahora compila.", "pt": "Consertado: {why} Agora compila."}
AGAIN = {
    "en": "The same mistake twice. Let me look at how it is written before I try again.",
    "es": "El mismo error dos veces. Voy a ver cómo se escribe antes de volver a intentarlo.",
    "pt": "O mesmo erro duas vezes. Vou ver como se escreve antes de tentar de novo.",
}
FIX_AGAIN = {"en": "That was it: {why} It compiles now.", "es": "Eso era: {why} Ahora compila.", "pt": "Era isso: {why} Agora compila."}
CHANGED = {"en": "Done: {what}.", "es": "Hecho: {what}.", "pt": "Pronto: {what}."}


def pick_profile(rng: random.Random) -> str:
    return rng.choices(list(WEIGHTS), weights=list(WEIGHTS.values()))[0]


def title_of(p: prog.Program) -> str:
    first = p.text.split("\n", 1)[0]
    m = re.match(r"^' ([^:]{3,40}):", first)
    if p.generator.startswith("template") and m:
        return m.group(1)
    stem = Path(p.sources[0]).stem.replace("_", " ")
    return stem[:1].upper() + stem[1:]


STOP = set("a an the that this these those program programs console window form one small simple with and of me my i it in to for from on is are by as or at into when each every than then them its their number numbers show shows shown press presses button buttons".split())


def content_words(text: str) -> list[str]:
    """The words of an English request that say what it is about."""
    return [w for w in re.findall(r"[A-Za-z0-9$]+", text) if w.lower() not in STOP and len(w) > 2]


def first_sentence(text: str) -> str:
    return re.split(r"(?<=[.!?])\s", text.strip())[0]


class Builder:
    def __init__(self, seed: int, quick: bool):
        self.seed = seed
        self.quick = quick
        self.checker = Checker()
        self.setups = Setups()
        self.items: list[dict] = []
        self.keys: set[str] = set()
        self.rejected: collections.Counter = collections.Counter()
        self.recipe_title: dict[str, str] = {}
        for f in sorted((REPO / "examples/recipes").glob("*.rr")):
            m = re.search(r"^' Recipe:\s*(.+)$", f.read_text(), re.M)
            if m:
                self.recipe_title[f.name] = m.group(1).strip()
        self.mut_counts: collections.Counter = collections.Counter()
        self.blocked_titles = {t for f, t in self.recipe_title.items() if f in EVAL_RECIPES | HOLDOUT_ADJACENT_RECIPES}

    # ---- helpers -----------------------------------------------------------------------------
    def rng(self, *key) -> random.Random:
        return random.Random(f"{self.seed}/" + "/".join(map(str, key)))

    def tool(self, name: str, inp: dict, files=None, main: str = "") -> dict:
        return self.checker.run(files or [], main, [(name, inp)])[0]

    def recipe_context(self, question: str) -> str:
        return self.tool("__recipe_context", {"question": question})["content"]

    def find_example(self, query: str) -> str:
        return self.tool("find_example", {"query": query})["content"]

    def context_title(self, text: str) -> str | None:
        m = re.search(r'(?:Example|Recipe) "([^"]+)"', text)
        return m.group(1) if m else None

    def reject(self, why: str) -> None:
        self.rejected[why] += 1

    def add(self, key: str, kind: str, profile: str, lang: str, project, user: str, steps: list[Step], prov: dict, group: str, context: str = "", expect_final: list | None = None) -> bool:
        """Plays the conversation against the real tools and keeps it when everything holds:
        every step that is learned and calls a writing tool made code that compiles, and no learned
        tool call was refused."""
        if key in self.keys:
            return False
        conv = conversation(self.checker, self.setups, profile, project, user, steps, context)
        # the learned assistant turns: their tool calls must be valid, and what they wrote must have worked
        k = 0
        names = self.setups.tool_names[profile]
        bad = None
        idx_of_call = []  # (assistant message index, call index)
        results = conv["results"]
        step_results = {}
        pos = 0
        for si, s in enumerate(steps):
            n = len([c for c in s.calls if c[0] != "studio_status"])
            step_results[si] = results[pos : pos + n]
            pos += n
        for si, s in enumerate(steps):
            if not s.train:
                continue
            real = [c for c in s.calls if c[0] != "studio_status"]
            for (name, inp), r in zip(real, step_results[si]):
                if name not in names:
                    bad = f"tool {name} not in {profile}"
                elif r["is_error"]:
                    bad = f"learned call {name} refused: {r['content'][:80]}"
                elif name in ("write_file", "create_project", "propose_edit") and ("doesn't compile" in r["content"] or "errors to fix" in r["note"]):
                    bad = f"learned {name} does not compile"
                schema = self.setups.tool_schema[profile].get(name)
                if schema:
                    for req in schema.get("required", []):
                        if req not in inp:
                            bad = f"{name} lacks {req}"
        if bad:
            self.reject(bad.split(":")[0])
            return False
        rows = rows_of(conv)
        if not rows:
            self.reject("no rows")
            return False
        self.keys.add(key)
        self.items.append({"id": key, "kind": kind, "profile": profile, "lang": lang, "conv": conv, "rows": rows, "group": group, "prov": prov, "project": project})
        return True

    # ---- the project shapes ---------------------------------------------------------------------
    @staticmethod
    def placeholder() -> tuple[str, str, list[tuple[str, str]]]:
        return ("MyApp", "main.rr", [("main.rr", "' MyApp: an empty program.\n")])

    @staticmethod
    def open_with(code: str, name: str = "MyApp") -> tuple[str, str, list[tuple[str, str]]]:
        return (name, "main.rr", [("main.rr", code)])

    # ---- b: whole programs ------------------------------------------------------------------------
    def build_flow(self, p: prog.Program, lang: str, profile: str, rng: random.Random, kind: str, break_first: mut.Mutation | None = None, repeat: bool = False):
        """The steps that make program p in `profile`; with `break_first`, a refused (or failing) first
        try learned from (c2), and with `repeat` the same slip twice (e)."""
        L = LANGS.index(lang)
        x = rng.choice([p.req[lang]] + p.req_alts.get(lang, []))
        user = rng.choice(BUILD[lang]).format(x=x)
        code = p.text
        title = title_of(p)
        summary = first_sentence(p.does["en"])
        final_text = p.does[lang] + " " + (RUN if p.kind == "gui" else RUN_CONSOLE)[lang]
        steps: list[Step] = []
        context = ""
        project = None
        group = p.id

        def write_calls(text: str):
            if profile == "mcp":
                return [("write_file", {"path": "main.rr", "text": text, "summary": summary})]
            return [("create_project", {"name": title, "main": "main.rr", "files": [{"path": "main.rr", "text": text}], "summary": summary})]

        # --- finding what to start from
        if profile == "mcp":
            project = self.placeholder()
            steps.append(Step(calls=[("studio_status", {})]))
            q, found = self.example_query(p, rng)
            if q is None:
                return None
            steps.append(Step(calls=[("find_example", {"query": q})]))
            if not p.recipe and rng.random() < 0.4:
                steps.append(Step(calls=[("search_manual", {"query": self.manual_query(p, rng)})]))
        else:
            context = self.recipe_context(user)
            t = self.context_title(context) if context else None
            if t in self.blocked_titles:
                self.reject("blocked recipe injected")
                return None
            if p.recipe:
                if t != self.recipe_title.get(p.recipe):
                    self.reject("recipe not injected")
                    return None
            elif t is not None:
                self.reject("other recipe injected")
                return None
            if not p.recipe and rng.random() < 0.35:
                steps.append(Step(calls=[("search_manual", {"query": self.manual_query(p, rng)})]))
        # --- writing (with a slip first, when asked)
        if break_first is not None:
            bad = break_first.text
            steps.append(Step(calls=write_calls(bad), train=False))
            if repeat:
                steps.append(Step(calls=write_calls(bad), train=False))
                look = ("search_manual", {"query": mut.LOOKUP[break_first.name]})
                if p.recipe and profile != "lite" and rng.random() < 0.6:
                    look = ("find_example", {"query": self.example_query(p, rng)[0] or self.recipe_title[p.recipe]})
                steps.append(Step(text=AGAIN[lang], calls=[look]))
        steps.append(Step(calls=write_calls(code)))
        if profile == "mcp":
            steps.append(Step(calls=[("check_code", {})]))
        why = mut.WHY[break_first.name][L] if break_first else ""
        if break_first is not None:
            final = (FIX_AGAIN if repeat else FIXED)[lang].format(why=why) + " " + p.does[lang]
        else:
            final = final_text
        steps.append(Step(text=final))
        return user, steps, context, project, group

    def example_query(self, p: prog.Program, rng: random.Random):
        """A query for find_example that finds this program's recipe, or no recipe for a program that has none."""
        cands = []
        if p.recipe:
            cands.append(self.recipe_title[p.recipe])
        words = content_words(p.req["en"])
        cands += [" ".join(words[:4]), " ".join(words[:3]), " ".join(words[:6])]
        for q in cands:
            text = self.find_example(q)
            t = self.context_title(text)
            if p.recipe:
                if t == self.recipe_title[p.recipe]:
                    return q, text
            elif t is None and "No recipe is close" in text:
                return q, text
        return None, ""

    def manual_query(self, p: prog.Program, rng: random.Random) -> str:
        return " ".join(content_words(p.req["en"])[: rng.choice([3, 4, 5])])

    def gen_b(self, pool: list[prog.Program], quota: int) -> None:
        made = 0
        order = list(pool)
        rng = self.rng("b-order")
        rng.shuffle(order)
        rounds = 0
        while made < quota and rounds < 12:
            for p in order:
                if made >= quota:
                    break
                r = self.rng("b", p.id, rounds)
                cap_lang = next((t.split("=")[1] for t in p.tags if t.startswith("lang=")), None)
                lang = cap_lang if (cap_lang and p.kind == "gui" and r.random() < 0.9) else r.choice(LANGS)
                profile = pick_profile(r)
                flow = self.build_flow(p, lang, profile, r, "b")
                if flow is None:
                    continue
                user, steps, context, project, group = flow
                key = f"b/{p.id}/{lang}/{profile}/{sha(user)}"
                prov = {"sources": p.sources, "generator": f"build_corpus.gen_b+{p.generator}", "program": p.id, "recipe": p.recipe or None}
                if self.add(key, "b", profile, lang, project, user, steps, prov, group, context):
                    made += 1
            rounds += 1
        log(f"b: {made} whole-program conversations")

    # ---- c / e: foreign syntax, the real message, the fix -----------------------------------------------------
    def candidates(self, p: prog.Program, rng: random.Random, limit: int) -> list[mut.Mutation]:
        out = []
        seen = set()
        for m in mut.all_mutations(p.text, rng):
            if m.name in seen or not mut.usable(m, p.text):
                continue
            r = self.tool("check_code", {"path": "main.rr"}, files=[("main.rr", m.text)], main="main.rr")
            lines = [l for l in r["content"].splitlines() if l.startswith("error")]
            if not lines:
                self.reject("slip accepted by the compiler: " + m.name)
                continue
            seen.add(m.name)
            out.append(m)
            if len(out) >= limit:
                break
        return out

    def pick_mutation(self, ms: list, rng: random.Random):
        """The kind of slip least used so far (so `SET` does not drown the rest)."""
        return min(ms, key=lambda m: (self.mut_counts[m.name], rng.random()))

    def gen_c(self, pool: list[prog.Program], quota: int) -> None:
        """c1: a broken program in the open project: look, check, fix with propose_edit.
        c2: the model's first try slips (not learned), the real error, the fix."""
        made = 0
        order = [p for p in pool if len(p.text.splitlines()) <= 120]
        self.rng("c-order").shuffle(order)
        rounds = 0
        while made < quota and rounds < 8:
            for p in order:
                if made >= quota:
                    break
                r = self.rng("c", p.id, rounds)
                ms = self.candidates(p, r, 8)
                if not ms:
                    continue
                m = self.pick_mutation(ms, r)
                lang = r.choice(LANGS)
                L = LANGS.index(lang)
                profile = pick_profile(r)
                prov = {"sources": p.sources, "generator": f"build_corpus.gen_c+{p.generator}", "program": p.id, "mutation": m.name}
                if r.random() < 0.55:
                    # c1
                    steps = []
                    if profile == "mcp":
                        steps.append(Step(calls=[("studio_status", {})]))
                    steps.append(Step(calls=[("read_file", {"path": "main.rr"})]))
                    steps.append(Step(calls=[("check_code", {"path": "main.rr"})]))
                    steps.append(Step(calls=[("propose_edit", {"path": "main.rr", "edits": [{"old_text": m.old, "new_text": m.new}], "summary": mut.WHY[m.name][L]})]))
                    steps.append(Step(text=FIXED[lang].format(why=mut.WHY[m.name][L])))
                    user = r.choice(BROKEN[lang])
                    key = f"c1/{p.id}/{m.name}/{lang}/{profile}/{sha(user + m.text)}"
                    if self.add(key, "c", profile, lang, self.open_with(m.text), user, steps, {**prov, "shape": "c1"}, p.id):
                        made += 1
                        self.mut_counts[m.name] += 1
                else:
                    flow = self.build_flow(p, lang, profile, r, "c", break_first=m)
                    if flow is None:
                        continue
                    user, steps, context, project, group = flow
                    key = f"c2/{p.id}/{m.name}/{lang}/{profile}/{sha(user)}"
                    if self.add(key, "c", profile, lang, project, user, steps, {**prov, "shape": "c2"}, p.id, context):
                        made += 1
                        self.mut_counts[m.name] += 1
            rounds += 1
        log(f"c: {made} error-to-fix conversations")

    def gen_e(self, pool: list[prog.Program], quota: int) -> None:
        made = 0
        order = [p for p in pool if len(p.text.splitlines()) <= 40]
        self.rng("e-order").shuffle(order)
        rounds = 0
        while made < quota and rounds < 12:
            for p in order:
                if made >= quota:
                    break
                r = self.rng("e", p.id, rounds)
                ms = self.candidates(p, r, 8)
                if not ms:
                    continue
                m = self.pick_mutation(ms, r)
                lang = r.choice(LANGS)
                profile = pick_profile(r)
                flow = self.build_flow(p, lang, profile, r, "e", break_first=m, repeat=True)
                if flow is None:
                    continue
                user, steps, context, project, group = flow
                key = f"e/{p.id}/{m.name}/{lang}/{profile}/{sha(user)}"
                prov = {"sources": p.sources, "generator": f"build_corpus.gen_e+{p.generator}", "program": p.id, "mutation": m.name, "shape": "e"}
                if self.add(key, "e", profile, lang, project, user, steps, prov, p.id, context):
                    made += 1
                    self.mut_counts[m.name] += 1
            rounds += 1
        log(f"e: {made} do-not-repeat conversations")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=20261029)
    ap.add_argument("--scale", type=float, default=1.0)
    ap.add_argument("--quick", action="store_true", help="a small corpus (a few hundred items) to try the pipeline")
    ap.add_argument("--kinds", default="abcde")
    ap.add_argument("--out", default=str(DATA))
    ap.add_argument("--max-tokens", type=int, default=8192, help="rows longer than this (Qwen tokens) are left out: they would not fit the training window")
    args = ap.parse_args()
    t0 = time.time()
    b = Builder(args.seed, args.quick)
    real, notes1 = prog.load_real(b.checker)
    tmpl, notes2 = prog.make_templates(b.checker, args.seed, per_template=4 if args.quick else 26)
    pool = real + tmpl
    log(f"programs: {len(real)} real + {len(tmpl)} templates (notes: {len(notes1) + len(notes2)})")
    q = {"a": 1100, "b": 700, "c": 1100, "d": 650, "e": 350}
    q = {k: max(4, int(v * args.scale * (0.1 if args.quick else 1))) for k, v in q.items()}
    if "b" in args.kinds:
        b.gen_b(pool, q["b"])
    if "c" in args.kinds:
        b.gen_c(pool, q["c"])
    if "e" in args.kinds:
        b.gen_e(pool, q["e"])
    if "a" in args.kinds or "d" in args.kinds:
        import build_corpus_more as more  # noqa: E402  (kinds a and d)

        if "a" in args.kinds:
            more.gen_a(b, q["a"])
        if "d" in args.kinds:
            more.gen_d(b, pool, q["d"])
    write_outputs(b, Path(args.out), args, pool, notes1 + notes2, time.time() - t0)
    b.checker.close()
    return 0


def write_outputs(b: Builder, out: Path, args, pool, notes, secs) -> None:
    out.mkdir(parents=True, exist_ok=True)
    # valid: whole groups (a program, a component) so no item of valid shares a program with train
    groups = sorted({it["group"] for it in b.items})
    r = random.Random(f"{args.seed}/split")
    r.shuffle(groups)
    n_valid = max(2, int(len(groups) * 0.04))
    valid_groups = set(groups[:n_valid])
    train_rows, valid_rows = [], []
    from rai_corpus import row_tokens  # noqa: E402

    dropped_long = dropped_items = 0
    kept_items = []
    for it in b.items:
        keep, lost_key = [], False
        for row in it["rows"]:
            if row_tokens(row) > args.max_tokens:
                dropped_long += 1
                # (the turn that writes the program is the one worth learning: without it the
                # conversation teaches little, so the whole item goes)
                lost_key = lost_key or any(c["function"]["name"] in ("write_file", "create_project", "propose_edit") for c in row["messages"][-1].get("tool_calls", []))
            else:
                keep.append(row)
        if lost_key or not keep:
            dropped_items += 1
            continue
        it["train"] = [i for i, row in zip(it["conv"]["train"], it["rows"]) if len(row["messages"]) == i + 1 and row in keep]
        it["rows"] = keep
        kept_items.append(it)
    b.items = kept_items
    b.dropped_long = dropped_long
    b.dropped_items = dropped_items
    with (out / "items.jsonl").open("w") as fi, (out / "provenance.jsonl").open("w") as fp:
        for it in b.items:
            split = "valid" if it["group"] in valid_groups else "train"
            (valid_rows if split == "valid" else train_rows).extend(it["rows"])
            fi.write(json.dumps({"id": it["id"], "kind": it["kind"], "profile": it["profile"], "lang": it["lang"], "split": split, "project": it["project"], "messages": it["conv"]["messages"], "train": it["train"], "slips": it["conv"]["slips"]}, ensure_ascii=False) + "\n")
            tools_used = sorted({c["function"]["name"] for m in it["conv"]["messages"] for c in m.get("tool_calls", [])})
            fp.write(json.dumps({"id": it["id"], "kind": it["kind"], "profile": it["profile"], "lang": it["lang"], "split": split, "rows": len(it["rows"]), "tools_used": tools_used, "paraphrased_by": None, **it["prov"]}, ensure_ascii=False) + "\n")
    for name, rows in (("train", train_rows), ("valid", valid_rows)):
        with (out / f"{name}.jsonl").open("w") as f:
            for row in rows:
                f.write(json.dumps(row, ensure_ascii=False) + "\n")
    log(f"{len(b.items)} items -> {len(train_rows)} train rows, {len(valid_rows)} valid rows in {out} ({secs:.0f} s; checker cache {b.checker.hits} hits, {b.checker.misses} misses)")
    import corpus_report  # noqa: E402

    corpus_report.write(b, out, args, pool, notes, secs, len(train_rows), len(valid_rows))


if __name__ == "__main__":
    sys.exit(main())
