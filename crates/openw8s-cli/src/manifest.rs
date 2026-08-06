use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported lineage relationship kinds (spec v0.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LineageKind {
    /// Root / base open-weight checkpoint (no parent).
    #[default]
    Base,
    /// Instruction / domain fine-tune of a parent model.
    FinetuneOf,
    /// Quantized derivative of a parent model.
    QuantizeOf,
    /// Merge / MoE / soup derived from one or more parents.
    MergeOf,
}

impl LineageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::FinetuneOf => "finetune_of",
            Self::QuantizeOf => "quantize_of",
            Self::MergeOf => "merge_of",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "base" => Ok(Self::Base),
            "finetune_of" | "finetune" => Ok(Self::FinetuneOf),
            "quantize_of" | "quantize" | "quant" => Ok(Self::QuantizeOf),
            "merge_of" | "merge" => Ok(Self::MergeOf),
            other => bail!(
                "unknown lineage kind `{other}` (expected base|finetune_of|quantize_of|merge_of)"
            ),
        }
    }
}

impl fmt::Display for LineageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Optional lineage edge for environment / model ancestry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lineage {
    pub kind: LineageKind,
    /// Parent Hugging Face repo id (required unless kind is `base`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

/// Root schema for `.openw8s.yml` (spec version 0.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub metadata: Metadata,
    pub hardware: Hardware,
    pub runtime: Runtime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evals: Option<serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub base_model: String,
    /// Structured lineage (preferred).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<Lineage>,
    /// Deprecated alias for `lineage.parent` (still accepted on load).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_branch: Option<String>,
    pub author: String,
    pub license: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hardware {
    pub min_vram_gb: f64,
    pub recommended_vram_gb: f64,
    pub quantization: String,
    pub context_length: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runtime {
    pub engine: String,
    pub command: String,
}

const SUPPORTED_VERSIONS: &[&str] = &["0.1"];
const ALLOWED_ENGINES: &[&str] = &["vllm", "ollama", "llamacpp", "docker"];
const ALLOWED_QUANTS: &[&str] = &["FP16", "BF16", "INT8", "INT4", "AWQ", "GGUF-Q4_K_M", "GGUF-Q5_K_M", "GGUF-Q8_0"];

impl Manifest {
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    pub fn from_yaml(contents: &str) -> Result<Self> {
        let mut manifest: Self =
            serde_yaml::from_str(contents).context("invalid YAML for .openw8s.yml")?;
        manifest.normalize();
        manifest.validate()?;
        Ok(manifest)
    }

    /// Promote deprecated `parent_branch` into structured `lineage` when needed.
    pub fn normalize(&mut self) {
        if self.metadata.lineage.is_none() {
            if let Some(parent) = self
                .metadata
                .parent_branch
                .as_ref()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
            {
                self.metadata.lineage = Some(Lineage {
                    kind: LineageKind::FinetuneOf,
                    parent: Some(parent),
                });
            }
        }

        // Keep parent_branch mirrored for older consumers when lineage has a parent.
        if let Some(lineage) = &self.metadata.lineage {
            if let Some(parent) = &lineage.parent {
                if self.metadata.parent_branch.is_none() {
                    self.metadata.parent_branch = Some(parent.clone());
                }
            }
        }

        self.runtime.engine = self.runtime.engine.trim().to_ascii_lowercase();
        self.hardware.quantization = self.hardware.quantization.trim().to_string();
        self.runtime.command = self.runtime.command.trim().to_string();
    }

    pub fn validate(&self) -> Result<()> {
        if !SUPPORTED_VERSIONS.contains(&self.version.as_str()) {
            bail!(
                "unsupported manifest version `{}` (supported: {})",
                self.version,
                SUPPORTED_VERSIONS.join(", ")
            );
        }

        require_nonempty("metadata.name", &self.metadata.name)?;
        require_hf_repo("metadata.base_model", &self.metadata.base_model)?;
        require_nonempty("metadata.author", &self.metadata.author)?;
        require_nonempty("metadata.license", &self.metadata.license)?;

        if let Some(lineage) = &self.metadata.lineage {
            match lineage.kind {
                LineageKind::Base => {
                    if lineage
                        .parent
                        .as_ref()
                        .is_some_and(|p| !p.trim().is_empty())
                    {
                        bail!("lineage.kind=base must not set lineage.parent");
                    }
                }
                LineageKind::FinetuneOf | LineageKind::QuantizeOf | LineageKind::MergeOf => {
                    let parent = lineage.parent.as_deref().unwrap_or("").trim();
                    if parent.is_empty() {
                        bail!(
                            "lineage.kind={} requires lineage.parent (HF repo id)",
                            lineage.kind
                        );
                    }
                    require_hf_repo("lineage.parent", parent)?;
                }
            }
        }

        if self.hardware.min_vram_gb <= 0.0 {
            bail!("hardware.min_vram_gb must be > 0");
        }
        if self.hardware.recommended_vram_gb < self.hardware.min_vram_gb {
            bail!(
                "hardware.recommended_vram_gb ({}) must be >= min_vram_gb ({})",
                self.hardware.recommended_vram_gb,
                self.hardware.min_vram_gb
            );
        }
        if self.hardware.context_length == 0 {
            bail!("hardware.context_length must be > 0");
        }

        let quant = self.hardware.quantization.as_str();
        if !ALLOWED_QUANTS
            .iter()
            .any(|q| q.eq_ignore_ascii_case(quant))
        {
            bail!(
                "hardware.quantization `{quant}` not in allow-list ({})",
                ALLOWED_QUANTS.join(", ")
            );
        }

        if !ALLOWED_ENGINES.contains(&self.runtime.engine.as_str()) {
            bail!(
                "runtime.engine `{}` not in allow-list ({})",
                self.runtime.engine,
                ALLOWED_ENGINES.join(", ")
            );
        }
        require_nonempty("runtime.command", &self.runtime.command)?;

        Ok(())
    }

    pub fn effective_lineage(&self) -> Lineage {
        self.metadata.lineage.clone().unwrap_or(Lineage {
            kind: LineageKind::Base,
            parent: None,
        })
    }
}

fn require_nonempty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{field} must not be empty");
    }
    Ok(())
}

