#!/bin/bash
# RAI-TRAIN: a trained adapter -> a model Ollama runs.
#   merge the LoRA into the Hugging Face weights (merge_lora.py)
#   -> llama.cpp's converter (a pinned tag) makes an f16 GGUF
#   -> llama-quantize (built by setup.sh from the same llama.cpp tag) makes the quantized GGUF
#   -> `ollama create` registers it.
#
#   tools/rai_train/export_gguf.sh RUN OLLAMA_NAME [--model 2B|4B] [--quant Q4_K_M] [--no-verify]
#
# RUN is a folder under $RAI_ROOT/runs with adapters/ (train_lora.sh made it).
# Nothing is published: the model lives in the local Ollama (whose folder is on the SSD).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=common.sh
. "$HERE/common.sh"

[ $# -ge 2 ] || { sed -n '2,12p' "$0"; exit 1; }
RUNNAME="$1"; ONAME="$2"; shift 2
MODEL=2B; QUANT=Q4_K_M; VERIFY=--verify
while [ $# -gt 0 ]; do
  case "$1" in
    --model) MODEL="$2"; shift 2 ;;
    --quant) QUANT="$(echo "$2" | tr a-z A-Z)"; shift 2 ;;
    --no-verify) VERIFY=""; shift ;;
    *) die "unknown option $1" ;;
  esac
done
guard_disk
RUN="$RAI_ROOT/runs/$RUNNAME"
BASE="$RAI_ROOT/models/Qwen3.5-$MODEL"
[ -f "$RUN/adapters/adapters.safetensors" ] || die "no adapters in $RUN/adapters (run train_lora.sh first)"
command -v ollama > /dev/null || die "ollama is not installed"

T0=$(date +%s)
if [ "${RAI_REUSE:-0}" = 1 ] && [ -f "$RUN/$ONAME-f16.gguf" ]; then
  # (RAI_REUSE=1: another quantization of a run already merged and converted)
  say "1-2/3 reusing $RUN/$ONAME-f16.gguf"
  T1=$T0; T2=$T0
else
  say "1/3 merging the adapter into the base weights"
  "$RAI_ROOT/venv/bin/python" -u "$HERE/merge_lora.py" --base "$BASE" --adapters "$RUN/adapters" --out "$RUN/merged" $VERIFY 2>&1 | grep -v "^wrote "
  T1=$(date +%s)
  say "merged in $((T1 - T0)) s ($(du -sh "$RUN/merged" | cut -f1))"

  say "2/3 converting to GGUF (llama.cpp $LLAMACPP_TAG)"
  "$RAI_ROOT/venv-convert/bin/python" "$RAI_ROOT/llama.cpp/convert_hf_to_gguf.py" "$RUN/merged" --outfile "$RUN/$ONAME-f16.gguf" --outtype f16 > "$RUN/convert.log" 2>&1 \
    || { tail -15 "$RUN/convert.log"; die "the converter failed (log: $RUN/convert.log)"; }
  T2=$(date +%s)
  say "converted in $((T2 - T1)) s: $RUN/$ONAME-f16.gguf ($(du -h "$RUN/$ONAME-f16.gguf" | cut -f1))"
fi

say "3/3 quantizing to $QUANT and registering in ollama"
QUANTIZE="$RAI_ROOT/llama.cpp/build/bin/llama-quantize"
[ -x "$QUANTIZE" ] || die "llama-quantize is missing: run tools/rai_train/setup.sh (it needs cmake)"
GGUF="$RUN/$ONAME-$QUANT.gguf"
"$QUANTIZE" "$RUN/$ONAME-f16.gguf" "$GGUF" "$QUANT" > "$RUN/quantize.log" 2>&1 || { tail -10 "$RUN/quantize.log"; die "llama-quantize failed"; }
say "quantized: $(du -h "$GGUF" | cut -f1)"
# the chat format, tool-call parser and sampling of the base model's own Ollama entry
{
  echo "FROM $GGUF"
  echo 'TEMPLATE {{ .Prompt }}'
  echo 'RENDERER qwen3.5'
  echo 'PARSER qwen3.5'
  echo 'PARAMETER top_k 20'
  echo 'PARAMETER top_p 0.95'
  echo 'PARAMETER temperature 0.7'
  echo 'PARAMETER presence_penalty 1.5'
  echo 'LICENSE """'
  cat "$BASE/LICENSE"
  echo '"""'
} > "$RUN/Modelfile"
ollama create "$ONAME" -f "$RUN/Modelfile" > "$RUN/ollama-create.log" 2>&1 || { tail -10 "$RUN/ollama-create.log"; die "ollama create failed"; }
T3=$(date +%s)
say "created in $((T3 - T2)) s"
ollama list | grep -E "^(NAME|$ONAME)" || true
# (the f16 and quantized files stay in the run's folder; Ollama keeps its own copy)
say "done in $((T3 - T0)) s. Unload it when finished: ollama stop $ONAME"
