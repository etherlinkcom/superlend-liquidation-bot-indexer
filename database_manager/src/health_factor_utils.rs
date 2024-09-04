use std::env;

use lazy_static::lazy_static;

#[derive(Debug, Clone)]
pub struct HealthFactorRange {
    pub name: String,
    pub min_factor: f32,
    pub max_factor: f32,
    // wait time in seconds
    pub wait_time: u64,
}

impl HealthFactorRange {
    pub fn matches(&self, factor: f32) -> bool {
        factor >= self.min_factor && factor < self.max_factor
    }
}

lazy_static! {
    pub static ref HEALTH_FACTORS_RANGES: Vec<HealthFactorRange> = {
        let max_health_check_time: f32 = env::var("MAX_HEALTH_CHECK_TIME")
            .unwrap_or_else(|_| "7200".to_string())
            .parse()
            .expect("Invalid MAX_HEALTH_CHECK_TIME");

        let min_health_check_time: f32 = env::var("MIN_HEALTH_CHECK_TIME")
            .unwrap_or_else(|_| "300".to_string())
            .parse()
            .expect("Invalid MIN_HEALTH_CHECK_TIME");

        let cap_time_between_tables: f32 = env::var("CAP_TIME_BETWEEN_TABLES")
            .unwrap_or_else(|_| "1200".to_string())
            .parse()
            .expect("Invalid CAP_TIME_BETWEEN_TABLES");

        let starting_health_factor: f32 = env::var("STARTING_HEALTH_FACTOR")
            .unwrap_or_else(|_| "1.1".to_string())
            .parse()
            .expect("Invalid STARTING_HEALTH_FACTOR");

        let cap_max_health_factor: f32 = env::var("CAP_MAX_HEALTH_FACTOR")
            .unwrap_or_else(|_| "100".to_string())
            .parse()
            .expect("Invalid CAP_MAX_HEALTH_FACTOR");

        let total_time_range = max_health_check_time - min_health_check_time;
        let number_of_tables = (total_time_range / cap_time_between_tables).ceil() as usize;
        let health_factor_step = (cap_max_health_factor - starting_health_factor) / number_of_tables as f32;

        let mut ranges = Vec::new();

        for i in 0..number_of_tables {
            let min_factor = if i == 0 {
                0.0
            } else {
                starting_health_factor + health_factor_step * (i as f32 - 1.0)
            };

            let max_factor = starting_health_factor + health_factor_step * i as f32;
            let time_suffix = (min_health_check_time + (i as f32 * cap_time_between_tables)) / 60.0;

            let variant_name = format!("USER_{}", time_suffix as usize);

            ranges.push(HealthFactorRange {
                name: variant_name,
                min_factor,
                max_factor,
                // wait time in seconds
                wait_time: (time_suffix * 60.0) as u64,
            });
        }

        // Add the final range
        let final_min_factor = starting_health_factor + health_factor_step * number_of_tables as f32;
        let final_time_suffix = (min_health_check_time + (number_of_tables as f32 * cap_time_between_tables)) / 60.0;

        ranges.push(HealthFactorRange {
            name: format!("USER_{}", final_time_suffix as usize),
            min_factor: final_min_factor,
            max_factor: f32::INFINITY,
            wait_time: (final_time_suffix * 60.0) as u64,
        });

        ranges
    };
}

pub fn find_health_factor_variant(factor: f32) -> Option<&'static HealthFactorRange> {
    for range in HEALTH_FACTORS_RANGES.iter() {
        if range.matches(factor) {
            return Some(range);
        }
    }
    None
}

pub fn get_all_variants() -> Vec<String> {
    HEALTH_FACTORS_RANGES
        .iter()
        .map(|hfr| hfr.name.clone())
        .collect()
}

pub fn get_all_health_factor_ranges() -> Vec<HealthFactorRange> {
    HEALTH_FACTORS_RANGES.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_health_factor_variant() {
        dotenv::dotenv().ok();

        println!("HEALTH_FACTORS_RANGES: {:?}", HEALTH_FACTORS_RANGES.len());

        // Access the generated health factor ranges globally
        for range in HEALTH_FACTORS_RANGES.iter() {
            println!("{:?}", range);
        }

        // Example usage of the lookup function
        let test_factor = 20.0;
        match find_health_factor_variant(test_factor) {
            Some(variant) => println!("Health factor {} falls into {:?}", test_factor, variant),
            None => println!(
                "Health factor {} does not fall into any range.",
                test_factor
            ),
        }
    }
}
