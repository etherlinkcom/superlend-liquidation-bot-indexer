use anyhow::{Context, Result};
use sea_orm::{sea_query::OnConflict, DatabaseConnection, EntityTrait, Set};
use tracing::debug;

use crate::entities::user_debt_collateral;

/// Updates or creates user's collateral and debt positions in the database
///
/// This function handles both the creation of new positions and updates to existing ones.
/// It uses a batch insert with ON CONFLICT DO UPDATE strategy for efficiency.
///
/// # Arguments
/// * `db` - Database connection handle
/// * `user_address` - Ethereum address of the user
/// * `collateral_assets` - Vector of (asset_address, amount) pairs for collateral positions
/// * `debt_assets` - Vector of (asset_address, amount) pairs for debt positions
///
/// # Returns
/// * `Result<()>` - Success or error result of the database operation
pub async fn add_or_update_user_debt_collateral(
    db: &DatabaseConnection,
    user_address: &str,
    collateral_assets: Vec<(String, f32)>,
    debt_assets: Vec<(String, f32)>,
) -> Result<()> {
    let timestamp = chrono::Utc::now().naive_utc();
    let mut models = Vec::with_capacity(collateral_assets.len() + debt_assets.len());

    // Process collateral positions
    for (reserve_address, amount) in collateral_assets {
        models.push(create_position_model(
            user_address,
            reserve_address,
            amount,
            true,
            timestamp,
        ));
    }

    // Process debt positions
    for (reserve_address, amount) in debt_assets {
        models.push(create_position_model(
            user_address,
            reserve_address,
            amount,
            false,
            timestamp,
        ));
    }

    // Skip database operation if no positions to update
    if models.is_empty() {
        debug!("No positions to update for user {}", user_address);
        return Ok(());
    }

    debug!(
        "Updating {} positions for user {}",
        models.len(),
        user_address
    );

    // Perform batch insert/update
    user_debt_collateral::Entity::insert_many(models)
        .on_conflict(
            OnConflict::columns([
                user_debt_collateral::Column::UserAddress,
                user_debt_collateral::Column::ReserveAddress,
                user_debt_collateral::Column::IsCollateral,
            ])
            .update_columns([
                user_debt_collateral::Column::Amount,
                user_debt_collateral::Column::Timestamp,
            ])
            .to_owned(),
        )
        .exec(db)
        .await
        .context("Failed to update user positions")?;

    Ok(())
}

/// Creates an ActiveModel for a user's position
fn create_position_model(
    user_address: &str,
    reserve_address: String,
    amount: f32,
    is_collateral: bool,
    timestamp: chrono::NaiveDateTime,
) -> user_debt_collateral::ActiveModel {
    user_debt_collateral::ActiveModel {
        user_address: Set(user_address.to_string()),
        reserve_address: Set(reserve_address),
        amount: Set(amount),
        is_collateral: Set(is_collateral),
        timestamp: Set(timestamp),
        ..Default::default()
    }
}
