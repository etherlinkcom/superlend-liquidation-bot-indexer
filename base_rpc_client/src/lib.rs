pub mod block_watcher;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::info;
#[derive(Debug, Serialize, Deserialize)]
struct EthRpcRequest {
    id: u64,
    jsonrpc: String,
    method: String,
    params: Vec<Value>,
}

impl EthRpcRequest {
    fn new(method: &str, params: Vec<Value>) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        }
    }
}

pub struct BaseRpcClient {
    client: Client,
    url: String,
    max_retries: u32,
    retry_delay: Duration,
}

impl BaseRpcClient {
    pub fn new(url: &str, max_retries: u32) -> Self {
        let client = Client::builder()
            .tcp_keepalive(Duration::from_secs(60))
            .pool_idle_timeout(None)
            .timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(32)
            .use_rustls_tls()
            .build()
            .unwrap();
        Self {
            client,
            url: url.to_string(),
            max_retries,
            retry_delay: Duration::from_millis(1000),
        }
    }

    fn sleep(duration: Duration) {
        std::thread::sleep(duration);
    }

    async fn make_request<T: for<'de> Deserialize<'de>>(
        &self,
        request: &impl Serialize,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let mut retries = 0;
        loop {
            match self.client.post(&self.url).json(request).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response.json().await?);
                    } else {
                        info!("Request failed with status: {}", response.status());
                    }
                }
                Err(e) => info!("Demo Request error: {:?}", e),
            }

            retries += 1;
            if retries >= self.max_retries {
                return Err("Max retries reached".into());
            }
            Self::sleep(self.retry_delay);
        }
    }

    pub async fn get_block_number(&self) -> Result<u64, Box<dyn std::error::Error>> {
        let request = EthRpcRequest::new("eth_blockNumber", vec![]);
        let response: Value = self.make_request(&request).await?;

        let block_number_hex = response["result"].as_str().ok_or("Invalid response")?;
        Ok(u64::from_str_radix(&block_number_hex[2..], 16)?)
    }

    pub async fn get_logs(
        &self,
        address: &str,
        start_block: &str,
        end_block: &str,
        topic: &str,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let request = EthRpcRequest::new(
            "eth_getLogs",
            vec![json!({
                "address": address,
                "fromBlock": start_block,
                "toBlock": end_block,
                "topics": [topic],
            })],
        );
        self.make_request(&request).await
    }

    pub async fn get_batch_requests_logs(
        &self,
        requests: Vec<(String, String, String, String)>,
    ) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let batch_requests: Vec<EthRpcRequest> = requests
            .into_iter()
            .map(|(address, start_block, end_block, topic)| {
                EthRpcRequest::new(
                    "eth_getLogs",
                    vec![json!({
                        "address": address,
                        "fromBlock": format!("0x{}", start_block),
                        "toBlock": format!("0x{}", end_block),
                        "topics": [topic],
                    })],
                )
            })
            .collect();

        self.make_request(&batch_requests).await
    }

    pub async fn eth_call(
        &self,
        from: &str,
        to: &str,
        function_selector: &str,
        params: Vec<String>,
        value: Option<String>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        // Encode the function selector and parameters into the data field
        let mut data = String::from(function_selector);
        for param in params {
            let padded_param = format!("{:0>64}", param.trim_start_matches("0x"));
            data.push_str(&padded_param);
        }

        // Create the JSON-RPC parameters
        let params = json!({
            "from": from,
            "to": to,
            "data": format!("0x{}", data),
            "value": value.unwrap_or_else(|| "0x0".to_string()),
        });

        // Build and send the JSON-RPC request
        let request = EthRpcRequest::new("eth_call", vec![params, json!("latest")]);
        self.make_request(&request).await
    }

    pub async fn eth_call_batch(
        &self,
        requests: Vec<(String, String, String, Vec<String>, Option<String>)>,
    ) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let mut batch_requests: Vec<EthRpcRequest> = Vec::new();
        for (from, to, function_selector, params, value) in requests {
            let mut data = String::from(function_selector);
            for param in params {
                let padded_param = format!("{:0>64}", param.trim_start_matches("0x"));
                data.push_str(&padded_param);
            }
            let params = json!({
                "from": from,
                "to": to,
                "data": format!("0x{}", data),
                "value": value.unwrap_or_else(|| "0x0".to_string()),
            });
            batch_requests.push(EthRpcRequest::new(
                "eth_call",
                vec![params, json!("latest")],
            ));
        }
        self.make_request(&batch_requests).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_get_block_number() {
        let client = Arc::new(BaseRpcClient::new("https://node.ghostnet.etherlink.com", 3));

        for _ in 0..10 {
            let start = std::time::Instant::now();
            let block_number = client.get_block_number().await.unwrap();
            println!("Time taken: {:?}", start.elapsed());
            println!("Block number: {}", block_number);
        }
    }

    #[tokio::test]
    async fn test_get_logs() {
        let client = Arc::new(BaseRpcClient::new("https://node.ghostnet.etherlink.com", 3));

        client.get_block_number().await.unwrap();
        let start = std::time::Instant::now();
        let logs = client
            .get_logs(
                "0xB0462c142FE3dEEDA33C6Dad2528C509A009136D",
                "0x47AF2C",
                "0x47B313",
                "0xb3d084820fb1a9decffb176436bd02558d15fac9b0ddfed8c465bc7359d7dce0",
            )
            .await
            .unwrap();
        println!("Time taken: {:?}", start.elapsed().as_millis());
        println!("Logs: {:?}", logs);
    }

    #[tokio::test]
    async fn test_get_batch_requests_logs() {
        let client = Arc::new(BaseRpcClient::new("https://node.ghostnet.etherlink.com", 3));

        let requests: Vec<(String, String, String, String)> = vec![
            (
                "0xB0462c142FE3dEEDA33C6Dad2528C509A009136D".to_string(),
                "0x47AF2C".to_string(),
                "0x47B313".to_string(),
                "0xb3d084820fb1a9decffb176436bd02558d15fac9b0ddfed8c465bc7359d7dce0".to_string(),
            ),
            (
                "0xB0462c142FE3dEEDA33C6Dad2528C509A009136D".to_string(),
                "0x47B313".to_string(),
                "0x47B6FA".to_string(),
                "0xb3d084820fb1a9decffb176436bd02558d15fac9b0ddfed8c465bc7359d7dce0".to_string(),
            ),
        ];
        let start = std::time::Instant::now();
        let logs = client.get_batch_requests_logs(requests).await.unwrap();
        println!("Time taken: {:?}", start.elapsed().as_millis());
        println!("Logs: {:?}", logs);
    }

    #[tokio::test]
    async fn test_eth_call() {
        let client = Arc::new(BaseRpcClient::new("https://node.ghostnet.etherlink.com", 3));
        let from_address = "0x469D7Fd0d97Bb8603B89228D79c7F037B2833859";
        let contract_address = "0x8DEF68408Bc96553003094180E5C90d9fe5b88C1";
        let function_selector = "70a08231"; // balanceOf(address)
        let account_address = "0x469D7Fd0d97Bb8603B89228D79c7F037B2833859"; // Address to query

        // Call balanceOf function
        let result = client
            .eth_call(
                from_address,
                contract_address,
                function_selector,
                vec![account_address.to_string()],
                None,
            )
            .await
            .unwrap();

        println!("Balance: {:?}", result);
    }
}
