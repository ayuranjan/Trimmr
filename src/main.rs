mod commands;
mod filters;

use anyhow::Result;
use clap::{Parser, Subcommand};

use commands::{cost::handle_cost_git, git::handle_git};

#[derive(Parser)]
#[command(
    name = "trimr",
    version,
    about = "Proxy shell commands and compress output to save LLM tokens"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a git subcommand with compact, token-efficient output
    #[command(disable_help_flag = true)]
    Git {
        /// git subcommand (status, diff, log, add, commit, push, pull, …)
        #[arg(allow_hyphen_values = true)]
        sub: String,
        /// Additional arguments forwarded to git
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Show raw vs filtered comparison with token counts
    Cost {
        #[command(subcommand)]
        tool: CostTool,
    },
}

#[derive(Subcommand)]
enum CostTool {
    /// Compare raw vs filtered output for a git subcommand
    #[command(disable_help_flag = true)]
    Git {
        /// git subcommand
        #[arg(allow_hyphen_values = true)]
        sub: String,
        /// Additional arguments forwarded to git
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Git { sub, args } => handle_git(&sub, &args)?,
        Commands::Cost {
            tool: CostTool::Git { sub, args },
        } => handle_cost_git(&sub, &args)?,
    }

    Ok(())
}
