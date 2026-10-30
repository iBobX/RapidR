#!/bin/bash
# RAI-TRAIN: a LoRA fine-tune of a Qwen3.5 base with MLX (on this Mac's GPU).
#
#   tools/rai_train/train_lora.sh NAME [options]
#
#   --model 2B|4B        which base (default 2B; the weights are in $RAI_ROOT/models)
#   --rows N             train on a seeded slice of N rows (default: all of data/train.jsonl)
#   --profiles lite,...  set-ups the slice takes rows from (default lite; with --rows)
#   --iters N            training steps (default 50 for a smoke run; a real run: ~ rows*epochs/batch)
#   --batch N            rows per step (default 2)
#   --max-seq N          longest row, in tokens (default 4096)
#   --layers N           LoRA on the last N layers (default 16 of 24; -1 = all)
#   --lr X               learning rate (default 1e-5 for the smoke run; 1e-4 is usual for LoRA)
#   --data DIR           another corpus folder (train.jsonl, valid.jsonl)
#
# Writes $RAI_ROOT/runs/NAME/{data,adapters,train.log}. Only the answer of each row (its last
# assistant message) is learned: --mask-prompt. Nothing is shipped or published by this script.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=common.sh
. "$HERE/common.sh"

[ $# -ge 1 ] || { sed -n '2,17p' "$0"; exit 1; }
NAME="$1"; shift
MODEL=2B; ROWS=""; PROFILES=lite; ITERS=50; BATCH=2; MAXSEQ=4096; LAYERS=16; LR=1e-5; DATA="$RAI_ROOT/data"
while [ $# -gt 0 ]; do
  case "$1" in
    --model) MODEL="$2"; shift 2 ;;
    --rows) ROWS="$2"; shift 2 ;;
    --profiles) PROFILES="$2"; shift 2 ;;
    --iters) ITERS="$2"; shift 2 ;;
    --batch) BATCH="$2"; shift 2 ;;
    --max-seq) MAXSEQ="$2"; shift 2 ;;
    --layers) LAYERS="$2"; shift 2 ;;
    --lr) LR="$2"; shift 2 ;;
    --data) DATA="$2"; shift 2 ;;
    *) die "unknown option $1" ;;
  esac
done
guard_disk
RUN="$RAI_ROOT/runs/$NAME"
BASE="$RAI_ROOT/models/Qwen3.5-$MODEL"
[ -f "$BASE/config.json" ] || die "no weights at $BASE (run tools/rai_train/setup.sh; --with-4b for 4B)"
mkdir -p "$RUN"

if [ -n "$ROWS" ]; then
  say "slice: $ROWS rows from $DATA ($PROFILES, up to $MAXSEQ tokens)"
  "$RAI_ROOT/venv/bin/python" -I "$HERE/slice_data.py" --data "$DATA" --out "$RUN/data" --rows "$ROWS" --max-tokens "$MAXSEQ" --profiles "$PROFILES"
  DATA="$RUN/data"
fi

say "LoRA on $BASE: $ITERS steps x $BATCH rows, last $LAYERS layers, lr $LR, max $MAXSEQ tokens"
START=$(date +%s)
"$RAI_ROOT/venv/bin/python" -u "$HERE/lora_train.py" \
  --model "$BASE" --train --data "$DATA" \
  --fine-tune-type lora --mask-prompt --grad-checkpoint \
  --num-layers "$LAYERS" --batch-size "$BATCH" --iters "$ITERS" \
  --learning-rate "$LR" --max-seq-length "$MAXSEQ" \
  --steps-per-report 5 --steps-per-eval 25 --val-batches 5 --save-every 25 \
  --adapter-path "$RUN/adapters" --seed 1 2>&1 | tee "$RUN/train.log"
END=$(date +%s)
say "trained in $((END - START)) s: adapters in $RUN/adapters ($(du -sh "$RUN/adapters" | cut -f1))"
