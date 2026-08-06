import type { SampleManifest } from "@/lib/manifest-types";

export type { LineageKind, SampleManifest } from "@/lib/manifest-types";
export { SAMPLE_MANIFESTS } from "@/data/catalog.generated";

export function manifestToYaml(m: SampleManifest): string {
  const lineage = m.metadata.lineage;
  let lineageBlock = "";
  if (lineage) {
    lineageBlock = `\n  lineage:\n    kind: ${lineage.kind}`;
    if (lineage.parent) {
      lineageBlock += `\n    parent: "${lineage.parent}"`;
    }
  }
  const parent = m.metadata.parent_branch
    ? `\n  parent_branch: "${m.metadata.parent_branch}"`
    : "";
  const evals = m.evals
    ? `\nevals:\n${Object.entries(m.evals)
        .map(([k, v]) => `  ${k}: ${v}`)
        .join("\n")}`
    : "";

  return `version: "${m.version}"
metadata:
  name: "${m.metadata.name}"
  base_model: "${m.metadata.base_model}"${lineageBlock}${parent}
  author: "${m.metadata.author}"
  license: "${m.metadata.license}"

hardware:
  min_vram_gb: ${m.hardware.min_vram_gb}
  recommended_vram_gb: ${m.hardware.recommended_vram_gb}
  quantization: "${m.hardware.quantization}"
  context_length: ${m.hardware.context_length}

runtime:
  engine: "${m.runtime.engine}"
  command: >
    ${m.runtime.command}
${evals}`;
}
