use std::collections::HashMap;

use schemars::JsonSchema;
use serde::Deserialize;

use crate::utils::{
    dir::{DirUtils, YamlResult, YamlType},
    errors::ResultWithError,
};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Vars(pub HashMap<String, String>);

impl Vars {
    pub fn all_from_curr_dir() -> ResultWithError<Vec<YamlResult<Self>>> {
        let all_vars = DirUtils::parse_all_from_curr_dir::<Self>(YamlType::Vars)?;

        Ok(all_vars
            .into_iter()
            .filter(|v| !v.content.0.is_empty())
            .collect::<Vec<_>>())
    }
}
