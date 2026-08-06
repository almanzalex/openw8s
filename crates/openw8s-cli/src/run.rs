//! Execute the runtime command from a local `.openw8s.yml`.

use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;
use std::process::Stdio;

use crate::manifest::Manifest;

pub async fn run(manifest_path: Option<&Path>, dry_run: bool, force: bool) -> Result<()> {
    let path = manifest_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| Path::new(".openw8s.yml").to_path_buf());

    if !path.exists() {
        bail!(
            "no manifest found at `{}` — run `openw8s init` first",
            path.display()
        );
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let manifest = Manifest::from_yaml(&contents)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    let lineage = manifest.effective_lineage();

    println!();
    println!("{}", "openw8s run".cyan().bold());
    println!();
    println!("  {} {}", "name:".dimmed(), manifest.metadata.name.bold());
    println!(
        "  {} {}",
        "base:".dimmed(),
        manifest.metadata.base_model
    );
    println!(
        "  {} {}{}",
        "lineage:".dimmed(),
        lineage.kind,
        lineage
            .parent
            .as_ref()
            .map(|p| format!(" → {p}"))
            .unwrap_or_default()
    );
    println!(
        "  {} {}",
        "engine:".dimmed(),
        manifest.runtime.engine.yellow()
    );
    println!(
        "  {} {} GB min / {} GB recommended ({})",
        "vram:".dimmed(),
        manifest.hardware.min_vram_gb,
        manifest.hardware.recommended_vram_gb,
        manifest.hardware.quantization
    );
    println!(
        "  {} {}",
        "context:".dimmed(),
        manifest.hardware.context_length
    );
    println!();
    println!("{}", "command:".dimmed());
    println!("  {}", manifest.runtime.command.trim().green());
    println!();

    // Preflight: compare local free VRAM / unified memory against manifest minimum.
    let preflight = vram_preflight(manifest.hardware.min_vram_gb, force, dry_run).await?;
    match &preflight {
        Preflight::Ok { free_gb, source } => {
            println!(
                "  {} {:.1} GB free ({source}) ≥ {:.1} GB min {}",
                "preflight:".dimmed(),
                free_gb,
                manifest.hardware.min_vram_gb,
                "✓".green()
            );
            println!();
        }
        Preflight::SkippedNoProbe => {
            println!(
                "  {} {}",
                "preflight:".dimmed(),
                "no GPU probe available (nvidia-smi / Apple Metal) — skipping VRAM check"
                    .yellow()
            );
            println!();
        }
        Preflight::ForcedBelow { free_gb, source } => {
            println!(
                "  {} {:.1} GB free ({source}) < {:.1} GB min — continuing due to --force",
                "preflight:".yellow().bold(),
                free_gb,
                manifest.hardware.min_vram_gb
            );
            println!();
        }
        Preflight::WarnedBelow { free_gb, source } => {
            println!(
                "  {} {:.1} GB free ({source}) < {:.1} GB min — dry-run only, not blocking",
                "preflight:".yellow().bold(),
                free_gb,
                manifest.hardware.min_vram_gb
            );
            println!();
        }
    }

    if dry_run {
        println!("{}", "dry-run — not spawning process".yellow());
        return Ok(());
    }

    let before = probe_free_vram_gb().await;

    let command = manifest.runtime.command.trim();
    println!("{} spawning runtime…\n", "→".cyan());

    let status = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .with_context(|| format!("failed to spawn command: {command}"))?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        bail!("runtime exited with status {code}");
    }

    let after = probe_free_vram_gb().await;
    write_run_lock(&path, &manifest, before.as_ref(), after.as_ref())?;

    Ok(())
}

