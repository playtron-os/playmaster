use std::{fs, path::PathBuf};

use serde::de::DeserializeOwned;

use crate::utils::errors::{ResultTrait, ResultWithError};

pub enum YamlType {
    FeatureTest,
    Vars,
}

pub struct YamlResult<T> {
    pub file_name: String,
    pub content: T,
}

pub struct DirUtils;

impl DirUtils {
    pub fn curr_dir() -> ResultWithError<std::path::PathBuf> {
        std::env::current_dir().auto_err("Could not read current directory")
    }

    pub fn parse_all_from_curr_dir<T>(yaml_type: YamlType) -> ResultWithError<Vec<YamlResult<T>>>
    where
        T: DeserializeOwned,
    {
        let config_path = DirUtils::curr_dir()?.join("feature_test");

        if !config_path.exists() {
            return Err("feature_test directory not found".into());
        }

        let res = Self::find_all_yaml(&config_path, yaml_type)?;
        Ok(res)
    }

    fn find_all_yaml<T>(
        config_path: &PathBuf,
        yaml_type: YamlType,
    ) -> ResultWithError<Vec<YamlResult<T>>>
    where
        T: DeserializeOwned,
    {
        let mut features = Vec::new();
        let ends_with = match yaml_type {
            YamlType::FeatureTest => vec![".test.yaml", ".test.yml"],
            YamlType::Vars => vec![".vars.yaml", ".vars.yml"],
        };

        // Iterate over all YAML files
        for entry in fs::read_dir(config_path).auto_err("Could not read directory")? {
            let entry = entry.auto_err("Could not read directory entry")?;
            let path = entry.path();
            let Some(file_name) = path.file_name() else {
                continue;
            };
            let file_name = file_name.to_string_lossy().to_string();

            // Only process .yaml or .yml files
            if ends_with.iter().any(|ending| file_name.ends_with(ending)) {
                let content = fs::read_to_string(&path)
                    .auto_err(&format!("Failed to read file: {:?}", path))?;

                let feature: T = serde_yaml::from_str(&content)
                    .auto_err(&format!("Failed to parse YAML: {:?}", path))?;

                features.push(YamlResult {
                    file_name,
                    content: feature,
                });
            }
        }

        Ok(features)
    }
}
