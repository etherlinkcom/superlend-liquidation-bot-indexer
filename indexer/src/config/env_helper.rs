use std::str::FromStr;

use anyhow::{Context, Result};

/// Load an environment variable and parse it to the given type
///
/// # Errors
///
/// Returns an error if the environment variable is not set or is not a valid value for the given type
pub fn load_env_var<T: FromStr>(var_name: &str) -> Result<T> {
    let var = std::env::var(var_name).context(format!("{} is not set", var_name))?;
    var.parse::<T>()
        .map_err(|_| anyhow::anyhow!("{} is not a valid {}", var_name, var))
}
