use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

use anyhow::{Ok, Result};
use debug_print::debug_println;
use rand::rngs::OsRng;

use crate::{
    create_and_init_mint, create_market, mint_to_new_account, write_file, COIN_MINT, CONFIG_DIR,
    MARKET_PUBKEY, PC_MINT, COIN_WALLET, PC_WALLET,
};

pub fn initialize(client: &RpcClient, program_id: &Pubkey, payer: &Keypair) -> Result<()> {
    let coin_mint = Keypair::generate(&mut OsRng);
    debug_println!("Coin mint: {}", coin_mint.pubkey());
    create_and_init_mint(client, payer, &coin_mint, &payer.pubkey(), 3)?;

    let pc_mint = Keypair::generate(&mut OsRng);
    debug_println!("Pc mint: {}", pc_mint.pubkey());
    create_and_init_mint(client, payer, &pc_mint, &payer.pubkey(), 3)?;

    debug_println!("Minting coin...");
    let coin_wallet = mint_to_new_account(
        client,
        payer,
        payer,
        &coin_mint.pubkey(),
        1_000_000_000_000_000,
    )?;
    debug_println!("Minted {}", coin_wallet.pubkey());

    debug_println!("Minting price currency...");
    let pc_wallet = mint_to_new_account(
        client,
        payer,
        payer,
        &pc_mint.pubkey(),
        1_000_000_000_000_000,
    )?;
    debug_println!("Minted {}", pc_wallet.pubkey());

    let market_keys = create_market(
        client,
        program_id,
        payer,
        &coin_mint.pubkey(),
        &pc_mint.pubkey(),
        1_000_000,
        10_000,
    )?;
    debug_println!("Market keys: {:#?}", market_keys);

    // saving pubkeys into json files
    let result = write_file(
        CONFIG_DIR,
        COIN_MINT,
        coin_mint.pubkey().to_string().as_str()
    );

    if result.is_err() {
        debug_println!("{:?}", result.err());
    }

    let result = write_file(
        CONFIG_DIR, 
        PC_MINT, 
        pc_mint.pubkey().to_string().as_str()
    );

    if result.is_err() {
        debug_println!("{:?}", result.err());
    }

    let result = write_file(
        CONFIG_DIR,
        COIN_WALLET,
        coin_wallet.pubkey().to_string().as_str()
    );

    if result.is_err() {
        debug_println!("{:?}", result.err());
    }

    let result = write_file(
        CONFIG_DIR, 
        PC_WALLET, 
        pc_wallet.pubkey().to_string().as_str()
    );

    if result.is_err() {
        debug_println!("{:?}", result.err());
    }

    let result = write_file(
        CONFIG_DIR,
        MARKET_PUBKEY,
        market_keys.market.to_string().as_str()
    );

    if result.is_err() {
        debug_println!("{:?}", result.err());
    }

    Ok(())
}
