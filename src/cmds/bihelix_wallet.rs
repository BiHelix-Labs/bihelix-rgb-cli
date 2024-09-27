use std::collections::HashSet;

use std::path::PathBuf;
use std::str::FromStr;

use amplify::{
    Display,
};
use bpwallet::AnyIndexer;
use dw_psbt::serialize::Deserialize;
use ifaces::IssuerWrapper;
use psrgbt::{Psbt, PsbtVer};

use bp::seals::txout::CloseMethod;
use clap::{Subcommand};
use rgb::containers::ConsignmentExt;
use rgb::interface::{IfaceClass};
use rgb::invoice::Pay2Vout;
use rgb::resolvers::AnyResolver;
use rgb::validation::Validity;
use rgb::{Identity, Precision};
use rgbstd::containers::{UniversalFile, Transfer, Kit, ValidKit, ValidContract, ValidTransfer};
use rgbstd::{ContractId, GraphSeal};
use rgbstd::interface::{FilterExclude, ContractIface};
use rgbstd::invoice::{Beneficiary, RgbInvoiceBuilder, XChainNet};
use rgbstd::persistence::{MemContractState, MemContract};
use rgbstd::invoice::RgbInvoice;
use rgbstd::XChain;
use schemata::NonInflatableAsset;
use serde_json::Value::Null as JsonNull;
use strict_types::encoding::{FieldName, TypeName};
use strict_types::{fname, tn};
use super::*;

pub struct BiHelixWallet {
    pub wallet: BhlxWallet<Wallet<XpubDerivable, RgbDescr>>,
    pub account: MemorySigningAccount,
    pub stock: String,
    pub wallet_name: String,
}
const ELECTRUM_URL: &str = "bihelix-testnet-electrs.iftas.tech:50001";


fn get_indexer() -> AnyIndexer {
    AnyIndexer::Electrum(Box::new(Client::new(ELECTRUM_URL).unwrap()))
}

pub fn get_resolver() -> AnyResolver {
    AnyResolver::electrum_blocking(ELECTRUM_URL, None).unwrap()
}

fn broadcast_tx(indexer: &AnyIndexer, tx: &[u8]) {
    match indexer {
        AnyIndexer::Electrum(inner) => {
            inner
                .transaction_broadcast_raw(tx)
                .unwrap();
        }
        AnyIndexer::Esplora(_) => {
            // inner.broadcast(tx).unwrap();
        }
        _ => unreachable!("unsupported indexer"),
    }
}

impl BiHelixWallet {

    pub fn store_wallet(&self, name: &str) {
        self.wallet.store_wallet_by_rocksdb(name);
    }

    pub fn contract_iface(
        &self,
        contract_id: ContractId,
        iface_type_name: &TypeName,
    ) -> ContractIface<MemContract<&MemContractState>> {
        self.wallet
            .stock()
            .contract_iface(contract_id, iface_type_name.clone())
            .expect("get contract iface failed")
    }

    pub fn transfer(
        &mut self,
        invoice: RgbInvoice,
        sats: Option<u64>,
        fee: Option<u64>,
        broadcast: bool,
    ) -> Transfer {
        // self.sync();
        let fee = Sats::from_sats(fee.unwrap_or(3000));
        let sats = Sats::from_sats(sats.unwrap_or(2000));
        let params = TransferParams::with(fee, sats);
        let (psbt, _psbt_meta, _, consignment) = self.wallet.pay(&invoice, params).unwrap();

        let secp = Secp256k1::new();
        let mut key_provider = MemoryKeyProvider::with(&secp, true);
        key_provider.add_account(self.account.clone());
        let mut psbt = DwPsbt::deserialize(&psbt.serialize(PsbtVer::V0)).unwrap();
        let _ = psbt.sign_all(&key_provider).unwrap();
       
        let mut rgb_psbt = RgbPsbt::from_str(&psbt.to_string()).unwrap();
        let _ = rgb_psbt.finalize(self.wallet.wallet().descriptor());
        let tx = rgb_psbt.extract().unwrap();
        let txid = rgb_psbt.txid().to_string();
        let tx_bytes = tx.consensus_serialize();
        if broadcast {
            let indexer = get_indexer();
            broadcast_tx(&indexer, &tx_bytes);
        }

        // let txid = tx.txid().to_string();
        println!("transfer txid: {txid:?}");

        consignment
    }

    pub fn get_address(&self) -> String {
        self.wallet
            .wallet()
            .addresses(RgbKeychain::Rgb)
            .next()
            .expect("no addresses left")
            .addr
            .to_string()
    }

