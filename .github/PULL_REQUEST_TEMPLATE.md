## Manifest submission

Thanks for contributing a `.openw8s.yml` environment.

### Checklist
- [ ] Added `examples/<slug>.openw8s.yml`
- [ ] Ran `bash scripts/sync-catalog.sh`
- [ ] Ran `openw8s validate -m examples/<slug>.openw8s.yml`
- [ ] Ran `openw8s run -m examples/<slug>.openw8s.yml --dry-run`
- [ ] Lineage `kind` is one of: `base`, `finetune_of`, `quantize_of`, `merge_of`
- [ ] `runtime.engine` is one of: `vllm`, `ollama`, `llamacpp`, `docker`
- [ ] Command is copy-paste runnable (or uses `scripts/ollama-generate.sh`)

### Notes
<!-- Why this environment, hardware target, and any caveats -->
