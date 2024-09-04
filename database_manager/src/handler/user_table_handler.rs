use libsql::params;

use crate::{
    health_factor_utils::{self, HealthFactorRange},
    DatabaseManager,
};

pub trait UserTableHandler {
    fn create_user_table(
        &self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;

    /// @dev this function is used to insert a new user into the database </br>
    /// @param user_address the address of the user </br>
    /// @param block_number the block number at which the user is inserted </br>
    /// @param health_factor the health factor of the user </br>
    /// @param leading_collateral_reserve the address of the leading collateral reserve </br>
    /// @param leading_debt_reserve the address of the leading debt reserve </br>
    /// @param total_collateral_value_in_usd the total collateral value in usd </br>
    /// @param total_debt_value_in_usd the total debt value in usd </br>
    fn insert_user(
        &self,
        user_address: &str,
        block_number: u64,
        health_factor: f32,
        leading_collateral_reserve: &str,
        leading_debt_reserve: &str,
        total_collateral_value_in_usd: f32,
        total_debt_value_in_usd: f32,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;

    fn update_user_health_factor(
        &self,
        user_address: &str,
        health_factor: f32,
        block_number: u64,
        leading_collateral_reserve: &str,
        leading_debt_reserve: &str,
        total_collateral_value_in_usd: f32,
        total_debt_value_in_usd: f32,
        past_table_name: &str,
    ) -> impl std::future::Future<Output = Result<(bool, String), Box<dyn std::error::Error>>> + Send;

    fn get_last_block(
        &self,
    ) -> impl std::future::Future<Output = Result<u64, Box<dyn std::error::Error>>> + Send;

    fn check_if_user_exists(
        &self,
        user_address: &str,
    ) -> impl std::future::Future<
        Output = Result<
            (bool, Option<HealthFactorRange>, Option<String>),
            Box<dyn std::error::Error>,
        >,
    > + Send;

    fn get_users_in_table(
        &self,
        table_name: &str,
    ) -> impl std::future::Future<Output = Result<Vec<(String, f32)>, Box<dyn std::error::Error>>> + Send;
}

impl UserTableHandler for DatabaseManager {
    async fn create_user_table(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.get_connection().await?;

        // let table_variants = user_manager::get_all_variants();
        let table_variants = health_factor_utils::get_all_variants();

        for variant in table_variants {
            conn.execute(
                format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    user_address TEXT NOT NULL UNIQUE,
                    block_number INTEGER DEFAULT 0,
                    health_factor REAL DEFAULT 0.0,
                    totalCollateralValueInUsd REAL DEFAULT 0.0,
                    totalDebtValueInUsd REAL DEFAULT 0.0,
                    leadingCollateralReserve TEXT DEFAULT '',
                    leadingDebtReserve TEXT DEFAULT '',
                    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
                );",
                    variant
                )
                .as_str(),
                (),
            )
            .await?;
        }
        Ok(())
    }

    async fn check_if_user_exists(
        &self,
        user_address: &str,
    ) -> Result<(bool, Option<HealthFactorRange>, Option<String>), Box<dyn std::error::Error>> {
        let conn = self.get_connection().await?;

        let table_variants = health_factor_utils::get_all_health_factor_ranges();

        for variant in table_variants {
            let table_name = variant.name.clone();
            let query = format!("SELECT * FROM {} WHERE user_address = ?", table_name);
            let mut result = conn.query(query.as_str(), params![user_address]).await?;
            if let Some(_) = result.next().await? {
                return Ok((true, Some(variant), Some(table_name)));
            }
        }

        Ok((false, None, None))
    }

    async fn insert_user(
        &self,
        user_address: &str,
        block_number: u64,
        health_factor: f32,
        leading_collateral_reserve: &str,
        leading_debt_reserve: &str,
        total_collateral_value_in_usd: f32,
        total_debt_value_in_usd: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let table = health_factor_utils::find_health_factor_variant(health_factor);

        let table_name = match table {
            Some(t) => t.name.clone(),
            None => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Table not found",
                )))
            }
        };

        if let (true, _, _) = self.check_if_user_exists(user_address).await? {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "User already exists",
            )));
        }

        let conn = self.get_connection().await?;

        conn.execute(
            format!(
                "INSERT OR IGNORE INTO {} (user_address, block_number, health_factor, totalCollateralValueInUsd, totalDebtValueInUsd, leadingCollateralReserve, leadingDebtReserve) VALUES (?, ?, ?, ?, ?, ?, ?)",
                table_name.as_str()
            )
            .as_str(),
            (user_address, block_number as i64, health_factor, total_collateral_value_in_usd, total_debt_value_in_usd , leading_collateral_reserve, leading_debt_reserve),
        )
        .await?;

        Ok(())
    }

    /// returns true if user is moved to new table else false and updates the health factor in the database
    async fn update_user_health_factor(
        &self,
        user_address: &str,
        health_factor: f32,
        block_number: u64,
        leading_collateral_reserve: &str,
        leading_debt_reserve: &str,
        total_collateral_value_in_usd: f32,
        total_debt_value_in_usd: f32,
        past_table_name: &str,
    ) -> Result<(bool, String), Box<dyn std::error::Error>> {
        let table = health_factor_utils::find_health_factor_variant(health_factor);

        let table_name = match table {
            Some(t) => t.name.clone(),
            None => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Table not found",
                )));
            }
        };

        // if past table name and table name are different then we need to move the user to new table and delete from old table
        let conn = self.get_connection().await?;
        if past_table_name != table_name {
            conn.execute(
                format!("INSERT OR IGNORE INTO {} (user_address, block_number, health_factor, totalCollateralValueInUsd, totalDebtValueInUsd, leadingCollateralReserve, leadingDebtReserve) VALUES (?, ?, ?, ?, ?, ?, ?)", table_name.as_str()).as_str(),
                params![user_address, block_number as i64, health_factor, total_collateral_value_in_usd, total_debt_value_in_usd, leading_collateral_reserve, leading_debt_reserve],
            )
            .await?;

            conn.execute(
                format!("DELETE FROM {} WHERE user_address = ?", past_table_name).as_str(),
                params![user_address],
            )
            .await?;
            Ok((true, table_name))
        } else {
            conn.execute(
                format!(
                    "UPDATE {} SET health_factor = ?, totalCollateralValueInUsd = ?, totalDebtValueInUsd = ?, leadingCollateralReserve = ?, leadingDebtReserve = ? WHERE user_address = ?",
                    table_name.as_str()
                )
                .as_str(),
                (health_factor, total_collateral_value_in_usd, total_debt_value_in_usd, leading_collateral_reserve, leading_debt_reserve, user_address),
            )
            .await?;

            Ok((false, table_name))
        }
    }

    async fn get_last_block(&self) -> Result<u64, Box<dyn std::error::Error>> {
        let conn = self.get_connection().await?;

        let row: Option<i64> = conn
            .query("SELECT MAX(block_number) FROM users", ())
            .await?
            .next()
            .await?
            .unwrap()
            .get(0)
            .unwrap();
        Ok(row.unwrap_or(0) as u64)
    }

    async fn get_users_in_table(
        &self,
        table_name: &str,
    ) -> Result<Vec<(String, f32)>, Box<dyn std::error::Error>> {
        let mut users_rows = self
            .get_connection()
            .await?
            .query(
                format!("SELECT user_address, health_factor FROM {}", table_name).as_str(),
                (),
            )
            .await?;
        let mut users = Vec::new();

        while let Some(row) = users_rows.next().await? {
            users.push((row.get::<String>(0)?, row.get::<f64>(1)? as f32));
        }

        Ok(users)
    }
}
