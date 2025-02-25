# rust-uniswap-example

## Overview

This repository contains a Rust-based implementation for monitoring swap events on the Uniswap protocol. The application connects to an Ethereum node, retrieves Uniswap pool events, and processes swap details in real-time.

## About Uniswap

[Uniswap](https://docs.uniswap.org/protocol/introduction) is a decentralized exchange protocol that allows users to swap cryptocurrencies directly on the Ethereum blockchain. It operates via smart contracts that maintain liquidity pools for different token pairs. Swaps are executed by calling the [`swap` function](https://github.com/Uniswap/v3-core/blob/412d9b236a1e75a98568d49b1aeb21e3a1430544/contracts/UniswapV3Pool.sol#L596), which emits event logs containing transaction details.

This application fetches and decodes swap events from Uniswap’s DAI/USDC pool and processes relevant transaction details.

## Features

- Monitors Uniswap swap events in real-time.
- Extracts and formats transaction details such as:
  - Sender and receiver addresses
  - Token amounts in human-readable format
  - Swap direction (DAI → USDC or USDC → DAI)
- Implements reorganization protection to ensure data accuracy.
- Supports Ethereum node integration via RPC providers like [Infura](https://infura.io/).

## Installation & Usage

### Prerequisites
- Install Rust and Cargo.
- Set up an Ethereum RPC endpoint (e.g., Infura).

### Build & Run
```sh
cargo build --release
cargo run
```

## Handling Blockchain Reorganizations

Since Ethereum blocks may undergo temporary reorganization, this implementation introduces a buffer of 5 blocks before confirming events. If a deeper reorganization occurs, the application exits to prevent incorrect data processing.

## Dependencies
- [`rust-web3`](https://github.com/tomusdrw/rust-web3) for Ethereum interaction.
- [`serde_json`](https://docs.rs/serde_json/) for parsing Uniswap ABI.

## Contributing
Feel free to contribute by submitting pull requests or opening issues for improvements and bug fixes.

## License
This project is released under the MIT License.
