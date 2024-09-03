use base_rpc_client::BaseRpcClient;
use core::f32;
use database_manager::{
    handler::{
        last_index_block_handler::LastIndexBlockHandler, user_table_handler::UserTableHandler,
    },
    DatabaseManager,
};
use ethers_types_rs::U256;
use std::sync::Arc;
use tracing::{error, info};

use crate::constant::BORROW_TOPIC;

// Configuration struct
pub struct IndexerConfig {
    pub pool_address: String,
    pub start_block: u64,
    pub max_blocks_per_request: u64,
    pub max_parallel_requests: u64,
    pub delay_between_requests: u64,
    pub wait_block_diff: u64,
    pub cap_max_health_factor: u64,
}

// Main indexer struct
pub struct IndexerBorrowers {
    provider: Arc<BaseRpcClient>,
    db: Arc<DatabaseManager>,
    config: IndexerConfig,
    current_reserve: Option<Vec<String>>,
}

impl IndexerBorrowers {
    pub async fn new(
        provider: Arc<BaseRpcClient>,
        db: Arc<DatabaseManager>,
        config: IndexerConfig,
    ) -> Self {
        let mut indexer = Self {
            provider,
            db,
            config,
            current_reserve: None,
        };

        match indexer.init_reserve().await {
            Ok(_) => (),
            Err(e) => {
                error!("Failed to initialize reserve: {}", e);
                std::process::exit(1);
            }
        }

        indexer
    }

    // eth call getReservesList (d1946dbc)
    async fn init_reserve(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let reserves = self
            .provider
            .eth_call(
                "0x0000000000000000000000000000000000000000",
                &self.config.pool_address,
                "d1946dbc",
                vec![],
                Some("0".to_string()),
            )
            .await?;
        // parse reserves
        let result = reserves.get("result").unwrap().as_str().unwrap();
        let addresses = Self::parse_reserve(result);
        self.current_reserve = Some(addresses);

        info!(
            "Fetched reserve successfully: {:?}",
            self.current_reserve.as_ref().unwrap()
        );

        Ok(())
    }