fn require_hf_repo(field: &str, value: &str) -> Result<()> {
    require_nonempty(field, value)?;
    if !value.contains('/') || value.starts_with('/') || value.ends_with('/') {
        bail!("{field} must look like an HF repo id (org/name), got `{value}`");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_manifest() {
        let yaml = r#"
version: "0.1"
metadata:
  name: demo
  base_model: Qwen/Qwen2.5-7B-Instruct
  author: community
  license: apache-2.0
  lineage:
    kind: finetune_of
    parent: Qwen/Qwen2.5-7B
hardware:
  min_vram_gb: 8
  recommended_vram_gb: 16
  quantization: INT4
  context_length: 8192
runtime:
  engine: vllm
  command: vllm serve Qwen/Qwen2.5-7B-Instruct
"#;
        let m = Manifest::from_yaml(yaml).unwrap();
        assert_eq!(m.effective_lineage().kind, LineageKind::FinetuneOf);
    }

    #[test]
    fn promotes_parent_branch() {
        let yaml = r#"
version: "0.1"
metadata:
  name: demo
  base_model: org/model
  parent_branch: org/base
  author: community
  license: apache-2.0
hardware:
  min_vram_gb: 4
  recommended_vram_gb: 8
  quantization: FP16
  context_length: 4096
runtime:
  engine: ollama
  command: ollama run demo
"#;
        let m = Manifest::from_yaml(yaml).unwrap();
        assert_eq!(m.effective_lineage().kind, LineageKind::FinetuneOf);
        assert_eq!(
            m.effective_lineage().parent.as_deref(),
            Some("org/base")
        );
    }

    #[test]
    fn rejects_bad_engine() {
        let yaml = r#"
version: "0.1"
metadata:
  name: demo
  base_model: org/model
  author: community
  license: apache-2.0
hardware:
  min_vram_gb: 4
  recommended_vram_gb: 8
  quantization: FP16
  context_length: 4096
runtime:
  engine: foobar
  command: echo hi
"#;
        assert!(Manifest::from_yaml(yaml).is_err());
    }
}
