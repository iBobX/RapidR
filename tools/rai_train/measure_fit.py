#!/usr/bin/env python3
"""Does a model fit a small computer? What Ollama says about a model on this Mac: download
size, the memory it holds with an 8K and a 16K context, and its speed (prompt processing and
generation), with the GPU and without (num_gpu 0: a laptop with no good GPU, roughly).

    measure_fit.py OLLAMA_NAME [--ctx 8192,16384] [--prompt-tokens 3000] [--gen-tokens 200]
                   [--no-cpu] [--out FILE.md]

Memory: `ollama ps` SIZE (what the loaded model holds for that context) and the peak resident
memory of Ollama's runner process while it answers (ps, polled). Speed: Ollama's own counters
(prompt_eval_duration, eval_duration). One run per cell after a warm-up, so read them as
indications, not benchmarks. Each cell unloads the model first, so the context is the one asked.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import threading
import time
import urllib.request

URL = "http://127.0.0.1:11434"


def post(path: str, body: dict, timeout: int = 900) -> dict:
    req = urllib.request.Request(URL + path, data=json.dumps(body).encode(), headers={"Content-Type": "application/json"})
    return json.loads(urllib.request.urlopen(req, timeout=timeout).read())


def runner_rss_mb() -> float:
    """Resident memory (MB) of Ollama's runner processes."""
    out = subprocess.run(["ps", "-axo", "rss,command"], capture_output=True, text=True).stdout
    total = 0
    for line in out.splitlines():
        if "ollama" in line and ("runner" in line or "serve" in line):
            m = re.match(r"\s*(\d+)\s", line)
            if m:
                total += int(m.group(1))
    return total / 1024


def ps_size(name: str) -> str:
    out = subprocess.run(["ollama", "ps"], capture_output=True, text=True).stdout
    for line in out.splitlines()[1:]:
        if line.startswith(name):
            cols = re.split(r"\s{2,}", line.strip())
            return f"{cols[2]} ({cols[3]})" if len(cols) > 3 else cols[2]
    return "?"


def download_size(name: str) -> str:
    out = subprocess.run(["ollama", "list"], capture_output=True, text=True).stdout
    for line in out.splitlines()[1:]:
        if line.startswith(name):
            return re.split(r"\s{2,}", line.strip())[2]
    return "?"


FILLER = "The quick brown fox jumps over the lazy dog while the program waits for the user to press a button. "


def cell(name: str, ctx: int, prompt_tokens: int, gen_tokens: int, cpu: bool) -> dict:
    subprocess.run(["ollama", "stop", name], capture_output=True)
    time.sleep(2)
    opts = {"num_ctx": ctx, "num_predict": gen_tokens, "temperature": 0}
    if cpu:
        opts["num_gpu"] = 0
    # (about 4 characters a token: the prompt is a little under the asked count)
    prompt = (FILLER * (prompt_tokens * 4 // len(FILLER) + 1))[: prompt_tokens * 4]
    # warm-up: loads the model for this context
    post("/api/generate", {"model": name, "prompt": "Hi", "stream": False, "options": {**opts, "num_predict": 1}, "think": False})
    peak = [0.0]
    stop = threading.Event()

    def poll() -> None:
        while not stop.is_set():
            peak[0] = max(peak[0], runner_rss_mb())
            time.sleep(0.25)

    t = threading.Thread(target=poll, daemon=True)
    t.start()
    r = post("/api/generate", {"model": name, "prompt": prompt + "\n\nWrite a long story about it.", "stream": False, "options": opts, "think": False})
    stop.set()
    t.join()
    size = ps_size(name)
    pe = r.get("prompt_eval_count", 0) / max(r.get("prompt_eval_duration", 1) / 1e9, 1e-9)
    ge = r.get("eval_count", 0) / max(r.get("eval_duration", 1) / 1e9, 1e-9)
    return {"ctx": ctx, "cpu": cpu, "ps_size": size, "peak_rss_mb": peak[0], "prompt_tokens": r.get("prompt_eval_count", 0), "prompt_tps": pe, "gen_tokens": r.get("eval_count", 0), "gen_tps": ge}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("name")
    ap.add_argument("--ctx", default="8192,16384")
    ap.add_argument("--prompt-tokens", type=int, default=3000)
    ap.add_argument("--gen-tokens", type=int, default=200)
    ap.add_argument("--no-cpu", action="store_true")
    ap.add_argument("--out")
    a = ap.parse_args()
    rows = []
    for cpu in ([False] if a.no_cpu else [False, True]):
        for ctx in [int(x) for x in a.ctx.split(",")]:
            print(f"measuring {a.name}: context {ctx}, {'CPU only' if cpu else 'GPU'} ...", flush=True)
            rows.append(cell(a.name, ctx, min(a.prompt_tokens, ctx - a.gen_tokens - 200), a.gen_tokens, cpu))
    subprocess.run(["ollama", "stop", a.name], capture_output=True)
    lines = [
        f"Model `{a.name}`: download {download_size(a.name)}.",
        "",
        "| context | where | held by `ollama ps` | peak resident (runner) | prompt tokens/s | generation tokens/s |",
        "|---:|---|---|---:|---:|---:|",
    ]
    for r in rows:
        lines.append(f"| {r['ctx']} | {'CPU only (num_gpu 0)' if r['cpu'] else 'GPU (Metal)'} | {r['ps_size']} | {r['peak_rss_mb'] / 1024:.1f} GB | {r['prompt_tps']:.0f} (of {r['prompt_tokens']} tokens) | {r['gen_tps']:.0f} (of {r['gen_tokens']} tokens) |")
    text = "\n".join(lines)
    print("\n" + text)
    if a.out:
        open(a.out, "w").write(text + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
