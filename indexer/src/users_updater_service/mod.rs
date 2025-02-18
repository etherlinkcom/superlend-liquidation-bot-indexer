use std::sync::Arc;

use alloy::{network::Ethereum, primitives::Address, providers::Provider};
use anyhow::Result;
use indexer_database::users_tables_helper;
use sea_orm::DatabaseConnection;
use tokio::task::JoinHandle;
use tracing::{error, info, instrument};

use crate::{
    blockchain_manager::{multicall::MulticallManager, AaveHelperContract, BlockchainManager},
    config::LocalConfig,
    users_helper::UserHelper,
};

pub struct UsersUpdaterService;

impl UsersUpdaterService {
    #[instrument("UPDATER_SERVICE", skip(db, local_config))]
    pub async fn start_users_updater_service(
        db: &DatabaseConnection,
        local_config: &Arc<LocalConfig>,
    ) -> Result<JoinHandle<Result<()>>> {
        let db = db.clone();
        let local_config = local_config.clone();

        let handle = tokio::spawn(async move {
            info!("Starting updater service");

            let mut last_liquidatable_users_update = chrono::Utc::now().timestamp() as u64;
            let mut last_at_risk_users_update = chrono::Utc::now().timestamp() as u64;
            let mut last_healthy_users_update = chrono::Utc::now().timestamp() as u64;

            let provider = BlockchainManager::get_provider(&local_config).await?;

            let aave_helper_contracts = Arc::new(
                BlockchainManager::get_aave_helper_contracts(&provider, &local_config).await?,
            );

            let aave_reserves = aave_helper_contracts
                .pool_contract
                .getReservesList()
                .call()
                .await?
                ._0;

            let mut multicall_manager = MulticallManager::new(&provider).await?;

            loop {
                let now = chrono::Utc::now().timestamp() as u64;
                let block_number = provider.get_block_number().await?;

                // Update liquidatable users
                if now - last_liquidatable_users_update
                    >= local_config.liquidatable_users_update_frequency
                {
                    info!("Updating liquidatable users");
                    match Self::update_liquidatable_users(
                        &db,
                        &local_config,
                        &aave_helper_contracts,
                        &aave_reserves,
                        block_number,
                        &mut multicall_manager,
                    )
                    .await
                    {
                        Ok(_) => {
                            info!("Liquidatable users updated");
                            last_liquidatable_users_update = now;
                        }
                        Err(e) => error!("Error updating liquidatable users: {}", e),
                    }
                }

                // Update at risk users
                if now - last_at_risk_users_update >= local_config.at_risk_users_update_frequency {
                    info!("Updating at risk users");
                    match Self::update_at_risk_users(
                        &db,
                        &local_config,
                        &aave_helper_contracts,
                        &aave_reserves,
                        block_number,
                        &mut multicall_manager,
                    )
                    .await
                    {
                        Ok(_) => {
                            info!("At risk users updated");
                            last_at_risk_users_update = now;
                        }
                        Err(e) => error!("Error updating at risk users: {}", e),
                    }
                }

                // Update healthy users
                if now - last_healthy_users_update >= local_config.healthy_users_update_frequency {
                    info!("Updating healthy users");
                    match Self::update_healthy_users(
                        &db,
                        &local_config,
                        &aave_helper_contracts,
                        &aave_reserves,
                        block_number,
                        &mut multicall_manager,
                    )
                    .await
                    {
                        Ok(_) => {
                            info!("Healthy users updated");
                            last_healthy_users_update = now;
                        }
                        Err(e) => error!("Error updating healthy users: {}", e),
                    }
                }

                // Wait for the next update
                tokio::time::sleep(std::time::Duration::from_secs(
                    local_config.liquidatable_users_update_frequency,
                ))
                .await;
            }
        });
        Ok(handle)
    }

    #[instrument("UPDATE_LIQUIDATABLE_USERS", skip_all)]
    async fn update_liquidatable_users<'a, P: Provider<Ethereum>>(
        db: &DatabaseConnection,
        local_config: &Arc<LocalConfig>,
        aave_helper_contracts: &Arc<AaveHelperContract<'a, P>>,
        aave_reserves: &Vec<Address>,
        block_number: u64,
        multicall_manager: &mut MulticallManager<&'a P>,
    ) -> Result<()> {
        let liquidatable_users = users_tables_helper::get_all_liquidatable_users(db).await?;
        for user in liquidatable_users {
            info!("Updating user: {}", user);

            UserHelper::update_user(
                db,
                local_config,
                &user,
                block_number,
                aave_helper_contracts,
                aave_reserves,
                multicall_manager,
            )
            .await?;
        }
        Ok(())
    }

    #[instrument("UPDATE_AT_RISK_USERS", skip_all)]
    async fn update_at_risk_users<'a, P: Provider<Ethereum>>(
        db: &DatabaseConnection,
        local_config: &Arc<LocalConfig>,
        aave_helper_contracts: &Arc<AaveHelperContract<'a, P>>,
        aave_reserves: &Vec<Address>,
        block_number: u64,
        multicall_manager: &mut MulticallManager<&'a P>,
    ) -> Result<()> {
        let at_risk_users = users_tables_helper::get_all_at_risk_users(db).await?;
        for user in at_risk_users {
            info!("Updating user: {}", user);

            UserHelper::update_user(
                db,
                local_config,
                &user,
                block_number,
                aave_helper_contracts,
                aave_reserves,
                multicall_manager,
            )
            .await?;
        }
        Ok(())
    }

    #[instrument("UPDATE_HEALTHY_USERS", skip_all)]
    async fn update_healthy_users<'a, P: Provider<Ethereum>>(
        db: &DatabaseConnection,
        local_config: &Arc<LocalConfig>,
        aave_helper_contracts: &Arc<AaveHelperContract<'a, P>>,
        aave_reserves: &Vec<Address>,
        block_number: u64,
        multicall_manager: &mut MulticallManager<&'a P>,
    ) -> Result<()> {
        let healthy_users = users_tables_helper::get_all_healthy_users(db).await?;
        for user in healthy_users {
            info!("Updating user: {}", user);
            UserHelper::update_user(
                db,
                local_config,
                &user,
                block_number,
                aave_helper_contracts,
                aave_reserves,
                multicall_manager,
            )
            .await?;
        }
        Ok(())
    }
}
