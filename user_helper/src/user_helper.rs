use std::{collections::HashMap, sync::Arc};

use base_rpc_client::BaseRpcClient;

use ethers_types_rs::U256;
use tracing::{error, info};

use crate::{
    constant::{
        FUNCTION_SELECTOR_GET_ASSET_PRICES, FUNCTION_SELECTOR_GET_DECIMALS,
        FUNCTION_SELECTOR_GET_RESERVE_LIST, FUNCTION_SELECTOR_GET_USER_ACCOUNT_DATA,
        FUNCTION_SELECTOR_GET_USER_RESERVE_DATA_V2,
    },
    ReserveAsset, UserAccountData, UserReserveData,
};

pub struct UserHelperConfig {
    pool_address: String,
    pool_data_provider: String,
    price_oracle: String,
}

impl Default for UserHelperConfig {
    fn default() -> Self {
        let pool_address = std::env::var("POOL_ADDRESS").expect("POOL_ADDRESS not set");
        let pool_data_provider =
            std::env::var("POOL_DATA_PROVIDER").expect("POOL_DATA_PROVIDER not set");
        let price_oracle = std::env::var("PRICE_ORACLE").expect("PRICE_ORACLE not set");
        UserHelperConfig {
            pool_address,
            pool_data_provider,
            price_oracle,
        }
    }
}

pub struct UserHelper {
    rpc_client: Arc<BaseRpcClient>,
    config: Arc<UserHelperConfig>,
    // reserve address, reserve token decimals
    reserve_assets: Vec<(String, u8)>,
}

impl UserHelper {
    pub async fn new(rpc_client: Arc<BaseRpcClient>) -> Self {
        let config = Arc::new(UserHelperConfig::default());
        let mut user_helper = UserHelper {
            rpc_client,
            config,
            reserve_assets: vec![],
        };

        match user_helper.init_reserve_assets().await {
            Ok(_) => (),
            Err(e) => {
                error!("Failed to initialize reserve assets: {:?}", e);
                std::process::exit(1);
            }
        }

        user_helper
    }

    async fn init_reserve_assets(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let reserves = self
            .rpc_client
            .eth_call(
                "0x0000000000000000000000000000000000000000",
                &self.config.pool_address,
                FUNCTION_SELECTOR_GET_RESERVE_LIST,
                vec![],
                Some("0".to_string()),
            )
            .await?;
        // parse reserves
        let result = reserves.get("result").unwrap().as_str().unwrap();
        let addresses = Self::parse_reserve(result);

        for address in addresses {
            let decimals = self
                .rpc_client
                .eth_call(
                    "0x0000000000000000000000000000000000000000",
                    &address,
                    FUNCTION_SELECTOR_GET_DECIMALS,
                    vec![],
                    Some("0".to_string()),
                )
                .await?;
            let result = decimals.get("result").unwrap().as_str().unwrap();
            let decoded = hex::decode(&result[result.len() - 2..])?;
            let decimals = *decoded.last().unwrap();
            self.reserve_assets.push((address, decimals));
        }

        info!("Fetched reserve successfully: {:?}", self.reserve_assets);

        Ok(())
    }

