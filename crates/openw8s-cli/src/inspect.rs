//! Hugging Face model inspection and VRAM matrix printing.

use anyhow::{anyhow, bail, Context, Result};
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Attribute, Cell, Color, ContentArrangement, Table};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

use crate::vram::{
    estimate_total_vram_gb, format_params, params_to_billions, ArchHints, Quantization,
};

const HF_API: &str = "https://huggingface.co/api/models";
const HF_RAW: &str = "https://huggingface.co";
const CONTEXT_LENGTHS: [u64; 3] = [4_096, 16_384, 32_768];

#[derive(Debug, Deserialize)]
struct HfModelInfo {
    id: String,
    #[serde(default)]
    private: bool,
    #[serde(default)]
    gated: Value,
    #[serde(default)]
    siblings: Vec<HfSibling>,
    #[serde(default)]
    card_data: Option<Value>,
    #[serde(default)]
    safetensors: Option<HfSafetensorsSummary>,
}

#[derive(Debug, Deserialize)]
struct HfSibling {
    rfilename: String,
    #[serde(default)]
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct HfSafetensorsSummary {
    #[serde(default)]
    total: Option<u64>,
    #[serde(default)]
    parameters: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ModelProfile {
    pub repo_id: String,
    pub model_name: String,
    pub total_params: f64,
    pub file_format: String,
    pub arch: ArchHints,
}

pub async fn inspect(repo_id: &str) -> Result<()> {
    let repo_id = normalize_repo_id(repo_id);

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    spinner.enable_steady_tick(Duration::from_millis(80));
    spinner.set_message(format!("Fetching Hugging Face metadata for {repo_id}…"));

    let client = reqwest::Client::builder()
        .user_agent(format!("openw8s/{}", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
        .context("failed to build HTTP client")?;

    let info = fetch_model_info(&client, &repo_id).await?;
    spinner.set_message("Resolving parameter count and architecture…");

    let profile = build_profile(&client, &info).await?;
    spinner.finish_and_clear();

    print_profile(&profile);
    Ok(())
}

fn normalize_repo_id(input: &str) -> String {
    let trimmed = input.trim();
    let without_scheme = trimmed
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let without_host = without_scheme
        .trim_start_matches("huggingface.co/")
        .trim_start_matches("www.huggingface.co/")
        .trim_start_matches("hf.co/");
    without_host.trim_matches('/').to_string()
}

async fn fetch_model_info(client: &reqwest::Client, repo_id: &str) -> Result<HfModelInfo> {
    let url = format!("{HF_API}/{repo_id}");
    let mut request = client.get(&url);
    if let Ok(token) = std::env::var("HF_TOKEN") {
        if !token.is_empty() {
            request = request.bearer_auth(token);
        }
    }

    let response = request
        .send()
        .await
        .with_context(|| format!("failed to reach Hugging Face API for `{repo_id}`"))?;

    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        bail!(
            "repository `{repo_id}` was not found, or is private/gated (HTTP {status}). \
             Set HF_TOKEN if you have access."
        );
    }
    if status.as_u16() == 404 {
        bail!("repository `{repo_id}` was not found on Hugging Face");
    }
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("Hugging Face API returned HTTP {status}: {body}");
    }

    let info: HfModelInfo = response
        .json()
        .await
        .context("failed to parse Hugging Face model metadata")?;

    if info.private {
        bail!("repository `{repo_id}` is marked private");
    }

    let gated = match &info.gated {
        Value::Bool(true) => true,
        Value::String(s) if !s.is_empty() && s != "false" => true,
        _ => false,
    };
    if gated {
        eprintln!(
            "{} repository `{repo_id}` is gated — public metadata may be incomplete",
            "warning:".yellow().bold()
        );
    }

    Ok(info)
}

async fn build_profile(client: &reqwest::Client, info: &HfModelInfo) -> Result<ModelProfile> {
    let filenames: Vec<&str> = info.siblings.iter().map(|s| s.rfilename.as_str()).collect();

    let has_index = filenames.contains(&"model.safetensors.index.json");
    let has_safetensors = filenames.iter().any(|f| f.ends_with(".safetensors"));
    let has_gguf = filenames.iter().any(|f| f.ends_with(".gguf"));
    let has_bin = filenames.iter().any(|f| f.ends_with(".bin"));

    let file_format = if has_gguf && !has_safetensors {
        "GGUF".to_string()
    } else if has_safetensors || has_index {
        "safetensors".to_string()
    } else if has_bin {
        "pytorch (.bin)".to_string()
    } else {
        "unknown".to_string()
    };

    let config = fetch_json_file(client, &info.id, "config.json").await.ok();
    let (mut arch, config_params) = parse_config(config.as_ref());

    let mut total_params = None;

    if let Some(summary) = &info.safetensors {
        if let Some(total) = summary.total {
            total_params = Some(total as f64);
        } else if let Some(parameters) = &summary.parameters {
            total_params = Some(sum_parameter_map(parameters));
        }
    }

    if total_params.is_none() && has_index {
        if let Ok(index) =
            fetch_json_file(client, &info.id, "model.safetensors.index.json").await
        {
            if let Some(p) = params_from_index(&index) {
                total_params = Some(p);
            }
        }
    }

    if total_params.is_none() {
        total_params = config_params;
    }

    if total_params.is_none() {
        if let Some(card) = &info.card_data {
            if let Some(p) = params_from_card(card) {
                total_params = Some(p);
            }
        }
    }

    // GGUF / filename heuristics: "TinyLlama-1.1B-…", "*-7B-*", repo ids, etc.
    if total_params.is_none() {
        total_params = params_from_names(
            std::iter::once(info.id.as_str()).chain(filenames.iter().copied()),
        );
    }

    // Stronger GGUF path: range-fetch metadata from a representative .gguf file.
    let mut gguf_label: Option<String> = None;
    if has_gguf {
        if let Some(gguf_name) = pick_gguf_filename(&info.siblings) {
            if let Ok(gguf) = crate::gguf::fetch_gguf_info(client, &info.id, gguf_name).await {
                let hints = gguf.arch_hints();
                if total_params.is_none() {
                    total_params = gguf
                        .parameter_count()
                        .or_else(|| gguf.estimate_params_from_arch());
                }
                if arch.num_hidden_layers.is_none() {
                    arch.num_hidden_layers = hints.num_hidden_layers;
                }
                if arch.hidden_size.is_none() {
                    arch.hidden_size = hints.hidden_size;
                }
                if arch.num_attention_heads.is_none() {
                    arch.num_attention_heads = hints.num_attention_heads;
                }
                if arch.num_key_value_heads.is_none() {
                    arch.num_key_value_heads = hints.num_key_value_heads;
                }
                if arch.head_dim.is_none() {
                    arch.head_dim = hints.head_dim;
                }
                gguf_label = Some(gguf.summary_label());
            }
        }
    }

    // Last resort: estimate from weight file sizes
    if total_params.is_none() {
        let gguf_bytes: u64 = info
            .siblings
            .iter()
            .filter(|s| s.rfilename.to_lowercase().ends_with(".gguf"))
            .filter_map(|s| s.size)
            .max()
            .unwrap_or(0);
        if gguf_bytes > 0 {
            // Q4_K_M ≈ 0.55 bytes/param; prefer largest single shard
            total_params = Some(gguf_bytes as f64 / 0.55);
        } else {
            let weight_bytes: u64 = info
                .siblings
                .iter()
                .filter(|s| {
                    let name = s.rfilename.to_lowercase();
                    name.ends_with(".safetensors") || name.ends_with(".bin")
                })
                .filter_map(|s| s.size)
                .sum();
            if weight_bytes > 0 {
                // FP16 / BF16 ≈ 2 bytes/param
                total_params = Some(weight_bytes as f64 / 2.0);
            }
        }
    }

    let total_params = total_params.ok_or_else(|| {
        anyhow!(
            "could not determine parameter count for `{}` — \
             missing config.json / model.safetensors.index.json / safetensors metadata",
            info.id
        )
    })?;

    let model_name = info
        .id
        .rsplit('/')
        .next()
        .unwrap_or(&info.id)
        .to_string();

    let file_format = if let Some(label) = gguf_label {
        label
    } else {
        file_format
    };

    Ok(ModelProfile {
        repo_id: info.id.clone(),
        model_name,
        total_params,
        file_format,
        arch,
    })
}

async fn fetch_json_file(
    client: &reqwest::Client,
    repo_id: &str,
    filename: &str,
) -> Result<Value> {
    let url = format!("{HF_RAW}/{repo_id}/resolve/main/{filename}");
    let response = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("failed to download `{filename}`"))?;

    if !response.status().is_success() {
        bail!(
            "could not fetch `{filename}` (HTTP {})",
            response.status()
        );
    }

    response
        .json()
        .await
        .with_context(|| format!("failed to parse `{filename}` as JSON"))
}

fn parse_config(config: Option<&Value>) -> (ArchHints, Option<f64>) {
    let Some(config) = config else {
        return (ArchHints::default(), None);
    };

    let arch = ArchHints {
        num_hidden_layers: json_u64(config, &["num_hidden_layers", "n_layer", "num_layers"]),
        hidden_size: json_u64(config, &["hidden_size", "n_embd", "d_model"]),
        num_attention_heads: json_u64(config, &["num_attention_heads", "n_head"]),
        num_key_value_heads: json_u64(config, &["num_key_value_heads", "num_kv_heads"]),
        head_dim: json_u64(config, &["head_dim"]),
    };

    let mut params = None;
    if let (Some(layers), Some(hidden), Some(heads)) = (
        arch.num_hidden_layers,
        arch.hidden_size,
        arch.num_attention_heads,
    ) {
        // Rough transformer param estimate when exact counts unavailable
        let vocab = json_u64(config, &["vocab_size"]).unwrap_or(32_000);
        let intermediate =
            json_u64(config, &["intermediate_size"]).unwrap_or(hidden.saturating_mul(4));
        let kv_heads = arch.num_key_value_heads.unwrap_or(heads);
        let head_d = arch
            .head_dim
            .or_else(|| hidden.checked_div(heads.max(1)))
            .unwrap_or(128);

        // embeddings
        let embed = (vocab * hidden) as f64;
        // attention: Q, K, V, O
        let attn = (layers
            * (hidden * heads * head_d // Q
                + hidden * kv_heads * head_d // K
                + hidden * kv_heads * head_d // V
                + heads * head_d * hidden)) as f64; // O
                                                    // MLP
        let mlp = (layers * (hidden * intermediate + intermediate * hidden)) as f64;
        // norms (approx 2 * hidden per layer + final)
        let norms = ((layers * 2 + 1) * hidden) as f64;
        params = Some(embed + attn + mlp + norms);
    }

    (arch, params)
}

fn json_u64(value: &Value, keys: &[&str]) -> Option<u64> {
    for key in keys {
        if let Some(v) = value.get(*key) {
            if let Some(n) = v.as_u64() {
                return Some(n);
            }
            if let Some(n) = v.as_i64() {
                return Some(n as u64);
            }
            if let Some(n) = v.as_f64() {
                return Some(n as u64);
            }
        }
    }
    None
}

fn sum_parameter_map(parameters: &Value) -> f64 {
    match parameters {
        Value::Object(map) => map.values().filter_map(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64))).sum(),
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn params_from_index(index: &Value) -> Option<f64> {
    if let Some(meta) = index.get("metadata") {
        if let Some(total) = meta.get("total_size").and_then(|v| v.as_u64()) {
            // total_size is bytes of tensors; for FP16/BF16 ≈ 2 bytes/param
            return Some(total as f64 / 2.0);
        }
        // Some indexes include per-dtype param counts under metadata
        if let Some(obj) = meta.as_object() {
            let sum: f64 = obj
                .iter()
                .filter(|(k, _)| {
                    let k = k.to_lowercase();
                    k.contains("param") || k == "total"
                })
                .filter_map(|(_, v)| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
                .sum();
            if sum > 0.0 {
                return Some(sum);
            }
        }
    }

    // Count unique tensor entries as a weak signal — prefer weight byte sum instead
    if let Some(weight_map) = index.get("weight_map").and_then(|v| v.as_object()) {
        if !weight_map.is_empty() {
            // Can't get exact params without dtype sizes; return None
            return None;
        }
    }
    None
}

fn params_from_card(card: &Value) -> Option<f64> {
    // model-index / cardData sometimes stores model size strings
    let candidates = ["model_size", "parameters", "parameter_count", "num_parameters"];
    for key in candidates {
        if let Some(v) = card.get(key) {
            if let Some(n) = v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)) {
                // Heuristic: values < 1000 are likely already in billions
                if n < 1000.0 {
                    return Some(n * 1e9);
                }
                return Some(n);
            }
            if let Some(s) = v.as_str() {
                if let Some(p) = parse_param_label(s) {
                    return Some(p);
                }
            }
        }
    }
    None
}

