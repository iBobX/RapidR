#!/bin/bash
# RAI-TRAIN: how big is a model at each quantization llama.cpp's llama-quantize offers?
# Sizes only (the quality of the small ones is not measured: a 2B model loses a lot below Q4).
#
#   tools/rai_train/measure_quants.sh RUN OLLAMA_NAME [TYPES...]
#
# RUN's f16 GGUF (export_gguf.sh made it) is quantized to each type in a scratch folder under the
# run; the files are deleted after their size is read.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=common.sh
. "$HERE/common.sh"
[ $# -ge 2 ] || { sed -n '2,10p' "$0"; exit 1; }
RUN="$RAI_ROOT/runs/$1"; ONAME="$2"; shift 2
TYPES=("$@")
[ ${#TYPES[@]} -gt 0 ] || TYPES=(Q8_0 Q6_K Q5_K_M Q4_K_M Q4_K_S IQ4_XS IQ4_NL Q3_K_L Q3_K_M Q3_K_S IQ3_M IQ3_S IQ3_XS Q2_K)
guard_disk
F16="$RUN/$ONAME-f16.gguf"
[ -f "$F16" ] || die "no $F16 (run export_gguf.sh first)"
Q="$RAI_ROOT/llama.cpp/build/bin/llama-quantize"
mkdir -p "$RUN/quants"
printf '| type | size | note |\n|---|---:|---|\n'
printf '| f16 | %s | the export |\n' "$(du -h "$F16" | cut -f1)"
for t in "${TYPES[@]}"; do
  out="$RUN/quants/$ONAME-$t.gguf"
  if "$Q" "$F16" "$out" "$t" > "$RUN/quants/$t.log" 2>&1; then
    printf '| %s | %s | |\n' "$t" "$(du -h "$out" | cut -f1)"
    rm -f "$out"
  else
    printf '| %s | - | %s |\n' "$t" "$(grep -m1 -iE 'imatrix|importance|error|not supported' "$RUN/quants/$t.log" | cut -c1-90)"
  fi
done
