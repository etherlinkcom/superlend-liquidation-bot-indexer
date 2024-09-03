use std::{collections::HashMap, sync::Arc};

use base_rpc_client::BaseRpcClient;
use database_manager::{
    handler::user_table_handler::UserTableHandler,
    health_factor_utils::{HealthFactorRange, HEALTH_FACTORS_RANGES},
    DatabaseManager,
};
use tokio::{runtime::Handle, task::JoinHandle};
use tracing::info;

use chrono::{DateTime, Utc};

mod fetch_debt_coll_assets;
mod user_helper;

#[derive(Debug, Clone)]
pub struct IndexerUsersConfig {
    pub pool_address: String,
    pub health_factor_variants: Vec<HealthFactorRange>,
    pub max_users_chunk_size: u64,
    pub cap_max_health_factor: f32,
}
impl Default for IndexerUsersConfig {
    fn default() -> Self {
        Self {
            pool_address: dotenv::var("POOL_ADDRESS").unwrap(),
            health_factor_variants: HEALTH_FACTORS_RANGES.clone(),
            max_users_chunk_size: dotenv::var("MAX_USERS_CHUNK_SIZE")
                .unwrap()
                .parse()
                .unwrap(),
            cap_max_health_factor: dotenv::var("CAP_MAX_HEALTH_FACTOR")
                .unwrap()
                .parse()
                .unwrap(),
        }
    }
}

pub struct IndexerUsers {
    db: Arc<DatabaseManager>,
    provider: Arc<BaseRpcClient>,
    config: Arc<IndexerUsersConfig>,
}

impl Default for IndexerUsers {
    fn default() -> Self {
        let url = dotenv::var("RPC_URL").unwrap();
        let client = Arc::new(BaseRpcClient::new(url.as_str(), 5));

        let db = tokio::task::block_in_place(|| {
            Handle::current().block_on(async { DatabaseManager::new().await })
        });

        Self {
            db: Arc::new(db),
            provider: client,
            config: Arc::new(Default::default()),
        }
    }
}

pub struct VariantState {
    pub last_checked_block: u64,
    pub last_checked_time: DateTime<Utc>,
    // wait time in seconds
    pub wait_time: u64,
}

impl IndexerUsers {
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        // info!("Variants: {:?}", self.config.health_factor_variants);

        let mut variants_states = self.get_variant_states_map();

        loop {
            let block_number = self.provider.get_block_number().await?;
            for (table_name, variant_state) in variants_states.iter_mut() {
                // check if the elapsed time is greater than the wait time in last checked time
                if variant_state.last_checked_time.timestamp() + variant_state.wait_time as i64
                    > Utc::now().timestamp()
                {
                    continue;
                }

                // info!("Checking variant: {}", table_name);

                let users: Vec<(String, f32)> = self.db.get_users_in_table(table_name).await?;
                // info!("Users in table {}: {:?}", table_name, users);

                for chunk in users.chunks(self.config.max_users_chunk_size as usize) {
                    let users_chunk = chunk.to_vec();

                    let mut tasks: Vec<JoinHandle<Result<f32, String>>> = Vec::new();
                    for (user_address, _) in users_chunk.clone() {
                        let user_address = user_address.clone();
                        let provider = self.provider.clone();
                        let config = self.config.clone();
                        tasks.push(tokio::spawn(async move {
                            user_helper::fetch_user_health_factor(
                                user_address.as_str(),
                                provider,
                                config,
                            )
                            .await
                        }));
                    }

                    let results = futures::future::join_all(tasks).await;

                    for ((user_address, _), result) in users_chunk.iter().zip(results) {
                        let health_factor = match result {
                            Ok(data) => match data {
                                Ok(health_factor) => health_factor,
                                Err(e) => {
                                    tracing::error!("Error fetching health factor: {}", e);
                                    continue;
                                }
                            },
                            Err(e) => {
                                tracing::error!("Error fetching health factor: {}", e);
                                continue;
                            }
                        };

                        let (is_moved, moved_table_name) = self
                            .db
                            .update_user_health_factor(
                                user_address.as_str(),
                                health_factor,
                                block_number,
                                table_name,
                            )
                            .await?;

                        if is_moved {
                            info!(
                                "User {} moved to table from {} to {} with health factor {}",
                                user_address, table_name, moved_table_name, health_factor
                            );
                        } else {
                            info!(
                                "User {} health factor updated to {}",
                                user_address, health_factor
                            );
                        }
                    }
                }

                variant_state.last_checked_block = self.provider.get_block_number().await?;
                variant_state.last_checked_time = Utc::now();
            }
        }
    }

    fn get_variant_states_map(&self) -> HashMap<String, VariantState> {
        let mut variants_states: HashMap<String, VariantState> =
            HashMap::with_capacity(self.config.health_factor_variants.len());
        variants_states.extend(self.config.health_factor_variants.iter().map(|variant| {
            (
                variant.name.clone(),
                VariantState {
                    wait_time: variant.wait_time,
                    last_checked_block: 0,
                    last_checked_time: DateTime::<Utc>::MIN_UTC,
                },
            )
        }));
        variants_states
    }
}
