use anyhow::Result;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entities::{at_risk_accounts, healthy_accounts, liquidatable_accounts};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum UserCurrentLocation {
    Liquidatable,
    AtRisk,
    Healthy,
    NotFound,
}

pub struct UserDetails {
    pub id: i32,
    pub user_address: String,
    pub last_updated_block_number: i32,
    pub health_factor: f32,
    pub total_collateral_value_in_usd: f32,
    pub total_debt_value_in_usd: f32,
    pub leading_collateral_reserve: String,
    pub leading_debt_reserve: String,
    pub leading_collateral_reserve_value: f32,
    pub leading_debt_reserve_value: f32,
    pub timestamp: DateTime<Utc>,
    pub current_location: UserCurrentLocation,
}

pub async fn get_user(db: &DatabaseConnection, user_address: &str) -> Result<Option<UserDetails>> {
    // First check liquidatable accounts
    if let Some(user) = liquidatable_accounts::Entity::find()
        .filter(liquidatable_accounts::Column::UserAddress.eq(user_address))
        .one(db)
        .await?
    {
        return Ok(Some(UserDetails {
            id: user.id,
            user_address: user.user_address,
            last_updated_block_number: user.last_updated_block_number,
            health_factor: user.health_factor,
            total_collateral_value_in_usd: user.total_collateral_value_in_usd,
            total_debt_value_in_usd: user.total_debt_value_in_usd,
            leading_collateral_reserve: user.leading_collateral_reserve,
            leading_debt_reserve: user.leading_debt_reserve,
            leading_collateral_reserve_value: user.leading_collateral_reserve_value,
            leading_debt_reserve_value: user.leading_debt_reserve_value,
            timestamp: DateTime::from_naive_utc_and_offset(user.timestamp, Utc),
            current_location: UserCurrentLocation::Liquidatable,
        }));
    }

    // Then check at risk accounts
    if let Some(user) = at_risk_accounts::Entity::find()
        .filter(at_risk_accounts::Column::UserAddress.eq(user_address))
        .one(db)
        .await?
    {
        return Ok(Some(UserDetails {
            id: user.id,
            user_address: user.user_address,
            last_updated_block_number: user.last_updated_block_number,
            health_factor: user.health_factor,
            total_collateral_value_in_usd: user.total_collateral_value_in_usd,
            total_debt_value_in_usd: user.total_debt_value_in_usd,
            leading_collateral_reserve: user.leading_collateral_reserve,
            leading_debt_reserve: user.leading_debt_reserve,
            leading_collateral_reserve_value: user.leading_collateral_reserve_value,
            leading_debt_reserve_value: user.leading_debt_reserve_value,
            timestamp: DateTime::from_naive_utc_and_offset(user.timestamp, Utc),
            current_location: UserCurrentLocation::AtRisk,
        }));
    }

    // Finally check healthy accounts
    if let Some(user) = healthy_accounts::Entity::find()
        .filter(healthy_accounts::Column::UserAddress.eq(user_address))
        .one(db)
        .await?
    {
        return Ok(Some(UserDetails {
            id: user.id,
            user_address: user.user_address,
            last_updated_block_number: user.last_updated_block_number,
            health_factor: user.health_factor,
            total_collateral_value_in_usd: user.total_collateral_value_in_usd,
            total_debt_value_in_usd: user.total_debt_value_in_usd,
            leading_collateral_reserve: user.leading_collateral_reserve,
            leading_debt_reserve: user.leading_debt_reserve,
            leading_collateral_reserve_value: user.leading_collateral_reserve_value,
            leading_debt_reserve_value: user.leading_debt_reserve_value,
            timestamp: DateTime::from_naive_utc_and_offset(user.timestamp, Utc),
            current_location: UserCurrentLocation::Healthy,
        }));
    }

    // If user not found in any table
    return Ok(None);
}

pub async fn delete_user(
    db: &DatabaseConnection,
    id: i32,
    location: UserCurrentLocation,
) -> Result<()> {
    match location {
        UserCurrentLocation::Liquidatable => {
            liquidatable_accounts::Entity::delete_by_id(id)
                .exec(db)
                .await?;
        }
        UserCurrentLocation::AtRisk => {
            at_risk_accounts::Entity::delete_by_id(id).exec(db).await?;
        }
        UserCurrentLocation::Healthy => {
            healthy_accounts::Entity::delete_by_id(id).exec(db).await?;
        }
        UserCurrentLocation::NotFound => {
            return Err(anyhow::anyhow!("User not found"));
        }
    }
    Ok(())
}

pub async fn add_user(
    db: &DatabaseConnection,
    user: UserDetails,
    new_location: UserCurrentLocation,
) -> Result<()> {
    match new_location {
        UserCurrentLocation::Liquidatable => {
            let active_model = user_details_to_liquidatable_account(&user);
            active_model.insert(db).await?;
        }
        UserCurrentLocation::AtRisk => {
            let active_model = user_details_to_at_risk_account(&user);
            active_model.insert(db).await?;
        }
        UserCurrentLocation::Healthy => {
            let active_model = user_details_to_healthy_account(&user);
            active_model.insert(db).await?;
        }
        UserCurrentLocation::NotFound => {
            return Err(anyhow::anyhow!("User not found"));
        }
    }
    Ok(())
}

