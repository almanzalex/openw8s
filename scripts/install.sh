#!/usr/bin/env bash
# Install openw8s CLI from a GitHub Release binary, or fall back to cargo.
set -euo pipefail

REPO="${OPENW8S_REPO:-almanzalex/openw8s}"
REPO_URL="${OPENW8S_GIT_URL:-https://github.com/${REPO}}"
BIN_DIR="${OPENW8S_BIN_DIR:-${CARGO_HOME:-$HOME/.cargo}/bin}"
TAG="${OPENW8S_TAG:-latest}"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: missing required command: $1" >&2
    exit 1
  fi
}

detect_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "${os}-${arch}" in
    Linux-x86_64) echo "x86_64-unknown-linux-gnu" ;;
    Darwin-arm64) echo "aarch64-apple-darwin" ;;
    Darwin-x86_64) echo "x86_64-apple-darwin" ;;
    *)
      echo ""
      ;;
  esac
}

install_from_release() {
  need_cmd curl
  need_cmd tar
  local target archive url tmp
  target="$(detect_target)"
  if [[ -z "${target}" ]]; then
    return 1
  fi
  # Intel mac binaries may not ship yet — signal fallback.
  if [[ "${target}" == "x86_64-apple-darwin" ]]; then
    echo "==> no Intel mac release artifact yet; falling back to cargo" >&2
    return 1
  fi
  archive="openw8s-${target}"
  if [[ "${TAG}" == "latest" ]]; then
    url="https://github.com/${REPO}/releases/latest/download/${archive}.tar.gz"
  else
    url="https://github.com/${REPO}/releases/download/${TAG}/${archive}.tar.gz"
  fi
  echo "==> downloading ${url}"
  tmp="$(mktemp -d)"
  if ! curl -fsSL "${url}" -o "${tmp}/openw8s.tar.gz"; then
    echo "==> release download failed; falling back to cargo" >&2
    rm -rf "${tmp}"
    return 1
  fi
  tar -xzf "${tmp}/openw8s.tar.gz" -C "${tmp}"
  mkdir -p "${BIN_DIR}"
  install -m 755 "${tmp}/${archive}/openw8s" "${BIN_DIR}/openw8s"
  rm -rf "${tmp}"
  echo "✓ installed ${BIN_DIR}/openw8s"
  "${BIN_DIR}/openw8s" --version || true
  return 0
}

install_from_cargo() {
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
}

echo "==> openw8s installer"
if install_from_release; then
  echo
  echo "Try:"
  echo "  openw8s inspect Qwen/Qwen2.5-7B-Instruct"
  exit 0
fi

install_from_cargo
echo
echo "Try:"
echo "  openw8s inspect Qwen/Qwen2.5-7B-Instruct"
echo "  openw8s init"
echo "  openw8s validate && openw8s run --dry-run"
