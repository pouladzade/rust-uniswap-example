use ethabi::{decode, ethereum_types, ParamType, Token};
use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use num_traits::{FromPrimitive, Signed, Zero};
use web3::types::{Log, H160, H256};

/// Represents a swap event.
#[derive(Debug)]
pub struct SwapEvent {
	pub sender: H160,
	pub receiver: H160,
	pub amount0: BigInt,
	pub amount1: BigInt,
}

/// Represents a confirmed block.
#[derive(Debug)]
pub struct ConfirmedBlock {
	pub number: web3::types::U64,
	pub hash: H256,
	pub events: Vec<SwapEvent>,
}

/// Decodes a log into a SwapEvent.
///
/// The log must have at least three topics:
/// - topics[0]: event signature (ignored here)
/// - topics[1]: sender (last 20 bytes)
/// - topics[2]: receiver (last 20 bytes)
pub fn decode_swap_event(log: &Log) -> Option<SwapEvent> {
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

/// Converts an Ethereum U256 (interpreted as a two's complement int256) to BigInt.
pub fn ethereum_int_to_bigint(value: &ethereum_types::U256) -> BigInt {
	let mut bytes = [0u8; 32];
	value.to_big_endian(&mut bytes);
	let unsigned = BigInt::from_bytes_be(Sign::Plus, &bytes);
	let two = BigInt::from_u8(2).expect("Failed to create BigInt from 2");
	let two_256 = two.pow(256);
	let two_255 = two.pow(255);
	if unsigned >= two_255 {
		unsigned - two_256
	} else {
		unsigned
	}
}

/// Converts a fixed-point amount (stored as a BigInt) into a decimal string.
///
/// # Arguments
///
/// * `amount` - The raw amount as BigInt.
/// * `decimals` - The number of decimal places.
///
/// Returns a string representation of the amount.
pub fn convert_amount(amount: &BigInt, decimals: u32) -> String {
	let ten = BigInt::from_u8(10).expect("Failed to create BigInt from 10");
	let factor = ten.pow(decimals);
	let (quotient, remainder) = amount.div_rem(&factor);
	if remainder.is_zero() {
		quotient.to_string()
	} else {
		// Format with trimmed trailing zeros.
		let remainder_str = remainder.abs().to_string();
		let trimmed_remainder = remainder_str.trim_end_matches('0');
		format!("{}.{}", quotient, trimmed_remainder)
	}
}

/// Prints the swap events for a confirmed block.
pub fn print_swap_events(block: &ConfirmedBlock) {
	if block.events.is_empty() {
		println!("Block {}: No swap events", block.number);
		return;
	}
	for evt in &block.events {
		let direction = if evt.amount0 > num_bigint::BigInt::zero() &&
			evt.amount1 < num_bigint::BigInt::zero()
		{
			"DAI -> USDC"
		} else if evt.amount0 < num_bigint::BigInt::zero() &&
			evt.amount1 > num_bigint::BigInt::zero()
		{
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

#[cfg(test)]
mod tests {
	use crate::events;

	use super::*;
	use ethereum_types::U256;
	use num_bigint::BigInt;

	#[test]
	fn test_ethereum_int_to_bigint_positive() {
		let value = U256::from(1000u64);
		let bigint = events::ethereum_int_to_bigint(&value);
		assert_eq!(bigint, BigInt::from(1000));
	}

	#[test]
	fn test_ethereum_int_to_bigint_negative() {
		// In two's complement, -50 is represented as 2^256 - 50.
		let value = U256::max_value() - U256::from(49u64);
		let bigint = events::ethereum_int_to_bigint(&value);
		assert_eq!(bigint, BigInt::from(-50));
	}

	#[test]
	fn test_convert_amount_no_decimal() {
		// When the amount is exactly divisible by 10^decimals.
		let factor = BigInt::from(10u32).pow(18);
		let amount = BigInt::from(1234) * &factor;
		let result = events::convert_amount(&amount, 18);
		// Expect no fractional part if remainder is zero.
		assert_eq!(result, "1234");
	}

	#[test]
	fn test_convert_amount_with_decimal() {
		// Represent 1.5 as an amount with 18 decimals:
		// 1.5 * 10^18 = 1500000000000000000.
		let factor = BigInt::from(10u32).pow(18);
		let amount = BigInt::from(15) * &factor / BigInt::from(10); // equals 1500000000000000000
		let result = events::convert_amount(&amount, 18);
		// With our formatting (trimming trailing zeros), we expect "1.5".
		assert_eq!(result, "1.5");
	}
}
