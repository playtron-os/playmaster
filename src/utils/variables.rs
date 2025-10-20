use regex::Regex;
use std::{collections::HashMap, env};

use crate::utils::string::StringUtils;

lazy_static::lazy_static! {
    static ref VERSION_RE: Regex =
        Regex::new(r"\{\{\s*([^}]+?)\s*\}\}").expect("Failed to compile regex");

        static ref ENV_VAR_RE: Regex = Regex::new(r"\$\{([A-Za-z0-9_]+)\}|\$([A-Za-z_][A-Za-z0-9_]*)").expect("Failed to compile regex");
}

pub struct VariablesUtils {}

impl VariablesUtils {
    /// Expands ${VAR} or $VAR patterns using the current environment.
    pub fn expand_env_vars(input: &str) -> String {
        ENV_VAR_RE
            .replace_all(input, |caps: &regex::Captures| {
                // Capture either ${VAR} or $VAR
                let key = caps.get(1).or(caps.get(2)).unwrap().as_str();
                env::var(key).unwrap_or_else(|_| String::new())
            })
            .to_string()
    }

    pub fn replace_var_usage(input: &str) -> String {
        VERSION_RE
            .replace_all(input, |caps: &regex::Captures| {
                let Some(cap) = caps.get(1) else {
                    return "".to_owned();
                };

                let key = cap.as_str().trim();
                format!(
                    "${{{}}}",
                    StringUtils::to_pascal_case_with_dots(&key.replace("vars.", ""))
                )
            })
            .to_string()
    }

    pub fn replace_vars(
        input: &str,
        vars: &HashMap<String, String>,
        extra_map: Option<&HashMap<String, String>>,
    ) -> String {
        VERSION_RE
            .replace_all(input, |caps: &regex::Captures| {
                let Some(cap) = caps.get(1) else {
                    return "".to_owned();
                };

                let key = cap.as_str().trim();

                if let Some(extra) = extra_map
                    && let Some(val) = extra.get(key)
                {
                    return val.clone();
                }

                vars.get(key)
                    .cloned()
                    .unwrap_or_else(|| format!("${{{{{}}}}}", key))
            })
            .to_string()
    }
}
