use std::{num::NonZeroU64, str::FromStr, time::SystemTime};

use clap::{Parser, Subcommand};
use debug_print::debug_println;

use serum_dex::{
    instruction::{NewOrderInstructionV3, SelfTradeBehavior},
    matching::{OrderType, Side},
};

use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

use serum_rust_cli::*;

#[derive(Parser, Debug)]
#[clap(author = "RHO Markets", version, about)]
#[clap(propagate_version = true)]
/// A simple CLI application to interact with Serum DEX to place new order, fetch orders, match orders and settle funds.
struct Arguments {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate and initialize new accounts on-chain for market, event queue, bids and asks which is required by Serum DEX.
    Init {
        #[clap(long, forbid_empty_values = true, validator = validate_url_address)]
        /// Network URL, for instance: http://localhost:8899
        url: String,

        #[clap(long, forbid_empty_values = true, validator = validate_input_for_space)]
        /// Path to your wallet, such as ~/.config/solana/id.json, the wallet should have some funds
        path: String,

        #[clap(long, forbid_empty_values = true, validator = validate_input_for_space)]
        /// Program ID of the Serum DEX
        program_id: String,
    },
    /// Get info about mint, wallet, network, program, market and open order
    Info {},
    /// Place new order to lend
    Lend {
        #[clap(long, forbid_empty_values = true, validator = validate_input_for_space)]
        /// Path to your wallet, such as ~/.config/solana/id.json, the wallet should have some funds
        wallet: String,

        #[clap(long, forbid_empty_values = true, validator = validate_input_for_space)]
        /// coin pubkey, this is Associated Token Account address
        coin: String,

        #[clap(long, forbid_empty_values = true)]
        /// The size of the order.
        size: u64,

        #[clap(long = "rate", forbid_empty_values = true)]
        /// The interest rate of the order.
        interest_rate: u64,
    },
    /// Place new order to borrow
    Borrow {
        #[clap(long, forbid_empty_values = true, validator = validate_input_for_space)]
        /// Path to your wallet, such as ~/.config/solana/id.json, the wallet should have some funds
        wallet: String,

        #[clap(long, forbid_empty_values = true, validator = validate_input_for_space)]
        /// pc pubkey, this is Associated Token Account address
        pc: String,

        #[clap(long, forbid_empty_values = true)]
        /// The size of the order.
        size: u64,

        #[clap(long = "rate", forbid_empty_values = true)]
        /// The interest rate of the order.
        interest_rate: u64,
    },
    /// Displays orders from OrderBook
    Fetch {},
    /// Remove config files
    Clean {},
}

