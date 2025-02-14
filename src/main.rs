use anyhow::Context;
use dotenv::dotenv;
use futures::StreamExt;
use std::env;
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	dotenv().ok();
	let eth_node_url = env::var("INFURA_URL").context("INFURA_URL must be set")?;
	let eth_pool_address =
		env::var("USDC_DAI_UNISWAP_POOL_CONTRACT").context("Pool contract address must be set")?;
	let web3 = web3::Web3::new(web3::transports::ws::WebSocket::new(eth_node_url.as_str()).await?);

	let contract_address =
		web3::types::H160::from_slice(&hex::decode(eth_pool_address).unwrap()[..]);

	let contract = web3::contract::Contract::from_json(
		web3.eth(),
		contract_address,
		include_bytes!("contracts/uniswap_pool_abi.json"),
	)?;
	let swap_event = contract.abi().events_by_name("Swap")?.first().unwrap();
	let swap_event_signature = swap_event.signature();

	let mut block_stream = web3.eth_subscribe().subscribe_new_heads().await?;

	while let Some(Ok(block)) = block_stream.next().await {
		let swap_logs_in_block = web3
			.eth()
			.logs(
				web3::types::FilterBuilder::default()
					.block_hash(block.hash.unwrap())
					.address(vec![contract_address])
					.topics(Some(vec![swap_event_signature]), None, None, None)
					.build(),
			)
			.await?;

		for log in swap_logs_in_block {
			let parsed_log = swap_event
				.parse_log(web3::ethabi::RawLog { topics: log.topics, data: log.data.0 })?;
			println!("{:?}", parsed_log);
		}
	}

	Ok(())
}
