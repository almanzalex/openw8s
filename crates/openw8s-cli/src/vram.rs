//! VRAM estimation helpers for open-weight model inspection.

/// Supported quantization formats for the VRAM matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantization {
    Fp16,
    Int8,
    Int4,
}

impl Quantization {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fp16 => "FP16",
            Self::Int8 => "INT8/AWQ",
            Self::Int4 => "INT4/Q4",
        }
    }

    /// Approximate GB of VRAM per billion parameters for weight storage.
    pub fn gb_per_billion_params(self) -> f64 {
        match self {
            Self::Fp16 => 2.0,
            Self::Int8 => 1.0,
            Self::Int4 => 0.6,
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::Fp16, Self::Int8, Self::Int4]
    }
}

/// Optional transformer architecture fields used for KV-cache sizing.
#[derive(Debug, Clone, Default)]
pub struct ArchHints {
    pub num_hidden_layers: Option<u64>,
    pub hidden_size: Option<u64>,
    pub num_attention_heads: Option<u64>,
    pub num_key_value_heads: Option<u64>,
    pub head_dim: Option<u64>,
}

/// Estimate KV-cache VRAM (GB) for a given context length.
///
/// Uses a simplified formula:
/// `2 * n_layers * kv_heads * head_dim * context * 2 bytes (FP16) / 1e9`
/// When architecture details are missing, falls back to ~0.05 GB per 1B params
/// per 1k context tokens (rough empirical average for decoder-only LLMs).
pub fn estimate_kv_cache_gb(params_billions: f64, context_length: u64, arch: &ArchHints) -> f64 {
    if let (Some(layers), Some(hidden)) = (arch.num_hidden_layers, arch.hidden_size) {
        let attn_heads = arch.num_attention_heads.unwrap_or(32);
        let kv_heads = arch
            .num_key_value_heads
            .unwrap_or_else(|| (attn_heads / 8).max(1));
        let dim = arch
            .head_dim
            .or_else(|| hidden.checked_div(attn_heads))
            .unwrap_or(128);

        // K + V, FP16: 2 * layers * kv_heads * head_dim * seq_len * 2 bytes
        let bytes =
            2.0 * layers as f64 * kv_heads as f64 * dim as f64 * context_length as f64 * 2.0;
        return bytes / 1_000_000_000.0;
    }

    // Fallback heuristic when config.json architecture fields are unavailable
    params_billions * 0.05 * (context_length as f64 / 1000.0)
}

/// Total estimated VRAM (weights + KV cache) in GB.
pub fn estimate_total_vram_gb(
    params_billions: f64,
    quantization: Quantization,
    context_length: u64,
    arch: &ArchHints,
) -> f64 {
    let weights = params_billions * quantization.gb_per_billion_params();
    let kv = estimate_kv_cache_gb(params_billions, context_length, arch);
    // Small activation / overhead buffer (~10%)
    (weights + kv) * 1.10
}

/// Format a parameter count (absolute) as a human label like "7.0B".
pub fn format_params(params: f64) -> String {
    if params >= 1e12 {
        format!("{:.1}T", params / 1e12)
    } else if params >= 1e9 {
        format!("{:.1}B", params / 1e9)
    } else if params >= 1e6 {
        format!("{:.1}M", params / 1e6)
    } else {
        format!("{:.0}", params)
    }
}

/// Convert absolute parameter count to billions.
pub fn params_to_billions(params: f64) -> f64 {
    params / 1e9
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fp16_7b_weights_roughly_14gb() {
        let vram = estimate_total_vram_gb(7.0, Quantization::Fp16, 4096, &ArchHints::default());
        // 14 GB weights + KV + 10% overhead — should land roughly 15–20 GB
        assert!(vram > 14.0 && vram < 25.0, "got {vram}");
    }

    #[test]
    fn format_params_billions() {
        assert_eq!(format_params(7_000_000_000.0), "7.0B");
    }
}