fn main() {
    let args = Arguments::parse();

    match args.command {
        Commands::Init {
            url,
            path,
            program_id,
        } => {
            if std::path::Path::new(CONFIG_DIR).exists() {
                println!("To initialize and generate new on-chain accounts and market, please, firstly run clean command.");
                return;
            }

            let client = RpcClient::new(&url);
            let program_id_pk = Pubkey::from_str(&program_id).unwrap();
            let payer = read_keypair_file(&path).unwrap();

            let result = initialize(&client, &program_id_pk, &payer);
            if result.is_err() {
                println!("Initialization Error...");
            } else {
                println!("Initialization OK...");
            }

            // saving data into json files
            if let Err(err) = write_file(CONFIG_DIR, URL, &url) {
                debug_println!("{:?}", err);
            }

            if let Err(err) = write_file(CONFIG_DIR, PROGRAM_ID, &program_id) {
                debug_println!("{:?}", err);
            }

            if let Err(err) = write_file(CONFIG_DIR, WALLET, &path) {
                debug_println!("{:?}", err);
            }
        }
        Commands::Info {} => {
            info();
        }
        Commands::Lend {
            wallet,
            coin,
            size,
            interest_rate,
        } => {
            let path = CONFIG_DIR.to_string() + "/" + URL;
            let url = read_file(path.as_str()).unwrap();
            let client = RpcClient::new(&url);

            let path = CONFIG_DIR.to_string() + "/" + PROGRAM_ID;
            let program_id = read_file(path.as_str()).unwrap();
            let program_id_pk = Pubkey::from_str(&program_id).unwrap();

            let payer = read_keypair_file(&wallet).unwrap();

            let path = CONFIG_DIR.to_string() + "/" + MARKET_PUBKEY;
            let market_str = read_file(path.as_str()).unwrap();
            let market_pk = &Pubkey::from_str(market_str.as_str()).unwrap();
            let market_keys = get_keys_for_market(&client, &program_id_pk, &market_pk).unwrap();

            let path = CONFIG_DIR.to_string() + "/" + COIN_MINT;
            let coin_mint_str = read_file(path.as_str()).unwrap();
            let coin_mint = Pubkey::from_str(coin_mint_str.as_str()).unwrap();
            let associated_token = spl_associated_token_account::get_associated_token_address(
                &payer.pubkey(),
                &coin_mint,
            );

            println!("Wallet address: {:?}", payer.pubkey());
            println!("Coin mint address: {:?}", &coin_mint);
            println!("Coin associated token address: {:?}", associated_token);

            let coin_wallet = Pubkey::from_str(coin.as_str()).unwrap();
            println!("Coin wallet: {:?}", coin_wallet);

            // let open_order_result =
            //     get_open_order_pubkey(&client, &program_id_pk, &payer, &market_keys);
            // let mut orders = if open_order_result.is_ok() {
            //     open_order_result.ok()
            // } else {
            //     None
            // };
            // debug_println!("Open orders: {:?}", orders);

            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            let mut orders: Option<Pubkey> = None;
            debug_println!("Placing new order...");
            let result = place_order(
                &client,
                &program_id_pk,
                &payer,
                &associated_token,
                //&coin_wallet,
                &market_keys,
                &mut orders,
                NewOrderInstructionV3 {
                    side: Side::Ask,
                    limit_price: NonZeroU64::new(interest_rate).unwrap(),
                    // max_coin_qty: NonZeroU64::new(1_000).unwrap(),
                    max_coin_qty: NonZeroU64::new(size).unwrap(),
                    max_native_pc_qty_including_fees: NonZeroU64::new(std::u64::MAX).unwrap(),
                    order_type: OrderType::Limit,
                    limit: std::u16::MAX,
                    self_trade_behavior: SelfTradeBehavior::DecrementTake,
                    client_order_id: 1_000_000,
                    // max_ts: i64::MAX,
                    max_ts: now + 20,
                },
            );

            if result.is_err() {
                println!("{:?}", result.err());
            }
        }
        Commands::Borrow {
            wallet,
            pc,
            size,
            interest_rate,
        } => {
            let path = CONFIG_DIR.to_string() + "/" + URL;
            let url = read_file(path.as_str()).unwrap();
            let client = RpcClient::new(&url);

            let path = CONFIG_DIR.to_string() + "/" + PROGRAM_ID;
            let program_id = read_file(path.as_str()).unwrap();
            let program_id_pk = Pubkey::from_str(&program_id).unwrap();

            let payer = read_keypair_file(&wallet).unwrap();

            let path = CONFIG_DIR.to_string() + "/" + MARKET_PUBKEY;
            let market_str = read_file(path.as_str()).unwrap();
            let market_pk = &Pubkey::from_str(market_str.as_str()).unwrap();
            let market_keys = get_keys_for_market(&client, &program_id_pk, &market_pk).unwrap();

            let pc_wallet = Pubkey::from_str(pc.as_str()).unwrap();

            let open_order_result =
                get_open_order_pubkey(&client, &program_id_pk, &payer, &market_keys);
            let mut orders = if open_order_result.is_ok() {
                open_order_result.ok()
            } else {
                None
            };

            debug_println!("Placing new order...");
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            let result = place_order(
                &client,
                &program_id_pk,
                &payer,
                &pc_wallet,
                &market_keys,
                &mut orders,
                NewOrderInstructionV3 {
                    side: Side::Bid,
                    limit_price: NonZeroU64::new(interest_rate).unwrap(),
                    max_coin_qty: NonZeroU64::new(size).unwrap(),
                    max_native_pc_qty_including_fees: NonZeroU64::new(5_000_000).unwrap(),
                    // max_native_pc_qty_including_fees: NonZeroU64::new(size).unwrap(),
                    self_trade_behavior: SelfTradeBehavior::DecrementTake,
                    order_type: OrderType::Limit,
                    client_order_id: 1_000_100,
                    limit: std::u16::MAX,
                    // max_ts: i64::MAX,
                    max_ts: now + 20,
                },
            );

            if result.is_err() {
                println!("{:?}", result.err());
            }
        }
        Commands::Fetch {} => {
            if !std::path::Path::new(CONFIG_DIR).exists() {
                println!("Missing config files!");
                return;
            }

            let path = CONFIG_DIR.to_string() + "/" + URL;
            let url = read_file(path.as_str()).unwrap();

            let path = CONFIG_DIR.to_string() + "/" + PROGRAM_ID;
            let program_id = read_file(path.as_str()).unwrap();

            let client = RpcClient::new(&url);
            let program_id_pk = Pubkey::from_str(&program_id).unwrap();

            let path = CONFIG_DIR.to_string() + "/" + MARKET_PUBKEY;
            let market_str = read_file(path.as_str()).unwrap();
            let market_pk = &Pubkey::from_str(market_str.as_str()).unwrap();

            if let Err(err) = fetch_and_show_orders(&client, &program_id_pk, &market_pk) {
                debug_println!("{:?}", err);
            }
        }
        Commands::Clean {} => {
            if let Err(err) = remove_dir_and_files(CONFIG_DIR) {
                debug_println!("{:?}", err);
            }
        }
    }
}
