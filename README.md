# optifi-client
 wrap optifi-cpi for convenient usage

## **Overview**

dev tooling for [OptiFi](https://www.optifi.app)

- optifi-client: wrap optifi-cpi for convenient usage
- **[optifi-cpi](https://github.com/OptiFi-Team/optifi-cpi/tree/main/programs/optifi-cpi)**: provide anchor CPI and account struct
- **[cpi-examples](https://github.com/OptiFi-Team/optifi-cpi/tree/main/programs/cpi-examples)**: CPI integration examples

## Environment Setup

1. Install **[Solana CLI](https://solanacookbook.com/getting-started/installation.html#install-cli)**  
2. Install **[Rust](https://solanacookbook.com/getting-started/installation.html#install-rust)** 
3. Run `solana-keygen new -o ~/.config/solana/optifi.json`
4. Run `solana airdrop 1 -k ~/.config/solana/optifi.json`

## Test

1. Add `features = ["cpi", "devnet"]}` to  `optifi-cpi` in `programs/optifi-client/Cargo.toml` for running on **devnet**
2. Run some tests in `programs/optifi-client/tests/mod.rs`

## Error Handle
Run `rustup override set stable-x86_64-apple-darwin `