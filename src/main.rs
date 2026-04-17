use huii::cli::{Cli, Commands, commands};
use clap::Parser;
use std::process;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Run { file, model, force, args, verbose }) => {
            commands::execute_run(&file, model, force, args, verbose).await
        }
        None => {
            println!("No command specified. Use --help for usage.");
            return;
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        process::exit(1);
    }
}
