use crate::thread_pool::ThreadPool;
use base_rpc_client::BaseRpcClient;
use database_manager::{
    handler::{
        last_index_block_handler::LastIndexBlockHandler,
        user_debt_collateral_table_handler::UserDebtCollateralTableHandler,
        user_table_handler::UserTableHandler,
    },
    DatabaseManager,
};
use std::sync::Arc;
use tracing::{error, info, warn};
use user_helper::UserHelper;

use crate::constant::BORROW_TOPIC;

// Configuration struct
pub struct IndexerConfig {
    pub pool_address: String,
    pub start_block: u64,
    pub max_blocks_per_request: u64,
    pub max_parallel_requests: u64,
    pub delay_between_requests: u64,
    pub wait_block_diff: u64,
    #[allow(dead_code)]
    pub cap_max_health_factor: u64,
    pub batch_size: u64,
}

// Main indexer struct
pub struct IndexerBorrowers {
    provider: Arc<BaseRpcClient>,
    db: Arc<DatabaseManager>,
    user_helper: Arc<UserHelper>,
    config: IndexerConfig,
    thread_pool: ThreadPool,
}

impl IndexerBorrowers {
    pub async fn new(
        provider: Arc<BaseRpcClient>,
        db: Arc<DatabaseManager>,
        config: IndexerConfig,
    ) -> Self {
        Self {
            provider: provider.clone(),
            db,
            config,
            user_helper: Arc::new(UserHelper::new(provider.clone()).await),
            thread_pool: ThreadPool::default(),
        }
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
                .await
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
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
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error + Send + Sync>> {
        // info!(
        //     "Starting fetch_logs - start_block: {}, block_number: {}, batch_count: {}",
        //     start_block, block_number, batch_count
        // );

        let mut results = Vec::new();
        let batch_size = self.config.batch_size;
        let blocks_per_request = self.config.max_blocks_per_request;
        let (tx, mut rx) = tokio::sync::mpsc::channel(batch_count as usize);

        info!(
            "Initialized with batch_size: {}, blocks_per_request: {}, total batches: {}, start_block: {}",
            batch_size,
            blocks_per_request,
            batch_count / batch_size,
            start_block
        );

        // Process batches in parallel using thread pool
        for batch_num in 0..(batch_count / batch_size) {
            let mut batch_requests = Vec::new();
            let mut current_start = *start_block;

            // info!("Processing batch {}", batch_num);

            for _ in 0..batch_size {
                let start_block_hex = format!("{:x}", current_start);
                let end_block = std::cmp::min(block_number, current_start + blocks_per_request);
                let end_block_hex = format!("{:x}", end_block);

                // info!(
                //     "Batch {}, Request {}: Blocks {} (0x{}) to {} (0x{})",
                //     batch_num, req_num, current_start, start_block_hex, end_block, end_block_hex
                // );

                batch_requests.push((
                    self.config.pool_address.clone(),
                    start_block_hex,
                    end_block_hex,
                    BORROW_TOPIC.to_string(),
                ));

                current_start = end_block;
            }

            *start_block = current_start;

            let provider = self.provider.clone();
            let tx = tx.clone();
            let batch_id = batch_num;

            // Convert the error type to ensure Send + Sync
            self.thread_pool
                .execute(async move {
                    // info!("Executing batch {} in thread pool", batch_id);
                    let result = provider
                        .get_batch_requests_logs(batch_requests)
                        .await
                        .map_err(|e| {
                            Box::<dyn std::error::Error + Send + Sync>::from(e.to_string())
                        });

                    match result {
                        Ok(batch_results) => {
                            // info!(
                            //     "Batch {} completed successfully with {} results",
                            //     batch_id,
                            //     batch_results.len()
                            // );
                            if let Err(e) = tx.send(batch_results).await {
                                error!("Failed to send batch {} results: {}", batch_id, e);
                            }
                        }
                        Err(e) => {
                            error!("Batch {} failed: {}", batch_id, e);
                            // Send an empty result to maintain batch count
                            let _ = tx.send(vec![]).await;
                        }
                    }
                })
                .await;
        }

        // Handle remaining requests if batch_count is not divisible by batch_size
        let remaining_requests = batch_count % batch_size;
        if remaining_requests > 0 {
            // info!("Processing {} remaining requests", remaining_requests);

            let mut batch_requests = Vec::new();
            let mut current_start = *start_block;

            for _ in 0..remaining_requests {
                let start_block_hex = format!("{:x}", current_start);
                let end_block = std::cmp::min(block_number, current_start + blocks_per_request);
                let end_block_hex = format!("{:x}", end_block);

                // info!(
                //     "Remaining Request {}: Blocks {} (0x{}) to {} (0x{})",
                //     req_num, current_start, start_block_hex, end_block, end_block_hex
                // );

                batch_requests.push((
                    self.config.pool_address.clone(),
                    start_block_hex,
                    end_block_hex,
                    BORROW_TOPIC.to_string(),
                ));

                current_start = end_block;
            }

            *start_block = current_start;

            let provider = self.provider.clone();
            let tx = tx.clone();

            self.thread_pool
                .execute(async move {
                    // info!("Executing remaining requests batch");
                    let result = provider
                        .get_batch_requests_logs(batch_requests)
                        .await
                        .map_err(|e| {
                            Box::<dyn std::error::Error + Send + Sync>::from(e.to_string())
                        });

                    match result {
                        Ok(batch_results) => {
                            // info!(
                            //     "Remaining batch completed successfully with {} results",
                            //     batch_results.len()
                            // );
                            let _ = tx.send(batch_results).await;
                        }
                        Err(e) => {
                            error!("Remaining batch failed: {}", e);
                            let _ = tx.send(vec![]).await;
                        }
                    }
                })
                .await;
        }

        // Drop the sender to close the channel
        drop(tx);
        // info!("Starting to collect results");

        // Collect all results
        let mut total_results = 0;
        while let Some(batch_results) = rx.recv().await {
            for result in batch_results {
                if result != serde_json::Value::Null {
                    results.push(result);
                    total_results += 1;
                }
            }
        }

        info!(
            "Fetch logs completed. Total valid results: {}, Final start_block: {}",
            total_results, start_block
        );

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

                    let user_data = match self
                        .user_helper
                        .get_user_account_data(address.as_str())
                        .await
                    {
                        Ok(user_data) => user_data,
                        Err(e) => {
                            error!("Failed to get user data: {}", e);
                            continue;
                        }
                    };

                    let user_reserve_data = match self
                        .user_helper
                        .get_user_reserve_data(address.as_str())
                        .await
                    {
                        Ok(user_reserve_data) => user_reserve_data,
                        Err(e) => {
                            error!("Failed to get user reserve data: {}", e);
                            continue;
                        }
                    };

                    let leading_collateral_reserve_value = user_reserve_data
                        .collateral_assets
                        .iter()
                        .find(|asset| asset.address == user_reserve_data.leading_collateral_reserve)
                        .map(|asset| asset.amount_in_token)
                        .unwrap_or(0.0);

                    let leading_debt_reserve_value = user_reserve_data
                        .debt_assets
                        .iter()
                        .find(|asset| asset.address == user_reserve_data.leading_debt_reserve)
                        .map(|asset| asset.amount_in_token)
                        .unwrap_or(0.0);

                    match self
                        .db
                        .insert_user(
                            address.as_str(),
                            block_number,
                            user_data.health_factor,
                            &user_reserve_data.leading_collateral_reserve,
                            &user_reserve_data.leading_debt_reserve,
                            user_data.collateral_value,
                            user_data.debt_value,
                            leading_collateral_reserve_value,
                            leading_debt_reserve_value,
                        )
                        .await
                    {
                        Ok(_) => {
                            info!(
                                "Stored new user: {} at block {} with health factor: {}",
                                address, block_number, user_data.health_factor
                            );
                        }
                        Err(e) => {
                            warn!("Failed to insert user: {}, reason: {}", address, e);
                        }
                    }

                    // insert user collateral
                    match self
                        .db
                        .insert_or_update_user_debt_collateral(
                            address.as_str(),
                            user_reserve_data
                                .collateral_assets
                                .into_iter()
                                .map(|asset| (asset.address, asset.amount_in_usd))
                                .collect::<Vec<(String, f32)>>(),
                            true,
                        )
                        .await
                    {
                        Ok(_) => {
                            info!(
                                "Stored new user collateral: {} at block {}",
                                address, block_number
                            );
                        }
                        Err(e) => {
                            error!("Failed to insert user debt: {}, reason: {}", address, e);
                        }
                    }

                    // insert user debt
                    match self
                        .db
                        .insert_or_update_user_debt_collateral(
                            address.as_str(),
                            user_reserve_data
                                .debt_assets
                                .into_iter()
                                .map(|asset| (asset.address, asset.amount_in_usd))
                                .collect::<Vec<(String, f32)>>(),
                            false,
                        )
                        .await
                    {
                        Ok(_) => {
                            info!(
                                "Stored new user debt: {} at block {}",
                                address, block_number
                            );
                        }
                        Err(e) => {
                            error!("Failed to insert user debt: {}, reason: {}", address, e);
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
