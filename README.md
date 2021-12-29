# Pyth Client

A Rust library for consuming price feeds from the [pyth.network](https://pyth.network/) oracle on the Solana network.
This package includes a library for on-chain programs and an example program for printing product reference data.

Key features of this library include:

* Get the current price of over [50 products](https://pyth.network/markets/), including cryptocurrencies,
  US equities, forex and more.
* Combine listed products to create new price feeds, e.g., for baskets of tokens or non-USD quote currencies.
* Consume prices in on-chain Solana programs or off-chain applications.

Please see the [pyth.network documentation](https://docs.pyth.network/) for more information about pyth.network.

## Usage

Add a dependency to your Cargo.toml:

```toml
[dependencies]
pyth-client="<version>"
```

See [pyth-client on crates.io](https://crates.io/crates/pyth-client/) to get the latest version of the library.

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

## Development

This library can be built for either your native platform or in BPF (used by Solana programs). 
Use `cargo build` / `cargo test` to build and test natively.
Use `cargo build-bpf` / `cargo test-bpf` to build in BPF for Solana; these commands require you to have installed the [Solana CLI tools](https://docs.solana.com/cli/install-solana-cli-tools). 

The BPF tests will also run an instruction count program that logs the resource consumption
of various library functions.
This program can also be run on its own using `cargo test-bpf --test instruction_count`.

### Releases

To release a new version of this package, perform the following steps:

1. Increment the version number in `Cargo.toml`.
   You may use a version number with a `-beta.x` suffix such as `0.0.1-beta.0` to create opt-in test versions.
2. Merge your change into `main` on github.
3. Create and publish a new github release.
   The name of the release should be the version number, and the tag should be the version number prefixed with `v`.
   Publishing the release will trigger a github action that will automatically publish the [pyth-client](https://crates.io/crates/pyth-client) rust crate to `crates.io`.
