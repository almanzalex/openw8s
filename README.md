# openw8s

Open Weights Spec — CLI and registry for standardizing environments, VRAM profiling, and lineage for open-weight AI models.

In plain English: one YAML file describes how an open-weight model should run (VRAM, quant, engine, command, and parent lineage). The CLI inspects Hugging Face models, validates that file, and launches the runtime. The web app is a small registry + inspector UI.

## Install

```bash
# From this repo
bash scripts/install.sh

# Or with Cargo
cargo install --git https://github.com/almanzalex/openw8s --locked openw8s
```

## Quick start

```bash
openw8s inspect Qwen/Qwen2.5-7B-Instruct
openw8s init
openw8s validate
openw8s run --dry-run
openw8s run          # writes .openw8s.lock.yml with measured free-memory snapshot
```

Laptop Ollama one-shot helper (CLI or HTTP API fallback):

```bash
bash scripts/ollama-generate.sh llama3.2:3b "What is 2+2?"
```

### Web registry

```bash
bash scripts/sync-catalog.sh   # examples → public/m + generated catalog
cd apps/web && npm install && npm run dev
```

## Dogfood

```bash
bash scripts/dogfood.sh
```

## Workspace

| Path | Description |
| --- | --- |
| `crates/openw8s-cli` | Rust CLI — `inspect`, `init`, `validate`, `run` |
| `apps/web` | Next.js registry |
| `examples/` | Seed manifests (source of truth) |
| `scripts/sync-catalog.sh` | Sync examples → web catalog |
| `scripts/ollama-generate.sh` | Portable Ollama one-shot |
| `.github/workflows/ci.yml` | CI |

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
