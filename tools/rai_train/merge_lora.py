#!/usr/bin/env python3
"""Merges an mlx-lm LoRA adapter into the Hugging Face weights of its base model, leaving the
weights in the base's own layout and key names (so llama.cpp's converter can read them).

    merge_lora.py --base MODEL_DIR --adapters ADAPTER_DIR --out OUT_DIR [--verify]

Why not `mlx_lm fuse`: it saves MLX's layout (some Qwen3.5 tensors are transposed or renamed
when MLX loads them), which the converter does not expect. Here only the adapted linear
weights change: W' = W + scale * (A @ B)^T  (A: in x r, B: r x out, scale from adapter_config).
--verify compares the logits of base+adapter with those of the merged model on a sample.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
from pathlib import Path

import mlx.core as mx


def tail(key: str) -> str:
    """layers.N.rest (the part of a key that is the same in MLX's and the Hugging Face names)."""
    m = re.search(r"layers\.\d+\..+", key)
    return m.group(0) if m else key


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", required=True)
    ap.add_argument("--adapters", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--verify", action="store_true")
    a = ap.parse_args()
    base, adir, out = Path(a.base), Path(a.adapters), Path(a.out)
    cfg = json.loads((adir / "adapter_config.json").read_text())
    scale = float(cfg["lora_parameters"]["scale"])
    ad = mx.load(str(adir / "adapters.safetensors"))
    pairs: dict[str, dict[str, mx.array]] = {}
    for k, v in ad.items():
        mod, which = k.rsplit(".", 1)
        pairs.setdefault(tail(mod), {})[which] = v
    print(f"adapter: {len(pairs)} modules, scale {scale}")
    out.mkdir(parents=True, exist_ok=True)
    for f in base.iterdir():
        if f.suffix != ".safetensors" and f.is_file() and not f.name.startswith("."):
            shutil.copy2(f, out / f.name)
    used = set()
    for shard in sorted(base.glob("*.safetensors")):
        w = mx.load(str(shard))
        for k in list(w):
            # (only the language model: the multi-token-prediction head `mtp.layers.0…` and the
            # vision tower have layers of their own with the same tails)
            if not k.endswith(".weight") or not k.startswith("model.language_model."):
                continue
            t = tail(k[: -len(".weight")])
            if t in pairs and "lora_a" in pairs[t]:
                A, B = pairs[t]["lora_a"].astype(mx.float32), pairs[t]["lora_b"].astype(mx.float32)
                delta = scale * (A @ B).T
                if delta.shape != w[k].shape:
                    print(f"shape mismatch for {k}: {delta.shape} vs {w[k].shape}", file=sys.stderr)
                    return 1
                w[k] = (w[k].astype(mx.float32) + delta).astype(w[k].dtype)
                used.add(t)
        mx.eval(list(w.values()))
        mx.save_safetensors(str(out / shard.name), w, metadata={"format": "pt"})
        print(f"wrote {shard.name}")
    missing = set(pairs) - used
    if missing:
        print(f"adapter modules without a base weight: {sorted(missing)[:5]}", file=sys.stderr)
        return 1
    print(f"merged {len(used)} weights into {out}")
    if a.verify:
        from mlx_lm import load

        ids = mx.array([[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]])
        m1, _ = load(str(base), adapter_path=str(adir))
        l1 = m1(ids)
        m2, _ = load(str(out))
        l2 = m2(ids)
        mx.eval(l1, l2)
        diff = float(mx.max(mx.abs(l1.astype(mx.float32) - l2.astype(mx.float32))))
        ref = float(mx.max(mx.abs(l1.astype(mx.float32))))
        print(f"verify: max |logits(base+adapter) - logits(merged)| = {diff:.4f} (logits up to {ref:.1f})")
        if diff > 0.05 * ref:
            print("verify: the merged model does not match the adapted one", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
