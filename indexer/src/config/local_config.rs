use anyhow::Result;

use super::env_helper::load_env_var;

#[derive(Debug, Clone)]
pub struct LocalConfig {
    pub rpc_url: String,
    pub start_block: u64,
    pub pool_address: String,
    pub pool_data_provider: String,
    pub price_oracle: String,
    pub log_per_request: u64,
    pub max_block_lag: u64,
    pub max_cap_on_health_factor: u64,
    pub at_risk_health_factor: f64,
    pub liquidatable_users_update_frequency: u64,
    pub at_risk_users_update_frequency: u64,
    pub healthy_users_update_frequency: u64,
}

impl LocalConfig {
    pub fn load_from_env() -> Result<Self> {
        Ok(Self {
            rpc_url: load_env_var("RPC_URL")?,
            start_block: load_env_var("START_BLOCK")?,
            pool_address: load_env_var("POOL_ADDRESS")?,
            pool_data_provider: load_env_var("POOL_DATA_PROVIDER")?,
            price_oracle: load_env_var("PRICE_ORACLE")?,
            log_per_request: load_env_var("LOG_PER_REQUEST")?,
            max_block_lag: load_env_var("MAX_BLOCK_LAG")?,
            max_cap_on_health_factor: load_env_var("MAX_CAP_ON_HEALTH_FACTOR")?,
            at_risk_health_factor: load_env_var("AT_RISK_HEALTH_FACTOR")?,
            liquidatable_users_update_frequency: load_env_var(
                "LIQUIDATABLE_USERS_UPDATE_FREQUENCY",
            )?,
            at_risk_users_update_frequency: load_env_var("AT_RISK_USERS_UPDATE_FREQUENCY")?,
            healthy_users_update_frequency: load_env_var("HEALTHY_USERS_UPDATE_FREQUENCY")?,
        })
    }
}
