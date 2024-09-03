use crate::{handler::{last_index_block_handler::LastIndexBlockHandler, user_table_handler::UserTableHandler}, DatabaseManager};

pub trait DatabaseBootstrap {
    fn bootstrap(
        &self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;
}

impl DatabaseBootstrap for DatabaseManager {
    async fn bootstrap(&self) -> Result<(), Box<dyn std::error::Error>> {
        // create user table
        self.create_user_table().await?;

        // create last index block table
        self.create_last_index_block_table().await?;

        Ok(())
    }
}
