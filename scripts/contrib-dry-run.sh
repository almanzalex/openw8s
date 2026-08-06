#!/usr/bin/env bash
# Dry-run the community manifest contribution path against all seed examples.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "==> sync catalog"
bash scripts/sync-catalog.sh >/dev/null

if [[ -x "$ROOT/target/release/openw8s" ]]; then
  OW="$ROOT/target/release/openw8s"
else
  source "${CARGO_HOME:-$HOME/.cargo}/env" 2>/dev/null || true
  export CARGO_TARGET_DIR="$ROOT/target"
  cargo build -p openw8s --release
  OW="$ROOT/target/release/openw8s"
fi

echo "==> validate + dry-run every example (PR checklist)"
pass=0
fail=0
for f in examples/*.openw8s.yml; do
  slug="$(basename "$f" .openw8s.yml)"
  echo "---- ${slug}"
  if "$OW" validate -m "$f" >/dev/null \
    && "$OW" run -m "$f" --dry-run >/dev/null; then
    echo "  OK"
    pass=$((pass + 1))
  else
    echo "  FAIL" >&2
    fail=$((fail + 1))
  fi
done

# Ensure synced public copies exist
missing=0
for f in examples/*.openw8s.yml; do
  slug="$(basename "$f" .openw8s.yml)"
  if [[ ! -f "apps/web/public/m/${slug}.yml" ]]; then
    echo "missing public copy: apps/web/public/m/${slug}.yml" >&2
    missing=$((missing + 1))
  fi
done

echo
echo "contrib dry-run: ${pass} passed, ${fail} failed, ${missing} missing public copies"
if [[ "$fail" -gt 0 || "$missing" -gt 0 ]]; then
  exit 1
fi
