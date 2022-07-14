pub mod dex;
pub mod market;
pub mod order;
pub mod utils;

pub use dex::*;
pub use market::*;
pub use order::*;
pub use utils::*;

use solana_sdk::pubkey::Pubkey;

pub const CONFIG_DIR: &str = "configs";

pub const COIN_MINT: &str = "coin_mint.json";
pub const PC_MINT: &str = "pc_mint.json";

pub const MARKET_PUBKEY: &str = "market_pubkey.json";
pub const OPEN_ORDER: &str = "open_order_pubkey.json";

pub const URL: &str = "url.json";
pub const PROGRAM_ID: &str = "program_id.json";
pub const WALLET: &str = "wallet.json";

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
