use std::collections::HashMap;

use schemars::JsonSchema;
use serde::Deserialize;

use crate::utils::{
    dir::{DirUtils, YamlType},
    errors::ResultWithError,
};

#[allow(dead_code)]
#[derive(Debug, Deserialize, JsonSchema, Clone)]
pub struct FeatureTest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub before_each: Option<BeforeEach>,
    pub tests: Vec<TestCase>,
    #[serde(default)]
    pub vars: HashMap<String, String>,
    #[serde(default)]
    pub step_definitions: HashMap<String, Vec<Step>>,
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
pub struct TestCase {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub state: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
pub struct BeforeEach {
    #[serde(default)]
    pub steps: Vec<Step>,
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SimpleStep {
    Settle,
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(untagged)]
pub enum Step {
    WaitFor {
        wait_for: WaitFor,
    },
    NotFound {
        not_found: FindBy,
        timeout_millis: Option<u32>,
    },
    Tap {
        tap: FindBy,
    },
    Type {
        r#type: TypeAction,
    },
    Match {
        r#match: Match,
    },
    Scroll {
        scroll: ScrollTarget,
    },
    Pointer {
        pointer: PointerAction,
    },
    Use {
        use_step: String,
    },
    UserInput {
        user_input: String,
    },
    Simple(SimpleStep),
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ProgressWidgetType {
    Linear,
    Radial,
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(untagged)]
pub enum WaitFor {
    Key {
        key: String,
        #[serde(default)]
        timeout_millis: Option<u32>,
        #[serde(default)]
        settle: bool,
    },
    Text {
        text: String,
        #[serde(default)]
        timeout_millis: Option<u32>,
        #[serde(default)]
        settle: bool,
    },
    Delay {
        delay: u64,
        #[serde(default)]
        settle: bool,
    },
    Progress {
        progress: ProgressWidgetType,
        #[serde(default)]
        timeout_millis: Option<u32>,
        #[serde(default)]
        settle: bool,
    },
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]

pub struct ScrollTarget {
    pub by: FindBy,
    pub delta: Offset,
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(untagged)]
pub enum PointerAction {
    Move { to: Offset, remove: bool },
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
pub struct Offset {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(untagged)]
pub enum FindBy {
    Key { key: String },
    Text { text: String },
    Placeholder { placeholder: String },
    Type { r#type: String },
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
pub struct TypeAction {
    pub by: FindBy,
    pub value: String,
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
pub struct Match {
    #[serde(flatten)]
    pub target: MatchTarget,
}

#[derive(Debug, Deserialize, JsonSchema, Clone)]
#[serde(untagged)]
pub enum MatchTarget {
    Key { key: String },
    Text { text: String },
    Screenshot { screenshot: String },
}

impl FeatureTest {
    pub fn all_from_curr_dir() -> ResultWithError<Vec<Self>> {
        let res = DirUtils::parse_all_from_curr_dir::<Self>(YamlType::FeatureTest)?;
        Ok(res.into_iter().map(|f| f.content).collect::<Vec<_>>())
    }
}
