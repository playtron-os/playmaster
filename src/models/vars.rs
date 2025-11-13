use std::collections::HashMap;

use schemars::JsonSchema;
use serde::Deserialize;

use crate::utils::{
    dir::{DirUtils, YamlResult, YamlType},
    errors::ResultWithError,
};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VarsData(pub HashMap<String, String>);

#[derive(Debug)]
#[allow(dead_code)]
pub struct Vars {
    pub data: Vec<YamlResult<VarsData>>,
    pub all_vars: HashMap<String, String>,
}

impl Vars {
    pub fn all_from_curr_dir() -> ResultWithError<Vars> {
        let all_vars = DirUtils::parse_all_from_curr_dir::<VarsData>(YamlType::Vars)?;
        let data = all_vars
            .into_iter()
            .filter(|v| !v.content.0.is_empty())
            .collect::<Vec<_>>();
        let mut all_vars = HashMap::new();

        for var in data.iter() {
            for (key, value) in &var.content.0 {
                let var_key = format!("vars.{}.{}", var.file_name, key);
                all_vars.insert(var_key, value.clone());
            }
        }

        Ok(Vars { data, all_vars })
    }

    pub fn replace_var_usage(&self, input: &str) -> String {
        crate::utils::variables::VariablesUtils::replace_var_usage(input)
    }

    pub fn replace_var(&self, input: &str, extra_map: Option<&HashMap<String, String>>) -> String {
        crate::utils::variables::VariablesUtils::replace_vars(input, &self.all_vars, extra_map)
    }
}
