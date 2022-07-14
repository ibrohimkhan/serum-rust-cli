use bytemuck::bytes_of;
use debug_print::debug_println;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    message::Message, program_error::ProgramError, pubkey::Pubkey, signature::Keypair,
    signer::Signer, system_instruction, transaction::Transaction,
};

use crate::{MarketPubkeys, CONFIG_DIR, COIN_MINT, PC_MINT, WALLET, PROGRAM_ID, URL, MARKET_PUBKEY, OPEN_ORDER, read_file};
use anyhow::Result;

pub fn new(
    client: &RpcClient,
    payer: &Keypair,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    base_lot_size: u64,
    quote_lot_size: u64,
    dex_program_id: &Pubkey,
) -> Result<MarketPubkeys> {

    // Generating keypairs
    let market = Keypair::new();
    let request_queue = Keypair::new();
    let event_queue = Keypair::new();
    let bids = Keypair::new();
    let asks = Keypair::new();
    let base_vault = Keypair::new();
    let quote_vault = Keypair::new();
    let quote_dust_threshold: u64 = 100;

    debug_println!("generating vault owner...");
    let (vault_signer_nonce, vault_owner) = {
        let mut nonce: u64 = 0;
        loop {
            assert!(nonce < 100);
            if let Ok(pk) = gen_vault_signer_key(nonce, &market.pubkey(), dex_program_id) {
                break (nonce, pk);
            }

            nonce += 1;
        }
    };
    debug_println!("vault owner pubkey: {:?}", vault_owner);

    let data_len = <spl_token::state::Account as solana_sdk::program_pack::Pack>::LEN;
    let lamports = client.get_minimum_balance_for_rent_exemption(data_len)?;

    let base_vault_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &base_vault.pubkey(),
        lamports,
        data_len as u64,
        &spl_token::ID,
    );

    let base_vault_init_account_ix = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &base_vault.pubkey(),
        base_mint,
        &vault_owner,
    )?;

    let recent_blockhash = client.get_latest_blockhash()?;
    let base_vault_tx = Transaction::new_signed_with_payer(
        &[base_vault_account_ix, base_vault_init_account_ix],
        Some(&payer.pubkey()),
        &[&payer, &base_vault],
        recent_blockhash,
    );

    debug_println!("sending transaction to create base vault...");
    let signature = client.send_and_confirm_transaction(&base_vault_tx)?;
    debug_println!(
        "base vault transaction confirmed with signature: {:?}",
        signature
    );

    let quote_vault_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &quote_vault.pubkey(),
        lamports,
        data_len as u64,
        &spl_token::ID,
    );

    let quote_vault_init_account_ix = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &quote_vault.pubkey(),
        quote_mint,
        &vault_owner,
    )?;

    let recent_blockhash = client.get_latest_blockhash()?;
    let quote_vault_tx = Transaction::new_signed_with_payer(
        &[quote_vault_account_ix, quote_vault_init_account_ix],
        Some(&payer.pubkey()),
        &[&payer, &quote_vault],
        recent_blockhash,
    );

    debug_println!("sending transaction to create quote vault...");
    let signature = client.send_and_confirm_transaction(&quote_vault_tx)?;
    debug_println!(
        "quote vault transaction confirmed with signature: {:?}",
        signature
    );

    debug_println!("\ncreating accounts and initializing market...");
    let data_len = 376 + 12;
    let market_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &market.pubkey(),
        client.get_minimum_balance_for_rent_exemption(data_len)?,
        data_len as u64,
        dex_program_id,
    );

    let data_len = 640 + 12;
    let request_queue_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &request_queue.pubkey(),
        client.get_minimum_balance_for_rent_exemption(data_len)?,
        data_len as u64,
        dex_program_id,
    );

    let data_len = (1 << 20) + 12;
    let event_queue_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &event_queue.pubkey(),
        client.get_minimum_balance_for_rent_exemption(data_len)?,
        data_len as u64,
        dex_program_id,
    );

    let data_len = (1 << 16) + 12;
    let bids_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &bids.pubkey(),
        client.get_minimum_balance_for_rent_exemption(data_len)?,
        data_len as u64,
        dex_program_id,
    );

    let data_len = (1 << 16) + 12;
    let asks_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &asks.pubkey(),
        client.get_minimum_balance_for_rent_exemption(data_len)?,
        data_len as u64,
        dex_program_id,
    );

    let market_initialize_ix = serum_dex::instruction::initialize_market(
        &market.pubkey(),
        &dex_program_id,
        &base_mint,
        &quote_mint,
        &base_vault.pubkey(),
        &quote_vault.pubkey(),
        None,
        None,
        None,
        &bids.pubkey(),
        &asks.pubkey(),
        &request_queue.pubkey(),
        &event_queue.pubkey(),
        base_lot_size,
        quote_lot_size,
        vault_signer_nonce,
        quote_dust_threshold,
    )?;

    let message = Message::new(
        &[
            market_account_ix,
            request_queue_account_ix,
            event_queue_account_ix,
            bids_account_ix,
            asks_account_ix,
            market_initialize_ix,
        ],
        Some(&payer.pubkey()),
    );

    let signers = vec![payer, &market, &request_queue, &event_queue, &bids, &asks];

    let mut transaction = Transaction::new_unsigned(message);
    let blockhash = client.get_latest_blockhash()?;

    transaction.sign(&signers, blockhash);

    let signature = client.send_and_confirm_transaction(&transaction)?;
    debug_println!("Market is initialized with signature: {:?}\n", signature);

    Ok(MarketPubkeys {
        market: Box::new(market.pubkey()),
        req_q: Box::new(request_queue.pubkey()),
        event_q: Box::new(event_queue.pubkey()),
        bids: Box::new(bids.pubkey()),
        asks: Box::new(asks.pubkey()),
        coin_vault: Box::new(base_vault.pubkey()),
        pc_vault: Box::new(quote_vault.pubkey()),
        vault_signer_key: Box::new(vault_owner),
    })
}

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
            } else if file_name == PC_MINT {
                println!("Pc mint: {}", result.unwrap());
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

fn gen_vault_signer_key(
    nonce: u64,
    market: &Pubkey,
    program_id: &Pubkey,
) -> Result<Pubkey, ProgramError> {
    let seeds = gen_vault_signer_seeds(&nonce, market);
    Ok(Pubkey::create_program_address(&seeds, program_id)?)
}

fn gen_vault_signer_seeds<'a>(nonce: &'a u64, market: &'a Pubkey) -> [&'a [u8]; 2] {
    [market.as_ref(), bytes_of(nonce)]
}