    pub fn get_utxo(&mut self) -> HashSet<Outpoint> {
        self.sync();

        let mut vout = None;
        let mut txid = None;
        let mut txid_set = HashSet::new();
        let bp_runtime = self.wallet.wallet();
        for (_, utxos) in bp_runtime.address_coins() {
            for utxo in utxos {
                    txid = Some(utxo.outpoint.txid.to_string());
                    vout = Some(utxo.outpoint.vout_u32());
                    
                    txid_set.insert(Outpoint {
                        txid: Txid::from_str(&txid.unwrap()).unwrap(),
                        vout: Vout::from_u32(vout.unwrap()),
                    });
            }
        }
        txid_set
    }

    pub fn sync(&mut self) {
        let indexer = get_indexer();

        eprint!("Syncing");
        if let Some(errors) = self.wallet.wallet_mut().update(&indexer).into_err() {
            eprintln!(" partial, some requests has failed:");
            for err in errors {
                eprintln!("- {err}");
            }
        } else {
            eprintln!(" success");
        }
        let provider = FsTextStore::new(PathBuf::from_str(&self.wallet_name).unwrap()).unwrap();
        self.wallet.wallet_mut().make_persistent(provider, true).unwrap();

        if let Err(err) = self.wallet.wallet_mut().store() {
            println!("error: {err}");
        } else {
            println!("success");
        }

    }

    pub fn import_contract(&mut self, contract: &ValidContract, testnet: bool) {
        let resolver = self.any_resolver(testnet);
        
        self.wallet
            .stock_mut()
            .import_contract(contract.clone(), resolver)
            .unwrap();
    }

    pub fn import_kit(&mut self, kit: ValidKit) {
        let status = self.wallet.stock_mut().import_kit(kit).unwrap();
        eprintln!("import kit status {:?}", status);
    }

    pub fn issue_nia(
        &mut self,
        identity: &str,
        ticker: &str,
        name: &str,
        precision: Precision,
        issued_supply: u64,
        close_method: CloseMethod,
        allocate_outpoint: Outpoint,
        testnet: bool
    ) -> ContractId {
        let mut kit = Kit::default();
        let _ = kit.ifaces.push(Rgb20::iface(&rgb20::Rgb20::FIXED));
        let _ = kit.iimpls.push(NonInflatableAsset::issue_impl());
        let _ = kit.schemata.push(NonInflatableAsset::schema());
        let _ = kit.scripts.extend(NonInflatableAsset::scripts().into_values());
        kit.types = NonInflatableAsset::types();
        let valid_kit = kit.validate().unwrap();
        self.import_kit(valid_kit);

        let contract = Rgb20Wrapper::<MemContract>::testnet::<NonInflatableAsset>(identity, ticker, name, None, precision)
        .expect("invalid contract data")
        .allocate(close_method, allocate_outpoint, issued_supply)
        .expect("invalid allocations")
        .issue_contract()
        .expect("invalid contract data");
    
        self.import_contract(&contract, testnet);
        self.store_wallet(&self.stock);
        contract.contract_id()
    }

    pub fn invoice(
        &mut self,
        contract_id: ContractId,
        iface_type_name: &TypeName,
        amount: u64,
        close_method: CloseMethod,
        outpoint: Option<Outpoint>,
        operation: FieldName
    ) -> RgbInvoice {
        let network = self.wallet.wallet().network();
        let beneficiary = match outpoint {
            Some(outpoint) => {
                let seal = XChain::Bitcoin(GraphSeal::new_random(
                    close_method,
                    outpoint.txid,
                    outpoint.vout,
                ));
                self.wallet.stock_mut().store_secret_seal(seal).unwrap();
                self.store_wallet(&self.stock);

                Beneficiary::BlindedSeal(*seal.to_secret_seal().as_reduced_unsafe())
            }
            None => {
                let address = self
                    .wallet
                    .wallet()
                    .addresses(RgbKeychain::Rgb)
                    .next()
                    .expect("no addresses left")
                    .addr;
                Beneficiary::WitnessVout(Pay2Vout {
                    address: address.payload,
                    method: close_method,
                })
            }
        };

        RgbInvoiceBuilder::new(XChainNet::bitcoin(network, beneficiary))
            .set_contract(contract_id)
            .set_interface(iface_type_name.clone())
            .set_amount_raw(amount)
            .set_operation(operation)
            .finish()
    }

