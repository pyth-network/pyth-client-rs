# pyth-client-rs

A rust API for describing on-chain pyth account structures.  A primer on pyth accounts can be found at https://github.com/pyth-network/pyth-client/blob/main/doc/aggregate_price.md

Contains a library for use in on-chain program development and an off-chain example program for loading and printing product reference data and aggregate prices from all devnet pyth accounts.

### Running the Example

The example program prints the product reference data and current price information for Pyth on Solana devnet.
Run the following commands to try this example program:

```
cargo build --examples
cargo run --example get_accounts
```

The output of this command is a listing of Pyth's accounts, such as:

```
product_account .. 6MEwdxe4g1NeAF9u6KDG14anJpFsVEa2cvr5H6iriFZ8
  symbol.......... SRM/USD
  asset_type...... Crypto
  quote_currency.. USD
  description..... SRM/USD
  generic_symbol.. SRMUSD
  base............ SRM
  price_account .. 992moaMQKs32GKZ9dxi8keyM2bUmbrwBZpK4p2K6X5Vs
    price ........ 7398000000
    conf ......... 3200000
    price_type ... price
    exponent ..... -9
    status ....... trading
    corp_act ..... nocorpact
    num_qt ....... 1
    valid_slot ... 91340924
    publish_slot . 91340925
    twap ......... 7426390900
    twac ......... 2259870
```


### Development

Run `cargo test-bpf` to build in BPF and run the unit tests.
This command will also build an instruction count program that logs the resource consumption
of various functions.
