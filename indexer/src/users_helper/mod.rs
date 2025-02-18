mod models;

use std::sync::Arc;

use alloy::{
    network::Ethereum,
    primitives::{Address, Bytes},
    providers::Provider,
    sol_types::SolCall,
};
use anyhow::{Context, Result};
use chrono::Utc;
use indexer_database::{
    user_debt_collateral_helper,
    users_tables_helper::{self, UserCurrentLocation, UserDetails},
};
use sea_orm::DatabaseConnection;
use tracing::info;

use crate::{
    blockchain_manager::{multicall::MulticallManager, AaveHelperContract},
    config::LocalConfig,
    utils::{
        constants::{
            HEALTH_FACTOR_DECIMALS, LIQUIDATION_THRESHOLD, TOKEN_BALANCE_DECIMALS,
            USD_VALUE_DECIMALS,
        },
        contracts::{AavePoolContract, AavePoolDataProviderContract},
        math_helper,
    },
};

/// Helper struct for managing user-related operations in the Aave protocol
pub struct UserHelper;

impl UserHelper {
    /// Updates the user's data in the database with the given block number.
    /// This function fetches users' health factor and reserves data from the blockchain
    /// and updates the user in the database. It also moves users across 3 different
    /// health factor tiers based on their current health factor.
    ///
    /// # Arguments
    /// * `db` - Database connection handle
    /// * `local_config` - Local configuration settings
    /// * `user_address` - Ethereum address of the user
    /// * `block_number` - Current block number being processed
    /// * `aave_helper_contracts` - Arc reference to Aave protocol contract helpers
    /// * `aave_reserves` - List of Aave reserve token addresses
    ///
    /// # Returns
    /// * `Result<()>` - Success or error result of the update operation
    pub async fn update_user<'a, P: Provider<Ethereum>>(
        db: &DatabaseConnection,
        local_config: &LocalConfig,
        user_address: &str,
        block_number: u64,
        aave_helper_contracts: &Arc<AaveHelperContract<'a, P>>,
        aave_reserves: &[Address],
        multicall_manager: &mut MulticallManager<&'a P>,
    ) -> Result<()> {
        // Get user details
        let user_details = users_tables_helper::get_user(db, &user_address).await?;

        Self::update_user_in_db(
            db,
            user_address,
            block_number,
            aave_helper_contracts,
            aave_reserves,
            user_details,
            local_config,
            multicall_manager,
        )
        .await?;

        Ok(())
    }

    /// Updates a user's data in the database if necessary based on block lag settings
    /// This is an internal function called by update_user that handles the actual database operations
    ///
    /// # Arguments
    /// * `db` - Database connection handle
    /// * `user_address` - Ethereum address of the user
    /// * `block_number` - Current block number being processed
    /// * `aave_helper_contracts` - Arc reference to Aave protocol contract helpers
    /// * `aave_reserves` - List of Aave reserve token addresses
    /// * `user_details` - Optional existing user details from database
    /// * `local_config` - Local configuration settings
    ///
    /// # Returns
    /// * `Result<()>` - Success or error result of the database update operation
    async fn update_user_in_db<'a, P: Provider<Ethereum>>(
        db: &DatabaseConnection,
        user_address: &str,
        block_number: u64,
        aave_helper_contracts: &Arc<AaveHelperContract<'a, P>>,
        aave_reserves: &[Address],
        user_details: Option<users_tables_helper::UserDetails>,
        local_config: &LocalConfig,
        multicall_manager: &mut MulticallManager<&'a P>,
    ) -> Result<()> {
        // Skip update if user data is recent enough
        if let Some(details) = user_details.as_ref() {
            let blocks_since_last_update: i64 =
                block_number as i64 - details.last_updated_block_number as i64;
            if blocks_since_last_update < local_config.max_block_lag as i64 {
                info!(
                    "User {} data is recent (last updated: block {}, current: block {})",
                    user_address, details.last_updated_block_number, block_number
                );
                return Ok(());
            }
        }

        multicall_manager.add_call(
            &aave_helper_contracts.pool_contract.address(),
            &aave_helper_contracts
                .pool_contract
                .getUserAccountData(user_address.parse()?)
                .calldata(),
        );

        for reserve in aave_reserves {
            multicall_manager.add_call(
                &aave_helper_contracts.pool_data_provider_contract.address(),
                &aave_helper_contracts
                    .pool_data_provider_contract
                    .getUserReserveData(reserve.clone(), user_address.parse()?)
                    .calldata(),
            );
        }

        let results = multicall_manager.execute_calls(block_number).await?;
        multicall_manager.clear_calls();

        let user_account_data = AavePoolContract::getUserAccountDataCall::abi_decode_returns(
            results[0].as_ref(),
            false,
        )?;

        // Fetch user's current state from blockchain
        let (health_factor, total_collateral_usd, total_debt_usd) =
            Self::get_user_data(&user_account_data, local_config.max_cap_on_health_factor)
                .await
                .context("Failed to fetch user data from blockchain")?;

        // Get detailed reserve data for user's positions
        let user_positions = Self::get_user_reserve_data(&results[1..], aave_reserves)
            .await
            .context("Failed to fetch user reserve data")?;

        // Update user's risk category and basic info
        Self::add_or_update_user_to_db(
            db,
            local_config,
            user_address,
            block_number,
            health_factor,
            total_collateral_usd,
            total_debt_usd,
            user_positions.clone(),
            user_details,
        )
        .await
        .context("Failed to update user basic information")?;

        // Update user's detailed position data
        Self::add_or_update_user_debt_collateral(
            db,
            user_address,
            user_positions.collateral_assets,
            user_positions.debt_assets,
        )
        .await
        .context("Failed to update user positions")?;

        Ok(())
    }

    /// Fetches user's health factor and collateral/debt values from the Aave pool contract
    ///
    /// # Arguments
    /// * `pool_contract` - Reference to the Aave pool contract instance
    /// * `user_address` - Ethereum address of the user
    /// * `max_health_factor` - Maximum allowed health factor value
    ///
    /// # Returns
    /// * `Result<(f64, f64, f64)>` - Tuple containing (health_factor, total_collateral_value_in_usd, total_debt_value_in_usd)
    async fn get_user_data(
        account_data: &AavePoolContract::getUserAccountDataReturn,
        max_health_factor: u64,
    ) -> Result<(f64, f64, f64)> {
        // Calculate and cap health factor

        // Calculate and cap health factor
        let mut health_factor =
            math_helper::divide_by_precision_f64(account_data.healthFactor, HEALTH_FACTOR_DECIMALS);
        health_factor = health_factor.min(max_health_factor as f64);

        // Calculate USD values
        let collateral_usd = math_helper::divide_by_precision_f64(
            account_data.totalCollateralBase,
            USD_VALUE_DECIMALS,
        );
        let debt_usd =
            math_helper::divide_by_precision_f64(account_data.totalDebtBase, USD_VALUE_DECIMALS);

        Ok((health_factor, collateral_usd, debt_usd))
    }

    /// Fetches detailed reserve data for a user from the Aave pool data provider
    ///
    /// # Arguments
    /// * `pool_data_provider` - Reference to the Aave pool data provider contract
    /// * `reserves` - List of reserve token addresses to check
    /// * `user_address` - Ethereum address of the user
    ///
    /// # Returns
    /// * `Result<models::UserReserveData>` - Structured data containing user's collateral and debt positions
    async fn get_user_reserve_data(
        results: &[Bytes],
        reserves: &[Address],
    ) -> Result<models::UserReserveData> {
        let mut collateral_positions = Vec::new();
        let mut debt_positions = Vec::new();

        for (reserve, result) in reserves.iter().zip(results.iter()) {
            let position =
                AavePoolDataProviderContract::getUserReserveDataCall::abi_decode_returns(
                    result.as_ref(),
                    false,
                )?;

            // Process collateral position
            if !position.currentATokenBalance.is_zero() {
                let balance = math_helper::divide_by_precision_f64(
                    position.currentATokenBalance,
                    TOKEN_BALANCE_DECIMALS,
                );
                collateral_positions.push((reserve.to_string(), balance as f32));
            }

            // Process debt position
            if !position.currentVariableDebt.is_zero() {
                let balance = math_helper::divide_by_precision_f64(
                    position.currentVariableDebt,
                    TOKEN_BALANCE_DECIMALS,
                );
                debt_positions.push((reserve.to_string(), balance as f32));
            }
        }

        Ok(models::UserReserveData::new(
            collateral_positions,
            debt_positions,
        ))
    }

    /// Determines the user's risk category based on their health factor
    ///
    /// # Arguments
    /// * `health_factor` - User's current health factor
    /// * `at_risk_threshold` - Threshold for considering a position at risk
    ///
    /// # Returns
    /// * `UserCurrentLocation` - User's risk category (Healthy, AtRisk, or Liquidatable)
    fn get_user_new_location(health_factor: f64, at_risk_threshold: f64) -> UserCurrentLocation {
        if health_factor < LIQUIDATION_THRESHOLD {
            UserCurrentLocation::Liquidatable
        } else if health_factor <= at_risk_threshold {
            UserCurrentLocation::AtRisk
        } else {
            UserCurrentLocation::Healthy
        }
    }

    /// Adds or updates a user's basic information in the database
    ///
    /// # Arguments
    /// * `db` - Database connection handle
    /// * `local_config` - Local configuration settings
    /// * `user_address` - Ethereum address of the user
    /// * `block_number` - Current block number
    /// * `health_factor` - User's current health factor
    /// * `total_collateral_value_in_usd` - Total USD value of user's collateral
    /// * `total_debt_value_in_usd` - Total USD value of user's debt
    /// * `user_reserve_data` - Detailed data about user's positions in different reserves
    /// * `user_details` - Optional existing user details from database
    ///
    /// # Returns
    /// * `Result<()>` - Success or error result of the database operation
    async fn add_or_update_user_to_db(
        db: &DatabaseConnection,
        local_config: &LocalConfig,
        user_address: &str,
        block_number: u64,
        health_factor: f64,
        total_collateral_value_in_usd: f64,
        total_debt_value_in_usd: f64,
        user_reserve_data: models::UserReserveData,
        user_details: Option<users_tables_helper::UserDetails>,
    ) -> Result<()> {
        let user_old_location = match user_details.as_ref() {
            Some(user_details) => user_details.current_location.clone(),
            None => users_tables_helper::UserCurrentLocation::NotFound,
        };

        let new_location =
            Self::get_user_new_location(health_factor, local_config.at_risk_health_factor);

        // If user location has changed, update the user location or in case of not found, add the user to the database
        if user_old_location != new_location {
            let need_deletion = match user_old_location {
                users_tables_helper::UserCurrentLocation::NotFound => false,
                _ => true,
            };

            let user_details = match user_details {
                Some(user) => {
                    let mut user = user;
                    user.health_factor = health_factor as f32;
                    user.last_updated_block_number = block_number as i32;
                    user.total_collateral_value_in_usd = total_collateral_value_in_usd as f32;
                    user.total_debt_value_in_usd = total_debt_value_in_usd as f32;
                    user.leading_collateral_reserve = user_reserve_data.leading_collateral_reserve;
                    user.leading_debt_reserve = user_reserve_data.leading_debt_reserve;
                    user.leading_collateral_reserve_value =
                        user_reserve_data.leading_collateral_reserve_token_value;
                    user.leading_debt_reserve_value =
                        user_reserve_data.leading_debt_reserve_token_value;
                    user.timestamp = Utc::now();
                    user
                }
                None => UserDetails {
                    id: 0,
                    user_address: user_address.to_string(),
                    last_updated_block_number: block_number as i32,
                    health_factor: health_factor as f32,
                    total_collateral_value_in_usd: total_collateral_value_in_usd as f32,
                    total_debt_value_in_usd: total_debt_value_in_usd as f32,
                    leading_collateral_reserve: user_reserve_data.leading_collateral_reserve,
                    leading_debt_reserve: user_reserve_data.leading_debt_reserve,
                    leading_collateral_reserve_value: user_reserve_data
                        .leading_collateral_reserve_token_value,
                    leading_debt_reserve_value: user_reserve_data.leading_debt_reserve_token_value,
                    timestamp: Utc::now(),
                    current_location: user_old_location.clone(),
                },
            };

            if need_deletion {
                // Delete the user from the database
                users_tables_helper::delete_user(db, user_details.id, user_old_location.clone())
                    .await
                    .context("Failed to delete user from the database")?;
            }

            // Add the user to the database
            users_tables_helper::add_user(db, user_details, new_location.clone())
                .await
                .context("Failed to add user to the database")?;

            info!(
                "Moved user [HF: {}] {} from {:?} to {:?}",
                health_factor, user_address, user_old_location, new_location
            );
        }
        // Update the user at the same location
        else {
            info!(
                "User [HF: {}] {} is at {:?}, updating user",
                health_factor, user_address, user_old_location
            );
            let user_details = user_details.unwrap();
            users_tables_helper::update_user(
                db,
                user_details.id,
                user_details,
                new_location.clone(),
            )
            .await
            .context("Failed to update user in the database")?;
        }

        Ok(())
    }

    /// Updates or adds a user's detailed collateral and debt positions in the database
    ///
    /// # Arguments
    /// * `db` - Database connection handle
    /// * `user_address` - Ethereum address of the user
    /// * `collateral_assets` - Vector of (asset_address, amount) pairs for collateral
    /// * `debt_assets` - Vector of (asset_address, amount) pairs for debt
    ///
    /// # Returns
    /// * `Result<()>` - Success or error result of the database operation
    async fn add_or_update_user_debt_collateral(
        db: &DatabaseConnection,
        user_address: &str,
        collateral_assets: Vec<(String, f32)>,
        debt_assets: Vec<(String, f32)>,
    ) -> Result<()> {
        user_debt_collateral_helper::add_or_update_user_debt_collateral(
            db,
            user_address,
            collateral_assets,
            debt_assets,
        )
        .await?;

        Ok(())
    }
}
