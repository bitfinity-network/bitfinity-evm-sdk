use anyhow::{Ok, Result};
use clap::Parser;
use cli::{get_wallet, Commands, RegisterMinterCli};

mod agent;
mod cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = RegisterMinterCli::parse();

    match cli.command {
        Commands::GenerateWallet => {
            get_wallet(None)?;
            Ok(())
        }
        Commands::Register(register_args) => register_args.exec().await,
    }
}
