//! Interactive `.openw8s.yml` generator.

use anyhow::{Context, Result};
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::fs;
use std::path::Path;

use crate::manifest::{Hardware, Lineage, LineageKind, Manifest, Metadata, Runtime};

const ENGINES: &[&str] = &["vllm", "ollama", "llamacpp", "docker"];
const QUANTIZATIONS: &[&str] = &[
    "FP16",
    "BF16",
    "INT8",
    "INT4",
    "AWQ",
    "GGUF-Q4_K_M",
    "GGUF-Q5_K_M",
    "GGUF-Q8_0",
];
const LINEAGE_KINDS: &[&str] = &["base", "finetune_of", "quantize_of", "merge_of"];

pub fn init(force: bool) -> Result<()> {
    let path = Path::new(".openw8s.yml");
    if path.exists() && !force {
        let overwrite = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(".openw8s.yml already exists. Overwrite?")
            .default(false)
            .interact()?;
        if !overwrite {
            println!("{}", "Aborted — existing manifest left untouched.".yellow());
            return Ok(());
        }
    }

    println!();
    println!("{}", "openw8s init — create a .openw8s.yml manifest".cyan().bold());
    println!();

    let theme = ColorfulTheme::default();

    let base_model: String = Input::with_theme(&theme)
        .with_prompt("Base Hugging Face Repo ID")
        .with_initial_text("Qwen/Qwen2.5-Coder-32B-Instruct")
        .interact_text()
        .context("failed to read base model")?;

    let name_default = format!(
        "{}-{}",
        base_model.rsplit('/').next().unwrap_or(&base_model),
        "openw8s"
    );
    let name: String = Input::with_theme(&theme)
        .with_prompt("Manifest / environment name")
        .with_initial_text(name_default)
        .interact_text()?;

    let author: String = Input::with_theme(&theme)
        .with_prompt("Author")
        .with_initial_text("community")
        .interact_text()?;

    let license: String = Input::with_theme(&theme)
        .with_prompt("License")
        .with_initial_text("apache-2.0")
        .interact_text()?;

    let lineage_idx = Select::with_theme(&theme)
        .with_prompt("Lineage kind")
        .items(LINEAGE_KINDS)
        .default(0)
        .interact()?;
    let lineage_kind = LineageKind::parse(LINEAGE_KINDS[lineage_idx])?;

    let lineage = match lineage_kind {
        LineageKind::Base => Lineage {
            kind: LineageKind::Base,
            parent: None,
        },
        kind => {
            let parent: String = Input::with_theme(&theme)
                .with_prompt("Lineage parent (HF repo id)")
                .with_initial_text(&base_model)
                .interact_text()?;
            Lineage {
                kind,
                parent: Some(parent.trim().to_string()),
            }
        }
    };

    let engine_idx = Select::with_theme(&theme)
        .with_prompt("Target runtime engine")
        .items(ENGINES)
        .default(0)
        .interact()?;
    let engine = ENGINES[engine_idx].to_string();

    let quant_idx = Select::with_theme(&theme)
        .with_prompt("Recommended quantization")
        .items(QUANTIZATIONS)
        .default(3)
        .interact()?;
    let quantization = QUANTIZATIONS[quant_idx].to_string();

    let recommended_vram_gb: f64 = Input::<String>::with_theme(&theme)
        .with_prompt("Recommended VRAM (GB)")
        .with_initial_text("24")
        .interact_text()?
        .parse()
        .context("recommended VRAM must be a number")?;

    let min_vram_gb: f64 = Input::<String>::with_theme(&theme)
        .with_prompt("Minimum VRAM (GB)")
        .with_initial_text(((recommended_vram_gb * 0.75).round() as u64).to_string())
        .interact_text()?
        .parse()
        .context("minimum VRAM must be a number")?;

    let context_length: u64 = Input::<String>::with_theme(&theme)
        .with_prompt("Context length")
        .with_initial_text("16384")
        .interact_text()?
        .parse()
        .context("context length must be an integer")?;

    let default_command = default_command_for(&engine, &base_model, &quantization, context_length);
    let command: String = Input::with_theme(&theme)
        .with_prompt("Execution command")
        .with_initial_text(&default_command)
        .interact_text()?;

    let parent_branch = lineage.parent.clone();

    let mut manifest = Manifest {
        version: "0.1".into(),
        metadata: Metadata {
            name,
            base_model: base_model.clone(),
            lineage: Some(lineage),
            parent_branch,
            author,
            license,
        },
        hardware: Hardware {
            min_vram_gb,
            recommended_vram_gb,
            quantization,
            context_length,
        },
        runtime: Runtime { engine, command },
        evals: None,
    };
    manifest.normalize();
    manifest.validate()?;

    let yaml = manifest.to_yaml().context("failed to serialize manifest")?;
    let contents = format!(
        "# Generated by openw8s init\n# Spec: https://openw8s.com\n{yaml}"
    );
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))?;

    println!();
    println!(
        "{} wrote {}",
        "✓".green().bold(),
        path.display().to_string().white().bold()
    );
    println!(
        "  Validate with {} · launch with {}",
        "openw8s validate".cyan(),
        "openw8s run".cyan()
    );
    println!();

    Ok(())
}

fn default_command_for(engine: &str, repo: &str, quantization: &str, context: u64) -> String {
    match engine {
        "vllm" => {
            let quant_flag = match quantization {
                "AWQ" | "INT4" => " --quantization awq",
                "INT8" => " --quantization fp8",
                _ => "",
            };
            format!(
                "vllm serve {repo}{quant_flag} --max-model-len {context} --gpu-memory-utilization 0.95"
            )
        }
        "ollama" => {
            format!(
                "bash scripts/ollama-generate.sh {repo} \"Hello from openw8s\""
            )
        }
        "llamacpp" => {
            format!("llama-server -m model.gguf -c {context} -ngl 99")
        }
        "docker" => format!(
            "docker run --gpus all -p 8000:8000 vllm/vllm-openai:latest --model {repo}"
        ),
        _ => format!("echo 'Configure a command for {repo}'"),
    }
}
