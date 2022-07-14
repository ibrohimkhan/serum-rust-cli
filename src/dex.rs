use anyhow::{format_err, Result};
use std::str::FromStr;
use std::{borrow::Cow, mem::size_of};

use rand::rngs::OsRng;
use safe_transmute::*;
use std::convert::identity;

use crate::{read_file, write_file, MarketPubkeys, CONFIG_DIR, OPEN_ORDER};
use serum_dex::instruction::init_open_orders as init_open_orders_ix;
use serum_dex::state::{gen_vault_signer_key, AccountFlag, Market, MarketState, MarketStateV2};

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::Transaction,
};

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

pub fn get_open_order_pubkey(
    client: &RpcClient,
    program_id: &Pubkey,
    owner: &Keypair,
    state: &MarketPubkeys,
) -> Result<Pubkey> {
    let mut orders = None;
    let path =
        CONFIG_DIR.to_string() + "/" + owner.pubkey().to_string().as_str() + "_" + OPEN_ORDER;

    let result = read_file(path.as_str());

    if result.is_err() {
        if let Err(err) = init_open_orders(&client, &program_id, &owner, &state, &mut orders) {
            panic!("{:?}", err);
        } else {
            let file_name = owner.pubkey().to_string() + "_" + OPEN_ORDER;
            write_file(CONFIG_DIR, &file_name, orders.unwrap().to_string().as_str()).unwrap();

            return Ok(orders.unwrap());
        }
    }

    Ok(Pubkey::from_str(result.unwrap().as_str())?)
}

fn init_open_orders(
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
