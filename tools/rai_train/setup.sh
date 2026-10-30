#!/bin/bash
# RAI-TRAIN: sets up the LoRA fine-tuning environment on the external SSD.
# Idempotent: run it as often as you like; every step says what it did or
# that it was already done. Nothing is installed on the internal disk
# except this repo's files.
#
#   tools/rai_train/setup.sh                 everything for the 2B model
#   tools/rai_train/setup.sh --with-4b       also the Qwen3.5-4B weights (only when 2B works)
#   tools/rai_train/setup.sh --no-build      do not (re)build rapidr / ai_eval / rai_check
#   RAI_TRAIN_ROOT=/Volumes/X/rai-train tools/rai_train/setup.sh   another root
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=common.sh
. "$HERE/common.sh"

WITH_4B=0
BUILD=1
for a in "$@"; do
  case "$a" in
    --with-4b) WITH_4B=1 ;;
    --no-build) BUILD=0 ;;
    -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
    *) die "unknown option $a" ;;
  esac
done

DID=()
SKIPPED=()
did() { DID+=("$1"); say "done: $1"; }
skip() { SKIPPED+=("$1"); say "already there: $1"; }

guard_disk
mkdir_tree
START=$(date +%s)
say "root: $RAI_ROOT   (internal disk free: $(df -h / | awk 'NR==2 {print $4}'))"

# ---- 1. the programs the corpus is checked against ---------------------------------
if [ "$BUILD" = 1 ]; then
  say "building rapidr, ai_eval and rai_check into $RAI_ROOT/target (incremental)"
  (cd "$RAI_REPO" && CARGO_TARGET_DIR="$RAI_ROOT/target" cargo build --release -p rapidr-cli 2>&1 | tail -2)
  (cd "$RAI_REPO" && CARGO_TARGET_DIR="$RAI_ROOT/target" cargo build --release -p rapidr-studio --example ai_eval --example rai_check 2>&1 | tail -2)
  for b in rapidr examples/ai_eval examples/rai_check; do
    cp "$RAI_ROOT/target/release/$b" "$RAI_ROOT/bin/$(basename "$b")"
  done
  RAPIDR_VERSION=$("$RAI_ROOT/bin/rapidr" version 2>&1 | head -1)
  echo "$RAPIDR_VERSION  (repo $(cd "$RAI_REPO" && git rev-parse --short HEAD))" > "$RAI_ROOT/meta/rapidr-version.txt"
  did "rapidr, ai_eval, rai_check built: $RAPIDR_VERSION"
else
  [ -x "$RAI_ROOT/bin/rapidr" ] || die "--no-build, but $RAI_ROOT/bin/rapidr is missing"
  skip "rapidr, ai_eval, rai_check (--no-build)"
fi

# ---- 2. Python environments --------------------------------------------------------
PY="${PYTHON:-python3}"
"$PY" -c 'import sys; assert (3,10) <= sys.version_info[:2] <= (3,13), sys.version' \
  || die "Python 3.10 - 3.13 is needed (mlx has no wheel for newer ones yet); set PYTHON=/path/to/python3.13"

make_venv() { # NAME
  if [ ! -x "$RAI_ROOT/$1/bin/python" ]; then
    "$PY" -m venv "$RAI_ROOT/$1"
    did "venv $1 created ($("$RAI_ROOT/$1/bin/python" --version))"
  else
    skip "venv $1"
  fi
}

# pip into a venv, quietly; "done" only when the set of packages changed
pip_into() { # VENV  pip-args...
  local venv="$1"; shift
  local before after
  before=$("$RAI_ROOT/$venv/bin/pip" freeze 2>/dev/null || true)
  "$RAI_ROOT/$venv/bin/pip" install -q "$@" > "$RAI_ROOT/logs/pip-$venv.log" 2>&1 \
    || { tail -15 "$RAI_ROOT/logs/pip-$venv.log"; die "pip install into $venv failed (log: $RAI_ROOT/logs/pip-$venv.log)"; }
  after=$("$RAI_ROOT/$venv/bin/pip" freeze 2>/dev/null)
  echo "$after" > "$RAI_ROOT/meta/freeze-$venv.txt"
  if [ "$before" = "$after" ]; then skip "packages of $venv ($(echo "$after" | wc -l | tr -d ' ') installed)"; else did "packages of $venv installed from PyPI"; fi
}

make_venv venv
LOCK="$HERE/requirements-train.lock.txt"
CONSTRAINT=()
[ -f "$LOCK" ] && CONSTRAINT=(-c "$LOCK")
pip_into venv ${CONSTRAINT[@]+"${CONSTRAINT[@]}"} -r "$HERE/requirements-train.txt"
"$RAI_ROOT/venv/bin/python" - <<'PYEOF'
import importlib.metadata as m
for p in ("mlx", "mlx-lm", "transformers", "huggingface_hub", "numpy", "safetensors"):
    print(f"[rai-train]   {p} {m.version(p)}")
