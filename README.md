# openw8s

Open Weights Spec — CLI and registry for standardizing environments, VRAM profiling, and lineage for open-weight AI models.

In plain English: one YAML file describes how an open-weight model should run (VRAM, quant, engine, command, and parent lineage). The CLI inspects Hugging Face models, validates that file, and launches the runtime. The web app is a small registry + inspector UI for local/dev use.

## Install

```bash
# Preferred: download a GitHub Release binary (falls back to cargo)
curl -fsSL https://raw.githubusercontent.com/almanzalex/openw8s/master/scripts/install.sh | bash

# From a clone
bash scripts/install.sh

# Or build from git with Cargo
cargo install --git https://github.com/almanzalex/openw8s --locked openw8s
```

Release archives (when published):

| Platform | Asset |
| --- | --- |
| Linux x86_64 | `openw8s-x86_64-unknown-linux-gnu.tar.gz` |
| macOS Apple Silicon | `openw8s-aarch64-apple-darwin.tar.gz` |

```bash
# Manual binary install example (Apple Silicon)
curl -fsSL -o openw8s.tar.gz \
  https://github.com/almanzalex/openw8s/releases/latest/download/openw8s-aarch64-apple-darwin.tar.gz
tar -xzf openw8s.tar.gz
install -m 755 openw8s-aarch64-apple-darwin/openw8s ~/.cargo/bin/openw8s
```

Pin a version with `OPENW8S_TAG=v0.1.2 bash scripts/install.sh`.

## Quick start

```bash
openw8s inspect Qwen/Qwen2.5-7B-Instruct
openw8s inspect TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF
openw8s init
openw8s validate
openw8s run --dry-run
openw8s run          # writes .openw8s.lock.yml with measured free-memory snapshot
```

Laptop Ollama one-shot helper (CLI or HTTP API fallback):

```bash
bash scripts/ollama-generate.sh llama3.2:3b "What is 2+2?"
```

### Web registry (local)

```bash
bash scripts/sync-catalog.sh   # examples → public/m + generated catalog
cd apps/web && npm install && npm run dev
```

## Contribute a manifest

See [CONTRIBUTING.md](./CONTRIBUTING.md). Short path:

```bash
# add examples/<slug>.openw8s.yml
bash scripts/sync-catalog.sh
./target/release/openw8s validate -m examples/<slug>.openw8s.yml
./target/release/openw8s run -m examples/<slug>.openw8s.yml --dry-run
# open a PR (checklist auto-loads)
```

## Dogfood

```bash
bash scripts/dogfood.sh
```

## Workspace

| Path | Description |
| --- | --- |
| `crates/openw8s-cli` | Rust CLI — `inspect`, `init`, `validate`, `run` |
| `apps/web` | Next.js registry (local/dev) |
| `examples/` | Seed manifests (source of truth) |
| `scripts/sync-catalog.sh` | Sync examples → web catalog |
| `scripts/ollama-generate.sh` | Portable Ollama one-shot |
| `scripts/install.sh` | Release binary or cargo install |
| `.github/workflows/` | CI + Release |

## Manifest schema (v0.1)

Lineage:

```yaml
metadata:
  lineage:
    kind: finetune_of   # base | finetune_of | quantize_of | merge_of
    parent: org/base-model
```

## License

Apache-2.0

See [ROADMAP.md](./ROADMAP.md) for near-term work. Public web deployment is deferred (ways later).