fn pick_gguf_filename(siblings: &[HfSibling]) -> Option<&str> {
    // Prefer a mid-size quant if present, else the largest named .gguf
    const PREFERRED: &[&str] = &["Q4_K_M", "Q4_K_S", "Q5_K_M", "Q4_0", "Q8_0"];
    for tag in PREFERRED {
        if let Some(s) = siblings.iter().find(|s| {
            let n = s.rfilename.to_uppercase();
            n.ends_with(".GGUF") && n.contains(tag)
        }) {
            return Some(s.rfilename.as_str());
        }
    }
    siblings
        .iter()
        .filter(|s| s.rfilename.to_lowercase().ends_with(".gguf"))
        .max_by_key(|s| s.size.unwrap_or(0))
        .map(|s| s.rfilename.as_str())
}

fn parse_param_label(label: &str) -> Option<f64> {
    let cleaned = label.trim().to_uppercase().replace(' ', "");
    let (num_str, mult) = if let Some(rest) = cleaned.strip_suffix('B') {
        (rest, 1e9)
    } else if let Some(rest) = cleaned.strip_suffix('M') {
        (rest, 1e6)
    } else if let Some(rest) = cleaned.strip_suffix('T') {
        (rest, 1e12)
    } else {
        return cleaned.parse::<f64>().ok();
    };
    num_str.parse::<f64>().ok().map(|n| n * mult)
}

