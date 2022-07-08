use anyhow::{format_err, Result};
use std::str::FromStr;
use std::{borrow::Cow, mem::size_of};

use debug_print::debug_println;
use rand::rngs::OsRng;
use safe_transmute::*;
use std::convert::identity;

use crate::{read_file, ListingKeys, MarketPubkeys, OPEN_ORDER, write_file, CONFIG_DIR};
use serum_dex::instruction::init_open_orders as init_open_orders_ix;
use serum_dex::state::{gen_vault_signer_key, AccountFlag, Market, MarketState, MarketStateV2};

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};

use spl_token::instruction::{self as token_instruction};

pub fn get_open_order_pubkey(
    client: &RpcClient,
    program_id: &Pubkey,
    owner: &Keypair,
    state: &MarketPubkeys,
) -> Result<Pubkey> {

    let mut orders = None;
    let path = CONFIG_DIR.to_string() + "/" + owner.pubkey().to_string().as_str() + "_" + OPEN_ORDER;
    let result = read_file(path.as_str());

    if result.is_err() {
        if let Err(err) = init_open_orders(&client, &program_id, &owner, &state, &mut orders) {
            debug_println!("{:?}", err);
        } else {
            let file_name = orders.unwrap().to_string() + "_" + OPEN_ORDER;
            write_file(CONFIG_DIR, &file_name, orders.unwrap().to_string().as_str()).unwrap();
            
            return Ok(orders.unwrap());
        }
    }

    Ok(Pubkey::from_str(result.unwrap().as_str())?)
}

pub fn init_open_orders(
    client: &RpcClient,
    program_id: &Pubkey,
    owner: &Keypair,
    state: &MarketPubkeys,
    orders: &mut Option<Pubkey>,
) -> Result<()> {
    let mut instructions = Vec::new();
    let mut signers = Vec::new();

    let orders_keypair;

    let orders_pubkey = match *orders {
        Some(pk) => pk,
        None => {
            let (orders_key, instruction) = create_dex_account(
                client,
                program_id,
                &owner.pubkey(),
                size_of::<serum_dex::state::OpenOrders>(),
            )?;
            orders_keypair = orders_key;
            signers.push(&orders_keypair);
            instructions.push(instruction);
            orders_keypair.pubkey()
        }
    };

    *orders = Some(orders_pubkey);

    instructions.push(init_open_orders_ix(
        program_id,
        &orders_pubkey,
        &owner.pubkey(),
        &state.market,
        None,
    )?);

    signers.push(owner);

    let recent_hash = client.get_latest_blockhash()?;
    let txn = Transaction::new_signed_with_payer(
        &instructions,
        Some(&owner.pubkey()),
        &signers,
        recent_hash,
    );

    let _signature = client.send_and_confirm_transaction(&txn)?;
    Ok(())
}

pub fn create_and_init_mint(
    client: &RpcClient,
    payer_keypair: &Keypair,
    mint_keypair: &Keypair,
    owner_pubkey: &Pubkey,
    decimals: u8,
) -> Result<Signature> {
    let signers = vec![payer_keypair, mint_keypair];
    let lamports = client.get_minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN)?;

    let create_mint_account_instruction = solana_sdk::system_instruction::create_account(
        &payer_keypair.pubkey(),
        &mint_keypair.pubkey(),
        lamports,
        spl_token::state::Mint::LEN as u64,
        &spl_token::ID,
    );

    let initialize_mint_instruction = token_instruction::initialize_mint(
        &spl_token::ID,
        &mint_keypair.pubkey(),
        owner_pubkey,
        None,
        decimals,
    )?;

    let instructions = vec![create_mint_account_instruction, initialize_mint_instruction];

    let recent_hash = client.get_latest_blockhash()?;
    let txn = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer_keypair.pubkey()),
        &signers,
        recent_hash,
    );

    let signature = client.send_and_confirm_transaction(&txn)?;
    Ok(signature)
}

pub fn mint_to_new_account(
    client: &RpcClient,
    payer: &Keypair,
    minting_key: &Keypair,
    mint: &Pubkey,
    quantity: u64,
) -> Result<Keypair> {
    let recip_keypair = Keypair::generate(&mut OsRng);
    let lamports = client.get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)?;
    let signers = vec![payer, minting_key, &recip_keypair];

    let create_recip_instr = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &recip_keypair.pubkey(),
        lamports,
        spl_token::state::Account::LEN as u64,
        &spl_token::ID,
    );

    let init_recip_instr = token_instruction::initialize_account(
        &spl_token::ID,
        &recip_keypair.pubkey(),
        mint,
        &payer.pubkey(),
    )?;

    let mint_tokens_instr = token_instruction::mint_to(
        &spl_token::ID,
        mint,
        &recip_keypair.pubkey(),
        &minting_key.pubkey(),
        &[],
        quantity,
    )?;

    let instructions = vec![create_recip_instr, init_recip_instr, mint_tokens_instr];

    let recent_hash = client.get_latest_blockhash()?;
    let txn = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &signers,
        recent_hash,
    );

    let _signature = client.send_and_confirm_transaction(&txn)?;
    Ok(recip_keypair)
}

