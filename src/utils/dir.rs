use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;
use tracing::debug;

use crate::{
    models::app_state::RemoteInfo,
    utils::errors::{ResultTrait, ResultWithError},
};

pub enum YamlType {
    FeatureTest,
    Vars,
}

#[derive(Debug)]
pub struct YamlResult<T> {
    pub file_name: String,
    pub content: T,
}

pub struct DirUtils;

impl DirUtils {
    pub fn curr_dir() -> ResultWithError<std::path::PathBuf> {
        std::env::current_dir().auto_err("Could not read current directory")
    }

    pub fn root_dir(remote: Option<&RemoteInfo>) -> ResultWithError<std::path::PathBuf> {
        let home = if let Some(remote) = remote {
            remote
                .exec("pwd")
                .map(|output| PathBuf::from(output.stdout.trim()))
        } else {
            dirs::home_dir().ok_or_else(|| "Could not determine home directory".into())
        };

        Ok(home?.join("playmaster"))
    }

    pub fn parse_all_from_curr_dir<T>(yaml_type: YamlType) -> ResultWithError<Vec<YamlResult<T>>>
    where
        T: DeserializeOwned,
    {
        let config_path = DirUtils::curr_dir()?.join("feature_test");

        if !config_path.exists() {
            return Err("feature_test directory not found".into());
        }

        debug!("Searching for YAML files in {:?}", config_path);
        let res = Self::find_all_yaml(&config_path, yaml_type)?;
        Ok(res)
    }

    fn find_all_yaml<T>(
        config_path: &Path,
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

        // Use an explicit stack to traverse directories recursively
        let mut dirs = vec![config_path.to_path_buf()];

        while let Some(dir) = dirs.pop() {
            for entry in fs::read_dir(&dir).auto_err("Could not read directory")? {
                let entry = entry.auto_err("Could not read directory entry")?;
                let path = entry.path();

                if path.is_dir() {
                    dirs.push(path);
                    continue;
                }

                let Some(file_name) = path.file_name() else {
                    continue;
                };
                let file_name = file_name.to_string_lossy().to_string();

                if ends_with.iter().any(|ending| file_name.ends_with(ending)) {
                    let Some(file_name) = file_name
                        .strip_suffix(".vars.yaml")
                        .or_else(|| file_name.strip_suffix(".vars.yml"))
                        .or_else(|| file_name.strip_suffix(".test.yaml"))
                        .or_else(|| file_name.strip_suffix(".test.yml"))
                        .or_else(|| file_name.strip_suffix(".yaml"))
                    else {
                        continue;
                    };

                    let content = fs::read_to_string(&path)
                        .auto_err(&format!("Failed to read file: {:?}", path))?;

                    let feature: T = serde_yaml::from_str(&content)
                        .auto_err(&format!("Failed to parse YAML: {:?}", path))?;

                    features.push(YamlResult {
                        file_name: file_name.to_string(),
                        content: feature,
                    });
                }
            }
        }

        Ok(features)
    }
}
