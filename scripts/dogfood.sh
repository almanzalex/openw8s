#!/usr/bin/env bash
# Dogfood openw8s inspect + validate against real Hugging Face models / seed manifests.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ -x "$ROOT/target/release/openw8s" ]]; then
  OW="$ROOT/target/release/openw8s"
elif command -v openw8s >/dev/null 2>&1; then
  OW="$(command -v openw8s)"
else
  echo "==> building release CLI"
  source "${CARGO_HOME:-$HOME/.cargo}/env" 2>/dev/null || true
  export CARGO_TARGET_DIR="$ROOT/target"
  cargo build -p openw8s --release
  OW="$ROOT/target/release/openw8s"
fi

MODELS=(
  "Qwen/Qwen2.5-7B-Instruct"
  "Qwen/Qwen2.5-14B-Instruct"
  "Qwen/Qwen2.5-32B-Instruct"
  "mistralai/Mistral-Nemo-Instruct-2407"
  "microsoft/Phi-3.5-mini-instruct"
  "TinyLlama/TinyLlama-1.1B-Chat-v1.0"
  "HuggingFaceH4/zephyr-7b-beta"
  "google/gemma-2-9b-it"
  # GGUF weight dump (validates non-safetensors inspect path)
  "TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF"
)

echo "==> using CLI: $OW"
echo "==> $($OW --version)"
echo

pass=0
fail=0

echo "==> validate seed manifests"
for f in examples/*.openw8s.yml; do
  if "$OW" validate -m "$f"; then
    pass=$((pass + 1))
  else
    echo "FAIL validate $f" >&2
    fail=$((fail + 1))
  fi
done

echo
echo "==> inspect real HF models"
for repo in "${MODELS[@]}"; do
  echo "---- inspect $repo"
  if "$OW" inspect "$repo"; then
    pass=$((pass + 1))
  else
    echo "FAIL inspect $repo (gated models may need HF_TOKEN)" >&2
    fail=$((fail + 1))
  fi
  echo
done

echo "==> dry-run seed manifests"
for f in examples/*.openw8s.yml; do
  echo "---- run --dry-run $f"
  if "$OW" run -m "$f" --dry-run; then
    pass=$((pass + 1))
  else
    echo "FAIL dry-run $f" >&2
    fail=$((fail + 1))
  fi
done

echo
echo "dogfood summary: ${pass} passed, ${fail} failed"
if [[ "$fail" -gt 0 ]]; then
  exit 1
fi
