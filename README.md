### CLI to interact with OrderBook
A simple CLI application to interact with Serum to place a new order in orderbook, fetch open orders, metch orders and settle funds.

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
|init|Generate and initialize new accounts on-chain for market, event queue, bids and asks which is required by Serum|
|lend|Place a new order in orderbook for lending|
|borrow|Place a new order in orderbook for borrowing|
|fetch|Display open orders in orderbook|
|info|Display app's config information|
|clean|Remove config files. After running `clean` command you will need to run `init` command again|

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

First of all, we need to generate and initialize accounts for market. __init__ subcommand is used to initialize market:

```console
cargo run -- init --url <URL> --path <PATH_TO_YOUR_WALLET> --program-id <SERUM_DEX_PROGRAM_ID>
```

To place a new order in orderbook for lending or borrowing, run:

```console
cargo run -- lend <SIZE_OF_THE_ORDER> --rate <INTEREST_RATE>

cargo run -- borrow <SIZE_OF_THE_ORDER> --rate <INTEREST_RATE>
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
