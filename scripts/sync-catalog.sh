#!/usr/bin/env bash
# Sync examples/*.openw8s.yml → apps/web/public/m + generated catalog JSON/TS.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXAMPLES="$ROOT/examples"
PUBLIC_M="$ROOT/apps/web/public/m"
OUT_JSON="$ROOT/apps/web/src/data/catalog.json"
OUT_TS="$ROOT/apps/web/src/data/catalog.generated.ts"

mkdir -p "$PUBLIC_M" "$(dirname "$OUT_JSON")"

python3 - <<'PY' "$EXAMPLES" "$PUBLIC_M" "$OUT_JSON" "$OUT_TS"
import json, re, sys
from pathlib import Path

examples_dir, public_m, out_json, out_ts = map(Path, sys.argv[1:])
public_m.mkdir(parents=True, exist_ok=True)

def parse_simple_yaml(text: str) -> dict:
    """Minimal YAML subset parser for our seed manifests (no external deps)."""
    # Strip comments
    lines = []
    for raw in text.splitlines():
        if raw.strip().startswith("#"):
            continue
        lines.append(raw)
    text = "\n".join(lines)

    def parse_value(v: str):
        v = v.strip()
        if v.startswith(">") or v.startswith("|"):
            return ""  # folded block handled specially
        if (v.startswith('"') and v.endswith('"')) or (v.startswith("'") and v.endswith("'")):
            return v[1:-1]
        if re.fullmatch(r"-?\d+\.\d+", v):
            return float(v)
        if re.fullmatch(r"-?\d+", v):
            return int(v)
        if v in ("true", "false"):
            return v == "true"
        return v

    root: dict = {}
    stack = [( -1, root )]
    pending_folded = None  # (dict, key)

    i = 0
    raw_lines = text.splitlines()
    while i < len(raw_lines):
        line = raw_lines[i]
        if not line.strip():
            i += 1
            continue
        indent = len(line) - len(line.lstrip(" "))
        stripped = line.strip()

        # Close stack
        while stack and indent < stack[-1][0]:
            stack.pop()
        parent = stack[-1][1]

        if pending_folded is not None:
            target, key = pending_folded
            if indent > pending_folded_indent:
                chunk = stripped
                prev = target.get(key, "")
                target[key] = (prev + " " + chunk).strip() if prev else chunk
                i += 1
                continue
            else:
                pending_folded = None

        if stripped.endswith(">") or stripped.endswith("|"):
            key = stripped.split(":", 1)[0].strip()
            parent[key] = ""
            pending_folded = (parent, key)
            pending_folded_indent = indent
            i += 1
            continue

        if ":" in stripped:
            key, rest = stripped.split(":", 1)
            key = key.strip()
            rest = rest.strip()
            if rest == "":
                child = {}
                parent[key] = child
                stack.append((indent + 2, child))
            else:
                parent[key] = parse_value(rest)
        i += 1

    return root

# Fix folded-block tracking
# Re-implement more carefully below

def load_manifest(path: Path) -> dict:
    text = path.read_text()
    # Prefer PyYAML if present
    try:
        import yaml  # type: ignore
        data = yaml.safe_load(text)
        if not isinstance(data, dict):
            raise ValueError("root must be mapping")
        return data
    except Exception:
        pass

    # Fallback: regex field extraction good enough for our seed files
    def grab(pattern, default=None, flags=0):
        m = re.search(pattern, text, flags)
        return m.group(1).strip() if m else default

    def grab_num(pattern, default=None):
        v = grab(pattern, None)
        if v is None:
            return default
        return float(v) if "." in v else int(v)

    lineage_kind = grab(r"lineage:\s*\n\s*kind:\s*(\w+)")
    lineage_parent = grab(r"lineage:\s*\n\s*kind:\s*\w+\s*\n\s*parent:\s*\"([^\"]+)\"")
    if lineage_kind is None:
        lineage_kind = grab(r"kind:\s*(\w+)")
        lineage_parent = grab(r"parent:\s*\"([^\"]+)\"")

    # command: fold
    cmd = grab(r"command:\s*>\s*\n((?:\s+.+\n?)+)", flags=re.M)
    if cmd:
        cmd = " ".join(x.strip() for x in cmd.splitlines() if x.strip())
    else:
        cmd = grab(r'command:\s*"([^"]+)"') or grab(r"command:\s*'([^']+)'") or ""

    evals = {}
    em = re.search(r"evals:\s*\n((?:\s+\w+:\s*[0-9.]+\s*\n?)+)", text)
    if em:
        for line in em.group(1).splitlines():
            mm = re.match(r"\s*(\w+):\s*([0-9.]+)", line)
            if mm:
                evals[mm.group(1)] = float(mm.group(2))

    meta = {
        "name": grab(r'name:\s*"([^"]+)"') or "",
        "base_model": grab(r'base_model:\s*"([^"]+)"') or "",
        "author": grab(r'author:\s*"([^"]+)"') or "",
        "license": grab(r'license:\s*"([^"]+)"') or "",
    }
    parent_branch = grab(r'parent_branch:\s*"([^"]+)"')
    if parent_branch:
        meta["parent_branch"] = parent_branch
    if lineage_kind:
        lin = {"kind": lineage_kind}
        if lineage_parent:
            lin["parent"] = lineage_parent
        meta["lineage"] = lin

    out = {
        "version": grab(r'version:\s*"([^"]+)"') or "0.1",
        "metadata": meta,
        "hardware": {
            "min_vram_gb": grab_num(r"min_vram_gb:\s*([0-9.]+)", 0),
            "recommended_vram_gb": grab_num(r"recommended_vram_gb:\s*([0-9.]+)", 0),
            "quantization": grab(r'quantization:\s*"([^"]+)"') or "",
            "context_length": grab_num(r"context_length:\s*([0-9]+)", 0),
        },
        "runtime": {
            "engine": grab(r'engine:\s*"([^"]+)"') or "",
            "command": cmd,
        },
    }
    if evals:
        out["evals"] = evals
    return out

catalog = []
for path in sorted(examples_dir.glob("*.openw8s.yml")):
    slug = path.name.removesuffix(".openw8s.yml")
    data = load_manifest(path)
    data["slug"] = slug
    # copy raw yaml to public
    dest = public_m / f"{slug}.yml"
    dest.write_text(path.read_text())
    catalog.append(data)
    print(f"synced {slug}")

out_json.write_text(json.dumps(catalog, indent=2) + "\n")
ts = (
    "/* Generated by scripts/sync-catalog.sh — do not edit by hand. */\n"
    "import type { SampleManifest } from \"@/lib/manifest-types\";\n\n"
    "export const SAMPLE_MANIFESTS = "
    + json.dumps(catalog, indent=2)
    + " as SampleManifest[];\n"
)
# Make it valid TS: JSON true/false/null already fine; ensure trailing
out_ts.write_text(ts)
print(f"wrote {out_json}")
print(f"wrote {out_ts}")
print(f"{len(catalog)} manifests")
PY
