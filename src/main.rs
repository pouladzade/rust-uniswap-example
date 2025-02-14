use anyhow::{Context, Result};
use dotenv::dotenv;
use futures::StreamExt;
use log::{error, info, warn};
use rust_uniswap_task::{config::*, ethereum, events, reorg};
use std::collections::BTreeMap;
use web3::types::{H160, U64};

#[tokio::main]
async fn main() -> Result<()> {
	dotenv().ok();

	let config = Config::from_env()?;
	info!(
		"Starting with configuration: node URL: {}, pool contract: {}",
		config.eth_node_url, config.pool_contract_address
	);

	let web3 = ethereum::create_web3(&config.eth_node_url).await?;
	let pool_address_bytes = hex::decode(&config.pool_contract_address)
		.context("Failed to decode pool contract address")?;
	let contract_address = H160::from_slice(&pool_address_bytes);

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
	info!("Block subscription started");

	let mut pending_blocks: BTreeMap<U64, events::ConfirmedBlock> = BTreeMap::new();
	while let Some(message) = block_stream.next().await {
		match message {
			Ok(block_header) => {
				let block_hash = match block_header.hash {
					Some(hash) => hash,
					None => {
						warn!("Received block without hash; skipping");
						continue;
					},
				};
				let block_number = match block_header.number {
					Some(num) => num,
					None => {
						warn!("Received block without number; skipping");
						continue;
					},
				};
				info!("Processing block {}", block_number);

				// Fetch logs for the Swap event in this block.
				let filter = web3::types::FilterBuilder::default()
					.block_hash(block_hash)
					.address(vec![contract_address])
					.topics(Some(vec![swap_event_signature]), None, None, None)
					.build();
				let swap_logs =
					web3.eth().logs(filter).await.context("Failed to fetch logs for block")?;
				let events_vec = swap_logs.iter().filter_map(events::decode_swap_event).collect();
				let confirmed_block = events::ConfirmedBlock {
					number: block_number,
					hash: block_hash,
					events: events_vec,
				};
				pending_blocks.insert(block_number, confirmed_block);

				// Confirm blocks that are at least 5 blocks deep.
				let confirmed_cutoff = block_number - U64::from(5u64);
				match reorg::check_confirmed_blocks(&web3, &pending_blocks, confirmed_cutoff).await
				{
					Ok(to_print) =>
						for bn in to_print {
							if let Some(cb) = pending_blocks.remove(&bn) {
								events::print_swap_events(&cb);
							}
						},
					Err(e) => {
						error!("Error during reorg check: {:?}", e);
						return Err(e);
					},
				}
			},
			Err(e) => {
				error!("Error receiving block header: {:?}", e);
			},
		}
	}
	Ok(())
}