    pub fn accept_transfer(&mut self, consignment: Transfer) {
        self.sync();
        let mut resolver = get_resolver();
        let validated_consignment = consignment.validate(&mut resolver, true).unwrap();
        let validation_status = validated_consignment.clone().into_validation_status();
        eprintln!("status {:?}", validation_status);
        let validity = validation_status.validity();
        assert_eq!(validity, Validity::Valid);
        let mut attempts = 0;
        while let Err(e) = self
            .wallet
            .stock_mut()
            .accept_transfer(validated_consignment.clone(), get_resolver())
        {
            attempts += 1;
            if attempts > 3 {
                panic!("error accepting transfer: {e}");
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        self.store_wallet(&self.stock);

    }

    pub fn query_state(&mut self, contract_id: ContractId, iface_type_name: &TypeName) {
        // self.sync();
        
        let contract = self.contract_iface(contract_id, iface_type_name);
        println!("Global:");
        
        for global in &contract.iface.global_state {
            if let Ok(values) = contract.global(global.name.clone()) {
                for val in values {
                    println!("  {} := {}", global.name, val);
                }
            }
        }
        println!("\nOwned:");
        for owned in &contract.iface.assignments {
            println!("  {}:", owned.name);
            if let Ok(allocations) =
                contract.fungible(owned.name.clone(), &self.wallet.wallet().filter())
            {
                for allocation in allocations {
                    println!(
                        "    amount={}, utxo={}, witness={:?} # owned by the wallet",
                        allocation.state.value(), allocation.seal, allocation.witness
                    );
                }
            }
            if let Ok(allocations) = contract.fungible(
                owned.name.clone(),
                &FilterExclude(&self.wallet.wallet().filter()),
            ) {
                for allocation in allocations {
                    println!(
                        "    amount={}, utxo={}, witness={:?} # owner unknown",
                        allocation.state.value(), allocation.seal, allocation.witness
                    );
                }
            }
        }
        let bp_runtime = self.wallet.wallet();
        println!("Balance of {}", bp_runtime.descriptor());
        println!("Balance {:?}", bp_runtime.balance());
        println!("\nHeight\t{:>12}\t{:68}", "Amount, ṩ", "Outpoint");
        for (derived_addr, utxos) in bp_runtime.address_coins() {
            println!("{:?}\t{:?}", derived_addr.addr, derived_addr.terminal);
            for row in utxos {
                println!("{:?}\t{: >12}\t{:68}", row.height, row.amount, row.outpoint);
            }
            println!()
        }
    }

    pub fn any_resolver(&self, testnet: bool) -> AnyResolver {
        let testnet_url = ELECTRUM_URL;
        let mainnet_url = "blockstream.info:110";
        let resolver = if testnet {
            let resolver = AnyResolver::electrum_blocking(testnet_url, None).unwrap();
            resolver.check(Network::Testnet3).unwrap();
            resolver
        } else {
            let resolver = AnyResolver::electrum_blocking(mainnet_url, None).unwrap();
            resolver.check(Network::Mainnet).unwrap();
            resolver
        };

        resolver
    }
}


/// BiHelix wallet operation subcommands


#[derive(Subcommand, Clone, PartialEq, Eq, Debug, Display)]
#[display(lowercase)]
#[allow(clippy::large_enum_variant)]
pub enum BiHelixSubCommand {
 
    /// Prints out list of known RGB contracts
    Contracts,

    /// Reports information about state of a contract
    #[display("state")]
    State {
        /// Show all state - not just the one owned by the wallet
        #[clap(short, long)]
        all: bool,

        /// Contract identifier
        contract_id: ContractId,

        /// Interface to interpret the state data
        iface: String,

        /// address conflict with all
        #[clap(long, conflicts_with = "all")]
        address: Option<String>,
    },

    /// Issues new contract
    #[display("issue")]
    Issue {
        /// Schema name to use for the contract
        ticker: String, //String,
        name: String,
        issued_supply: u64,
        /// File containing contract genesis description in YAML format
        inflation_allowance: u64,
        method: String,
    },

    /// Create new invoice
    #[display("invoice")]
    Invoice {
        /// Contract identifier
        contract_id: ContractId,

        /// Interface to interpret the state data
        iface: String,

        /// Value to transfer
        value: u64,
        // Outpoint input to the invoice
        seal: String,
        // Method which can be opret or tapret depend on the input
        #[clap(name = "method", short = 'm', long = "method")]
        method: String,
        // Operation to the invoice, such as 'transfer'
        #[clap(name = "operation", short = 'o', long = "operation")]
        operation: String,
    },


    /// Validate transfer consignment & accept to the stash
    #[display("accept")]
    Accept {

        /// File with the transfer consignment
        file: PathBuf,
    },
    
}

const STATE: &str = "state";
const STASH: &str = "stash";
const INDEX: &str = "index";

pub fn load_rgb_data_from_rocksdb(db_name: &str) -> BhlxStock {
    let db = DB::open_default(db_name).unwrap();
    let stash_data = if let Some(value) = db.get(STASH).unwrap() {
        // eprintln!("stash {:?}", value);
        value
    } else {
        vec![]
    };
    let index_data = if let Some(value) = db.get(INDEX).unwrap() {
        // eprintln!("index {:?}", value);
        value
    } else {
        vec![]
    };
    let state_data = if let Some(value) = db.get(STATE).unwrap() {
        // eprintln!("state {:?}", value);
        value
    } else {
        vec![]
    };
    if stash_data.len() == 0 || index_data.len() ==0 || state_data.len() == 0 {
        BhlxStock::in_memory()
    } else {
        BhlxStock::<MemStash, MemState, MemIndex>::load_stock_bytes(&stash_data, &index_data, &state_data).map_err(|err| eprintln!("err {:?}", err)).expect("deseralize failed")
    }

}



pub fn handle_rgb_subcommand(
    stock_database: &str,
    wallet_name: &str,
    master_prv: &str,
    network: &bpwallet::Network,
    electrum: Option<String>,
    subcommand: BiHelixSubCommand,
) -> Result<serde_json::Value, anyhow::Error> {
    let electrum = electrum.as_deref().unwrap_or_else(|| match network {
        Network::Mainnet => "blockstream.info:110",
        Network::Testnet3 => "blockstream.info:143",
        _ => {
            eprint!("No electrum server for this network");
            std::process::exit(1);
        }
    });
    let mut bhlx_wallet = crate::utils::load_wallet(stock_database, wallet_name, master_prv);

    match subcommand {
    
        BiHelixSubCommand::State {
            all: _,
            contract_id,
            iface,
            address: _,
        } => {
            bhlx_wallet.query_state(contract_id, &tn!(iface));
            Ok(JsonNull)
        }
        BiHelixSubCommand::Issue { ticker, name, issued_supply, inflation_allowance: _, method } => {
            let testnet = true;
            let allocate_outpoint = bhlx_wallet.get_utxo();
            let vec_utxo = allocate_outpoint.iter().collect::<Vec<_>>();
            let close_method = if method == "opret" {
                CloseMethod::OpretFirst
            } else {
                CloseMethod::TapretFirst
            };
            // the Precision just set to default ceti now.
            let contract_id = bhlx_wallet.issue_nia(&Identity::default().to_string(), &ticker, &name, Precision::Centi, issued_supply, close_method, vec_utxo[0].clone(), testnet);
            eprintln!(
                "A new contract {contract_id} is issued and added to the database.\n"
            );
            Ok(JsonNull)
        }

        BiHelixSubCommand::Accept { file } => {
            let bindle = UniversalFile::load_file(file)?;
            let consignment = match bindle {
                UniversalFile::Transfer(transfer) => transfer,
                UniversalFile::Contract(_) => todo!(),
                UniversalFile::Kit(_) => todo!(),
            };
            bhlx_wallet.accept_transfer(consignment);
            
            eprintln!("Transfer accepted into the database");
            Ok(JsonNull)
        }
        
        BiHelixSubCommand::Invoice {
            contract_id,
            iface,
            value,
            seal: _,
            method,
            operation,
        } => {
            let allocate_outpoint = bhlx_wallet.get_utxo();
            let vec_utxo = allocate_outpoint.iter().collect::<Vec<_>>();
            // now the opret method is only supported here
            let close_method = if method == "opret" {
                CloseMethod::OpretFirst
            } else {
                CloseMethod::TapretFirst
            };
            let invoice = bhlx_wallet.invoice(contract_id, &tn!(iface), value, close_method, Some(vec_utxo[0].clone()), fname!(operation));
            println!("{invoice}");
            Ok(JsonNull)
        }
        BiHelixSubCommand::Contracts => {
            let contracts = match bhlx_wallet.wallet.stock().contracts() {
                Ok(contract) => contract,
                Err(_) => todo!(),
            };
            contracts.into_iter().for_each(|info| eprintln!("contract id {:?}", info.id));
            Ok(JsonNull)

        },
    }
}
