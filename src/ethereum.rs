use anyhow::{Context, Result};
use web3::{
	transports::ws::WebSocket,
	types::{Block, BlockId, BlockNumber, H256, U64},
	Web3,
};

/// Creates a new Web3 client using a WebSocket transport.
pub async fn create_web3(url: &str) -> Result<Web3<WebSocket>> {
	let ws = WebSocket::new(url)
		.await
		.context("Failed to connect to Ethereum node via WebSocket")?;
	Ok(Web3::new(ws))
}

/// Fetches a block by its number.
pub async fn fetch_block(web3: &Web3<WebSocket>, block_number: U64) -> Result<Option<Block<H256>>> {
	web3.eth()
		.block(BlockId::Number(BlockNumber::Number(block_number)))
		.await
		.context("Failed to fetch block")
}
