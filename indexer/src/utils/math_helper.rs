use alloy::primitives::U256;

pub fn divide_by_precision_f64(value: U256, precision: u8) -> f64 {
    let ray = U256::from(10).pow(U256::from(precision));

    // Perform integer division and get both quotient and remainder
    let quotient = match value.checked_div(ray) {
        Some(q) => q,
        None => return f64::MAX,
    };

    let remainder = match value.checked_rem(ray) {
        Some(r) => r,
        None => return f64::MAX,
    };

    // Try to convert quotient to u128, return MAX if too large
    let quotient_u128 = match u128::try_from(quotient) {
        Ok(q) => q,
        Err(_) => return f64::MAX,
    };

    // Try to convert remainder to u128, return MAX if too large
    let remainder_u128 = match u128::try_from(remainder) {
        Ok(r) => r,
        Err(_) => return f64::MAX,
    };

    let ray_u128 = match u128::try_from(ray) {
        Ok(r) => r,
        Err(_) => return f64::MAX,
    };

    // Convert to f64 and combine
    let quotient_f64 = quotient_u128 as f64;
    let remainder_f64 = (remainder_u128 as f64) / (ray_u128 as f64);

    quotient_f64 + remainder_f64
}