    fn parse_reserve(data: &str) -> Vec<String> {
        let mut addresses = Vec::new();

        let data_bytes = hex::decode(&data[2..]).unwrap();
        let mut index = 0;

        // let bytes_gap =
        //     u64::from_str_radix(&hex::encode(&data_bytes[index..index + 32]), 16).unwrap();
        // first 32 is bytes length each item in array
        index += 32;

        let array_length =
            u64::from_str_radix(&hex::encode(&data_bytes[index..index + 32]), 16).unwrap();
        index += 32;

        for _ in 0..array_length {
            let address = hex::encode(&data_bytes[index + 12..index + 32]);
            addresses.push(format!("0x{}", address));
            index += 32;
        }

        addresses
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut block_number = self.provider.get_block_number().await?;
        let mut start_block = self.config.start_block;

        info!(
            "Index start block number: {}, Current block number: {}, Block diff: {}",
            start_block,
            block_number,
            block_number - start_block
        );

        loop {
            let diff = block_number - start_block;

            if diff < self.config.wait_block_diff {
                self.block_till_diff(start_block, self.config.wait_block_diff)
                    .await?;
                block_number = self.provider.get_block_number().await?;
                continue;
            }

            let batch_count = if diff < 999 {
                1
            } else {
                self.config.max_parallel_requests
            };
            let logs = self
                .fetch_logs(&mut start_block, block_number, batch_count)
                .await?;
            self.process_logs(logs).await?;

            // start_block = std::cmp::min(
            //     block_number,
            //     start_block + self.config.max_blocks_per_request * batch_count,
            // );
            self.db.update_last_index_block(start_block).await?;

            block_number = self.provider.get_block_number().await?;

            self.log_progress(start_block, block_number, diff, batch_count);

            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.config.delay_between_requests,
            ))
            .await;
        }
    }

    async fn fetch_logs(
        &self,
        start_block: &mut u64,
        block_number: u64,
        batch_count: u64,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        let mut tasks = Vec::new();

        for _ in 0..batch_count {
            let start_block_clone = *start_block;
            let end_block = std::cmp::min(
                block_number,
                start_block_clone + self.config.max_blocks_per_request,
            );
            let provider = self.provider.clone();
            let pool_address = self.config.pool_address.clone();
            // info!("Fetching logs from block {} to {}", start_block_clone, end_block);
            tasks.push(tokio::spawn(async move {
                match provider
                    .get_logs(
                        &pool_address,
                        start_block_clone.to_string().as_str(),
                        &end_block.to_string(),
                        &BORROW_TOPIC.to_string(),
                    )
                    .await
                {
                    Ok(log) => log,
                    Err(e) => {
                        error!("Failed to get logs: {}", e);
                        serde_json::Value::Null
                    }
                }
            }));
            *start_block = end_block;
        }

        let results = futures::future::join_all(tasks).await;
        let results = results
            .into_iter()
            .filter_map(|result| match result {
                Ok(log) => Some(log),
                Err(e) => {
                    error!("Task join error: {}", e);
                    None
                }
            })
            .collect::<Vec<serde_json::Value>>();

        Ok(results)
    }

    async fn process_logs<'a>(
        &self,
        logs: Vec<serde_json::Value>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for log in logs {
            if let Some(result) = log.get("result").and_then(|r| r.as_array()) {
                if !result.is_empty() {
                    let data = &result[0]["topics"];
                    let address = self.extract_address(data)?;
                    let block_number = self.extract_block_number(&result[0])?;

                    // user_manager
                    //     .add_user(address.as_str(), block_number)
                    //     .await?;

                    let health_factor = self.get_health_factor(&address).await;

                    if health_factor.is_err() {
                        error!(
                            "Failed to get health factor: {} for user: {}",
                            health_factor.err().unwrap(),
                            address
                        );
                    } else {
                        let health_factor = health_factor.unwrap();

                        match self
                            .db
                            .insert_user(address.as_str(), block_number, health_factor)
                            .await
                        {
                            Ok(_) => {
                                info!(
                                    "Stored new user: {} at block {} with health factor: {}",
                                    address, block_number, health_factor
                                );
                            }
                            Err(e) => {
                                error!("Failed to insert user: {}, reason: {}", address, e);
                            }
                        }
                    }
                }
            } else {
                error!("No result found in log: {}", log);
            }
        }
        Ok(())
    }

    fn extract_address(
        &self,
        data: &serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let address = &data[2].as_str().ok_or("Failed to get address as string")?[2..];
        let address = hex::decode(address)?;
        Ok(format!("0x{}", hex::encode(&address[address.len() - 20..])))
    }

    fn extract_block_number(
        &self,
        log: &serde_json::Value,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let block_number = log["blockNumber"]
            .as_str()
            .ok_or("Failed to get block number as string")?;
        Ok(u64::from_str_radix(&block_number[2..], 16)?)
    }

    fn log_progress(&self, start_block: u64, block_number: u64, diff: u64, batch_count: u64) {
        info!(
            "Processed {} blocks, index block: {}, block number: {}, In Sync: {:.2}%",
            std::cmp::min(self.config.max_blocks_per_request * batch_count, diff),
            start_block,
            block_number,
            ((start_block as f64 - self.config.start_block as f64)
                / (block_number as f64 - self.config.start_block as f64))
                * 100.0
        );
    }

    async fn block_till_diff(
        &self,
        start_block: u64,
        diff: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let block_number = self.provider.get_block_number().await?;
            if block_number - start_block >= diff {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(diff / 2)).await;
        }
        Ok(())
    }

    async fn get_health_factor(
        &self,
        user_address: &str,
    ) -> Result<f32, Box<dyn std::error::Error>> {
        // Eth call getUserAccountData (bf92857c)
        let user_account_data = self
            .provider
            .eth_call(
                user_address,
                &self.config.pool_address,
                "bf92857c",
                vec![user_address.to_string()],
                None,
            )
            .await?;

        match user_account_data.get("result") {
            Some(result) => {
                let data = result.as_str().unwrap();
                let hex_data = hex::decode(&data[2..]).unwrap();
                let index_of_6th_item = 160;
                let hex_health_factor =
                    hex::encode(&hex_data[index_of_6th_item..index_of_6th_item + 32]);

                let u256_health_factor = U256::from_str_radix(&hex_health_factor, 16)?;
                let final_health_factor = if u256_health_factor > U256::from(u128::MAX) {
                    self.config.cap_max_health_factor as f32
                } else {
                    u256_health_factor.as_u128() as f32 / 1e18 as f32
                };

                info!(
                    "Retrieved health factor: {} for user: {}",
                    final_health_factor, user_address
                );

                Ok(final_health_factor)
            }
            None => {
                info!("No health factor found for user: {}", user_address);
                error!(
                    "No result found in user_account_data: {}",
                    user_account_data
                );
                Ok(0.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_parse_reserve() {
        let data = "0x000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000050000000000000000000000008def68408bc96553003094180e5c90d9fe5b88c1000000000000000000000000b1ea698633d57705e93b0e40c1077d46cd6a51d8000000000000000000000000d21b917d2f4a4a8e3d12892160bffd8f4cd72d4f000000000000000000000000a7c9092a5d2c3663b7c5f714dba806d02d62b58a0000000000000000000000006bde94725379334b469449f4cf49bcfc85ebfb27";
        let data_bytes = hex::decode(&data[2..]).unwrap();
        println!("data_bytes: {:?}", data_bytes.len());

        let mut index = 0;

        // take first 64 bytes and parse into u64
        let bytes_gap =
            u64::from_str_radix(&hex::encode(&data_bytes[index..index + 32]), 16).unwrap();
        index += 32;
        println!("bytes_gap: {:?}", bytes_gap);

        // take next 32 bytes and parse into u64
        let array_length =
            u64::from_str_radix(&hex::encode(&data_bytes[index..index + 32]), 16).unwrap();
        index += 32;
        println!("array_length: {:?}", array_length);

        for _ in 0..array_length {
            // take next 32 bytes and parse last 20 bytes into address
            let address = hex::encode(&data_bytes[index + 12..index + 32]);
            index += 32;
            println!("address: {:?}", address);
        }
    }
}
