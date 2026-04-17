pub mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a .hbp or .has file
    Run {
        file: String,
        #[arg(short, long)]
        model: Option<String>,
        #[arg(long)]
        force: bool, // Force regeneration even if lock matches
        #[arg(long)]
        verbose: bool, // Enable verbose logging
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
}
