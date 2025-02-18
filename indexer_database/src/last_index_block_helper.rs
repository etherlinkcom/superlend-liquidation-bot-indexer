use anyhow::Result;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, Set};
use tracing::info;

use crate::entities::last_index_block::{ActiveModel as LastIndexBlockActiveModel, Model};
use crate::entities::prelude::LastIndexBlock;

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

pub async fn get_last_index_block(db: &DatabaseConnection) -> Result<Model> {
    let last_index_block = LastIndexBlock::find().one(db).await?;
    Ok(last_index_block.ok_or(anyhow::anyhow!("Last index block not found"))?)
}

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
