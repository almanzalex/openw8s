//! Minimal GGUF header + metadata KV parser (no weight download).
//!
//! Fetches a byte range from Hugging Face and extracts architecture fields
//! useful for VRAM estimation.

use anyhow::{anyhow, bail, Context, Result};
use std::collections::HashMap;
use std::io::Cursor;

use crate::vram::ArchHints;

const GGUF_MAGIC: u32 = 0x4655_4747; // "GGUF" LE
const META_UINT8: u32 = 0;
const META_INT8: u32 = 1;
const META_UINT16: u32 = 2;
const META_INT16: u32 = 3;
const META_UINT32: u32 = 4;
const META_INT32: u32 = 5;
const META_FLOAT32: u32 = 6;
const META_BOOL: u32 = 7;
const META_STRING: u32 = 8;
const META_ARRAY: u32 = 9;
const META_UINT64: u32 = 10;
const META_INT64: u32 = 11;
const META_FLOAT64: u32 = 12;

#[derive(Debug, Clone, Default)]
pub struct GgufInfo {
    pub version: u32,
    pub tensor_count: u64,
    pub metadata: HashMap<String, MetaValue>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum MetaValue {
    U64(u64),
    I64(i64),
    F64(f64),
    Bool(bool),
    String(String),
    Other,
}

impl GgufInfo {
    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self.metadata.get(key)? {
            MetaValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn get_u64(&self, key: &str) -> Option<u64> {
        match self.metadata.get(key)? {
            MetaValue::U64(v) => Some(*v),
            MetaValue::I64(v) if *v >= 0 => Some(*v as u64),
            _ => None,
        }
    }

    pub fn architecture(&self) -> Option<&str> {
        self.get_string("general.architecture")
    }

    pub fn model_name(&self) -> Option<&str> {
        self.get_string("general.name")
    }

    pub fn summary_label(&self) -> String {
        format!(
            "GGUFv{} · {} tensors · {}{}",
            self.version,
            self.tensor_count,
            self.architecture().unwrap_or("unknown-arch"),
            self.model_name()
                .map(|n| format!(" · {n}"))
                .unwrap_or_default()
        )
    }

    /// Prefer explicit parameter count when writers include it.
    pub fn parameter_count(&self) -> Option<f64> {
        for key in [
            "general.parameter_count",
            "general.param_count",
            "general.parameters",
        ] {
            if let Some(v) = self.get_u64(key) {
                return Some(v as f64);
            }
        }
        None
    }

    pub fn arch_hints(&self) -> ArchHints {
        let arch = self.architecture().unwrap_or("").to_string();
        let prefix = if arch.is_empty() {
            String::new()
        } else {
            format!("{arch}.")
        };

        let pick_u64 = |suffix: &str| -> Option<u64> {
            let key = format!("{prefix}{suffix}");
            self.get_u64(&key).or_else(|| {
                // Fallback: any metadata key ending with the suffix
                self.metadata.iter().find_map(|(k, _)| {
                    if k.ends_with(suffix) {
                        self.get_u64(k)
                    } else {
                        None
                    }
                })
            })
        };

        ArchHints {
            num_hidden_layers: pick_u64("block_count"),
            hidden_size: pick_u64("embedding_length"),
            num_attention_heads: pick_u64("attention.head_count"),
            num_key_value_heads: pick_u64("attention.head_count_kv"),
            head_dim: pick_u64("attention.key_length")
                .or_else(|| pick_u64("attention.value_length")),
        }
    }

    /// Rough transformer param estimate from GGUF architecture KVs when
    /// `general.parameter_count` is absent.
    pub fn estimate_params_from_arch(&self) -> Option<f64> {
        let hints = self.arch_hints();
        let layers = hints.num_hidden_layers?;
        let hidden = hints.hidden_size?;
        let heads = hints.num_attention_heads.unwrap_or(32).max(1);
        let kv_heads = hints.num_key_value_heads.unwrap_or(heads).max(1);
        let head_dim = hints.head_dim.unwrap_or(hidden / heads).max(1);

        let arch = self.architecture().unwrap_or("");
        let prefix = if arch.is_empty() {
            String::new()
        } else {
            format!("{arch}.")
        };
        let ffn = self
            .get_u64(&format!("{prefix}feed_forward_length"))
            .unwrap_or(hidden * 4);
        let vocab = self
            .get_u64(&format!("{prefix}vocab_size"))
            .or_else(|| self.get_u64("tokenizer.ggml.tokens")) // may be array — skip
            .unwrap_or(32_000);

        let embed = (vocab.saturating_mul(hidden)) as f64;
        let attn = (layers
            * (hidden * heads * head_dim
                + hidden * kv_heads * head_dim
                + hidden * kv_heads * head_dim
                + heads * head_dim * hidden)) as f64;
        let mlp = (layers * (hidden * ffn + ffn * hidden)) as f64;
        let norms = ((layers * 2 + 1) * hidden) as f64;
        Some(embed + attn + mlp + norms)
    }
}

pub fn parse_gguf_prefix(bytes: &[u8]) -> Result<GgufInfo> {
    let mut cur = Cursor::new(bytes);
    let magic = read_u32(&mut cur)?;
    if magic != GGUF_MAGIC {
        bail!("not a GGUF file (bad magic)");
    }
    let version = read_u32(&mut cur)?;
    if !(2..=3).contains(&version) {
        bail!("unsupported GGUF version {version}");
    }
    let tensor_count = read_u64(&mut cur)?;
    let metadata_kv_count = read_u64(&mut cur)?;

    let mut metadata = HashMap::new();
    for _ in 0..metadata_kv_count {
        let key = read_string(&mut cur)?;
        match read_value(&mut cur) {
            Ok(value) => {
                metadata.insert(key, value);
            }
            Err(err) => {
                // Large tokenizer arrays often exceed the ranged prefix; keep
                // whatever architecture keys we already collected.
                let msg = err.to_string();
                if msg.contains("unexpected end") || msg.contains("too large") {
                    break;
                }
                return Err(err);
            }
        }
    }

    Ok(GgufInfo {
        version,
        tensor_count,
        metadata,
    })
}

/// Download a prefix of a HF-hosted GGUF via HTTP Range and parse metadata.
pub async fn fetch_gguf_info(
    client: &reqwest::Client,
    repo_id: &str,
    filename: &str,
) -> Result<GgufInfo> {
    let url = format!("https://huggingface.co/{repo_id}/resolve/main/{filename}");
    // Metadata is almost always in the first few hundred KB; 2 MiB is plenty.
    let response = client
        .get(&url)
        .header("Range", "bytes=0-2097151")
        .send()
        .await
        .with_context(|| format!("failed to range-fetch `{filename}`"))?;

    let status = response.status().as_u16();
    if status != 200 && status != 206 {
        bail!(
            "could not fetch GGUF prefix for `{filename}` (HTTP {status})"
        );
    }

    let bytes = response
        .bytes()
        .await
        .context("failed to read GGUF prefix body")?;
    parse_gguf_prefix(&bytes).map_err(|e| anyhow!("{e} (file `{filename}`)"))
}

fn read_u32(cur: &mut Cursor<&[u8]>) -> Result<u32> {
    let mut buf = [0u8; 4];
    read_exact(cur, &mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64(cur: &mut Cursor<&[u8]>) -> Result<u64> {
    let mut buf = [0u8; 8];
    read_exact(cur, &mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_i64(cur: &mut Cursor<&[u8]>) -> Result<i64> {
    let mut buf = [0u8; 8];
    read_exact(cur, &mut buf)?;
    Ok(i64::from_le_bytes(buf))
}

fn read_f32(cur: &mut Cursor<&[u8]>) -> Result<f32> {
    let mut buf = [0u8; 4];
    read_exact(cur, &mut buf)?;
    Ok(f32::from_le_bytes(buf))
}

fn read_f64(cur: &mut Cursor<&[u8]>) -> Result<f64> {
    let mut buf = [0u8; 8];
    read_exact(cur, &mut buf)?;
    Ok(f64::from_le_bytes(buf))
}

fn read_exact(cur: &mut Cursor<&[u8]>, buf: &mut [u8]) -> Result<()> {
    use std::io::Read;
    cur.read_exact(buf)
        .context("unexpected end of GGUF prefix — try a larger range")?;
    Ok(())
}

fn read_string(cur: &mut Cursor<&[u8]>) -> Result<String> {
    let len = read_u64(cur)? as usize;
    if len > 1_000_000 {
        bail!("GGUF string length too large ({len})");
    }
    let mut buf = vec![0u8; len];
    read_exact(cur, &mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn read_value(cur: &mut Cursor<&[u8]>) -> Result<MetaValue> {
    let ty = read_u32(cur)?;
    read_value_of_type(cur, ty)
}

fn read_value_of_type(cur: &mut Cursor<&[u8]>, ty: u32) -> Result<MetaValue> {
    Ok(match ty {
        META_UINT8 => {
            let mut b = [0u8; 1];
            read_exact(cur, &mut b)?;
            MetaValue::U64(b[0] as u64)
        }
        META_INT8 => {
            let mut b = [0u8; 1];
            read_exact(cur, &mut b)?;
            MetaValue::I64(b[0] as i8 as i64)
        }
        META_UINT16 => {
            let mut b = [0u8; 2];
            read_exact(cur, &mut b)?;
            MetaValue::U64(u16::from_le_bytes(b) as u64)
        }
        META_INT16 => {
            let mut b = [0u8; 2];
            read_exact(cur, &mut b)?;
            MetaValue::I64(i16::from_le_bytes(b) as i64)
        }
        META_UINT32 => MetaValue::U64(read_u32(cur)? as u64),
        META_INT32 => {
            let mut b = [0u8; 4];
            read_exact(cur, &mut b)?;
            MetaValue::I64(i32::from_le_bytes(b) as i64)
        }
        META_FLOAT32 => MetaValue::F64(read_f32(cur)? as f64),
        META_BOOL => {
            let mut b = [0u8; 1];
            read_exact(cur, &mut b)?;
            MetaValue::Bool(b[0] != 0)
        }
        META_STRING => MetaValue::String(read_string(cur)?),
        META_ARRAY => {
            let elem_ty = read_u32(cur)?;
            let len = read_u64(cur)?;
            // Tokenizer vocab arrays are huge; refuse to fully decode them in a
            // ranged prefix and signal the caller to stop.
            if len > 256 && elem_ty == META_STRING {
                bail!("GGUF string array too large ({len})");
            }
            for _ in 0..len {
                let _ = read_value_of_type(cur, elem_ty)?;
            }
            MetaValue::Other
        }
        META_UINT64 => MetaValue::U64(read_u64(cur)?),
        META_INT64 => MetaValue::I64(read_i64(cur)?),
        META_FLOAT64 => MetaValue::F64(read_f64(cur)?),
        other => bail!("unknown GGUF metadata type {other}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_magic() {
        assert!(parse_gguf_prefix(&[0, 1, 2, 3, 4, 5, 6, 7]).is_err());
    }
}
