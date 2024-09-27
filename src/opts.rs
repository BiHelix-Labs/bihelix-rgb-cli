#![allow(clippy::large_enum_variant)]

use clap::{Parser, Subcommand, ValueHint};

use crate::cmds::{key::KeySubCommand, bihelix_wallet::BiHelixSubCommand};

pub const STOCK_DATABASE: &str = "stock";
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long)]
    pub verbose: bool,

    /// Sets the network. bitcoin|testnet|regtest|signet.
    #[clap(
        name = "NETWORK",
        short = 'n',
        long = "network",
        default_value = "testnet"
    )]
    pub network: bpwallet::Network,

    /// Command to execute.
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
  

    /// Execute key commands.
    Key {
        #[clap(subcommand)]
        subcommand: KeySubCommand,
    },

    /// RGB operations
    Rgb {
        /// Data directory path.
        ///
        /// Path to the directory that contains RGB stored data.
        #[clap(
            short = 'd',
            long,
            default_value = STOCK_DATABASE,
            value_hint = ValueHint::DirPath
        )]
        stock_database: String,
        wallet_name: String,
        master_prv: String,

        /// Electrum server to use.
        #[clap(short = 's', long)]
        electrum: Option<String>,

        #[clap(subcommand)]
        subcommand: BiHelixSubCommand,
    },
}
