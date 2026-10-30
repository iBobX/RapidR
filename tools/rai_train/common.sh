#!/bin/bash
# Shared settings of the RAI-TRAIN scripts (sourced, never run).
#
# Everything big lives on the external SSD, never on the internal disk:
#   $RAI_ROOT/venv          Python env: mlx, mlx-lm (training)
#   $RAI_ROOT/venv-convert  Python env: llama.cpp's converter (torch, transformers 4.57)
#   $RAI_ROOT/models        base model weights (Hugging Face snapshots)
#   $RAI_ROOT/llama.cpp     llama.cpp at a pinned tag (the HF -> GGUF converter)
#   $RAI_ROOT/target, bin   the rapidr + ai_eval + rai_check this corpus is built against
#   $RAI_ROOT/data          train.jsonl, valid.jsonl, provenance.jsonl, check cache
#   $RAI_ROOT/runs          LoRA adapters, fused models, GGUF files, eval outputs
#   $RAI_ROOT/logs, hf, pip-cache, cache, tmp   logs and every tool's cache

RAI_ROOT="${RAI_TRAIN_ROOT:-/Volumes/RapidRBuild/rai-train}"
RAI_REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# ---- pins (change them here, nowhere else) ---------------------------------
# The official Qwen repo of the post-trained 2B (Apache-2.0; there is no
# "-Instruct" suffix: the "-Base" repos are the pre-trained ones).
MODEL_2B_REPO="Qwen/Qwen3.5-2B"
MODEL_2B_REV="15852e8c16360a2fea060d615a32b45270f8a8fc"
MODEL_4B_REPO="Qwen/Qwen3.5-4B"
MODEL_4B_REV="851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a"   # fetched only by setup.sh --with-4b
LLAMACPP_TAG="b11981"    # ggml-org/llama.cpp (MIT)

# ---- environment: every cache on the SSD -----------------------------------
export HF_HOME="$RAI_ROOT/hf"
export HF_HUB_DISABLE_TELEMETRY=1
export PIP_CACHE_DIR="$RAI_ROOT/pip-cache"
export PIP_DISABLE_PIP_VERSION_CHECK=1
export TMPDIR="$RAI_ROOT/tmp"
export XDG_CACHE_HOME="$RAI_ROOT/cache"
export PYTHONDONTWRITEBYTECODE=1
export TOKENIZERS_PARALLELISM=false

say() { printf '[rai-train] %s\n' "$*"; }
die() { printf '[rai-train] ERROR: %s\n' "$*" >&2; exit 1; }

# The internal disk must keep >= 20 GB free; the root must be on the SSD.
guard_disk() {
  case "$RAI_ROOT" in
    /Volumes/*) ;;
    *) [ "${RAI_TRAIN_ALLOW_INTERNAL:-}" = 1 ] || die "RAI_ROOT=$RAI_ROOT is not on an external volume (set RAI_TRAIN_ALLOW_INTERNAL=1 to override)" ;;
  esac
  [ -d "$(dirname "$RAI_ROOT")" ] || die "the SSD is not mounted: $(dirname "$RAI_ROOT")"
  local free_kb
  free_kb=$(df -k / | awk 'NR==2 {print $4}')
  if [ "$free_kb" -lt $((20 * 1024 * 1024)) ]; then
    die "the internal disk has only $((free_kb / 1024 / 1024)) GB free (< 20 GB): stopping"
  fi
}

mkdir_tree() {
  mkdir -p "$RAI_ROOT"/{logs,bin,data,runs,models,hf,pip-cache,cache,tmp,meta}
}
