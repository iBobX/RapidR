#!/bin/bash
# RAI-TRAIN: measures a model Ollama serves with the eval harness (tools/ai_eval), the same
# apps, the same click tests, the same scoring as every other model in dist/ai-eval-*/report.md.
#
#   tools/rai_train/eval_lora.sh OLLAMA_NAME [--driver A|B|AB] [--set apps,held-out,school]
#                                [--apps tip-calculator,...] [--rep N] [--num-ctx 32768] [--out DIR]
#
#   driver A = an outside AI app through Studio's MCP server; B = Studio's own RAI.
#   Compare with the base model on the same apps (A:ollama/qwen3.5:2b-q4_K_M) and never train on
#   the eval's prompts (tools/rai_train/validate_corpus.py checks the corpus).
# Studio runs headless in a folder of its own with a registry, keychain, printer and documents
# folder of its own; nothing reaches the user's. The model is unloaded at the end.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=common.sh
. "$HERE/common.sh"

[ $# -ge 1 ] || { sed -n '2,14p' "$0"; exit 1; }
ONAME="$1"; shift
DRIVER=A; SET=apps; APPS=""; REP=1; NUMCTX=32768; OUT=""
while [ $# -gt 0 ]; do
  case "$1" in
    --driver) DRIVER="$2"; shift 2 ;;
    --set) SET="$2"; shift 2 ;;
    --apps) APPS="$2"; shift 2 ;;
    --rep) REP="$2"; shift 2 ;;
    --num-ctx) NUMCTX="$2"; shift 2 ;;
    --out) OUT="$2"; shift 2 ;;
    *) die "unknown option $1" ;;
  esac
done
guard_disk
[ -x "$RAI_ROOT/bin/rapidr" ] && [ -x "$RAI_ROOT/bin/ai_eval" ] || die "bin/rapidr or bin/ai_eval missing: run tools/rai_train/setup.sh"
SAFE=$(echo "$ONAME" | tr '/:' '__')
OUT="${OUT:-$RAI_ROOT/runs/eval-$SAFE}"
mkdir -p "$OUT"
CELLS=""
case "$DRIVER" in
  A) CELLS="A:ollama/$ONAME" ;;
  B) CELLS="B:ollama/$ONAME" ;;
  AB) CELLS="A:ollama/$ONAME,B:ollama/$ONAME" ;;
  *) die "--driver is A, B or AB" ;;
esac
EXTRA=()
[ -n "$APPS" ] && EXTRA+=(--apps "$APPS")
say "evaluating $ONAME: cells $CELLS, set $SET${APPS:+, apps $APPS}, num_ctx $NUMCTX -> $OUT"
START=$(date +%s)
cd "$RAI_REPO"
RAPIDR="$RAI_ROOT/bin/rapidr" RAPIDR_AI_EVAL_BIN="$RAI_ROOT/bin/ai_eval" RAPIDR_AI_KEYSTORE=memory \
  RAPIDR_REGISTRY="$OUT/registry.reg" RAPIDR_PRINT_TO="$OUT/prints" \
  node tools/ai_eval/run_matrix.mjs --out "$OUT/runs" --cells "$CELLS" --set "$SET" --rep "$REP" --num-ctx "$NUMCTX" --unload ${EXTRA[@]+"${EXTRA[@]}"} 2>&1 | tee "$OUT/eval.log"
END=$(date +%s)
say "evaluated in $((END - START)) s"
node tools/ai_eval/analyse.mjs "$OUT/runs" 2>&1 | tee "$OUT/tables.txt" | head -40
ollama stop "$ONAME" > /dev/null 2>&1 || true
