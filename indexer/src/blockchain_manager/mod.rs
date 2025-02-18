pub mod multicall;

use alloy::{
    network::Ethereum,
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    rpc::client::RpcClient,
    transports::{http::reqwest::Url, layers::RetryBackoffLayer},
};
use anyhow::{Ok, Result};

use crate::{
    config::LocalConfig,
    utils::contracts::{AavePoolContract, AavePoolDataProviderContract},
};

/// BlockchainManager handles blockchain-related operations and connections.
/// It provides functionality to create provider instances and contract interactions.
pub struct BlockchainManager;

pub struct AaveHelperContract<'a, P: Provider<Ethereum>> {
    pub pool_contract: AavePoolContract::AavePoolContractInstance<(), &'a P>,
    pub pool_data_provider_contract:
        AavePoolDataProviderContract::AavePoolDataProviderContractInstance<(), &'a P>,
}

impl BlockchainManager {
    /// Creates and returns a WebSocket provider instance for blockchain interactions.
    ///
    /// # Arguments
    /// * `local_config` - Local configuration containing the RPC URL
    ///
    /// # Returns
    /// * `Result<impl Provider<Ethereum>>` - A Result containing either the provider instance or an error
    pub async fn get_provider(
        local_config: &LocalConfig,
    ) -> Result<impl alloy::providers::Provider<Ethereum>> {
        // Instantiate the RetryBackoffLayer with the configuration
        let retry_layer = RetryBackoffLayer::new(10, 1000, 10000);

        let client = RpcClient::builder()
            .layer(retry_layer)
            .http(Url::parse(&local_config.rpc_url)?);

        let provider = ProviderBuilder::new().on_client(client);

        Ok(provider)
    }

    pub async fn get_aave_helper_contracts<'a, P: Provider<Ethereum>>(
        provider: &'a P,
        local_config: &LocalConfig,
    ) -> Result<AaveHelperContract<'a, P>> {
        let contract = AaveHelperContract {
            pool_contract: Self::get_aave_pool_contract(
                provider,
                local_config.pool_address.parse()?,
            )
            .await?,
            pool_data_provider_contract: Self::get_aave_pool_data_provider_contract(
                provider,
                local_config.pool_data_provider.parse()?,
            )
            .await?,
        };
        Ok(contract)
    }

    pub async fn get_aave_pool_contract<'a, P: Provider<Ethereum>>(
        provider: &'a P,
        address: Address,
    ) -> Result<AavePoolContract::AavePoolContractInstance<(), &'a P>> {
        let contract = AavePoolContract::new(address, provider);
        Ok(contract)
    }

    pub async fn get_aave_pool_data_provider_contract<'a, P: Provider<Ethereum>>(
        provider: &'a P,
        address: Address,
    ) -> Result<AavePoolDataProviderContract::AavePoolDataProviderContractInstance<(), &'a P>> {
        let contract = AavePoolDataProviderContract::new(address, provider);
        Ok(contract)
    }
}
