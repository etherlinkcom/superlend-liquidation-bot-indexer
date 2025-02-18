use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the liquidatable_accounts table
        manager
            .create_table(
                Table::create()
                    .table(LiquidatableAccounts::Table)
                    .if_not_exists()
                    .col(pk_auto(LiquidatableAccounts::Id))
                    .col(string(LiquidatableAccounts::UserAddress))
                    .col(integer(LiquidatableAccounts::LastUpdatedBlockNumber))
                    .col(float(LiquidatableAccounts::HealthFactor))
                    .col(float(LiquidatableAccounts::TotalCollateralValueInUsd))
                    .col(float(LiquidatableAccounts::TotalDebtValueInUsd))
                    .col(string(LiquidatableAccounts::LeadingCollateralReserve))
                    .col(string(LiquidatableAccounts::LeadingDebtReserve))
                    .col(float(LiquidatableAccounts::LeadingCollateralReserveValue))
                    .col(float(LiquidatableAccounts::LeadingDebtReserveValue))
                    .col(timestamp(LiquidatableAccounts::Timestamp))
                    .index(
                        Index::create()
                            .name("idx_liquidatable_accounts_user_address")
                            .unique()
                            .col(LiquidatableAccounts::UserAddress),
                    )
                    .to_owned(),
            )
            .await?;

        // Create the at_risk_accounts table
        manager
            .create_table(
                Table::create()
                    .table(AtRiskAccounts::Table)
                    .if_not_exists()
                    .col(pk_auto(AtRiskAccounts::Id))
                    .col(string(AtRiskAccounts::UserAddress))
                    .col(integer(AtRiskAccounts::LastUpdatedBlockNumber))
                    .col(float(AtRiskAccounts::HealthFactor))
                    .col(float(AtRiskAccounts::TotalCollateralValueInUsd))
                    .col(float(AtRiskAccounts::TotalDebtValueInUsd))
                    .col(string(AtRiskAccounts::LeadingCollateralReserve))
                    .col(string(AtRiskAccounts::LeadingDebtReserve))
                    .col(float(AtRiskAccounts::LeadingCollateralReserveValue))
                    .col(float(AtRiskAccounts::LeadingDebtReserveValue))
                    .col(timestamp(AtRiskAccounts::Timestamp))
                    .index(
                        Index::create()
                            .name("idx_at_risk_accounts_user_address")
                            .unique()
                            .col(AtRiskAccounts::UserAddress),
                    )
                    .to_owned(),
            )
            .await?;

        // Create the healthy_accounts table
        manager
            .create_table(
                Table::create()
                    .table(HealthyAccounts::Table)
                    .if_not_exists()
                    .col(pk_auto(HealthyAccounts::Id))
                    .col(string(HealthyAccounts::UserAddress))
                    .col(integer(HealthyAccounts::LastUpdatedBlockNumber))
                    .col(float(HealthyAccounts::HealthFactor))
                    .col(float(HealthyAccounts::TotalCollateralValueInUsd))
                    .col(float(HealthyAccounts::TotalDebtValueInUsd))
                    .col(string(HealthyAccounts::LeadingCollateralReserve))
                    .col(string(HealthyAccounts::LeadingDebtReserve))
                    .col(float(HealthyAccounts::LeadingCollateralReserveValue))
                    .col(float(HealthyAccounts::LeadingDebtReserveValue))
                    .col(timestamp(HealthyAccounts::Timestamp))
                    .index(
                        Index::create()
                            .name("idx_healthy_accounts_user_address")
                            .unique()
                            .col(HealthyAccounts::UserAddress),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order
        manager
            .drop_table(Table::drop().table(HealthyAccounts::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(AtRiskAccounts::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(LiquidatableAccounts::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum LiquidatableAccounts {
    Table,
    Id,
    UserAddress,
    LastUpdatedBlockNumber,
    HealthFactor,
    TotalCollateralValueInUsd,
    TotalDebtValueInUsd,
    LeadingCollateralReserve,
    LeadingDebtReserve,
    LeadingCollateralReserveValue,
    LeadingDebtReserveValue,
    Timestamp,
}

#[derive(DeriveIden)]
enum AtRiskAccounts {
    Table,
    Id,
    UserAddress,
    LastUpdatedBlockNumber,
    HealthFactor,
    TotalCollateralValueInUsd,
    TotalDebtValueInUsd,
    LeadingCollateralReserve,
    LeadingDebtReserve,
    LeadingCollateralReserveValue,
    LeadingDebtReserveValue,
    Timestamp,
}

#[derive(DeriveIden)]
enum HealthyAccounts {
    Table,
    Id,
    UserAddress,
    LastUpdatedBlockNumber,
    HealthFactor,
    TotalCollateralValueInUsd,
    TotalDebtValueInUsd,
    LeadingCollateralReserve,
    LeadingDebtReserve,
    LeadingCollateralReserveValue,
    LeadingDebtReserveValue,
    Timestamp,
}