fn user_details_to_liquidatable_account(user: &UserDetails) -> liquidatable_accounts::ActiveModel {
    liquidatable_accounts::ActiveModel {
        user_address: Set(user.user_address.clone()),
        last_updated_block_number: Set(user.last_updated_block_number),
        health_factor: Set(user.health_factor),
        total_collateral_value_in_usd: Set(user.total_collateral_value_in_usd),
        total_debt_value_in_usd: Set(user.total_debt_value_in_usd),
        leading_collateral_reserve: Set(user.leading_collateral_reserve.clone()),
        leading_debt_reserve: Set(user.leading_debt_reserve.clone()),
        leading_collateral_reserve_value: Set(user.leading_collateral_reserve_value),
        leading_debt_reserve_value: Set(user.leading_debt_reserve_value),
        timestamp: Set(user.timestamp.naive_utc()),
        ..Default::default()
    }
}

fn user_details_to_at_risk_account(user: &UserDetails) -> at_risk_accounts::ActiveModel {
    at_risk_accounts::ActiveModel {
        user_address: Set(user.user_address.clone()),
        last_updated_block_number: Set(user.last_updated_block_number),
        health_factor: Set(user.health_factor),
        total_collateral_value_in_usd: Set(user.total_collateral_value_in_usd),
        total_debt_value_in_usd: Set(user.total_debt_value_in_usd),
        leading_collateral_reserve: Set(user.leading_collateral_reserve.clone()),
        leading_debt_reserve: Set(user.leading_debt_reserve.clone()),
        leading_collateral_reserve_value: Set(user.leading_collateral_reserve_value),
        leading_debt_reserve_value: Set(user.leading_debt_reserve_value),
        timestamp: Set(user.timestamp.naive_utc()),
        ..Default::default()
    }
}

fn user_details_to_healthy_account(user: &UserDetails) -> healthy_accounts::ActiveModel {
    healthy_accounts::ActiveModel {
        user_address: Set(user.user_address.clone()),
        last_updated_block_number: Set(user.last_updated_block_number),
        health_factor: Set(user.health_factor),
        total_collateral_value_in_usd: Set(user.total_collateral_value_in_usd),
        total_debt_value_in_usd: Set(user.total_debt_value_in_usd),
        leading_collateral_reserve: Set(user.leading_collateral_reserve.clone()),
        leading_debt_reserve: Set(user.leading_debt_reserve.clone()),
        leading_collateral_reserve_value: Set(user.leading_collateral_reserve_value),
        leading_debt_reserve_value: Set(user.leading_debt_reserve_value),
        timestamp: Set(user.timestamp.naive_utc()),
        ..Default::default()
    }
}

pub async fn update_user(
    db: &DatabaseConnection,
    id: i32,
    user: UserDetails,
    new_location: UserCurrentLocation,
) -> Result<()> {
    match new_location {
        UserCurrentLocation::Liquidatable => {
            let mut active_model = user_details_to_liquidatable_account(&user);
            active_model.id = Set(id);
            active_model.update(db).await?;
        }
        UserCurrentLocation::AtRisk => {
            let mut active_model = user_details_to_at_risk_account(&user);
            active_model.id = Set(id);
            active_model.update(db).await?;
        }
        UserCurrentLocation::Healthy => {
            let mut active_model = user_details_to_healthy_account(&user);
            active_model.id = Set(id);
            active_model.update(db).await?;
        }
        UserCurrentLocation::NotFound => {
            return Err(anyhow::anyhow!("User not found"));
        }
    }
    Ok(())
}

/// Get all liquidatable users
///
/// # Arguments
///
/// * `db` - Database connection
///
/// # Returns
/// * `Vec<String>` - List of user addresses
///
pub async fn get_all_liquidatable_users(db: &DatabaseConnection) -> Result<Vec<String>> {
    let users = liquidatable_accounts::Entity::find().all(db).await?;
    Ok(users.into_iter().map(|user| user.user_address).collect())
}

/// Get all at risk users
///
/// # Arguments
///
/// * `db` - Database connection
///
/// # Returns
/// * `Vec<String>` - List of user addresses
pub async fn get_all_at_risk_users(db: &DatabaseConnection) -> Result<Vec<String>> {
    let users = at_risk_accounts::Entity::find().all(db).await?;
    Ok(users.into_iter().map(|user| user.user_address).collect())
}

/// Get all healthy users
///
/// # Arguments
///
/// * `db` - Database connection
///
/// # Returns
/// * `Vec<String>` - List of user addresses
pub async fn get_all_healthy_users(db: &DatabaseConnection) -> Result<Vec<String>> {
    let users = healthy_accounts::Entity::find().all(db).await?;
    Ok(users.into_iter().map(|user| user.user_address).collect())
}
