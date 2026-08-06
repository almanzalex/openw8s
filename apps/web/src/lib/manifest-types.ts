export type LineageKind = "base" | "finetune_of" | "quantize_of" | "merge_of";

export interface SampleManifest {
  slug: string;
  version: string;
  metadata: {
    name: string;
    base_model: string;
    lineage?: {
      kind: LineageKind;
      parent?: string;
    };
    /** @deprecated prefer metadata.lineage.parent */
    parent_branch?: string;
    author: string;
    license: string;
  };
  hardware: {
    min_vram_gb: number;
    recommended_vram_gb: number;
    quantization: string;
    context_length: number;
  };
  runtime: {
    engine: string;
    command: string;
  };
  evals?: Record<string, number>;
}
