use crate::DatabaseManager;

pub trait LastIndexBlockHandler {
    fn create_last_index_block_table(
        &self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;

    fn get_last_index_block(
        &self,
    ) -> impl std::future::Future<Output = Result<u64, Box<dyn std::error::Error>>> + Send;

    fn update_last_index_block(
        &self,
        block_number: u64,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;
}

impl LastIndexBlockHandler for DatabaseManager {
    async fn create_last_index_block_table(&self) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.get_connection().await?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS last_index_block (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                block_number INTEGER DEFAULT 0,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            (),
        )
        .await?;

        Ok(())
    }

    async fn get_last_index_block(&self) -> Result<u64, Box<dyn std::error::Error>> {
        let conn = self.get_connection().await?;

        let row: u64 = conn
            .query(
                "SELECT block_number FROM last_index_block ORDER BY id DESC LIMIT 1",
                (),
            )
            .await?
            .next()
            .await?
            .map(|row| row.get::<i64>(0).unwrap_or(0) as u64)
            .unwrap_or(0);

        Ok(row)
    }

    async fn update_last_index_block(
        &self,
        block_number: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.get_connection().await?;

        conn.execute(
            "INSERT OR REPLACE INTO last_index_block (id, block_number) VALUES (1, ?)",
            [block_number as i64],
        )
        .await?;

        Ok(())
    }
}
