#!/usr/bin/env python3
"""How much of a small model's context RAI's fixed prompt takes (Qwen3.5 tokens): the system
prompt, the tool schemas as the chat template writes them, and one recipe.

    $RAI_ROOT/venv/bin/python tools/rai_train/measure_prompt.py [--window 8192]
"""

from __future__ import annotations

import argparse
import statistics
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from rai_corpus import REPO, Checker, Setups, tokenizer  # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--window", type=int, default=8192)
    a = ap.parse_args()
    tok = tokenizer()
    setups = Setups()
    chk = Checker()

    def count(messages, tools=None):
        return len(tok.apply_chat_template(messages, tools=tools, return_dict=False))

    print(f"Qwen3.5 tokens; window {a.window}\n")
    print("| set-up | system prompt | + tool schemas (as the template writes them) | = before the user speaks | share of the window |")
    print("|---|---:|---:|---:|---:|")
    base = {}
    for profile in ("lite", "full", "mcp"):
        system = setups.system(profile, None)
        sys_only = count([{"role": "system", "content": system}, {"role": "user", "content": "Build me a program."}])
        with_tools = count([{"role": "system", "content": system}, {"role": "user", "content": "Build me a program."}], setups.tools[profile])
        no_user = count([{"role": "system", "content": ""}, {"role": "user", "content": "."}])
        base[profile] = with_tools
        print(f"| {profile} | {sys_only - no_user} | {with_tools - sys_only} | {with_tools} | {100 * with_tools / a.window:.0f}% |")
    # the recipes: what RAI sends with a request for a new program
    import re

    sizes = []
    for f in sorted((REPO / "examples/recipes").glob("*.rr")):
        m = re.search(r"^' Recipe:\s*(.+)$", f.read_text(), re.M)
        if not m:
            continue
        ctx = chk.run([], "", [("find_example", {"query": m.group(1)})])[0]["content"]
        sizes.append((len(tok(ctx)["input_ids"]), f.name))
    toks = sorted(s for s, _ in sizes)
    print(f"\nOne recipe (find_example's answer, {len(sizes)} recipes): min {toks[0]}, median {int(statistics.median(toks))}, max {toks[-1]} tokens ({sorted(sizes)[-1][1]}).\n")
    print("| set-up | + median recipe | + largest recipe | left for the conversation (largest recipe) |")
    print("|---|---:|---:|---:|")
    for profile in ("lite", "full", "mcp"):
        med, big = base[profile] + int(statistics.median(toks)), base[profile] + toks[-1]
        print(f"| {profile} | {med} ({100 * med / a.window:.0f}%) | {big} ({100 * big / a.window:.0f}%) | {a.window - big} |")
    return 0


if __name__ == "__main__":
    sys.exit(main())
