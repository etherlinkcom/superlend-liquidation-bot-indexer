use std::sync::Arc;

use alloy::{
    network::Ethereum,
    primitives::{b256, Address},
    providers::Provider,
    rpc::types::Filter,
    sol_types::SolEventInterface,
};
use anyhow::{Context, Result};
use indexer_database::{entities::last_index_block, last_index_block_helper};
use sea_orm::DatabaseConnection;
use tokio::task::JoinHandle;
use tracing::{error, info, instrument};

use crate::{
    blockchain_manager::{multicall::MulticallManager, AaveHelperContract, BlockchainManager},
    config::LocalConfig,
    users_helper::UserHelper,
    utils::contracts::AavePoolContract::{self, AavePoolContractEvents},
};

/// Represents the main indexer for tracking user activities on Aave Pool
pub struct UsersIndexer;

/// Holds the current state of the Users Indexer
#[derive(Debug)]
pub struct UsersIndexerState {
    /// Initial block number from where indexing starts
    pub start_block: u64,
    /// Last processed block information
    pub last_index_block: last_index_block::Model,
    /// Current blockchain block number
    pub current_block: u64,
    /// Maximum allowed block lag before triggering reindex
    pub max_block_out_of_sync: u64,
    /// Number of blocks to process per iteration
    pub log_blocks_per_read: u64,
}

impl UsersIndexer {
    /// Creates a new instance of UsersIndexer
    ///
    /// # Returns
    /// * `Self` - A new UsersIndexer instance
    pub fn new() -> Self {
        Self
    }

    /// Starts the indexing process for user activities
    ///
    /// # Arguments
    /// * `db` - Arc wrapped database connection
    /// * `local_config` - Arc wrapped local configuration
    ///
    /// # Returns
    /// * `Result<JoinHandle<Result<()>>>` - A handle to the spawned indexing task
    #[instrument("USERS_INDEXER", skip(db, local_config))]
    pub async fn start_users_indexer(
        db: &Arc<DatabaseConnection>,
        local_config: &Arc<LocalConfig>,
    ) -> Result<JoinHandle<Result<()>>> {
        let db = db.clone();
        let local_config = local_config.clone();

        let handle = tokio::spawn(async move {
            info!("Starting indexer");

            // Initialize the last indexed block in database
            last_index_block_helper::init_last_index_block(&db, local_config.start_block).await?;

            let provider = BlockchainManager::get_provider(&local_config).await?;

            let mut multicall_manager = MulticallManager::new(&provider).await?;

            let aave_helper_contracts = Arc::new(
                BlockchainManager::get_aave_helper_contracts(&provider, &local_config).await?,
            );

            let aave_reserves = aave_helper_contracts
                .pool_contract
                .getReservesList()
                .call()
                .await?
                ._0;

            let mut users_indexer_state =
                Self::initialize_indexer_state(&db, &provider, &local_config).await?;

            Self::print_status(&users_indexer_state);

            loop {
                let next_to_block = Self::calculate_next_block(&users_indexer_state);

                if Self::should_wait(users_indexer_state.current_block as i64, next_to_block) {
                    tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                    users_indexer_state.current_block = provider.get_block_number().await?;
                    continue;
                }

                let logs = Self::fetch_logs(
                    &provider,
                    &local_config,
                    users_indexer_state.last_index_block.block_number as u64,
                    next_to_block as u64,
                )
                .await?;

                Self::process_logs(
                    &logs,
                    &db,
                    &local_config,
                    &aave_helper_contracts,
                    &aave_reserves,
                    &users_indexer_state,
                    &mut multicall_manager,
                )
                .await?;

                Self::update_states_and_print_status(
                    &db,
                    &mut users_indexer_state,
                    &provider,
                    next_to_block as u64,
                )
                .await?;
            }
        });

        Ok(handle)
    }