    fn parse_reserve(data: &str) -> Vec<String> {
        let mut addresses = Vec::new();

        let data_bytes = hex::decode(&data[2..]).unwrap();
        let mut index = 0;

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

    pub async fn get_user_account_data(
        &self,
        user_address: &str,
    ) -> Result<UserAccountData, Box<dyn std::error::Error>> {
        let user_account_data = self
            .rpc_client
            .eth_call(
                user_address,
                &self.config.pool_address,
                FUNCTION_SELECTOR_GET_USER_ACCOUNT_DATA,
                vec![user_address.to_string()],
                None,
            )
            .await?;

        match user_account_data.get("result") {
            Some(result) => {
                let data = result.as_str().unwrap();
                let hex_data = hex::decode(&data[2..])?;

                let index_of_0th_item = 0;
                let index_of_1th_item = 32;
                let index_of_6th_item = 160;

                let health_factor = {
                    let hex_health_factor =
                        hex::encode(&hex_data[index_of_6th_item..index_of_6th_item + 32]);
                    let u256_health_factor = U256::from_str_radix(&hex_health_factor, 16)?;
                    let health_factor = Self::u256_to_f32(u256_health_factor, 18);
                    health_factor
                };

                let collateral_value = {
                    let hex_collateral_value =
                        hex::encode(&hex_data[index_of_0th_item..index_of_1th_item]);
                    let u256_collateral_value = U256::from_str_radix(&hex_collateral_value, 16)?;
                    let collateral_value = Self::u256_to_f32(u256_collateral_value, 8);
                    collateral_value
                };

                let debt_value = {
                    let hex_debt_value =
                        hex::encode(&hex_data[index_of_1th_item..index_of_1th_item + 32]);
                    let u256_debt_value = U256::from_str_radix(&hex_debt_value, 16)?;
                    let debt_value = Self::u256_to_f32(u256_debt_value, 8);
                    debt_value
                };

                Ok(UserAccountData {
                    health_factor,
                    collateral_value,
                    debt_value,
                })
            }
            None => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get health factor",
            ))),
        }
    }

    pub async fn get_user_reserve_data(
        &self,
        user_address: &str,
    ) -> Result<UserReserveData, Box<dyn std::error::Error>> {
        let mut user_reserve_data: UserReserveData = UserReserveData::default();

        let mut leading_collateral_reserve: (String, f32) = (String::new(), 0.0);
        let mut leading_debt_reserve: (String, f32) = (String::new(), 0.0);

        let prices = self
            .get_price_of_assets(
                self.reserve_assets
                    .iter()
                    .map(|(address, _)| address.clone())
                    .collect(),
            )
            .await?;

        for reserve in &self.reserve_assets {
            let (collateral_value, debt_value) = self
                .eth_call_user_reserve_data(user_address, reserve.clone())
                .await?;
            if collateral_value > leading_collateral_reserve.1 {
                leading_collateral_reserve = (reserve.0.to_string(), collateral_value);
            }
            if debt_value > leading_debt_reserve.1 {
                leading_debt_reserve = (reserve.0.to_string(), debt_value);
            }
            user_reserve_data.collateral_assets.push(ReserveAsset {
                address: reserve.0.to_string(),
                price: prices[&reserve.0.to_string()],
                amount_in_token: collateral_value,
                amount_in_usd: collateral_value * prices[&reserve.0.to_string()],
            });
            user_reserve_data.debt_assets.push(ReserveAsset {
                address: reserve.0.to_string(),
                price: prices[&reserve.0.to_string()],
                amount_in_token: debt_value,
                amount_in_usd: debt_value * prices[&reserve.0.to_string()],
            });
        }

        user_reserve_data.leading_collateral_reserve = leading_collateral_reserve.0;
        user_reserve_data.leading_debt_reserve = leading_debt_reserve.0;

        Ok(user_reserve_data)
    }

    fn u256_to_f32(value: U256, decimals: u32) -> f32 {
        let divisor = U256::from(10u64.pow(decimals));
        let whole = value / divisor;
        let fractional = value % divisor;

        let whole_f32 = if whole > U256::from(u32::MAX) {
            whole.to_string().parse::<f64>().unwrap_or(f64::MAX) as f32
        } else {
            whole.as_u32() as f32
        };

        let fractional_f32 = fractional.to_string().parse::<f64>().unwrap_or(0.0) as f32
            / 10f32.powi(decimals as i32);

        whole_f32 + fractional_f32
    }

    // return (collateral_value, debt_value)
    async fn eth_call_user_reserve_data(
        &self,
        user_address: &str,
        reserve: (String, u8),
    ) -> Result<(f32, f32), Box<dyn std::error::Error>> {
        let reserve_data = self
            .rpc_client
            .eth_call(
                "0x0000000000000000000000000000000000000000",
                &self.config.pool_data_provider,
                FUNCTION_SELECTOR_GET_USER_RESERVE_DATA_V2,
                vec![reserve.0.to_string(), user_address.to_string()],
                None,
            )
            .await?;

        let result = reserve_data["result"].as_str().ok_or("Invalid result")?;
        let hex_data = hex::decode(&result[2..])?;

        /*
        currentATokenBalance (uint256) : 5031089308269350995698
        currentStableDebt (uint256) : 0
        currentVariableDebt (uint256) : 16380506603747908864522
        principalStableDebt (uint256) : 0
        scaledVariableDebt (uint256) : 16266697933364319842649
        stableBorrowRate (uint256) : 0
        liquidityRate (uint256) : 2650411517108074245846296807
        stableRateLastUpdated (uint40) : 0
        usageAsCollateralEnabled (bool) : true
        */

        // 0..32
        let a_token_balance_index = 0;
        // 64..96
        let variable_debt_index = 64;

        let current_a_token_balance = {
            let hex_current_a_token_balance = hex::encode(&hex_data[a_token_balance_index..32]);
            let u256_current_a_token_balance =
                U256::from_str_radix(&hex_current_a_token_balance, 16)?;
            Self::u256_to_f32(u256_current_a_token_balance, reserve.1 as u32)
        };

        let current_variable_debt = {
            let hex_current_variable_debt =
                hex::encode(&hex_data[variable_debt_index..variable_debt_index + 32]);
            let u256_current_variable_debt = U256::from_str_radix(&hex_current_variable_debt, 16)?;
            Self::u256_to_f32(u256_current_variable_debt, reserve.1 as u32)
        };

        Ok((current_a_token_balance, current_variable_debt))
    }

    pub async fn get_price_of_assets(
        &self,
        asset_addresses: Vec<String>,
    ) -> Result<HashMap<String, f32>, Box<dyn std::error::Error>> {
        let requests: Vec<(String, String, String, Vec<String>, Option<String>)> = asset_addresses
            .iter()
            .map(|address| {
                (
                    "0x0000000000000000000000000000000000000000".to_string(),
                    self.config.price_oracle.clone(),
                    FUNCTION_SELECTOR_GET_ASSET_PRICES.to_string(),
                    vec![address.to_string()],
                    Some("0".to_string()),
                )
            })
            .collect();

        let prices = self.rpc_client.eth_call_batch(requests).await?;

        let mut reserve_assets: HashMap<String, f32> = HashMap::new();

        for i in 0..prices.len() {
            let result = prices[i].get("result").unwrap().as_str().unwrap();
            let u256_result = U256::from_str_radix(&result, 16)?;
            let f32_result = Self::u256_to_f32(u256_result, 8);
            reserve_assets.insert(asset_addresses[i].to_string(), f32_result);
        }

        Ok(reserve_assets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_user_account_data() {
        dotenv::dotenv().ok();

        let rpc_client = Arc::new(BaseRpcClient::new("https://node.ghostnet.etherlink.com", 5));
        let user_helper = UserHelper::new(rpc_client).await;
        let user_account_data = user_helper
            .get_user_account_data("0x2b112f430d725897a0b6f55a582fe122d21f4ef7")
            .await;
        let user_reserve_data = user_helper
            .get_user_reserve_data("0x2b112f430d725897a0b6f55a582fe122d21f4ef7")
            .await;
        println!("{:?}", user_account_data.unwrap());
        println!("{:?}", user_reserve_data.unwrap());
    }
}
