use anyhow::Result;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, Set};
use tracing::info;

use crate::entities::last_index_block::{ActiveModel as LastIndexBlockActiveModel, Model};
use crate::entities::prelude::LastIndexBlock;

/// Initializes the last indexed block in the database if it doesn't exist
///
/// This function checks if there's already a record of the last indexed block.
/// If no record exists, it creates a new one with the provided start block number
/// and current timestamp.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `start_block` - The initial block number to start indexing from
///
/// # Returns
///
/// * `Result<(), DbErr>` - Success if initialization is complete or block already exists,
///                         error if database operations fail
pub async fn init_last_index_block(db: &DatabaseConnection, start_block: u64) -> Result<(), DbErr> {
    info!("Checking if last index block exists");
    let last_index_block = LastIndexBlock::find().one(db).await?;
    if last_index_block.is_some() {
        info!("Last index block already exists");
        return Ok(());
    }

    info!("Initializing last index block");
    let last_index_block = LastIndexBlockActiveModel {
        block_number: Set(start_block as i32),
        timestamp: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    };

    last_index_block.insert(db).await?;

    info!("Last index block initialized");

    Ok(())
}

/// Retrieves the last indexed block from the database
///
/// # Arguments
///
/// * `db` - Database connection
///
/// # Returns
///
/// * `Result<Model>` - The last indexed block model if found,
///                     error if not found or database operation fails
pub async fn get_last_index_block(db: &DatabaseConnection) -> Result<Model> {
    let last_index_block = LastIndexBlock::find().one(db).await?;
    Ok(last_index_block.ok_or(anyhow::anyhow!("Last index block not found"))?)
}

/// Updates the last indexed block with a new block number
///
/// This function updates both the block number and timestamp of the last indexed block.
/// The timestamp is automatically set to the current UTC time.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `model` - The existing last index block model to update
/// * `block_number` - The new block number to set
///
/// # Returns
///
/// * `Result<(), DbErr>` - Success if update is complete, error if database operation fails
pub async fn update_last_index_block(
    db: &DatabaseConnection,
    model: Model,
    block_number: u64,
) -> Result<(), DbErr> {
    let mut active_model: LastIndexBlockActiveModel = model.into();
    active_model.timestamp = Set(chrono::Utc::now().naive_utc());
    active_model.block_number = Set(block_number as i32);
    active_model.save(db).await?;

    Ok(())
}
