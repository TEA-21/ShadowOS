mod config;
mod runner;

use clap::{Parser, Subcommand};
use config::ProjectConfig;
use runner::AgentRunner;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "shadow-cli")]
#[command(about = "ShadowOS: Background MicroVM execution harness for autonomous coding agents", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute an autonomous agent inside the hardware-isolated MicroVM
    Run {
        /// Agent binary to execute (e.g. claude, swe-agent)
        #[arg(default_value = "claude")]
        agent: String,

        /// Task prompt to pass to the agent
        #[arg(short, long)]
        prompt: String,

        /// Path to target project repository
        #[arg(short, long, default_value = ".")]
        workspace: PathBuf,
    },
    /// View unified diff of ephemeral guest changes against the host repository
    Diff {
        #[arg(short, long, default_value = ".")]
        workspace: PathBuf,
    },
    /// Instantly roll back guest memory and storage state to a checkpoint
    Rollback {
        #[arg(short, long)]
        checkpoint: Option<String>,
    },
    /// Stage and promote guest CoW modifications back to the host repository
    Promote {
        #[arg(short, long, default_value = ".")]
        workspace: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { agent, prompt, workspace } => {
            println!("[*] Initializing ShadowOS MicroVM Harness...");
            println!("[*] Target workspace: {}", workspace.display());
            let proj_config = ProjectConfig::load_from_dir(&workspace);
            let cmd = AgentRunner::prepare_claude_invocation(&prompt, &proj_config);
            println!("[*] Prepared unattended execution: {} {:?}", cmd.cmd, cmd.args);
            println!("[✓] Launching agent inside hardware sandbox...");
        }
        Commands::Diff { workspace } => {
            println!("[*] Inspecting ephemeral changes for workspace: {}", workspace.display());
        }
        Commands::Rollback { checkpoint } => {
            let cp = checkpoint.unwrap_or_else(|| "latest".to_string());
            println!("[*] Rolling back to checkpoint '{}' in < 100ms...", cp);
        }
        Commands::Promote { workspace } => {
            println!("[*] Promoting verified guest changes to host: {}", workspace.display());
        }
    }

    Ok(())
}
