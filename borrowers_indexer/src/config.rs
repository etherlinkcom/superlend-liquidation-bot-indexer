use std::env;

pub struct Config {
    pub rpc_url: String,
    pub max_retries: u32,
    pub pool_address: String,
    pub start_block: u64,
    pub max_blocks_per_request: u64,
    pub max_parallel_requests: u64,
    pub delay_between_requests: u64,
    pub wait_block_diff: u64,
    pub cap_max_health_factor: u64,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv::dotenv().ok();

        Ok(Self {
            rpc_url: env::var("RPC_URL")?,
            max_retries: 5,
            pool_address: env::var("POOL_ADDRESS")?,
            start_block: env::var("START_BLOCK")?.parse()?,
            max_blocks_per_request: env::var("MAX_BLOCKS_PER_REQUEST_LOG")?.parse()?,
            max_parallel_requests: env::var("MAX_PARALLEL_REQUESTS")?.parse()?,
            delay_between_requests: env::var("DELAY_BETWEEN_REQUESTS")?.parse()?,
            wait_block_diff: env::var("WAIT_BLOCK_DIFF_LOG_REFRESH")?.parse()?,
            cap_max_health_factor: env::var("CAP_MAX_HEALTH_FACTOR")?.parse()?,
        })
    }
}
