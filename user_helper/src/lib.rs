mod constant;
mod user_helper;

pub use user_helper::UserHelper;

#[derive(Debug, Clone)]
pub struct UserAccountData {
    // index 6 -> value / 1e18
    pub health_factor: f32,
    // index 0 -> value / 1e8
    pub collateral_value: f32,
    // index 1 -> value / 1e8
    pub debt_value: f32,
}

#[derive(Debug, Clone)]
pub struct ReserveAsset {
    pub address: String,
    pub amount_in_token: f32,
    pub amount_in_usd: f32,
    pub price: f32,
}

#[derive(Debug, Clone)]
pub struct UserReserveData {
    pub leading_collateral_reserve: String,
    pub leading_debt_reserve: String,
    pub collateral_assets: Vec<ReserveAsset>,
    pub debt_assets: Vec<ReserveAsset>,
}

impl Default for UserReserveData {
    fn default() -> Self {
        Self {
            leading_collateral_reserve: String::new(),
            leading_debt_reserve: String::new(),
            collateral_assets: Vec::new(),
            debt_assets: Vec::new(),
        }
    }
}
