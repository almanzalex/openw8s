#!/usr/bin/env bash
# Portable one-shot Ollama generate for openw8s manifests.
# Prefers `ollama run` when the CLI can talk to a server; otherwise uses the HTTP API.
set -euo pipefail

MODEL="${1:?model tag required, e.g. llama3.2:3b}"
PROMPT="${2:?prompt required}"
HOST="${OLLAMA_HOST:-127.0.0.1:11434}"
HOST="${HOST#http://}"
HOST="${HOST#https://}"
BASE="http://${HOST}"

ensure_server() {
  if curl -sf "${BASE}/api/tags" >/dev/null; then
    return 0
  fi
  if ! command -v ollama >/dev/null 2>&1; then
    echo "error: ollama not installed and no server at ${BASE}" >&2
    exit 1
  fi
  nohup ollama serve >/tmp/ollama-serve.log 2>&1 &
  for _ in $(seq 1 40); do
    if curl -sf "${BASE}/api/tags" >/dev/null; then
      return 0
    fi
    sleep 0.25
  done
  echo "error: could not start ollama serve" >&2
  exit 1
}

ensure_server

# Prefer CLI when it works (Mac app / healthy client).
if command -v ollama >/dev/null 2>&1; then
  if OLLAMA_HOST="${HOST}" ollama list >/dev/null 2>&1; then
    OLLAMA_HOST="${HOST}" ollama run "${MODEL}" "${PROMPT}"
    exit 0
  fi
fi

# Fallback: HTTP API (Homebrew-only installs often need this).
curl -sf "${BASE}/api/generate" \
  -H "Content-Type: application/json" \
  -d "$(python3 -c "import json,sys; print(json.dumps({'model':sys.argv[1],'prompt':sys.argv[2],'stream':False}))" "$MODEL" "$PROMPT")" \
  | python3 -c 'import sys,json; print(json.load(sys.stdin)["response"].strip())'
