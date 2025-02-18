use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LastIndexBlock::Table)
                    .if_not_exists()
                    .col(pk_auto(LastIndexBlock::Id))
                    .col(integer(LastIndexBlock::BlockNumber))
                    .col(timestamp(LastIndexBlock::Timestamp))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LastIndexBlock::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum LastIndexBlock {
    Table,
    Id,
    BlockNumber,
    Timestamp,
}