/// Pull parameter counts embedded in repo ids / GGUF filenames (e.g. `…-1.1B-…`).
fn params_from_names<'a>(names: impl Iterator<Item = &'a str>) -> Option<f64> {
    let mut best: Option<f64> = None;
    for name in names {
        for token in split_size_tokens(name) {
            if let Some(p) = parse_param_label(&token) {
                // Ignore tiny matches like "0B" / noise under 100M unless explicit M
                if p >= 1e8 {
                    best = Some(best.map_or(p, |b: f64| b.max(p)));
                }
            }
        }
    }
    best
}

fn split_size_tokens(name: &str) -> Vec<String> {
    // Extract substrings like 1.1B / 7B / 70B / 405B from mixed punctuation.
    let mut out = Vec::new();
    let bytes = name.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            if i < bytes.len() && matches!(bytes[i] as char, 'B' | 'b' | 'M' | 'm' | 'T' | 't') {
                let end = i + 1;
                if let Ok(tok) = std::str::from_utf8(&bytes[start..end]) {
                    out.push(tok.to_string());
                }
                i = end;
                continue;
            }
        } else {
            i += 1;
        }
    }
    out
}

fn print_profile(profile: &ModelProfile) {
    let billions = params_to_billions(profile.total_params);

    println!();
    println!(
        "{} {}",
        "openw8s inspect".cyan().bold(),
        profile.repo_id.white().bold()
    );
    println!();

    let mut meta = Table::new();
    meta.load_preset(UTF8_FULL);
    meta.set_content_arrangement(ContentArrangement::Dynamic);
    meta.set_header(vec![
        Cell::new("Field").add_attribute(Attribute::Bold),
        Cell::new("Value").add_attribute(Attribute::Bold),
    ]);
    meta.add_row(vec!["Model Name", &profile.model_name]);
    meta.add_row(vec!["Repo ID", &profile.repo_id]);
    meta.add_row(vec!["Total Params", &format_params(profile.total_params)]);
    meta.add_row(vec!["File Format", &profile.file_format]);
    if let Some(h) = profile.arch.hidden_size {
        meta.add_row(vec!["Hidden Size".to_string(), h.to_string()]);
    }
    if let Some(l) = profile.arch.num_hidden_layers {
        meta.add_row(vec!["Layers".to_string(), l.to_string()]);
    }
    println!("{meta}");
    println!();
    println!(
        "{}",
        "Estimated VRAM (weights + KV cache + 10% overhead)".bold()
    );

    let mut matrix = Table::new();
    matrix.load_preset(UTF8_FULL);
    matrix.set_content_arrangement(ContentArrangement::Dynamic);
    matrix.set_header(vec![
        Cell::new("Quantization").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("4k ctx").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("16k ctx").add_attribute(Attribute::Bold).fg(Color::Cyan),
        Cell::new("32k ctx").add_attribute(Attribute::Bold).fg(Color::Cyan),
    ]);

    for quant in Quantization::all() {
        let mut row = vec![quant.label().to_string()];
        for ctx in CONTEXT_LENGTHS {
            let gb = estimate_total_vram_gb(billions, quant, ctx, &profile.arch);
            row.push(format!("{gb:.1} GB"));
        }
        matrix.add_row(row);
    }

    println!("{matrix}");
    println!();
    println!(
        "{} Estimates are approximate. Add activation / framework overhead for production serving.",
        "note:".dimmed()
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_hf_urls() {
        assert_eq!(
            normalize_repo_id("https://huggingface.co/Qwen/Qwen2.5-7B-Instruct"),
            "Qwen/Qwen2.5-7B-Instruct"
        );
        assert_eq!(
            normalize_repo_id("Qwen/Qwen2.5-7B-Instruct"),
            "Qwen/Qwen2.5-7B-Instruct"
        );
    }

    #[test]
    fn parses_param_labels() {
        assert_eq!(parse_param_label("7B"), Some(7e9));
        assert_eq!(parse_param_label("32b"), Some(32e9));
    }

    #[test]
    fn params_from_gguf_names() {
        assert_eq!(
            params_from_names(
                [
                    "TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF",
                    "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf",
                ]
                .into_iter()
            ),
            Some(1.1e9)
        );
    }
}