pub fn create_market(
    client: &RpcClient,
    program_id: &Pubkey,
    payer: &Keypair,
    coin_mint: &Pubkey,
    pc_mint: &Pubkey,
    coin_lot_size: u64,
    pc_lot_size: u64,
) -> Result<MarketPubkeys> {
    let (listing_keys, mut instructions) = create_listingkeys_and_instructions(
        client,
        program_id,
        &payer.pubkey(),
        coin_mint,
        pc_mint,
    )?;

    let ListingKeys {
        market_key,
        req_q_key,
        event_q_key,
        bids_key,
        asks_key,
        vault_signer_pk,
        vault_signer_nonce,
    } = listing_keys;

    debug_println!("Creating coin vault...");
    let coin_vault = create_token_account(client, coin_mint, &vault_signer_pk, payer)?;
    debug_println!("Created account: {} ...", coin_vault.pubkey());

    debug_println!("Creating pc vault...");
    let pc_vault = create_token_account(client, pc_mint, &listing_keys.vault_signer_pk, payer)?;
    debug_println!("Created account: {} ...", pc_vault.pubkey());

    let init_market_instruction = serum_dex::instruction::initialize_market(
        &market_key.pubkey(),
        program_id,
        coin_mint,
        pc_mint,
        &coin_vault.pubkey(),
        &pc_vault.pubkey(),
        None,
        None,
        None,
        &bids_key.pubkey(),
        &asks_key.pubkey(),
        &req_q_key.pubkey(),
        &event_q_key.pubkey(),
        coin_lot_size,
        pc_lot_size,
        vault_signer_nonce,
        100,
    )?;

    instructions.push(init_market_instruction);

    let recent_hash = client.get_latest_blockhash()?;
    let signers = vec![
        payer,
        &market_key,
        &req_q_key,
        &event_q_key,
        &bids_key,
        &asks_key,
        &req_q_key,
        &event_q_key,
    ];

    let txn = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &signers,
        recent_hash,
    );

    let signature = client.send_and_confirm_transaction(&txn)?;
    debug_println!("Market is created with signature: {:?}", signature);

    Ok(MarketPubkeys {
        market: Box::new(market_key.pubkey()),
        req_q: Box::new(req_q_key.pubkey()),
        event_q: Box::new(event_q_key.pubkey()),
        bids: Box::new(bids_key.pubkey()),
        asks: Box::new(asks_key.pubkey()),
        coin_vault: Box::new(coin_vault.pubkey()),
        pc_vault: Box::new(pc_vault.pubkey()),
        vault_signer_key: Box::new(vault_signer_pk),
    })
}

pub fn create_listingkeys_and_instructions(
    client: &RpcClient,
    program_id: &Pubkey,
    payer: &Pubkey,
    _coin_mint: &Pubkey,
    _pc_mint: &Pubkey,
) -> Result<(ListingKeys, Vec<Instruction>)> {
    let (market_key, create_market) = create_dex_account(client, program_id, payer, 376)?;
    let (req_q_key, create_req_q) = create_dex_account(client, program_id, payer, 640)?;
    let (event_q_key, create_event_q) = create_dex_account(client, program_id, payer, 1 << 20)?;
    let (bids_key, create_bids) = create_dex_account(client, program_id, payer, 1 << 16)?;
    let (asks_key, create_asks) = create_dex_account(client, program_id, payer, 1 << 16)?;

    let (vault_signer_nonce, vault_signer_pk) = {
        let mut i = 0;

        loop {
            assert!(i < 100);
            if let Ok(pk) = gen_vault_signer_key(i, &market_key.pubkey(), program_id) {
                break (i, pk);
            }

            i += 1;
        }
    };

    let info = ListingKeys {
        market_key,
        req_q_key,
        event_q_key,
        bids_key,
        asks_key,
        vault_signer_pk,
        vault_signer_nonce,
    };

    let instructions = vec![
        create_market,
        create_req_q,
        create_event_q,
        create_bids,
        create_asks,
    ];

    Ok((info, instructions))
}

pub fn create_dex_account(
    client: &RpcClient,
    program_id: &Pubkey,
    payer: &Pubkey,
    unpadded_len: usize,
) -> Result<(Keypair, Instruction)> {
    let len = unpadded_len + 12;
    let key = Keypair::generate(&mut OsRng);

    let create_account_instr = solana_sdk::system_instruction::create_account(
        payer,
        &key.pubkey(),
        client.get_minimum_balance_for_rent_exemption(len)?,
        len as u64,
        program_id,
    );

    Ok((key, create_account_instr))
}

