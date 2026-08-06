#!/usr/bin/env bash
# Install openw8s CLI (requires Rust / cargo).
set -euo pipefail

REPO_URL="${OPENW8S_REPO:-https://github.com/almanzalex/openw8s}"
BIN_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing required command: $1" >&2
    exit 1
  fi
}

echo "==> openw8s installer"

if ! command -v cargo >/dev/null 2>&1; then
  echo "==> Rust not found — installing via rustup (https://rustup.rs)"
  need_cmd curl
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
  # shellcheck disable=SC1091
  source "${CARGO_HOME:-$HOME/.cargo}/env"
fi

need_cmd cargo

ROOT="$(cd "$(dirname "$0")/.." 2>/dev/null && pwd || true)"
if [[ -n "${ROOT}" && -f "${ROOT}/crates/openw8s-cli/Cargo.toml" ]]; then
  echo "==> Installing from local checkout: ${ROOT}"
  cargo install --path "${ROOT}/crates/openw8s-cli" --force
else
  echo "==> Installing from ${REPO_URL}"
  cargo install --git "${REPO_URL}" --locked openw8s --force
fi

echo
echo "✓ installed: $(command -v openw8s || echo "${BIN_DIR}/openw8s")"
openw8s --version || true
echo
echo "Try:"
echo "  openw8s inspect Qwen/Qwen2.5-7B-Instruct"
echo "  openw8s init"
echo "  openw8s validate && openw8s run --dry-run"
