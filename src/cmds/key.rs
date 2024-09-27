use std::{path::PathBuf, str::FromStr};
use super::*;

use bhlx_rgb::XpubDerivable;
pub use bitcoin::{
    consensus,
    hashes::{sha256, Hash},
    util::{
        bip32::{ChildNumber, DerivationPath, ExtendedPrivKey, ExtendedPubKey},
        key::Secp256k1,
    },
};
use clap::Subcommand;
use rand::RngCore;
use serde_json::{json};

#[derive(Debug, Subcommand, Clone, PartialEq, Eq)]
pub enum KeySubCommand {
    /// Generates new random seed mnemonic phrase and corresponding master extended key.
    Generate ,

    /// Derive a child key pair from a master extended key and a derivation path string (eg. "m/84'/1'/0'/0" or "m/84h/1h/0h/0").
    GenerateWallet {
        #[clap(name = "descriptor", short = 'd', long = "desc")]
        descriptor_type: String,
        /// Extended private key to derive from.
        #[clap(name = "XPRV", short = 'x', long = "xprv")]
        xprv: String,
        // Stock: that is your rgb database in this repo
        #[clap(name = "stock", short = 's', long = "stock")]
        stock: String,
        // Wallet: bitcoin wallet that is used for rgb running, sync utxo
        #[clap(name = "wallet", short = 's', long = "wallet")]
        wallet: String,
    },
}

#[derive(Debug, Copy, Clone)]
pub enum DescriptorType {
    Wpkh,
    Tr,
}




pub fn handle_key_subcommand(
    subcommand: KeySubCommand,
) -> Result<serde_json::Value, anyhow::Error> {
    match subcommand {
        
        KeySubCommand::Generate => {
            let mut seed = vec![0u8; 128];
            rand::thread_rng().fill_bytes(&mut seed);

            let secp = Secp256k1::new();
            let master_xpriv = ExtendedPrivKey::new_master(bitcoin::Network::Testnet, &seed).unwrap();

            let master_xpub = ExtendedPubKey::from_priv(&secp, &master_xpriv);
            let fingerprint = master_xpriv.fingerprint(&secp);
            
            Ok(
                json!({"xpub": master_xpub.to_string(), "xprv": master_xpriv.to_string(), "fingerprint": fingerprint.to_string() }),
            )
        }

        // generate your extended public key and private key
        KeySubCommand::GenerateWallet { 
            descriptor_type,
             xprv,
             stock,
            wallet
        } => {
            let descriptor = if descriptor_type == "taproot" {
                DescriptorType::Tr
            } else {
                DescriptorType::Wpkh
            };

            let account = crate::utils::generate_sign_account(&xprv);

            let derivation_account = account.to_account();
            let derivation_account_rgb = derivation_account
                .to_string()
                .replace("/*/*", "/<0;1;9;10>/*");
            let xpub_derivable = XpubDerivable::from_str(&derivation_account_rgb).unwrap();

            let descriptor = match descriptor {
                DescriptorType::Wpkh => RgbDescr::Wpkh(Wpkh::from(xpub_derivable)),
                DescriptorType::Tr => RgbDescr::TapretKey(TapretKey::from(xpub_derivable)),
            };
            let provider = FsTextStore::new(PathBuf::from_str(&wallet).unwrap()).unwrap();
            let mut btc_wallet = Wallet::new_layer1(descriptor.clone(), Network::Testnet3);
            // let name = s!("wallet");
            btc_wallet.make_persistent(provider, true).unwrap();
            btc_wallet.set_name(wallet.clone());
            if let Err(err) = btc_wallet.store() {
                println!("error: {err}");
            } else {
                println!("success");
            }
            let bhlx_stock = BhlxStock::in_memory();
            let store = BhlxWallet::attach(stock.as_str(), None, bhlx_stock, btc_wallet);

            let wallet = BiHelixWallet {
                wallet: store,
                account,
                stock: stock.clone(),
                wallet_name: wallet.to_string()
            };
            wallet.store_wallet(stock.as_str());
            eprintln!("generate bihelix done");
            Ok(json!("done"))
        }
    }
}
