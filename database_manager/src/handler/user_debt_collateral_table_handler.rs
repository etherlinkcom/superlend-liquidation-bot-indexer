use crate::DatabaseManager;

pub trait UserDebtCollateralTableHandler {
    fn create_user_debt_collateral_table(
        &self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;

    fn insert_or_update_user_debt_collateral(
        &self,
        user_address: &str,
        address_amount: Vec<(String, f32)>,
        is_collateral: bool,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;
}

impl UserDebtCollateralTableHandler for DatabaseManager {
    // Fix: Modified to create a composite primary key for user_address and reserve_address
    async fn create_user_debt_collateral_table(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.get_connection().await?;

        let query = "CREATE TABLE IF NOT EXISTS user_debt_collateral (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_address TEXT NOT NULL,
            reserve_address TEXT NOT NULL,
            amount REAL DEFAULT 0.0,
            is_collateral BOOLEAN DEFAULT TRUE,
            UNIQUE(user_address, reserve_address)  
        )";

        conn.execute(query, ()).await?;

        Ok(())
    }

    async fn insert_or_update_user_debt_collateral(
        &self,
        user_address: &str,
        address_amount: Vec<(String, f32)>,
        is_collateral: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.get_connection().await?;

        // The query remains the same, since the composite primary key resolves the conflict issue
        let query = "INSERT INTO user_debt_collateral (user_address, reserve_address, amount, is_collateral) VALUES (?, ?, ?, ?) ON CONFLICT(user_address, reserve_address) DO UPDATE SET amount = excluded.amount";

        for (reserve_address, amount) in address_amount {
            if amount == 0.0 {
                continue;
            }

            conn.execute(
                query,
                (user_address, reserve_address, amount, is_collateral),
            )
            .await?;
        }

        Ok(())
    }
}
