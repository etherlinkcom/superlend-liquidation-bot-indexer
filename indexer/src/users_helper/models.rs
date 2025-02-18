#[derive(Debug, Clone)]
pub struct UserReserveData {
    pub leading_collateral_reserve: String,
    pub leading_debt_reserve: String,
    pub leading_collateral_reserve_token_value: f32,
    pub leading_debt_reserve_token_value: f32,
    pub collateral_assets: Vec<(String, f32)>,
    pub debt_assets: Vec<(String, f32)>,
}

impl UserReserveData {
    pub fn new(collateral_assets: Vec<(String, f32)>, debt_assets: Vec<(String, f32)>) -> Self {
        let mut leading_collateral_reserve = String::new();
        let mut leading_debt_reserve = String::new();
        let mut leading_collateral_reserve_token_value = 0.0;
        let mut leading_debt_reserve_token_value = 0.0;

        for (address, value) in collateral_assets.iter() {
            if value > &leading_collateral_reserve_token_value {
                leading_collateral_reserve = address.clone();
                leading_collateral_reserve_token_value = *value;
            }
        }

        for (address, value) in debt_assets.iter() {
            if value > &leading_debt_reserve_token_value {
                leading_debt_reserve = address.clone();
                leading_debt_reserve_token_value = *value;
            }
        }

        Self {
            leading_collateral_reserve,
            leading_debt_reserve,
            leading_collateral_reserve_token_value,
            leading_debt_reserve_token_value,
            collateral_assets,
            debt_assets,
        }
    }
}
