### CLI to interact with Serum DEX
A simple CLI application to interact with Serum to place a new order in orderbook, fetch open orders, metch orders and settle funds.

### Prerequisite
To initialize a market you have to provide base and quote currencies. You can create tokens and accounts through cli as follow:

```console
$ spl-token create-token 
```

This command generates a new token, pass it here:

```console
$ spl-token create-account <GENERATED_TOKEN>
```

This command generates an empty account. Next, mint some tokens as follow:

```console
$ spl-token mint <GENERATED_TOKEN> <AMOUNT_YOU_NEED>
```

Transfer some tokens to your wallet as follow:

```console
$ spl-token transfer <GENERATED_TOKEN> <AMOUNT_YOU_NEED> <YOUR_WALLET_PUBKEY> --fund-recipient
```

You have to run those commands for creating base and quote currencies, more info is [here](https://spl.solana.com/token).


### Help
To get help, run:

```console
cargo run -- -h
```

Supported options:

|Option|Description|
|-----|-----------|
|`-h, --help`|Print help information|
|`-V, --version`|Print version information|

Supported subcommands:

|Subcommand|Description|
|-----|-----------|
|init|Generate and initialize new accounts on-chain for market, request queue, event queue, bids and asks and also initialize new market|
|lend|Place a new order in orderbook for lending|
|borrow|Place a new order in orderbook for borrowing|
|fetch|Display open orders in orderbook|
|info|Display app's config information|
|clean|Remove config files|

To get help for subcommands, run:

```console
cargo run -- init -h
cargo run -- lend -h
cargo run -- borrow -h
cargo run -- fetch -h
cargo run -- info -h
cargo run -- clean -h
```


### Run

To create a new market you have to initialize it as follow:

```console
cargo run -- init --url <URL> --path <PATH_TO_YOUR_WALLET> --program-id <SERUM_DEX_PROGRAM_ID> --coin-mint <COIN_MINT> --pc-mint <PC_MINT>
```

To place a new order in orderbook for lending or borrowing, run:

```console
cargo run -- lend --wallet <WALLET> --coin-mint <COIN_MINT> --size <SIZE> --rate <INTEREST_RATE>

cargo run -- borrow --wallet <WALLET> --pc-mint <PC_MINT> --size <SIZE> --rate <INTEREST_RATE>
```

Fetch open orders in orderbook:

```console
cargo run -- fetch
```

Get information about application configuration:

```console
cargo run -- info
```

To clean config files (running init command will be required again), run:

```console
cargo run -- clean
```
