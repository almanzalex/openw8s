# Contributing to openw8s

Thanks for helping standardize open-weight environments.

## Add a manifest (community PR path)

1. Fork and branch from `master`
2. Create `examples/<slug>.openw8s.yml` (see existing examples)
3. Sync the web catalog:

```bash
bash scripts/sync-catalog.sh
```

4. Validate locally:

```bash
cargo build -p openw8s --release
./target/release/openw8s validate -m examples/<slug>.openw8s.yml
./target/release/openw8s run -m examples/<slug>.openw8s.yml --dry-run

# Or run the full seed checklist:
bash scripts/contrib-dry-run.sh
```

5. Open a PR — the template checklist will appear automatically

### Schema reminders

- `metadata.lineage.kind`: `base` | `finetune_of` | `quantize_of` | `merge_of`
- `runtime.engine`: `vllm` | `ollama` | `llamacpp` | `docker`
- Prefer `scripts/ollama-generate.sh` for laptop/Ollama one-shots

## CLI changes

```bash
cargo test
cargo clippy -- -D warnings
bash scripts/dogfood.sh   # network; optional HF_TOKEN for gated models
```

## Scope notes

- Public website deployment is **ways later** — do not block PRs on hosting
- Keep manifests honest about VRAM mins; dry-run must pass