enum Preflight {
    Ok { free_gb: f64, source: &'static str },
    SkippedNoProbe,
    ForcedBelow { free_gb: f64, source: &'static str },
    WarnedBelow { free_gb: f64, source: &'static str },
}

async fn vram_preflight(min_vram_gb: f64, force: bool, dry_run: bool) -> Result<Preflight> {
    let Some((free_gb, source)) = probe_free_vram_gb().await else {
        return Ok(Preflight::SkippedNoProbe);
    };

    if free_gb + 0.5 >= min_vram_gb {
        return Ok(Preflight::Ok { free_gb, source });
    }

    if force {
        return Ok(Preflight::ForcedBelow { free_gb, source });
    }

    // Dry-run should still succeed so manifests can be inspected without GPU headroom.
    if dry_run {
        return Ok(Preflight::WarnedBelow { free_gb, source });
    }

    bail!(
        "VRAM preflight failed: {:.1} GB free ({source}) < {:.1} GB required by manifest.\n\
         Free GPU memory or re-run with --force to override.",
        free_gb,
        min_vram_gb
    );
}

async fn probe_free_vram_gb() -> Option<(f64, &'static str)> {
    if let Some(gb) = probe_nvidia_free_vram_gb().await {
        return Some((gb, "nvidia-smi"));
    }
    probe_apple_unified_free_gb()
        .await
        .map(|gb| (gb, "apple-unified"))
}

async fn probe_nvidia_free_vram_gb() -> Option<f64> {
    let output = tokio::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut total_free_mb = 0.0_f64;
    let mut count = 0_u32;
    for line in text.lines() {
        if let Ok(mb) = line.trim().parse::<f64>() {
            total_free_mb += mb;
            count += 1;
        }
    }
    if count == 0 {
        return None;
    }
    Some(total_free_mb / 1024.0)
}

/// Best-effort Apple unified-memory estimate.
///
/// macOS keeps little memory in "Pages free"; inactive/purgeable pages are reclaimable
/// for Metal model loads. We sum free + inactive + speculative + purgeable.
async fn probe_apple_unified_free_gb() -> Option<f64> {
    if std::env::consts::OS != "macos" {
        return None;
    }

    let sp = tokio::process::Command::new("system_profiler")
        .args(["SPDisplaysDataType", "-json"])
        .output()
        .await
        .ok()?;
    if !sp.status.success() {
        return None;
    }
    let sp_text = String::from_utf8_lossy(&sp.stdout);
    if !sp_text.to_ascii_lowercase().contains("metal") {
        return None;
    }

    let page_size = tokio::process::Command::new("pagesize")
        .output()
        .await
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<f64>()
                .ok()
        })
        .unwrap_or(16384.0);

    let vm = tokio::process::Command::new("vm_stat")
        .output()
        .await
        .ok()?;
    if !vm.status.success() {
        return None;
    }
    let vm_text = String::from_utf8_lossy(&vm.stdout);

    let mut pages = 0.0_f64;
    for key in [
        "Pages free",
        "Pages inactive",
        "Pages speculative",
        "Pages purgeable",
    ] {
        if let Some(line) = vm_text.lines().find(|l| l.starts_with(key)) {
            let Some(raw) = line.split(':').nth(1) else {
                continue;
            };
            let num = raw.trim().trim_end_matches('.').replace(',', "");
            pages += num.parse::<f64>().unwrap_or(0.0);
        }
    }
    if pages <= 0.0 {
        return None;
    }

    let reclaimable = (pages * page_size) / 1_000_000_000.0;

    // Also surface a floor from total RAM so tiny free-page blips don't zero out.
    let total_gb = tokio::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .await
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .parse::<f64>()
                .ok()
                .map(|b| b / 1_000_000_000.0)
        });

    // Use the larger of reclaimable pages vs ~35% of total RAM (conservative headroom).
    let floor = total_gb.map(|t| t * 0.35).unwrap_or(0.0);
    Some(reclaimable.max(floor))
}

/// Write `.openw8s.lock.yml` next to the manifest with measured free-memory snapshot.
fn write_run_lock(
    manifest_path: &Path,
    manifest: &Manifest,
    before: Option<&(f64, &'static str)>,
    after: Option<&(f64, &'static str)>,
) -> Result<()> {
    let lock_path = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".openw8s.lock.yml");

    let measured = match (before, after) {
        (Some((b, src)), Some((a, _))) => {
            format!(
                "  probe: \"{src}\"\n  free_before_gb: {b:.2}\n  free_after_gb: {a:.2}\n  delta_gb: {:.2}\n",
                b - a
            )
        }
        (Some((b, src)), None) => {
            format!("  probe: \"{src}\"\n  free_before_gb: {b:.2}\n")
        }
        _ => "  probe: \"unavailable\"\n".to_string(),
    };

    let body = format!(
        "# Generated by openw8s run — measured environment lock\n\
version: \"0.1\"\n\
manifest: \"{}\"\n\
name: \"{}\"\n\
engine: \"{}\"\n\
quantization: \"{}\"\n\
declared_min_vram_gb: {}\n\
measured:\n\
{measured}",
        manifest_path.display(),
        manifest.metadata.name,
        manifest.runtime.engine,
        manifest.hardware.quantization,
        manifest.hardware.min_vram_gb,
    );

    fs::write(&lock_path, body)
        .with_context(|| format!("failed to write {}", lock_path.display()))?;
    println!(
        "{} wrote {}",
        "✓".green().bold(),
        lock_path.display().to_string().white().bold()
    );
    Ok(())
}
