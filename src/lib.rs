pub mod dex;
pub mod initial_setup;
pub mod order;
pub mod utils;

pub use dex::*;
pub use initial_setup::*;
pub use order::*;
pub use utils::*;

use solana_sdk::{pubkey::Pubkey, signature::Keypair};

pub const CONFIG_DIR: &str = "configs";

pub const COIN_MINT: &str = "coin_mint.json";
pub const COIN_WALLET: &str = "coin_wallet.json";
pub const PC_MINT: &str = "pc_mint.json";
pub const PC_WALLET: &str = "pc_wallet.json";
pub const MARKET_PUBKEY: &str = "market_pubkey.json";
pub const OPEN_ORDER: &str = "open_order_pubkey.json";

pub const URL: &str = "url.json";
pub const PROGRAM_ID: &str = "program_id.json";
pub const WALLET: &str = "wallet.json";


pub fn info() {
    if !std::path::Path::new(CONFIG_DIR).exists() {
        println!("There is no information!");
        return;
    }

    for entry in std::fs::read_dir(CONFIG_DIR).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_file() {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            let full_path = CONFIG_DIR.to_string() + "/" + file_name;
            let result = read_file(full_path.as_str());

            if file_name == COIN_MINT {
                println!("Coint mint: {}", result.unwrap());
            } else if file_name == COIN_WALLET {
                println!("Coin wallet: {}", result.unwrap());
            } else if file_name == PC_MINT {
                println!("Pc mint: {}", result.unwrap());
            } else if file_name == PC_WALLET {
                println!("Pc wallet: {}", result.unwrap());
            } else if file_name.starts_with(WALLET) {
                println!("Wallet: {}", result.unwrap());
            } else if file_name == URL {
                println!("URL: {}", result.unwrap());
            } else if file_name == PROGRAM_ID {
                println!("Program ID: {}", result.unwrap());
            } else if file_name == MARKET_PUBKEY {
                println!("Market pubkey: {}", result.unwrap());
            } else if file_name.starts_with(OPEN_ORDER) {
                println!("Open order pubkey: {}", result.unwrap());
            }
        }
    }
}

#[derive(Debug)]
pub struct MarketPubkeys {
    pub market: Box<Pubkey>,
    pub req_q: Box<Pubkey>,
    pub event_q: Box<Pubkey>,
    pub bids: Box<Pubkey>,
    pub asks: Box<Pubkey>,
    pub coin_vault: Box<Pubkey>,
    pub pc_vault: Box<Pubkey>,
    pub vault_signer_key: Box<Pubkey>,
}

pub struct ListingKeys {
    market_key: Keypair,
    req_q_key: Keypair,
    event_q_key: Keypair,
    bids_key: Keypair,
    asks_key: Keypair,
    vault_signer_pk: Pubkey,
    vault_signer_nonce: u64,
}
