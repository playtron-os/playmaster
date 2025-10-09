use std::collections::HashMap;

use schemars::JsonSchema;
use serde::Deserialize;

use crate::utils::{
    dir::{DirUtils, YamlType},
    errors::ResultWithError,
};

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FeatureTest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub tests: Vec<TestCase>,
    #[serde(default)]
    pub vars: HashMap<String, String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestCase {
    pub name: String,
    pub description: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Step {
    WaitFor { wait_for: WaitFor },
    Tap { tap: Tap },
    Type { r#type: TypeAction },
    Match { r#match: Match },
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProgressWidgetType {
    Linear,
    Radial,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum WaitFor {
    Text { text: String },
    Delay { delay: u64 },
    Progress { progress: ProgressWidgetType },
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Tap {
    #[serde(flatten)]
    pub target: Target,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TypeAction {
    pub by: Target,
    pub value: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Match {
    #[serde(flatten)]
    pub target: MatchTarget,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Target {
    Text { text: String },
    Placeholder { placeholder: String },
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum MatchTarget {
    Text { text: String },
    Screenshot { screenshot: String },
}

impl FeatureTest {
    pub fn all_from_curr_dir() -> ResultWithError<Vec<Self>> {
        let res = DirUtils::parse_all_from_curr_dir::<Self>(YamlType::FeatureTest)?;
        Ok(res.into_iter().map(|f| f.content).collect::<Vec<_>>())
    }
}
