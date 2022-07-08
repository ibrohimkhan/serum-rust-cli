use std::{cell::RefCell, mem::size_of, ops::Deref, rc::Rc};

use debug_print::debug_println;
use serum_dex::{
    critbit::SlabView,
    instruction::{
        cancel_orders_by_client_order_ids as cancel_order_by_client_order_ids_ix,
        MarketInstruction, NewOrderInstructionV3,
    },
    state::Market,
};

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    account::Account,
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};

use crate::{get_keys_for_market, MarketPubkeys};
use anyhow::Result;

pub fn place_order(
    client: &RpcClient,
    program_id: &Pubkey,
    payer: &Keypair,
    wallet: &Pubkey,
    state: &MarketPubkeys,
    orders: &mut Option<Pubkey>,
    new_order: NewOrderInstructionV3,
) -> Result<()> {
    let mut instructions = Vec::new();
    let mut signers = Vec::new();

    let orders_keypair;

    let orders_pubkey = match *orders {
        Some(pk) => pk,
        None => {
            let (orders_key, instruction) = crate::create_dex_account(
                client,
                program_id,
                &payer.pubkey(),
                size_of::<serum_dex::state::OpenOrders>(),
            )?;

            orders_keypair = orders_key;
            signers.push(&orders_keypair);

            instructions.push(instruction);
            orders_keypair.pubkey()
        }
    };

    *orders = Some(orders_pubkey);
    let _side = new_order.side;

    let data = MarketInstruction::NewOrderV3(new_order).pack();

    let instruction = Instruction {
        program_id: *program_id,
        data,
        accounts: vec![
            AccountMeta::new(*state.market, false),
            AccountMeta::new(orders_pubkey, false),
            AccountMeta::new(*state.req_q, false),
            AccountMeta::new(*state.event_q, false),
            AccountMeta::new(*state.bids, false),
            AccountMeta::new(*state.asks, false),
            AccountMeta::new(*wallet, false),
            AccountMeta::new_readonly(payer.pubkey(), true),
            AccountMeta::new(*state.coin_vault, false),
            AccountMeta::new(*state.pc_vault, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::rent::ID, false),
        ],
    };

    instructions.push(instruction);
    signers.push(payer);

    let recent_hash = client.get_latest_blockhash()?;
    let txn = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &signers,
        recent_hash,
    );

    let _signature = client.send_and_confirm_transaction(&txn)?;

    Ok(())
}

pub fn fetch_and_show_orders(
    client: &RpcClient,
    program_id: &Pubkey,
    market_pk: &Pubkey,
) -> Result<()> {
    let market_keys = get_keys_for_market(&client, &program_id, &market_pk)?;

    let market_account: Account = client.get_account(&market_pk)?;
    let mut lamp = market_account.lamports;
    let mut data = market_account.data;

    let market_account_info = AccountInfo {
        key: &market_account.owner,
        is_signer: false,
        is_writable: false,
        lamports: Rc::new(RefCell::new(&mut lamp)),
        data: Rc::new(RefCell::new(&mut data)),
        owner: &program_id,
        executable: market_account.executable,
        rent_epoch: market_account.rent_epoch,
    };

    let market = Market::load(&market_account_info, &program_id, false)?;

    let ask_key = *market_keys.asks;
    let ask_acc = client.get_account(&ask_key)?;

    let ask_owner = ask_acc.owner;
    let mut ask_lamp = ask_acc.lamports;
    let mut ask_data = ask_acc.data;

    let ask_account_info = AccountInfo {
        key: &ask_key,
        is_signer: false,
        is_writable: false,
        lamports: Rc::new(RefCell::new(&mut ask_lamp)),
        data: Rc::new(RefCell::new(&mut ask_data)),
        owner: &ask_owner,
        executable: ask_acc.executable,
        rent_epoch: ask_acc.rent_epoch,
    };

    let asks = market.load_asks_mut(&ask_account_info)?;
    let slab = asks.deref();

    debug_println!("Lending Orders:");
    for i in 0..slab.capacity() {
        if let Some(node) = slab.get(i as u32) {
            if let Some(leaf) = node.as_leaf() {
                debug_println!("order id: {:?}", leaf.order_id());
                debug_println!("    client order id: {:?}", leaf.client_order_id());
                debug_println!("    price: {:?}", leaf.price());
                debug_println!("    amount: {:?}", leaf.quantity());
                debug_println!("    lend\n");
            }
        }
    }

    let bid_key = *market_keys.bids;
    let bid_acc = client.get_account(&bid_key)?;

    let bid_owner = bid_acc.owner;
    let mut bid_lamp = bid_acc.lamports;
    let mut bid_data = bid_acc.data;

    let bid_account_info = AccountInfo {
        key: &bid_key,
        is_signer: false,
        is_writable: false,
        lamports: Rc::new(RefCell::new(&mut bid_lamp)),
        data: Rc::new(RefCell::new(&mut bid_data)),
        owner: &bid_owner,
        executable: bid_acc.executable,
        rent_epoch: bid_acc.rent_epoch,
    };

    let bids = market.load_bids_mut(&bid_account_info)?;
    let slab = bids.deref();

    debug_println!("Borrowing Orders:");
    for i in 0..slab.capacity() {
        if let Some(node) = slab.get(i as u32) {
            if let Some(leaf) = node.as_leaf() {
                debug_println!("order id: {:?}", leaf.order_id());
                debug_println!("    client order id: {:?}", leaf.client_order_id());
                debug_println!("    price: {:?}", leaf.price());
                debug_println!("    amount: {:?}", leaf.quantity());
                debug_println!("    borrow\n");
            }
        }
    }

    Ok(())
}

pub fn cancel_order_by_client_order_ids(
    client: &RpcClient,
    owner: &Keypair,
    program_id: &Pubkey,
    market_keys: &MarketPubkeys,
    orders: &Pubkey,
    client_order_id: [u64; 8],
) -> Result<()> {
    let ixs = &[cancel_order_by_client_order_ids_ix(
        program_id,
        &market_keys.market,
        &market_keys.bids,
        &market_keys.asks,
        orders,
        &owner.pubkey(),
        &market_keys.event_q,
        client_order_id,
    )?];

    let recent_hash = client.get_latest_blockhash()?;
    let txn = Transaction::new_signed_with_payer(
        ixs, 
        Some(&owner.pubkey()), 
        &[owner], 
        recent_hash
    );

    let _signature = client.send_and_confirm_transaction(&txn)?;

    Ok(())
}
