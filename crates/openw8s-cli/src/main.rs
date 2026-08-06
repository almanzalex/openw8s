mod gguf;
mod init;
mod inspect;
mod manifest;
mod run;
mod validate;
mod vram;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "openw8s",
    version,
    about = "Open Weights Spec — inspect models, generate manifests, run environments",
    long_about = "openw8s standardizes environments, VRAM profiling, and lineage for open-weight AI models."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Inspect a Hugging Face model and print a VRAM requirement matrix
    Inspect {
        /// Hugging Face repo id or URL (e.g. Qwen/Qwen2.5-7B-Instruct)
        repo_id: String,
    },
    /// Interactively generate a `.openw8s.yml` manifest in the current directory
    Init {
        /// Overwrite an existing `.openw8s.yml` without prompting
        #[arg(long, short = 'f')]
        force: bool,
    },
    /// Validate a `.openw8s.yml` against the v0.1 schema
    Validate {
        /// Path to a manifest file (defaults to ./.openw8s.yml)
        #[arg(long, short = 'm')]
        manifest: Option<PathBuf>,
    },
    /// Read `.openw8s.yml` and spawn the configured runtime command
    Run {
        /// Path to a manifest file (defaults to ./.openw8s.yml)
        #[arg(long, short = 'm')]
        manifest: Option<PathBuf>,
        /// Print execution parameters without spawning the process
        #[arg(long)]
        dry_run: bool,
        /// Override VRAM preflight failure when free memory is below min_vram_gb
        #[arg(long, short = 'f')]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect { repo_id } => inspect::inspect(&repo_id).await?,
        Commands::Init { force } => init::init(force)?,
        Commands::Validate { manifest } => validate::validate(manifest.as_deref())?,
        Commands::Run {
            manifest,
            dry_run,
            force,
        } => run::run(manifest.as_deref(), dry_run, force).await?,
    }

    Ok(())
}
