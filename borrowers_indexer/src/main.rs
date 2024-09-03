use std::sync::Arc;

use base_rpc_client::BaseRpcClient;
use database_manager::{
    bootstrap::DatabaseBootstrap, handler::last_index_block_handler::LastIndexBlockHandler,
    DatabaseManager,
};
use indexer_borrowers::{IndexerBorrowers, IndexerConfig};
use tracing_subscriber::fmt::format::FmtSpan;

mod config;
mod constant;
mod indexer_borrowers;

use config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_logging();
    let mut config = Config::load()?;

    let db = setup_database(&mut config).await?;
    let rpc_client = Arc::new(BaseRpcClient::new(&config.rpc_url, config.max_retries));

    let indexer = create_indexer(rpc_client, Arc::new(db), &config).await;

    indexer.run().await
}

fn setup_logging() {
    tracing_subscriber::fmt()
        .with_file(false)
        .with_span_events(FmtSpan::FULL)
        .with_target(false)
        .init();
}

async fn setup_database(
    config: &mut Config,
) -> Result<DatabaseManager, Box<dyn std::error::Error>> {
    let db = DatabaseManager::new().await;
    db.bootstrap().await?;

    let last_block = db.get_last_index_block().await?;
    config.start_block = if last_block != 0 {
        last_block
    } else {
        config.start_block
    };
    tracing::info!("Starting indexer from block {}", config.start_block);

    Ok(db)
}

async fn create_indexer(
    rpc_client: Arc<BaseRpcClient>,
    db: Arc<DatabaseManager>,
    config: &Config,
) -> IndexerBorrowers {
    IndexerBorrowers::new(
        rpc_client,
        db,
        IndexerConfig {
            pool_address: config.pool_address.clone(),
            start_block: config.start_block,
            max_blocks_per_request: config.max_blocks_per_request,
            max_parallel_requests: config.max_parallel_requests,
            delay_between_requests: config.delay_between_requests,
            wait_block_diff: config.wait_block_diff,
            cap_max_health_factor: config.cap_max_health_factor,
        },
    )
    .await
}
