use anyhow::{Context, Result};
use std::env;

/// Holds configuration parameters read from the environment.
pub struct Config {
	pub eth_node_url: String,
	pub pool_contract_address: String,
}

impl Config {
	/// Loads configuration from environment variables.
	pub fn from_env() -> Result<Self> {
		let eth_node_url =
			env::var("INFURA_URL").context("INFURA_URL environment variable must be set")?;
		let pool_contract_address = env::var("USDC_DAI_UNISWAP_POOL_CONTRACT")
			.context("USDC_DAI_UNISWAP_POOL_CONTRACT must be set")?;
		Ok(Self { eth_node_url, pool_contract_address })
	}
}
