mod index_users;

use index_users::IndexerUsers;
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    setup_logging();
    info!("Starting health factor service...");

    let indexer_users = IndexerUsers::default();
    indexer_users.run().await
}

fn setup_logging() {
    tracing_subscriber::fmt()
        .with_file(false)
        .with_span_events(FmtSpan::FULL)
        .with_target(false)
        .init();
}
