mod cmds;
mod opts;
mod utils;
pub use cmds::*;
use crate::cmds::{key::handle_key_subcommand, handle_rgb_subcommand};
use crate::opts::{
    Cli, Command
};

use clap::Parser;
use serde_json::{json, Value as JsonValue};

// 1040 / 630 = 1.65
pub const FEE_FACTOR: f32 = 1.65;

fn main() {
    env_logger::init();
    let cli = Cli::parse();
    log::debug!("cli: {:?}", cli);
    match handle_command(cli) {
        Ok(JsonValue::Null) => {}
        Ok(value) => println!("{}", serde_json::to_string_pretty(&value).unwrap()),
        Err(err) => {
            eprintln!("ERROR: {}", err);
            std::process::exit(-1);
        }
    }
}

pub fn handle_command(cli: Cli) -> Result<JsonValue, anyhow::Error> {
    let network = cli.network;
    match cli.command {
        
        Command::Key { subcommand } => handle_key_subcommand(subcommand),
        
        
        Command::Rgb {
            stock_database,
            wallet_name,
            master_prv,
            electrum,
            subcommand,
        } => {
            
            handle_rgb_subcommand(&stock_database, &wallet_name, &master_prv, &network, electrum, subcommand)

        }
    }
}
