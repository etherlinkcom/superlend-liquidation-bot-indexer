use std::sync::Arc;

use base_rpc_client::BaseRpcClient;
use ethers_types_rs::U256;
use tracing::{error, info};

use super::IndexerUsersConfig;
pub async fn fetch_user_health_factor(
    user_address: &str,
    provider: Arc<BaseRpcClient>,
    config: Arc<IndexerUsersConfig>,
) -> Result<f32, String> {
    // let health_factor = provider.get_health_factor(user_adress).await?;
    let user_account_data = provider
        .eth_call(
            user_address,
            &config.pool_address,
            "bf92857c",
            vec![user_address.to_string()],
            None,
        )
        .await
        .map_err(|e| e.to_string())?;

    match user_account_data.get("result") {
        Some(result) => {
            let data = result.as_str().unwrap();
            let hex_data = hex::decode(&data[2..]).unwrap();
            let index_of_6th_item = 160;
            let hex_health_factor =
                hex::encode(&hex_data[index_of_6th_item..index_of_6th_item + 32]);

            let u256_health_factor =
                U256::from_str_radix(&hex_health_factor, 16).map_err(|e| e.to_string())?;
            let final_health_factor = if u256_health_factor > U256::from(u128::MAX) {
                config.cap_max_health_factor as f32
            } else {
                u256_health_factor.as_u128() as f32 / 1e18 as f32
            };

            Ok(final_health_factor)
        }
        None => {
            info!("No health factor found for user: {}", user_address);
            error!(
                "No result found in user_account_data: {}",
                user_account_data
            );
            Err(format!(
                "No result found in user_account_data: {}",
                user_account_data
            ))
        }
    }
}