    /// Processes the logs and updates the users
    ///
    /// # Arguments
    /// * `logs` - Vector of blockchain logs
    /// * `db` - Database connection
    /// * `local_config` - Local configuration
    /// * `aave_helper_contracts` - Aave helper contracts
    /// * `aave_reserves` - Aave reserves
    /// * `users_indexer_state` - Users indexer state
    ///
    /// # Returns
    /// * `Result<()>` - A result of the operation
    #[instrument("USERS_INDEXER", skip_all)]
    async fn process_logs<'a, P: Provider<Ethereum>>(
        logs: &[alloy::rpc::types::Log],
        db: &DatabaseConnection,
        local_config: &LocalConfig,
        aave_helper_contracts: &Arc<AaveHelperContract<'a, P>>,
        aave_reserves: &Vec<Address>,
        users_indexer_state: &UsersIndexerState,
        multicall_manager: &mut MulticallManager<&'a P>,
    ) -> Result<()> {
        let borrow_events = Self::process_borrow_events(&logs)?;

        if !borrow_events.is_empty() {
            // Proccess each user by sending them to queue of mpsc channel
            for borrow_event in borrow_events {
                let user_address = borrow_event.user.to_string();
                info!("Updating user: {}", user_address);
                match UserHelper::update_user(
                    &db,
                    &local_config,
                    &user_address,
                    users_indexer_state.current_block,
                    &aave_helper_contracts,
                    &aave_reserves,
                    multicall_manager,
                )
                .await
                {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Failed to update user: {}", e);
                        return Err(e);
                    }
                }
            }
        }
        Ok(())
    }

    /// Updates the indexer states in database and prints the current status
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `users_indexer_state` - Current state of the indexer
    /// * `provider` - Blockchain provider
    /// * `next_to_block` - Next block number to process
    ///
    /// # Returns
    /// * `Result<()>` - A result of the operation
    async fn update_states_and_print_status(
        db: &DatabaseConnection,
        users_indexer_state: &mut UsersIndexerState,
        provider: &impl Provider,
        next_to_block: u64,
    ) -> Result<()> {
        users_indexer_state.current_block = provider.get_block_number().await?;
        users_indexer_state.last_index_block.block_number = next_to_block as i32;

        last_index_block_helper::update_last_index_block(
            db,
            users_indexer_state.last_index_block.clone(),
            users_indexer_state.last_index_block.block_number as u64,
        )
        .await?;

        Self::print_status(&users_indexer_state);

        Ok(())
    }

    /// Initializes the indexer state with current blockchain information
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `provider` - Blockchain provider
    /// * `local_config` - Local configuration
    ///
    /// # Returns
    /// * `Result<UsersIndexerState>` - Initialized indexer state
    async fn initialize_indexer_state(
        db: &DatabaseConnection,
        provider: &impl Provider,
        local_config: &LocalConfig,
    ) -> Result<UsersIndexerState> {
        Ok(UsersIndexerState {
            start_block: local_config.start_block,
            last_index_block: last_index_block_helper::get_last_index_block(db).await?,
            current_block: provider
                .get_block_number()
                .await
                .context("Failed to get current block")?,
            max_block_out_of_sync: local_config.max_block_lag,
            log_blocks_per_read: local_config.log_per_request,
        })
    }

    /// Fetches logs from the blockchain for the specified block range
    ///
    /// # Arguments
    /// * `provider` - Blockchain provider
    /// * `local_config` - Local configuration
    /// * `from_block` - Starting block number
    /// * `to_block` - Ending block number
    /// * `event_signature` - Event signature to filter logs
    ///
    /// # Returns
    /// * `Result<Vec<Log>>` - Vector of fetched logs
    async fn fetch_logs(
        provider: &impl Provider,
        local_config: &LocalConfig,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<alloy::rpc::types::Log>> {
        let filter = Filter::new()
            .address(vec![local_config.pool_address.parse()?])
            .event_signature(b256!(
                "b3d084820fb1a9decffb176436bd02558d15fac9b0ddfed8c465bc7359d7dce0"
            ))
            .from_block(from_block)
            .to_block(to_block);

        provider.get_logs(&filter).await.map_err(Into::into)
    }

    /// Processes blockchain logs to extract borrow events
    ///
    /// # Arguments
    /// * `logs` - Vector of blockchain logs
    ///
    /// # Returns
    /// * `Result<Vec<AavePoolContract::Borrow>>` - Vector of processed borrow events
    fn process_borrow_events(
        logs: &[alloy::rpc::types::Log],
    ) -> Result<Vec<AavePoolContract::Borrow>> {
        Ok(logs
            .iter()
            .map(|log| {
                AavePoolContractEvents::decode_log(&log.inner, false).map_err(anyhow::Error::from)
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter_map(|event| event.as_borrow().cloned())
            .collect())
    }

    /// Calculates the next block number to process and ensures it doesn't exceed the current block number
    ///
    /// # Arguments
    /// * `last_index_block` - Last processed block number
    /// * `log_blocks_per_read` - Number of blocks to process per iteration
    /// * `current_block` - Current blockchain block number
    ///
    /// # Returns
    /// * `i64` - Next block number to process
    fn calculate_next_block(users_indexer_state: &UsersIndexerState) -> i64 {
        // check if the diffrence between last and current is bigger or equal to logs_per_request
        if users_indexer_state.current_block as i64
            - users_indexer_state.last_index_block.block_number as i64
            >= users_indexer_state.log_blocks_per_read as i64
        {
            return users_indexer_state.last_index_block.block_number as i64
                + users_indexer_state.log_blocks_per_read as i64;
        }
        // else if check the diffrence between last and current is bigger then 20
        else if users_indexer_state.current_block as i64
            - users_indexer_state.last_index_block.block_number as i64
            >= users_indexer_state.max_block_out_of_sync as i64
        {
            return users_indexer_state.last_index_block.block_number as i64
                + users_indexer_state.max_block_out_of_sync as i64;
        }

        // else return the last index block + log_blocks_per_read this will be handled by the wait function
        users_indexer_state.last_index_block.block_number as i64
            + users_indexer_state.log_blocks_per_read as i64
    }

    /// Determines if the indexer should wait before processing next blocks
    ///
    /// # Arguments
    /// * `current_block` - Current blockchain block number
    /// * `next_to_block` - Next block number to process
    ///
    /// # Returns
    /// * `bool` - True if should wait, false otherwise
    fn should_wait(current_block: i64, next_to_block: i64) -> bool {
        next_to_block > current_block
    }

    /// Prints the current status of the indexer
    ///
    /// # Arguments
    /// * `users_indexer_state` - Current state of the indexer
    #[instrument("USERS_INDEXER", skip(users_indexer_state))]
    fn print_status(users_indexer_state: &UsersIndexerState) {
        let progress = ((users_indexer_state.last_index_block.block_number as f64
            - users_indexer_state.start_block as f64)
            / (users_indexer_state.current_block as f64 - users_indexer_state.start_block as f64))
            * 100.0;

        info!(
            "Last index block: {} | Current block: {} | In Sync: {:.4}%",
            users_indexer_state.last_index_block.block_number,
            users_indexer_state.current_block,
            progress
        );
    }
}
