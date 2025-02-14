use dotenv::dotenv;
use futures::StreamExt;
use std::env;

use anyhow::{Context, Result};
use ethabi::{decode, ethereum_types, ParamType, Token};
use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use num_traits::{FromPrimitive, Signed, Zero};
use web3::types::{Log, H160, U64};

#[derive(Debug)]
/// Represents a swap event in a Uniswap-like protocol.
struct SwapEvent {
	sender: H160,
	receiver: H160,
	amount0: BigInt,
	amount1: BigInt,
}

#[derive(Debug)]
/// A struct representing a confirmed block in the blockchain.
struct ConfirmedBlock {
	number: U64,
	events: Vec<SwapEvent>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	dotenv().ok();
	let eth_node_url = env::var("INFURA_URL").context("INFURA_URL must be set")?;
	let eth_pool_address =
		env::var("USDC_DAI_UNISWAP_POOL_CONTRACT").context("Pool contract address must be set")?;
	let web3 = web3::Web3::new(web3::transports::ws::WebSocket::new(eth_node_url.as_str()).await?);

	let contract_address =
		web3::types::H160::from_slice(&hex::decode(eth_pool_address).unwrap()[..]);

	// Load the contract ABI.
	let contract = web3::contract::Contract::from_json(
		web3.eth(),
		contract_address,
		include_bytes!("contracts/uniswap_pool_abi.json"),
	)
	.context("Failed to create contract from ABI")?;

	// Obtain the Swap event signature from the ABI.
	let swap_events = contract
		.abi()
		.events_by_name("Swap")
		.context("No 'Swap' event found in the ABI")?;
	let swap_event = swap_events.first().context("Swap event list is empty")?;
	let swap_event_signature = swap_event.signature();

	// Subscribe to new block headers.
	let mut block_stream = web3
		.eth_subscribe()
		.subscribe_new_heads()
		.await
		.context("Failed to subscribe to new block headers")?;
	println!("Block subscription started");

	while let Some(Ok(block)) = block_stream.next().await {
		let block_hash = block.hash.context("Received block without hash")?;
		let block_number = block.number.context("Received block without number")?;

		// Build a filter for the Swap events in this block.
		let filter = web3::types::FilterBuilder::default()
			.block_hash(block_hash)
			.address(vec![contract_address])
			.topics(Some(vec![swap_event_signature]), None, None, None)
			.build();

		let swap_logs = web3.eth().logs(filter).await.context("Failed to fetch logs for block")?;
		let events = swap_logs.iter().filter_map(decode_swap_event).collect();

		let confirmed_block = ConfirmedBlock { number: block_number, events };

		print_swap_events(&confirmed_block);
	}

	Ok(())
}

fn decode_swap_event(log: &Log) -> Option<SwapEvent> {
	if log.topics.len() < 3 {
		eprintln!("Not enough topics in log");
		return None;
	}

	let sender = H160::from_slice(&log.topics[1].as_bytes()[12..]);
	let receiver = H160::from_slice(&log.topics[2].as_bytes()[12..]);

	let tokens = decode(&[ParamType::Int(256), ParamType::Int(256)], &log.data.0).ok()?;
	if tokens.len() != 2 {
		eprintln!("Unexpected number of tokens in log data");
		return None;
	}

	let amount0 = match &tokens[0] {
		Token::Int(value) => ethereum_int_to_bigint(value),
		_ => {
			eprintln!("Expected int256 for amount0");
			return None;
		},
	};
	let amount1 = match &tokens[1] {
		Token::Int(value) => ethereum_int_to_bigint(value),
		_ => {
			eprintln!("Expected int256 for amount1");
			return None;
		},
	};

	Some(SwapEvent { sender, receiver, amount0, amount1 })
}

/// Converts a byte slice into a `BigInt` with a positive sign.
fn ethereum_int_to_bigint(value: &ethereum_types::U256) -> BigInt {
	let mut bytes = [0u8; 32];
	value.to_big_endian(&mut bytes);

	let unsigned = BigInt::from_bytes_be(Sign::Plus, &bytes);

	let two = BigInt::from_u8(2).unwrap();
	let two_256 = two.pow(256);
	let two_255 = two.pow(255);

	if unsigned >= two_255 {
		unsigned - two_256
	} else {
		unsigned
	}
}

/// Prints the swap events for a confirmed block.
fn print_swap_events(block: &ConfirmedBlock) {
	if block.events.is_empty() {
		println!("Block {}: No swap events", block.number);
		return;
	}

	for evt in block.events.iter() {
		let direction = if evt.amount0 > BigInt::zero() && evt.amount1 < BigInt::zero() {
			"DAI -> USDC"
		} else if evt.amount0 < BigInt::zero() && evt.amount1 > BigInt::zero() {
			"USDC -> DAI"
		} else {
			"Unknown"
		};

		let amount0_str = convert_amount(&evt.amount0, 18); // DAI has 18 decimals.
		let amount1_str = convert_amount(&evt.amount1, 6); // USDC has 6 decimals.
		println!(
			"Block {} | Swap {}: sender: {:?}, receiver: {:?},\n amount0: {} DAI, amount1: {} USDC",
			block.number, direction, evt.sender, evt.receiver, amount0_str, amount1_str
		);
	}
}

/// Converts a fixed-point amount (stored as an integer) into a decimal string.
fn convert_amount(amount: &BigInt, decimals: u32) -> String {
	let ten = BigInt::from_u8(10).unwrap();
	let factor = ten.pow(decimals);
	let (quotient, remainder) = amount.div_rem(&factor);

	if remainder.is_zero() {
		return quotient.to_string(); // No decimals if remainder is zero
	}

	// Convert remainder to string, removing trailing zeros
	let remainder_str = remainder.abs().to_string();
	let trimmed_remainder = remainder_str.trim_end_matches('0');

	format!("{}.{}", quotient, trimmed_remainder)
}

#[cfg(test)]
mod tests {
	use super::*;
	use num_traits::ToPrimitive;

	#[test]
	fn test_ethereum_int_to_bigint_positive() {
		let value = ethereum_types::U256::from(1000u64);
		let result = ethereum_int_to_bigint(&value);
		assert_eq!(result.to_i64().unwrap(), 1000);
	}

	#[test]
	fn test_ethereum_int_to_bigint_negative() {
		// For a 256-bit integer, -1 is represented as 2^256 - 1.
		let max = ethereum_types::U256::max_value();
		let result = ethereum_int_to_bigint(&max);
		assert_eq!(result, BigInt::from(-1));
	}

	#[test]
	fn test_convert_amount() {
		// For example, 1500000000000000000 represented with 18 decimals should be "1.5".
		let factor = BigInt::from(10).pow(18);
		let amount = &BigInt::from(15) * &factor / 10;
		let converted = convert_amount(&amount, 18);
		assert_eq!(converted, "1.5");
	}
}
