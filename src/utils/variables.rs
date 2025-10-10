use regex::Regex;
use std::env;

use crate::utils::errors::ResultWithError;

pub struct VariablesUtils {}

impl VariablesUtils {
    /// Expands ${VAR} or $VAR patterns using the current environment.
    pub fn expand_env_vars(input: &str) -> ResultWithError<String> {
        let re = Regex::new(r"\$\{([^}]+)\}|\$([A-Za-z0-9_]+)")?;
        Ok(re
            .replace_all(input, |caps: &regex::Captures| {
                // Capture either ${VAR} or $VAR
                let key = caps.get(1).or(caps.get(2)).unwrap().as_str();
                env::var(key).unwrap_or_else(|_| String::new())
            })
            .to_string())
    }
}
