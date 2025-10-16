use std::fs;

use crate::utils::{
    dir::DirUtils,
    errors::{ResultTrait as _, ResultWithError},
};

pub struct FlutterUtils;

impl FlutterUtils {
    pub fn get_name() -> ResultWithError<String> {
        let curr_dir = DirUtils::curr_dir()?;
        let parent_folder_name = curr_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let pubspec_path = curr_dir.join("pubspec.yaml");
        let content = fs::read_to_string(pubspec_path).auto_err("Could not read pubspec file")?;
        let pubspec: serde_yaml::Value =
            serde_yaml::from_str(&content).auto_err("Invalid config format")?;

        Ok(pubspec
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(parent_folder_name)
            .to_owned())
    }

    // Check if lib/main.dart contains main(List<String>
    pub fn has_main_with_args() -> ResultWithError<bool> {
        let curr_dir = DirUtils::curr_dir()?;
        let main_dart_path = curr_dir.join("lib").join("main.dart");
        if !main_dart_path.exists() {
            return Ok(false);
        }

        let content =
            fs::read_to_string(main_dart_path).auto_err("Could not read lib/main.dart file")?;
        Ok(content.contains("main(List<String>"))
    }
}
