#!/usr/bin/env python3
"""`mlx_lm lora` with a loss that fits in memory: the same training, the same options.

Qwen3.5 has a vocabulary of 248,320 tokens. mlx-lm's loss makes the logits of every position
at once: for one row of 3,500 tokens that is 3.5 GB in float32 (plus its gradient and the
softmax), and a batch of two took 42 GB of this Mac's 48 and ended in "Insufficient Memory".
Here the output layer and the cross-entropy run on 512 positions at a time, each recomputed in
the backward pass (mx.checkpoint), so the logits never exist whole. Same numbers, a fraction of
the memory (see README: measured).

    lora_train.py --model DIR --train --data DIR ...      (the options of `python -m mlx_lm lora`)

Only for models with `language_model` (Qwen3.5); any other model uses mlx-lm's own loss.
"""

import functools
import os
import sys

import mlx.core as mx
import mlx.nn as nn
from mlx_lm import lora

CHUNK = int(os.environ.get("RAI_LOSS_CHUNK", "512"))
SEGMENT = int(os.environ.get("RAI_DELTA_SEGMENT", "128"))

# Qwen3.5's linear-attention layers (gated delta rule). While training, mlx-lm runs them as a
# Python loop of ONE step per token (its Metal kernel has no gradient), and autograd keeps every
# step's state: ~5 MB a token a layer, so one 3,500-token row asks for 20 GB or more and the GPU
# says "Insufficient Memory". Here the loop runs in segments of SEGMENT tokens, each recomputed
# in the backward pass (mx.checkpoint): only one segment's steps live at a time. Same numbers.
from mlx_lm.models import gated_delta  # noqa: E402

_ops = gated_delta.gated_delta_ops


def segmented_ops(q, k, v, g, beta, state=None, mask=None):
    B, T = q.shape[:2]
    if T <= SEGMENT or mask is not None:
        return _ops(q, k, v, g, beta, state, mask)
    if state is None:
        state = mx.zeros((B, v.shape[-2], v.shape[-1], q.shape[-1]), dtype=mx.float32)
    ys = []
    for start in range(0, T, SEGMENT):
        part = slice(start, start + SEGMENT)

        def run(s, q_, k_, v_, g_, b_):
            return _ops(q_, k_, v_, g_, b_, s, None)

        y, state = mx.checkpoint(run)(state, q[:, part], k[:, part], v[:, part], g[:, part], beta[:, part])
        ys.append(y)
    return mx.concatenate(ys, axis=1), state


gated_delta.gated_delta_ops = segmented_ops


def chunked_loss(model, batch, lengths):
    inputs, targets = batch[:, :-1], batch[:, 1:]
    lm = getattr(model, "language_model", None)
    if lm is None or not hasattr(lm, "model"):
        from mlx_lm.tuner.trainer import default_loss

        return default_loss(model, batch, lengths)
    hidden = lm.model(inputs)
    steps = mx.arange(1, targets.shape[1] + 1)
    mask = mx.logical_and(steps >= lengths[:, 0:1], steps <= lengths[:, 1:])
    tied = lm.args.tie_word_embeddings

    def head(x):
        return lm.model.embed_tokens.as_linear(x) if tied else lm.lm_head(x)

    def piece(t, m):
        # (targets and mask are integers and a mask: constants of the piece, not inputs to differentiate)
        def f(h):
            return (nn.losses.cross_entropy(head(h).astype(mx.float32), t) * m).sum()

        return mx.checkpoint(f)

    total = mx.array(0.0)
    for start in range(0, targets.shape[1], CHUNK):
        part = slice(start, start + CHUNK)
        total = total + piece(targets[:, part], mask[:, part])(hidden[:, part])
    ntoks = mask.sum()
    return total / ntoks, ntoks


if __name__ == "__main__":
    lora.train = functools.partial(lora.train, loss=chunked_loss)
    sys.argv[0] = "mlx_lm.lora"
    lora.main()
