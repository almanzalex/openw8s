# openw8s

**One file. One command. Your open-weight model, ready to run.**

`openw8s` is a small CLI that standardizes how open-weight models get inspected, described, and launched. You write a short `.openw8s.yml` — VRAM needs, quantization, engine, lineage, and the exact command — then let the tool validate it and run it.

```bash
openw8s inspect Qwen/Qwen2.5-7B-Instruct
openw8s init
openw8s validate
openw8s run
```

## Install

**One-liner** (GitHub Release binary, falls back to Cargo):

```bash
curl -fsSL https://raw.githubusercontent.com/almanzalex/openw8s/master/scripts/install.sh | bash
```

**Homebrew:**

```bash
brew tap almanzalex/openw8s
brew install openw8s
```

**From Cargo:**

```bash
cargo install --git https://github.com/almanzalex/openw8s --locked openw8s
```

Pin a release with `OPENW8S_TAG=v0.1.3 bash scripts/install.sh`.

Supported release binaries: Linux x86_64, macOS Apple Silicon, macOS Intel.

## What you get

| Command | What it does |
| --- | --- |
| `inspect` | Peek at a Hugging Face model (no weight download) — size, files, GGUF metadata |
| `init` | Scaffold a `.openw8s.yml` for your environment |
| `validate` | Check the manifest against the v0.1 schema |
| `run` | Preflight VRAM / unified memory, then launch the configured command |

After a successful `run`, openw8s writes `.openw8s.lock.yml` with a measured free-memory snapshot.

## A manifest looks like this

```yaml
version: "0.1"
metadata:
  name: "Llama-3.2-3B-Ollama-Laptop"
  base_model: "meta-llama/Llama-3.2-3B-Instruct"
  lineage:
    kind: quantize_of   # base | finetune_of | quantize_of | merge_of
    parent: "meta-llama/Llama-3.2-3B-Instruct"
  author: "you"
  license: "llama3.2"

hardware:
  min_vram_gb: 2
  recommended_vram_gb: 4
  quantization: "GGUF-Q4_K_M"
  context_length: 8192

runtime:
  engine: "ollama"      # vllm | ollama | llamacpp | docker
  command: >
    bash scripts/ollama-generate.sh llama3.2:3b
    "What is 2+2? Reply with only the number."
```

Try an included example:

```bash
openw8s validate -m examples/llama32-3b-ollama-laptop.openw8s.yml
openw8s run -m examples/llama32-3b-ollama-laptop.openw8s.yml --dry-run
```

On a Mac with Ollama installed, the helper script talks to the local API:

```bash
bash scripts/ollama-generate.sh llama3.2:3b "What is 2+2?"
```

## Local registry UI (optional)

Browse the example catalog and inspect lineage in a small Next.js app:

```bash
bash scripts/sync-catalog.sh
cd apps/web && npm install && npm run dev
```

## Contribute an environment

Have a setup that works on your machine? Add `examples/<slug>.openw8s.yml`, sync the catalog, validate, dry-run, and open a PR. Full checklist in [CONTRIBUTING.md](./CONTRIBUTING.md).

```bash
bash scripts/sync-catalog.sh
openw8s validate -m examples/<slug>.openw8s.yml
openw8s run -m examples/<slug>.openw8s.yml --dry-run
```

## License

Apache-2.0

Near-term ideas live in [ROADMAP.md](./ROADMAP.md).
