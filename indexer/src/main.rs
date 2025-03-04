use std::sync::Arc;

use axum::{routing::get, Router};
use std::net::SocketAddr;
use anyhow::{Context, Result};
use futures::try_join;
use indexer::{
    config::LocalConfig, users_indexer::UsersIndexer, users_updater_service::UsersUpdaterService,
    utils,
};
use indexer_database::IndexerDatabase;
use tokio::task::JoinHandle;
use tracing::{error, info};


async fn start_health_check_server() -> Result<()> {
    let app = Router::new().route("/health", get(|| async { "OK" }));
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    info!("Starting health check server on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

/// Main entry point for the Liquidation Bot Indexer
///
/// This function performs the following steps:
/// 1. Initializes the pre-run environment
/// 2. Starts the users indexer service
/// 3. Starts the users updater service
/// 4. Handles if any of the services panics
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    init_pre_run().await?;

    info!("Starting the Liquidation Bot Indexer");

    let local_config = Arc::new(LocalConfig::load_from_env()?);

    let database_connection = Arc::new(IndexerDatabase::get_postgres_connection().await?);

    let users_indexer: JoinHandle<Result<()>> =
        UsersIndexer::start_users_indexer(&database_connection, &local_config).await?;

    let users_updater_service =
        UsersUpdaterService::start_users_updater_service(&database_connection, &local_config)
            .await?;

    let health_check_handle = tokio::spawn(start_health_check_server());

    tokio::select! {
        result = async {
            match try_join!(users_indexer, users_updater_service, health_check_handle) {
                Ok((users_indexer_result, users_updater_service_result, health_check_result)) => {
                    if let Err(e) = users_indexer_result {
                        let error_message = e.chain().into_iter().map(|e| e.to_string()).collect::<Vec<String>>().join(" -> ");
                        error!("Users indexer failed with error: {}", error_message);
                        return Err(anyhow::anyhow!("Users indexer failed: {}", error_message));
                    }

                    if let Err(e) = users_updater_service_result {
                        let error_message = e.chain().into_iter().map(|e| e.to_string()).collect::<Vec<String>>().join(" -> ");
                        error!("Users updater service failed with error: {}", error_message);
                        return Err(anyhow::anyhow!("Users updater service failed: {}", error_message));
                    }

                    if let Err(e) = health_check_result {
                        let error_message = e.chain().map(|e| e.to_string()).collect::<Vec<_>>().join(" -> ");
                        error!("Health check server failed with error: {}", error_message);
                    }

                    info!("All indexers stopped");
                    Ok(())
                }
                Err(e) => {
                    error!("Indexer task panicked: {}", e);
                    Err(anyhow::anyhow!("Indexer task panicked: {}", e))
                }
            }
        } => {
            result
        }
    }?;

    Ok(())
}

/// Initializes the pre-run environment
///
/// This function performs the following steps:
/// 1. Loads environment variables from the `.env` file
/// 2. Sets up the logger
/// 3. Reads command-line arguments
/// 4. Initializes the database
/// 5. Resets the database if the reset flag is provided
/// 6. Initializes the database if it doesn't exist
///
/// # Returns
/// * `Result<()>` - Success or error if any step fails
async fn init_pre_run() -> Result<()> {
    dotenvy::dotenv().context("Failed to load environment variables")?;
    utils::logger::setup_logger().context("Failed to setup logger")?;

    // read the first argument
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() > 1 && args[1] == "reset" {
        info!("Resetting the database");
        IndexerDatabase::reset().await?;
        info!("Database reset");
        return Ok(());
    }

    info!("Initializing the database");
    IndexerDatabase::init()
        .await
        .context("Failed to initialize the database")?;
    info!("Database initialized");

    Ok(())
}