pub fn create_token_account(
    client: &RpcClient,
    mint_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    payer: &Keypair,
) -> Result<Keypair> {
    let spl_account = Keypair::generate(&mut OsRng);
    let instructions = create_token_account_instructions(
        client,
        spl_account.pubkey(),
        mint_pubkey,
        owner_pubkey,
        payer,
    )?;

    let recent_hash = client.get_latest_blockhash()?;
    let signers = vec![payer, &spl_account];

    let txn = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &signers,
        recent_hash,
    );

    let _signature = client.send_and_confirm_transaction(&txn)?;
    Ok(spl_account)
}

pub fn create_token_account_instructions(
    client: &RpcClient,
    spl_account: Pubkey,
    mint_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    payer: &Keypair,
) -> Result<Vec<Instruction>> {
    let lamports = client.get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)?;

    let create_account_instr = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &spl_account,
        lamports,
        spl_token::state::Account::LEN as u64,
        &spl_token::ID,
    );

    let init_account_instr = token_instruction::initialize_account(
        &spl_token::ID,
        &spl_account,
        &mint_pubkey,
        &owner_pubkey,
    )?;

    let instructions = vec![create_account_instr, init_account_instr];
    Ok(instructions)
}

#[cfg(target_endian = "little")]
pub fn get_keys_for_market<'a>(
    client: &'a RpcClient,
    program_id: &'a Pubkey,
    market: &'a Pubkey,
) -> Result<MarketPubkeys> {
    let account_data: Vec<u8> = client.get_account_data(&market)?;
    let words: Cow<[u64]> = remove_dex_account_padding(&account_data)?;

    let market_state: MarketState = {
        let account_flags = Market::account_flags(&account_data)?;
        if account_flags.intersects(AccountFlag::Permissioned) {
            let state = transmute_one_pedantic::<MarketStateV2>(transmute_to_bytes(&words))
                .map_err(|e| e.without_src())?;
            state.check_flags(true)?;
            state.inner
        } else {
            let state = transmute_one_pedantic::<MarketState>(transmute_to_bytes(&words))
                .map_err(|e| e.without_src())?;
            state.check_flags(true)?;
            state
        }
    };

    let vault_signer_key =
        gen_vault_signer_key(market_state.vault_signer_nonce, market, program_id)?;

    assert_eq!(
        transmute_to_bytes(&identity(market_state.own_address)),
        market.as_ref()
    );

    Ok(MarketPubkeys {
        market: Box::new(*market),
        req_q: Box::new(Pubkey::new(transmute_one_to_bytes(&identity(
            market_state.req_q,
        )))),
        event_q: Box::new(Pubkey::new(transmute_one_to_bytes(&identity(
            market_state.event_q,
        )))),
        bids: Box::new(Pubkey::new(transmute_one_to_bytes(&identity(
            market_state.bids,
        )))),
        asks: Box::new(Pubkey::new(transmute_one_to_bytes(&identity(
            market_state.asks,
        )))),
        coin_vault: Box::new(Pubkey::new(transmute_one_to_bytes(&identity(
            market_state.coin_vault,
        )))),
        pc_vault: Box::new(Pubkey::new(transmute_one_to_bytes(&identity(
            market_state.pc_vault,
        )))),
        vault_signer_key: Box::new(vault_signer_key),
    })
}

#[cfg(target_endian = "little")]
fn remove_dex_account_padding<'a>(data: &'a [u8]) -> Result<Cow<'a, [u64]>> {
    use serum_dex::state::{ACCOUNT_HEAD_PADDING, ACCOUNT_TAIL_PADDING};

    let head = &data[..ACCOUNT_HEAD_PADDING.len()];

    if data.len() < ACCOUNT_HEAD_PADDING.len() + ACCOUNT_TAIL_PADDING.len() {
        return Err(format_err!(
            "dex account length {} is too small to contain valid padding",
            data.len()
        ));
    }

    if head != ACCOUNT_HEAD_PADDING {
        return Err(format_err!("dex account head padding mismatch"));
    }

    let tail = &data[data.len() - ACCOUNT_TAIL_PADDING.len()..];

    if tail != ACCOUNT_TAIL_PADDING {
        return Err(format_err!("dex account tail padding mismatch"));
    }

    let inner_data_range = ACCOUNT_HEAD_PADDING.len()..(data.len() - ACCOUNT_TAIL_PADDING.len());
    let inner: &'a [u8] = &data[inner_data_range];

    let words: Cow<'a, [u64]> = match transmute_many_pedantic::<u64>(inner) {
        Ok(word_slice) => Cow::Borrowed(word_slice),
        Err(transmute_error) => {
            let word_vec = transmute_error.copy().map_err(|e| e.without_src())?;
            Cow::Owned(word_vec)
        }
    };

    Ok(words)
}
