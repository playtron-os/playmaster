use std::{fs, path::PathBuf};

use serde::Deserialize;

use crate::utils::{
    dir::DirUtils,
    errors::{ResultTrait, ResultWithError},
};

#[derive(Debug, Deserialize)]
pub struct FeatureTest {
    pub name: String,
    pub description: String,
    pub tests: Vec<TestCase>,
}

#[derive(Debug, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub description: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Step {
    WaitFor { wait_for: WaitFor },
    Tap { tap: Tap },
    Type { r#type: TypeAction },
    Match { r#match: Match },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum WaitFor {
    Text { text: String },
    Delay { delay: u64 },
}

#[derive(Debug, Deserialize)]
pub struct Tap {
    #[serde(flatten)]
    pub target: Target,
}

#[derive(Debug, Deserialize)]
pub struct TypeAction {
    pub by: Target,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct Match {
    #[serde(flatten)]
    pub target: MatchTarget,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Target {
    Text { text: String },
    Placeholder { placeholder: String },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MatchTarget {
    Text { text: String },
    Screenshot { screenshot: String },
}

impl FeatureTest {
    pub fn all_from_curr_dir() -> ResultWithError<Vec<Self>> {
        let config_path = DirUtils::exec_dir()?.join("feature_test");

        if !config_path.exists() {
            return Err("test_features directory not found".into());
        }

        let feature_tests = Self::find_feature_tests(&config_path)?;
        Ok(feature_tests)
    }

    fn find_feature_tests(config_path: &PathBuf) -> ResultWithError<Vec<Self>> {
        let mut features = Vec::new();

        // Iterate over all YAML files
        for entry in fs::read_dir(config_path).auto_err("Could not read test_features directory")? {
            let entry = entry.auto_err("Could not read directory entry")?;
            let path = entry.path();

            // Only process .yaml or .yml files
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml") {
                    let content = fs::read_to_string(&path)
                        .auto_err(&format!("Failed to read file: {:?}", path))?;

                    let feature: FeatureTest = serde_yaml::from_str(&content)
                        .auto_err(&format!("Failed to parse YAML: {:?}", path))?;

                    features.push(feature);
                }
            }
        }

        Ok(features)
    }
}
