import {
  ArchHints,
  formatParams,
  normalizeRepoId,
  paramsToBillions,
} from "./vram";

export interface ModelProfile {
  repoId: string;
  modelName: string;
  totalParams: number;
  paramsLabel: string;
  paramsBillions: number;
  fileFormat: string;
  arch: ArchHints;
}

interface HfSibling {
  rfilename: string;
  size?: number;
}

interface HfModelInfo {
  id: string;
  private?: boolean;
  gated?: boolean | string;
  siblings?: HfSibling[];
  cardData?: Record<string, unknown>;
  safetensors?: {
    total?: number;
    parameters?: Record<string, number> | number;
  };
}

function jsonNumber(obj: Record<string, unknown>, keys: string[]): number | undefined {
  for (const key of keys) {
    const v = obj[key];
    if (typeof v === "number" && Number.isFinite(v)) return v;
  }
  return undefined;
}

function parseConfig(config: Record<string, unknown> | null): {
  arch: ArchHints;
  configParams?: number;
} {
  if (!config) return { arch: {} };

  const arch: ArchHints = {
    numHiddenLayers: jsonNumber(config, ["num_hidden_layers", "n_layer", "num_layers"]),
    hiddenSize: jsonNumber(config, ["hidden_size", "n_embd", "d_model"]),
    numAttentionHeads: jsonNumber(config, ["num_attention_heads", "n_head"]),
    numKeyValueHeads: jsonNumber(config, ["num_key_value_heads", "num_kv_heads"]),
    headDim: jsonNumber(config, ["head_dim"]),
  };

  let configParams: number | undefined;
  if (arch.numHiddenLayers && arch.hiddenSize && arch.numAttentionHeads) {
    const layers = arch.numHiddenLayers;
    const hidden = arch.hiddenSize;
    const heads = arch.numAttentionHeads;
    const vocab = jsonNumber(config, ["vocab_size"]) ?? 32_000;
    const intermediate = jsonNumber(config, ["intermediate_size"]) ?? hidden * 4;
    const kvHeads = arch.numKeyValueHeads ?? heads;
    const headD = arch.headDim ?? Math.floor(hidden / Math.max(heads, 1));
    const embed = vocab * hidden;
    const attn =
      layers *
      (hidden * heads * headD +
        hidden * kvHeads * headD +
        hidden * kvHeads * headD +
        heads * headD * hidden);
    const mlp = layers * (hidden * intermediate + intermediate * hidden);
    const norms = (layers * 2 + 1) * hidden;
    configParams = embed + attn + mlp + norms;
  }

  return { arch, configParams };
}

async function fetchJson(url: string): Promise<unknown | null> {
  const res = await fetch(url, {
    headers: { "User-Agent": "openw8s-web/0.1" },
    next: { revalidate: 3600 },
  });
  if (!res.ok) return null;
  return res.json();
}

export async function inspectModel(rawRepoId: string): Promise<ModelProfile> {
  const repoId = normalizeRepoId(rawRepoId);
  if (!repoId.includes("/")) {
    throw new Error("Expected a Hugging Face repo id like org/model-name");
  }

  const infoRes = await fetch(`https://huggingface.co/api/models/${repoId}`, {
    headers: { "User-Agent": "openw8s-web/0.1" },
    next: { revalidate: 3600 },
  });

  if (infoRes.status === 401 || infoRes.status === 403) {
    throw new Error(
      `Repository \`${repoId}\` is private or gated. Public metadata is unavailable.`,
    );
  }
  if (infoRes.status === 404) {
    throw new Error(`Repository \`${repoId}\` was not found on Hugging Face.`);
  }
  if (!infoRes.ok) {
    throw new Error(`Hugging Face API returned HTTP ${infoRes.status}`);
  }

  const info = (await infoRes.json()) as HfModelInfo;
  if (info.private) {
    throw new Error(`Repository \`${repoId}\` is marked private.`);
  }

  const siblings = info.siblings ?? [];
  const filenames = siblings.map((s) => s.rfilename);
  const hasIndex = filenames.includes("model.safetensors.index.json");
  const hasSafetensors = filenames.some((f) => f.endsWith(".safetensors"));
  const hasGguf = filenames.some((f) => f.endsWith(".gguf"));
  const hasBin = filenames.some((f) => f.endsWith(".bin"));

  let fileFormat = "unknown";
  if (hasGguf && !hasSafetensors) fileFormat = "GGUF";
  else if (hasSafetensors || hasIndex) fileFormat = "safetensors";
  else if (hasBin) fileFormat = "pytorch (.bin)";

  const config = (await fetchJson(
    `https://huggingface.co/${repoId}/resolve/main/config.json`,
  )) as Record<string, unknown> | null;
  const { arch, configParams } = parseConfig(config);

  let totalParams: number | undefined;

  if (info.safetensors?.total) {
    totalParams = info.safetensors.total;
  } else if (info.safetensors?.parameters) {
    const p = info.safetensors.parameters;
    if (typeof p === "number") totalParams = p;
    else totalParams = Object.values(p).reduce((a, b) => a + b, 0);
  }

  if (totalParams === undefined && hasIndex) {
    const index = (await fetchJson(
      `https://huggingface.co/${repoId}/resolve/main/model.safetensors.index.json`,
    )) as { metadata?: { total_size?: number } } | null;
    if (index?.metadata?.total_size) {
      totalParams = index.metadata.total_size / 2;
    }
  }

  if (totalParams === undefined) totalParams = configParams;

  if (totalParams === undefined) {
    const weightBytes = siblings
      .filter((s) => {
        const n = s.rfilename.toLowerCase();
        return n.endsWith(".safetensors") || n.endsWith(".gguf") || n.endsWith(".bin");
      })
      .reduce((sum, s) => sum + (s.size ?? 0), 0);
    if (weightBytes > 0) totalParams = weightBytes / 2;
  }

  if (totalParams === undefined) {
    throw new Error(
      `Could not determine parameter count for \`${repoId}\` — missing config / index / safetensors metadata.`,
    );
  }

  const modelName = repoId.split("/").pop() ?? repoId;

  return {
    repoId: info.id ?? repoId,
    modelName,
    totalParams,
    paramsLabel: formatParams(totalParams),
    paramsBillions: paramsToBillions(totalParams),
    fileFormat,
    arch,
  };
}
