use crate::{ethereum::fetch_block, events::ConfirmedBlock};
use anyhow::{bail, Result};
use std::collections::BTreeMap;
use web3::{transports::ws::WebSocket, types::U64, Web3};

/// Checks pending blocks to determine which blocks are confirmed (i.e., at least 5 blocks deep)
/// and validates that their hashes match to prevent reorganizations.
///
/// Returns a vector of block numbers that are confirmed.
pub async fn check_confirmed_blocks(
	web3: &Web3<WebSocket>,
	pending_blocks: &BTreeMap<U64, ConfirmedBlock>,
	confirmed_cutoff: U64,
) -> Result<Vec<U64>> {
	let mut to_print = Vec::new();
	for (&block_num, pending_block) in pending_blocks.iter() {
		if block_num <= confirmed_cutoff {
			if let Some(fetched_block) = fetch_block(web3, block_num).await? {
				if fetched_block.hash != Some(pending_block.hash) {
					bail!("Reorganization detected at block {}. Expected hash: {:?}, got: {:?}. Reorg depth greater than 5 detected.",
                              block_num, pending_block.hash, fetched_block.hash);
				} else {
					to_print.push(block_num);
				}
			}
		}
	}
	Ok(to_print)
}
