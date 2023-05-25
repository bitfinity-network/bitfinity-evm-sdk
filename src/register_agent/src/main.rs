use anyhow::{Ok, Result};
use clap::Parser;
use cli::{generate_wallet, Commands, RegisterMinterCli};

#[macro_use]
extern crate log;

mod agent;
mod cli;
mod constant;
mod error;
mod registration;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = RegisterMinterCli::parse();

    match cli.command {
        Commands::GenerateWallet => {
            generate_wallet()?;
            Ok(())
        }
        Commands::Register(register_args) => register_args.exec().await,
    }
}
