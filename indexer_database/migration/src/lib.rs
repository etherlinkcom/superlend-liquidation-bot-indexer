pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_user_tables;
mod m20220101_000002_create_user_debt_collateral;
mod m20220101_000003_create_last_block_indexed;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_user_tables::Migration),
            Box::new(m20220101_000002_create_user_debt_collateral::Migration),
            Box::new(m20220101_000003_create_last_block_indexed::Migration),
        ]
    }
}