PYEOF

# ---- 3. base model weights ---------------------------------------------------------
fetch_model() { # REPO REV DIRNAME
  local dir="$RAI_ROOT/models/$3"
  if [ -f "$dir/.revision" ] && [ "$(cat "$dir/.revision")" = "$2" ] && [ -f "$dir/LICENSE" ]; then
    skip "model $1 @ ${2:0:8}"
  else
    say "downloading $1 @ $2 to $dir (several GB)"
    "$RAI_ROOT/venv/bin/hf" download "$1" --revision "$2" --local-dir "$dir" > "$RAI_ROOT/logs/download-$3.log" 2>&1 \
      || { tail -5 "$RAI_ROOT/logs/download-$3.log"; die "download of $1 failed"; }
    echo "$2" > "$dir/.revision"
    did "model $1 @ ${2:0:8} downloaded ($(du -sh "$dir" | cut -f1))"
  fi
  # the licence file must be Apache-2.0
  grep -q "Apache License" "$dir/LICENSE" || die "$dir/LICENSE is not the Apache License"
  grep -q "Version 2.0" "$dir/LICENSE" || die "$dir/LICENSE is not Apache 2.0"
}
fetch_model "$MODEL_2B_REPO" "$MODEL_2B_REV" "Qwen3.5-2B"
if [ "$WITH_4B" = 1 ]; then
  [ -n "$MODEL_4B_REV" ] || die "set MODEL_4B_REV in common.sh first (curl https://huggingface.co/api/models/$MODEL_4B_REPO | sha)"
  fetch_model "$MODEL_4B_REPO" "$MODEL_4B_REV" "Qwen3.5-4B"
fi

# ---- 4. llama.cpp (HF -> GGUF converter), pinned tag -------------------------------
LC="$RAI_ROOT/llama.cpp"
if [ -d "$LC/.git" ] && [ "$(git -C "$LC" describe --tags --exact-match 2>/dev/null || true)" = "$LLAMACPP_TAG" ]; then
  skip "llama.cpp $LLAMACPP_TAG"
else
  rm -rf "$LC"
  git clone -q --depth 1 --branch "$LLAMACPP_TAG" https://github.com/ggml-org/llama.cpp "$LC"
  did "llama.cpp $LLAMACPP_TAG cloned ($(du -sh "$LC" | cut -f1)); licence: $(head -1 "$LC/LICENSE")"
fi
make_venv venv-convert
# (PyPI only: the converter's own requirements file names a PyTorch index; the
# PyPI wheel for macOS arm64 is the same CPU build)
pip_into venv-convert -c "$HERE/requirements-convert.lock.txt" -r "$HERE/requirements-convert.txt" "$LC/gguf-py"

# llama-quantize (same tag, MIT): the quantized GGUFs; CPU build, a few minutes the first time
if [ -x "$LC/build/bin/llama-quantize" ]; then
  skip "llama-quantize"
elif command -v cmake > /dev/null; then
  cmake -S "$LC" -B "$LC/build" -DGGML_METAL=OFF -DLLAMA_CURL=OFF -DLLAMA_BUILD_TESTS=OFF -DLLAMA_BUILD_EXAMPLES=OFF -DLLAMA_BUILD_SERVER=OFF -DCMAKE_BUILD_TYPE=Release > "$RAI_ROOT/logs/llama-cmake.log" 2>&1 \
    && cmake --build "$LC/build" --target llama-quantize -j 8 > "$RAI_ROOT/logs/llama-build.log" 2>&1 \
    && did "llama-quantize built" || say "WARNING: llama-quantize did not build (logs/llama-build.log): export_gguf.sh needs it"
else
  say "WARNING: cmake is not installed: no llama-quantize (export_gguf.sh needs it)"
fi

# ---- 5. checks ---------------------------------------------------------------------
"$RAI_ROOT/venv/bin/python" -c 'import mlx.core as mx; print("[rai-train]   mlx device:", mx.default_device())'
"$RAI_ROOT/venv/bin/python" -m mlx_lm lora --help > /dev/null && say "mlx_lm lora is available"
"$RAI_ROOT/venv-convert/bin/python" -c 'import torch, transformers, gguf; print("[rai-train]   convert env: torch", torch.__version__, "transformers", transformers.__version__)'
if [ -x "$(command -v ollama)" ]; then say "ollama: $(ollama --version 2>&1 | tail -1)"; else say "WARNING: ollama is not installed (export_gguf.sh needs it)"; fi

END=$(date +%s)
echo
say "finished in $((END - START)) s; internal disk free: $(df -h / | awk 'NR==2 {print $4}'); $RAI_ROOT uses $(du -sh "$RAI_ROOT" 2>/dev/null | cut -f1)"
say "did ${#DID[@]} step(s), found ${#SKIPPED[@]} already done"
