pub mod entities;
pub mod last_index_block_helper;
pub mod users_tables_helper;
pub mod user_debt_collateral_helper;
use std::time::Duration;

use anyhow::Result;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, DatabaseConnection};

/// A utility struct providing static methods for database management and migrations.
/// This struct offers functionality to initialize the database and manage connections
/// without maintaining any internal state.
pub struct IndexerDatabase;

impl IndexerDatabase {
    /// Initializes the database by establishing a connection and running all pending migrations.
    ///
    /// This method:
    /// 1. Establishes a new database connection
    /// 2. Runs all pending migrations on the database
    ///
    /// # Returns
    /// * `Result<(), DbErr>` - Returns Ok(()) if initialization and migrations are successful,
    ///                         or a DbErr if either the connection or migrations fail.
    ///
    /// # Example
    /// ```
    /// IndexerDatabase::init().await?;
    /// ```
    pub async fn init() -> Result<()> {
        let connection = Self::get_postgres_connection().await?;
        Migrator::up(&connection, None).await?;
        Ok(())
    }

    /// Resets the database by dropping all tables and running all migrations again.
    ///
    /// This method:
    /// 1. Establishes a new database connection
    /// 2. Drops all tables on the database
    /// 3. Runs all migrations on the database
    ///
    /// # Returns
    /// * `Result<(), DbErr>` - Returns Ok(()) if reset is successful, or a DbErr if the connection or migrations fail.
    ///
    /// # Example
    pub async fn reset() -> Result<()> {
        let connection = Self::get_postgres_connection().await?;
        Migrator::down(&connection, None).await?;
        Migrator::up(&connection, None).await?;
        Ok(())
    }

    /// Establishes a connection to the PostgreSQL database using environment variables.
    ///
    /// This function:
    /// 1. Reads the DATABASE_URL from environment variables
    /// 2. Configures connection options including:
    ///    - SQL logging (disabled)
    ///    - Connection pool limits (min: 2, max: 6)
    ///    - Connection lifetime (2 minutes)
    /// 3. Establishes and returns the database connection
    ///
    /// # Returns
    /// * `Result<DatabaseConnection, DbErr>` - Returns a database connection wrapped in Ok if successful,
    ///                                        or a DbErr if the connection fails.
    ///
    /// # Environment Variables
    /// * `DATABASE_URL` - Required PostgreSQL connection string
    ///
    /// # Panics
    /// * Panics if the DATABASE_URL environment variable is not set
    pub async fn get_postgres_connection() -> Result<DatabaseConnection> {
        let postgres_url = std::env::var("DATABASE_URL").unwrap();

        let mut options = ConnectOptions::new(postgres_url);

        options.sqlx_logging(false);
        options.min_connections(2);
        options.max_connections(6);

        // Set the maximum lifetime of a connection to 2 minutes
        options.max_lifetime(Duration::from_secs(60 * 2));

        let db = sea_orm::Database::connect(options).await?;

        Ok(db)
    }
}
