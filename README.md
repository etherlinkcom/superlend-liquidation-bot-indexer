# Liquidation Bot Indexer

## Run Locally
1. Setup the environment variables in the `.env` file
```bash
cp .env.example .env
```
2. Run the indexer
```bash
cargo run --bin indexer 
```

## Environment Variables (in .env file)

The following environment variables need to be configured in the `.env` file:

### Logging Configuration
- `LOG_INSIDE_FILE`: Boolean flag to enable/disable file logging (default: true)

### Database Configuration
- `DATABASE_URL`: PostgreSQL connection string (format: postgres://user:password@host:port/dbname)

### Blockchain Configuration
- `RPC_URL`: RPC endpoint URL
- `POOL_ADDRESS`: Aave lending pool contract address
- `POOL_DATA_PROVIDER`: Aave pool data provider contract address
- `PRICE_ORACLE`: Aave price oracle contract address
- `START_BLOCK`: Starting block number for indexing
- `LOG_PER_REQUEST`: Number of blocks to fetch logs per RPC request (1-MAX_ALLOWED)
- `MAX_BLOCK_OUT_OF_SYNC`: Maximum block difference before triggering reindex

### Health Factor Configuration
- `MAX_CAP_ON_HEALTH_FACTOR`: Maximum cap value for health factor (default: 1000)
- `AT_RISK_HEALTH_FACTOR`: Threshold for at-risk users (1.0 ≤ health factor ≤ value)

### Update Frequency Configuration
- `LIQUIDATABLE_USERS_UPDATE_FREQUENCY`: Update frequency for liquidatable users (in seconds, default: 30)
- `AT_RISK_USERS_UPDATE_FREQUENCY`: Update frequency for at-risk users (in seconds, default: 120)
- `HEALTHY_USERS_UPDATE_FREQUENCY`: Update frequency for healthy users (in seconds, default: 3600)

## Main loop logic (src/main.rs)

The main loop consists of two primary services running concurrently:

1. **Users Indexer Service**
   - Updates only the users from borrow events from the Aave pool contract
   - Continuously monitors blockchain events
   - Indexes new user positions
   - Updates user states based on health factor
   - Maintains synchronization with the latest block

2. **Users Updater Service**
   - Periodically updates user positions based on configured frequencies
   - Updates liquidatable users every 30 seconds
   - Updates at-risk users every 2 minutes
   - Updates healthy users every 1 hour
   - Recalculates health factors and updates user categories

The services are managed using Tokio's async runtime with error handling and graceful shutdown:
```rust
tokio::select! {
    result = async {
        match try_join!(users_indexer, users_updater_service) {
            Ok((users_indexer_result, users_updater_service_result)) => {
                // Handle service results and errors
            }
            Err(e) => {
                // Handle panic scenarios
            }
        }
    }
}
```

## Database Schema (indexer_database)

```mermaid
erDiagram
    LiquidatableAccounts {
        id integer PK
        user_address varchar(255) UK
        last_updated_block_number integer
        health_factor decimal
        total_collateral_value_in_usd decimal
        total_debt_value_in_usd decimal
        leading_collateral_reserve varchar(255)
        leading_debt_reserve varchar(255)
        leading_collateral_reserve_value decimal
        leading_debt_reserve_value decimal
        timestamp timestamptz
    }

    AtRiskAccounts {
        id integer PK
        user_address varchar(255) UK
        last_updated_block_number integer
        health_factor decimal
        total_collateral_value_in_usd decimal
        total_debt_value_in_usd decimal
        leading_collateral_reserve varchar(255)
        leading_debt_reserve varchar(255)
        leading_collateral_reserve_value decimal
        leading_debt_reserve_value decimal
        timestamp timestamptz
    }

    HealthyAccounts {
        id integer PK
        user_address varchar(255) UK
        last_updated_block_number integer
        health_factor decimal
        total_collateral_value_in_usd decimal
        total_debt_value_in_usd decimal
        leading_collateral_reserve varchar(255)
        leading_debt_reserve varchar(255)
        leading_collateral_reserve_value decimal
        leading_debt_reserve_value decimal
        timestamp timestamptz
    }

    UserDebtCollateral {
        id integer PK
        user_address varchar(255)
        reserve_address varchar(255)
        amount decimal
        is_collateral boolean
        timestamp timestamptz
    }

    LastIndexBlock {
        id integer PK
        block_number integer
        timestamp timestamptz
    }
```

The database schema consists of five main tables:

1. **LiquidatableAccounts**: Stores users with health factor < 1.0
   - Unique constraint on user_address
   - Tracks health factor, collateral, and debt values
   - Monitors leading positions and their values

2. **AtRiskAccounts**: Stores users with 1.0 ≤ health factor ≤ AT_RISK_HEALTH_FACTOR
   - Identical structure to LiquidatableAccounts
   - Used for monitoring potentially risky positions

3. **HealthyAccounts**: Stores users with health factor > AT_RISK_HEALTH_FACTOR
   - Identical structure to other account tables
   - Updated less frequently due to lower risk

4. **UserDebtCollateral**: Tracks all user positions
   - Records reserve address and position amount
   - Boolean flag to distinguish between collateral and debt
   - Composite unique index on (user_address, reserve_address, is_collateral) to ensure each user's position is unique

5. **LastIndexBlock**: Tracks indexing progress
   - Records the last processed block number
   - Used for maintaining sync

