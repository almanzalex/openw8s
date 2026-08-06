/** Shared VRAM estimation — mirrors crates/openw8s-cli/src/vram.rs */

export type Quantization = "FP16" | "INT8/AWQ" | "INT4/Q4";

export const QUANTIZATIONS: Quantization[] = ["FP16", "INT8/AWQ", "INT4/Q4"];
export const CONTEXT_LENGTHS = [4096, 16384, 32768] as const;

export interface ArchHints {
  numHiddenLayers?: number;
  hiddenSize?: number;
  numAttentionHeads?: number;
  numKeyValueHeads?: number;
  headDim?: number;
}

export function gbPerBillionParams(q: Quantization): number {
  switch (q) {
    case "FP16":
      return 2.0;
    case "INT8/AWQ":
      return 1.0;
    case "INT4/Q4":
      return 0.6;
  }
}

export function estimateKvCacheGb(
  paramsBillions: number,
  contextLength: number,
  arch: ArchHints = {},
): number {
  const { numHiddenLayers: layers, hiddenSize: hidden } = arch;
  if (layers && hidden) {
    const attnHeads = arch.numAttentionHeads ?? 32;
    const kvHeads = arch.numKeyValueHeads ?? Math.max(1, Math.floor(attnHeads / 8));
    const dim =
      arch.headDim ?? (Math.floor(hidden / Math.max(attnHeads, 1)) || 128);
    const bytes = 2 * layers * kvHeads * dim * contextLength * 2;
    return bytes / 1_000_000_000;
  }
  return paramsBillions * 0.05 * (contextLength / 1000);
}

export function estimateTotalVramGb(
  paramsBillions: number,
  quantization: Quantization,
  contextLength: number,
  arch: ArchHints = {},
): number {
  const weights = paramsBillions * gbPerBillionParams(quantization);
  const kv = estimateKvCacheGb(paramsBillions, contextLength, arch);
  return (weights + kv) * 1.1;
}

export function formatParams(params: number): string {
  if (params >= 1e12) return `${(params / 1e12).toFixed(1)}T`;
  if (params >= 1e9) return `${(params / 1e9).toFixed(1)}B`;
  if (params >= 1e6) return `${(params / 1e6).toFixed(1)}M`;
  return `${Math.round(params)}`;
}

export function paramsToBillions(params: number): number {
  return params / 1e9;
}

export function normalizeRepoId(input: string): string {
  const trimmed = input.trim();
  const withoutScheme = trimmed.replace(/^https?:\/\//, "");
  const withoutHost = withoutScheme
    .replace(/^(www\.)?huggingface\.co\//, "")
    .replace(/^hf\.co\//, "");
  return withoutHost.replace(/^\/+|\/+$/g, "");
}
