pub mod bootstrap;
pub mod handler;
pub mod health_factor_utils;
use std::env;

use libsql::{Builder, Connection};

pub struct DatabaseManager {
    db: libsql::Database,
}

impl DatabaseManager {
    pub async fn new() -> Self {
        let url = env::var("LIBSQL_URL").expect("LIBSQL_URL must be set");
        let token = env::var("LIBSQL_AUTH_TOKEN").unwrap_or_default();
        let db = Builder::new_remote(url, token).build().await.unwrap();
        Self { db }
    }

    pub async fn get_connection(&self) -> Result<Connection, libsql::Error> {
        self.db.connect()
    }
}
