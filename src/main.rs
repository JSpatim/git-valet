mod config;
mod git_helpers;
mod hooks;
mod aside;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "git-aside",
    about = "Version specific files in a separate private repo, transparently alongside your usual git commands",
    long_about = "git-aside — version sensitive files in a separate private repo,\ntransparently alongside your usual git commands."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize an aside repo for this project
    Init {
        /// Remote of the aside repo (e.g. git@github.com:user/project-private.git)
        remote: String,
        /// Files/directories to track in the aside repo
        files: Vec<String>,
    },
    /// Show the aside repo status
    Status,
    /// Synchronize the aside repo (add + commit + push)
    Sync {
        #[arg(short, long, default_value = "chore: sync aside")]
        message: String,
    },
    /// Push the aside repo
    Push,
    /// Pull the aside repo
    Pull,
    /// Add files to the aside repo
    Add {
        files: Vec<String>,
    },
    /// Remove git-aside from this project (hooks + config)
    Deinit,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { remote, files } => {
            if files.is_empty() {
                anyhow::bail!("Specify at least one file to track. Example: git aside init <remote> CLAUDE.md .claude/");
            }
            aside::init(&remote, &files)?;
        }
        Commands::Status => {
            aside::status()?;
        }
        Commands::Sync { message } => {
            aside::sync(&message)?;
        }
        Commands::Push => {
            aside::push()?;
        }
        Commands::Pull => {
            aside::pull()?;
        }
        Commands::Add { files } => {
            aside::add_files(&files)?;
        }
        Commands::Deinit => {
            aside::deinit()?;
        }
    }

    Ok(())
}
