#!/usr/bin/env python3
"""Proves lora_train.py's two memory savers change nothing: the loss and the gradients of
the LoRA weights are the same as with mlx-lm's own loss and its own gated-delta loop.

    $RAI_ROOT/venv/bin/python tools/rai_train/check_lora_train.py [--tokens 400]
"""

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import mlx.core as mx
import mlx.nn as nn
from mlx.utils import tree_flatten, tree_unflatten
from mlx_lm import load
from mlx_lm.tuner.trainer import default_loss
from mlx_lm.tuner.utils import linear_to_lora_layers

import lora_train
from rai_corpus import ROOT


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tokens", type=int, default=400)
    a = ap.parse_args()
    model, _ = load(str(ROOT / "models" / "Qwen3.5-2B"))
    model.freeze()
    model.train()
    linear_to_lora_layers(model, 4, {"rank": 8, "dropout": 0.0, "scale": 20.0})
    # (random but not zero B: a LoRA starts at zero, which would make every gradient of A zero)
    flat = tree_flatten(model.trainable_parameters())
    model.update(tree_unflatten([(k, mx.random.normal(v.shape) * 0.02) for k, v in flat]))
    T = a.tokens
    batch = mx.random.randint(0, 5000, (1, T + 1))
    lengths = mx.array([[T // 3, T]])
    lora_train.CHUNK = 96

    def run(loss, ops):
        lora_train.gated_delta.gated_delta_ops = ops
        (l, n), g = nn.value_and_grad(model, loss)(model, batch, lengths)
        mx.eval(l, g)
        return float(l), dict(tree_flatten(g))

    def compare(g1, g2):
        worst = 0.0
        for k in g1:
            a1, a2 = g1[k].astype(mx.float32), g2[k].astype(mx.float32)
            worst = max(worst, float(mx.sqrt(mx.sum((a1 - a2) ** 2))) / (float(mx.sqrt(mx.sum(a1**2))) + 1e-12))
        return worst

    l1, g1 = run(default_loss, lora_train._ops)
    results = {}
    for name, loss, ops in (("chunked loss only", lora_train.chunked_loss, lora_train._ops), ("segmented delta loop only", default_loss, lora_train.segmented_ops), ("both", lora_train.chunked_loss, lora_train.segmented_ops)):
        l2, g2 = run(loss, ops)
        results[name] = (abs(l1 - l2), compare(g1, g2))
        print(f"{name}: loss {l2:.5f} vs mlx-lm's {l1:.5f}; {len(g1)} LoRA gradients differ by {results[name][1]:.2e} (relative L2, largest)")
    worst = max(r[1] for r in results.values())
    loss_diff = max(r[0] for r in results.values())
    # (bfloat16 arithmetic and another order of sums give a percent or two on a gradient; a mistake gives order one)
    ok = loss_diff < 1e-3 * max(1.0, abs(l1)) and worst < 5e-2
    print("check_lora_train:", "PASSED" if ok else "FAILED")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
