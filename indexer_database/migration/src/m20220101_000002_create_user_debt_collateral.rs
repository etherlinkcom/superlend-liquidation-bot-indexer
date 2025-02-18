use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserDebtCollateral::Table)
                    .if_not_exists()
                    .col(pk_auto(UserDebtCollateral::Id))
                    .col(string(UserDebtCollateral::UserAddress))
                    .col(string(UserDebtCollateral::ReserveAddress))
                    .col(float(UserDebtCollateral::Amount))
                    .col(boolean(UserDebtCollateral::IsCollateral))
                    .col(timestamp(UserDebtCollateral::Timestamp))
                    .index(
                        Index::create()
                            .name("idx_user_debt_collateral_unique")
                            .unique()
                            .col(UserDebtCollateral::UserAddress)
                            .col(UserDebtCollateral::ReserveAddress)
                            .col(UserDebtCollateral::IsCollateral),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserDebtCollateral::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum UserDebtCollateral {
    Table,
    Id,
    UserAddress,
    ReserveAddress,
    Amount,
    IsCollateral,
    Timestamp,
}
