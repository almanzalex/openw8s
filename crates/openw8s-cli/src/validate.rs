//! Validate a `.openw8s.yml` against the v0.1 schema.

use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::manifest::Manifest;

pub fn validate(manifest_path: Option<&Path>) -> Result<()> {
    let path = manifest_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| Path::new(".openw8s.yml").to_path_buf());

    if !path.exists() {
        bail!("no manifest found at `{}`", path.display());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let manifest = Manifest::from_yaml(&contents)?;
    let lineage = manifest.effective_lineage();

    println!();
    println!(
        "{} {} is valid",
        "✓".green().bold(),
        path.display().to_string().white().bold()
    );
    println!(
        "  {} {} · {} · {} → min {:.0} GB",
        manifest.metadata.name,
        manifest.runtime.engine,
        manifest.hardware.quantization,
        lineage.kind,
        manifest.hardware.min_vram_gb
    );
    if let Some(parent) = lineage.parent {
        println!("  {} {parent}", "parent:".dimmed());
    }
    println!();
    Ok(())
}
